use crate::infra::errors::YntraError;
use super::stores::{ZeroCopyStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyAuditStore};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

// --- Pillar 2: Geo-Distributed Edge Replicas + P2P Mesh Sync ---

struct PeerRelayQueue {
    updates: Vec<Vec<u8>>,
    catchup_doc: Option<loro::LoroDoc>,
}

impl Default for PeerRelayQueue {
    fn default() -> Self {
        Self {
            updates: Vec::new(),
            catchup_doc: None,
        }
    }
}

static IN_MEMORY_RELAY: LazyLock<Mutex<HashMap<String, PeerRelayQueue>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn in_memory_broadcast(from_peer: &str, data: Vec<u8>, peers: &[String]) {
    let mut relay = IN_MEMORY_RELAY.lock().unwrap();
    for peer in peers {
        if peer != from_peer {
            let q = relay.entry(peer.clone()).or_default();
            if q.updates.len() >= 100 {
                let oldest = q.updates.remove(0);
                let doc = q.catchup_doc.get_or_insert_with(loro::LoroDoc::new);
                let _ = doc.import(&oldest);
            }
            q.updates.push(data.clone());
        }
    }
}

pub(crate) fn in_memory_poll(peer_id: &str) -> Vec<Vec<u8>> {
    let mut relay = IN_MEMORY_RELAY.lock().unwrap();
    if let Some(q) = relay.remove(peer_id) {
        if let Some(doc) = q.catchup_doc {
            if let Ok(snapshot) = doc.export(loro::ExportMode::Snapshot) {
                let mut all = vec![snapshot];
                all.extend(q.updates);
                return all;
            }
        }
        q.updates
    } else {
        Vec::new()
    }
}

async fn do_register_peer(
    client: reqwest::Client,
    relay_url: String,
    peer_id: String,
    signing_key: Option<ed25519_dalek::SigningKey>,
) {
    let mut body = serde_json::json!({ "peer_id": peer_id });
    if let Some(ref key) = signing_key {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let msg = format!("register:{}:{}", peer_id, timestamp);
        use ed25519_dalek::Signer;
        let signature = key.sign(msg.as_bytes());
        body = serde_json::json!({
            "peer_id": peer_id,
            "timestamp": timestamp,
            "signature_hex": const_hex::encode(signature.to_bytes()),
        });
    }

    let mut success = false;
    for attempt in 1..=3 {
        match client
            .post(&format!("{}/relay/register", relay_url))
            .json(&body)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                success = true;
                break;
            }
            Ok(res) => {
                tracing::warn!(
                    "Relay peer registration attempt {} failed with status: {}",
                    attempt,
                    res.status()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Relay peer registration attempt {} failed with connection error: {:?}",
                    attempt,
                    e
                );
            }
        }
        crate::infra::time::sleep_ms(500 * attempt).await;
    }
    if !success {
        tracing::error!(
            "Failed to register peer {} with relay after multiple attempts",
            peer_id
        );
    }
}

async fn do_broadcast_write(
    client: reqwest::Client,
    relay_url: String,
    from_peer: String,
    data: Vec<u8>,
    signing_key: Option<ed25519_dalek::SigningKey>,
    failed_queue: Option<Arc<Mutex<Vec<(String, Vec<u8>)>>>>,
) {
    let data_hex = const_hex::encode(&data);
    let mut body = serde_json::json!({ "from_peer": from_peer, "data_hex": data_hex });
    if let Some(ref key) = signing_key {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let mut msg = Vec::new();
        msg.extend_from_slice(b"broadcast:");
        msg.extend_from_slice(from_peer.as_bytes());
        msg.extend_from_slice(b":");
        msg.extend_from_slice(&timestamp.to_be_bytes());
        msg.extend_from_slice(b":");
        msg.extend_from_slice(&data);

        use ed25519_dalek::Signer;
        let signature = key.sign(&msg);
        body = serde_json::json!({
            "from_peer": from_peer,
            "data_hex": data_hex,
            "timestamp": timestamp,
            "signature_hex": const_hex::encode(signature.to_bytes()),
        });
    }

    let mut success = false;
    for attempt in 1..=3 {
        match client
            .post(&format!("{}/relay/broadcast", relay_url))
            .json(&body)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                success = true;
                break;
            }
            Ok(res) => {
                tracing::warn!(
                    "Relay broadcast attempt {} failed with status: {}",
                    attempt,
                    res.status()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Relay broadcast attempt {} failed with connection error: {:?}",
                    attempt,
                    e
                );
            }
        }
        crate::infra::time::sleep_ms(500 * attempt).await;
    }
    if !success {
        tracing::error!(
            "Failed to broadcast update from peer {} to relay after multiple attempts",
            from_peer
        );
        if let Some(queue) = failed_queue {
            queue.lock().unwrap().push((from_peer, data));
        }
    }
}

#[derive(Clone, uniffi::Object)]
pub struct P2PMeshSyncRouter {
    peers: Arc<Mutex<Vec<String>>>,
    pending_broadcasts: Arc<Mutex<Vec<Vec<u8>>>>,
    pub(crate) failed_broadcasts: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    relay_url: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
    signing_key: Arc<Mutex<Option<ed25519_dalek::SigningKey>>>,
}

#[uniffi::export]
impl P2PMeshSyncRouter {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            pending_broadcasts: Arc::new(Mutex::new(Vec::new())),
            failed_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(None)),
            client: reqwest::Client::new(),
            signing_key: Arc::new(Mutex::new(None)),
        }
    }

    #[uniffi::constructor]
    pub fn with_relay(relay_url: String) -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            pending_broadcasts: Arc::new(Mutex::new(Vec::new())),
            failed_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(Some(relay_url))),
            client: reqwest::Client::new(),
            signing_key: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_identity(&self, private_key_hex: String) -> Result<(), YntraError> {
        let key_bytes = const_hex::decode(&private_key_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        if key_bytes.len() != 32 {
            return Err(YntraError::CryptoError("Invalid private key length".to_string()));
        }
        let key_array: [u8; 32] = key_bytes.try_into().unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&key_array);
        let mut guard = self.signing_key.lock().unwrap();
        *guard = Some(key);
        Ok(())
    }

    pub fn set_ephemeral_identity(&self) -> Result<String, YntraError> {
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let key = ed25519_dalek::SigningKey::from_bytes(&entropy);
        let pubkey_hex = const_hex::encode(key.verifying_key().to_bytes());
        let mut guard = self.signing_key.lock().unwrap();
        *guard = Some(key);
        Ok(pubkey_hex)
    }

    pub fn register_peer(&self, peer_id: String) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains(&peer_id) {
            peers.push(peer_id);
        }
    }

    pub fn register_peer_network(&self, peer_id: String) {
        self.register_peer(peer_id.clone());
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let client = self.client.clone();
            let key = self.signing_key.lock().unwrap().clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(do_register_peer(client, relay_url, peer_id, key));
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(do_register_peer(client, relay_url, peer_id, key));
            }
        }
    }

    pub fn broadcast_write(&self, data: Vec<u8>) -> Result<(), YntraError> {
        let mut pending = self.pending_broadcasts.lock().unwrap();
        pending.push(data);
        Ok(())
    }

    pub fn retry_failed_broadcasts(&self) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let mut failed = self.failed_broadcasts.lock().unwrap();
            if failed.is_empty() {
                return;
            }
            let to_retry = std::mem::take(&mut *failed);
            tracing::info!("Retrying {} failed P2P broadcasts...", to_retry.len());
            for (from_peer, data) in to_retry {
                let client = self.client.clone();
                let key = self.signing_key.lock().unwrap().clone();
                let queue_clone = Some(self.failed_broadcasts.clone());
                #[cfg(not(target_arch = "wasm32"))]
                {
                    crate::database::native::get_runtime().spawn(do_broadcast_write(
                        client,
                        relay_url.clone(),
                        from_peer,
                        data,
                        key,
                        queue_clone,
                    ));
                }
                #[cfg(target_arch = "wasm32")]
                {
                    wasm_bindgen_futures::spawn_local(do_broadcast_write(
                        client,
                        relay_url.clone(),
                        from_peer,
                        data,
                        key,
                        queue_clone,
                    ));
                }
            }
        }
    }

    pub fn broadcast_write_network(&self, from_peer: String, data: Vec<u8>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        let peers = self.peers.lock().unwrap().clone();

        // Always store in-memory fallback
        in_memory_broadcast(&from_peer, data.clone(), &peers);

        // Retry any previously failed broadcasts before trying the new one
        self.retry_failed_broadcasts();

        if let Some(relay_url) = relay_opt {
            let client = self.client.clone();
            let key = self.signing_key.lock().unwrap().clone();
            let queue_clone = Some(self.failed_broadcasts.clone());
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(do_broadcast_write(
                    client,
                    relay_url,
                    from_peer,
                    data,
                    key,
                    queue_clone,
                ));
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(do_broadcast_write(
                    client,
                    relay_url,
                    from_peer,
                    data,
                    key,
                    queue_clone,
                ));
            }
        } else {
            let _ = self.broadcast_write(data);
        }
    }

    pub fn receive_update(
        &self,
        from_peer: String,
        update: Vec<u8>,
        store: Arc<ZeroCopyStore>,
    ) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P update from peer: {}", from_peer);
        Ok(())
    }

    pub fn receive_message_update(
        &self,
        from_peer: String,
        update: Vec<u8>,
        store: Arc<ZeroCopyMessageStore>,
    ) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P message update from peer: {}", from_peer);
        Ok(())
    }

    pub fn receive_note_update(
        &self,
        from_peer: String,
        update: Vec<u8>,
        store: Arc<ZeroCopyNoteStore>,
    ) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P note update from peer: {}", from_peer);
        Ok(())
    }

    pub fn receive_audit_update(
        &self,
        from_peer: String,
        update: Vec<u8>,
        store: Arc<ZeroCopyAuditStore>,
    ) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P audit update from peer: {}", from_peer);
        Ok(())
    }

    pub fn trigger_poll_relay_updates(&self, peer_id: String, store: Arc<ZeroCopyStore>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
        }
    }

    pub fn trigger_poll_relay_message_updates(
        &self,
        peer_id: String,
        store: Arc<ZeroCopyMessageStore>,
    ) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
        }
    }

    pub fn trigger_poll_relay_note_updates(&self, peer_id: String, store: Arc<ZeroCopyNoteStore>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
        }
    }

    pub fn trigger_poll_relay_audit_updates(&self, peer_id: String, store: Arc<ZeroCopyAuditStore>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock().unwrap().clone();
                    let mut query_params = vec![("peer_id", peer_id.clone())];
                    if let Some(ref signing_key) = key {
                        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
                        let msg = format!("poll:{}:{}", peer_id, timestamp);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(msg.as_bytes());
                        query_params.push(("timestamp", timestamp));
                        query_params.push(("signature_hex", const_hex::encode(signature.to_bytes())));
                    }

                    if let Ok(res) = client
                        .get(&format!("{}/relay/updates", relay_url))
                        .query(&query_params)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                let mut batch = Vec::new();
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (
                                        u.get("from_peer").and_then(|v| v.as_str()),
                                        u.get("data_hex").and_then(|v| v.as_str()),
                                    ) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            batch.push(update_bytes);
                                            tracing::info!("Buffered P2P update from peer: {}", from_peer);
                                        }
                                    }
                                }
                                if !batch.is_empty() {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                    let updates = in_memory_poll(&peer_id);
                    if !updates.is_empty() {
                        if store.apply_loro_updates_batch(updates).is_ok() {
                            crate::infra::observer::notify_observers();
                        }
                    }
                });
            }
        }
    }

    pub fn get_connected_peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().clone()
    }

    pub fn drain_pending_broadcasts(&self) -> Vec<Vec<u8>> {
        let mut pending = self.pending_broadcasts.lock().unwrap();
        std::mem::take(&mut *pending)
    }
}

macro_rules! trigger_once_body {
    ($self:expr, $store:expr) => {{
        let edge_url = $self.inner.edge_url.clone();
        let client = $self.inner.client.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::database::native::get_runtime().spawn(async move {
                if let Ok(local_changes) = $store.get_loro_changes() {
                    if let Ok(res) = client
                        .post(&format!("{}/sync", edge_url))
                        .body(local_changes)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    if let Err(e) = $store.apply_loro_update(remote_bytes.to_vec()) {
                                        tracing::warn!("Failed to apply sync update: {:?}", e);
                                    } else {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(local_changes) = $store.get_loro_changes() {
                    if let Ok(res) = client
                        .post(&format!("{}/sync", edge_url))
                        .body(local_changes)
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    if let Err(e) = $store.apply_loro_update(remote_bytes.to_vec()) {
                                        tracing::warn!("Failed to apply sync update: {:?}", e);
                                    } else {
                                        crate::infra::observer::notify_observers();
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }};
}

macro_rules! start_loop_body {
    ($self:expr, $store:expr, $running_flag:ident, $interval_secs:expr) => {{
        if $self
            .inner
            .$running_flag
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let edge_url = $self.inner.edge_url.clone();
        let is_running = $self.inner.$running_flag.clone();
        let client = $self.inner.client.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::database::native::get_runtime().spawn(async move {
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = $store.get_loro_changes() {
                        if let Ok(res) = client
                            .post(&format!("{}/sync", edge_url))
                            .body(local_changes)
                            .send()
                            .await
                        {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        if let Err(e) = $store.apply_loro_update(remote_bytes.to_vec()) {
                                            tracing::warn!("Failed to apply sync loop update: {:?}", e);
                                        } else {
                                            crate::infra::observer::notify_observers();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs($interval_secs as u64)).await;
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = $store.get_loro_changes() {
                        if let Ok(res) = client
                            .post(&format!("{}/sync", edge_url))
                            .body(local_changes)
                            .send()
                            .await
                        {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        if let Err(e) = $store.apply_loro_update(remote_bytes.to_vec()) {
                                            tracing::warn!("Failed to apply sync loop update: {:?}", e);
                                        } else {
                                            crate::infra::observer::notify_observers();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    crate::infra::time::sleep_ms($interval_secs as u64 * 1000).await;
                }
            });
        }

        Ok(())
    }};
}

struct EdgeSyncLoopInner {
    edge_url: String,
    is_running_todos: Arc<std::sync::atomic::AtomicBool>,
    is_running_messages: Arc<std::sync::atomic::AtomicBool>,
    is_running_notes: Arc<std::sync::atomic::AtomicBool>,
    is_running_audits: Arc<std::sync::atomic::AtomicBool>,
    client: reqwest::Client,
}

impl Drop for EdgeSyncLoopInner {
    fn drop(&mut self) {
        self.is_running_todos.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_messages.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_notes.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_audits.store(false, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("Edge sync loop stopped (all handles dropped)");
    }
}

#[derive(Clone, uniffi::Object)]
pub struct EdgeSyncLoop {
    inner: Arc<EdgeSyncLoopInner>,
}

#[uniffi::export]
impl EdgeSyncLoop {
    #[uniffi::constructor]
    pub fn new(edge_url: String) -> Self {
        Self {
            inner: Arc::new(EdgeSyncLoopInner {
                edge_url,
                is_running_todos: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                is_running_messages: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                is_running_notes: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                is_running_audits: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                client: reqwest::Client::new(),
            }),
        }
    }

    pub fn stop_sync_loop(&self) {
        self.inner.is_running_todos.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_messages.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_notes.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_audits.store(false, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("Edge sync loop stopped");
    }

    pub fn is_running(&self) -> bool {
        self.inner.is_running_todos.load(std::sync::atomic::Ordering::SeqCst)
            || self.inner.is_running_messages.load(std::sync::atomic::Ordering::SeqCst)
            || self.inner.is_running_notes.load(std::sync::atomic::Ordering::SeqCst)
            || self.inner.is_running_audits.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn trigger_sync_once(&self, store: Arc<ZeroCopyStore>) {
        trigger_once_body!(self, store);
    }

    pub fn trigger_message_sync_once(&self, store: Arc<ZeroCopyMessageStore>) {
        trigger_once_body!(self, store);
    }

    pub fn trigger_note_sync_once(&self, store: Arc<ZeroCopyNoteStore>) {
        trigger_once_body!(self, store);
    }

    pub fn trigger_audit_sync_once(&self, store: Arc<ZeroCopyAuditStore>) {
        trigger_once_body!(self, store);
    }

    pub fn start_sync_loop(
        &self,
        store: Arc<ZeroCopyStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_todos, interval_secs)
    }

    pub fn start_message_sync_loop(
        &self,
        store: Arc<ZeroCopyMessageStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_messages, interval_secs)
    }

    pub fn start_note_sync_loop(
        &self,
        store: Arc<ZeroCopyNoteStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_notes, interval_secs)
    }

    pub fn start_audit_sync_loop(
        &self,
        store: Arc<ZeroCopyAuditStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_audits, interval_secs)
    }
}
