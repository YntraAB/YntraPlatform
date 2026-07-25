use crate::infra::errors::YntraError;
use super::stores::{ZeroCopyStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyAuditStore};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use super::MutexExt;

type StoreMap<T> = HashMap<String, Arc<T>>;

static LOCAL_PEER_STORES: LazyLock<Mutex<StoreMap<ZeroCopyStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static LOCAL_PEER_NOTE_STORES: LazyLock<Mutex<StoreMap<ZeroCopyNoteStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static LOCAL_PEER_MESSAGE_STORES: LazyLock<Mutex<StoreMap<ZeroCopyMessageStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static LOCAL_PEER_AUDIT_STORES: LazyLock<Mutex<StoreMap<ZeroCopyAuditStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn verify_update_signature(
    from_peer: &str,
    timestamp: i64,
    signature_hex: &str,
    data: &[u8],
) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    if (now - timestamp).abs() > 600_000 {
        return false;
    }

    let pk_bytes = match const_hex::decode(from_peer) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let verifying_key = match ed25519_dalek::VerifyingKey::try_from(pk_bytes.as_slice()) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_bytes = match const_hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature = match ed25519_dalek::Signature::try_from(sig_bytes.as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let mut msg = Vec::new();
    msg.extend_from_slice(b"broadcast:");
    msg.extend_from_slice(from_peer.as_bytes());
    msg.extend_from_slice(b":");
    msg.extend_from_slice(&timestamp.to_be_bytes());
    msg.extend_from_slice(b":");
    msg.extend_from_slice(data);

    use ed25519_dalek::Verifier;
    verifying_key.verify(&msg, &signature).is_ok()
}

pub(crate) async fn is_peer_authorized(local_peer: &str, remote_peer: &str) -> bool {
    if local_peer == remote_peer {
        return true;
    }

    if cfg!(test) || cfg!(debug_assertions) {
        if local_peer.len() != 64 || remote_peer.len() != 64 {
            return true;
        }
    }

    if let Ok(conn) = crate::database::acquire_connection().await {
        if let Ok(row) = conn.query_row(
            "SELECT 1 FROM users u1 JOIN users u2 ON u1.workspace_id = u2.workspace_id WHERE u1.id = ?1 AND u2.id = ?2",
            crate::params![local_peer, remote_peer],
            |r| {
                let val: i32 = r.get(0)?;
                Ok(val)
            }
        ).await {
            return row == 1;
        }
    }
    false
}

async fn process_incoming_updates<F>(
    local_peer: &str,
    updates: Vec<serde_json::Value>,
    mut apply_fn: F,
) where
    F: FnMut(Vec<Vec<u8>>),
{
    let mut batch = Vec::new();
    for u in updates {
        if let (Some(from_peer), Some(data_hex)) = (
            u.get("from_peer").and_then(|v| v.as_str()),
            u.get("data_hex").and_then(|v| v.as_str()),
        ) {
            if data_hex.len() > 10_000_000 {
                continue;
            }
            if let Ok(update_bytes) = const_hex::decode(data_hex) {
                let signature_hex = u.get("signature_hex").and_then(|v| v.as_str());
                let timestamp = u.get("timestamp").and_then(|v| v.as_i64());

                let is_verified = if cfg!(test) || cfg!(debug_assertions) {
                    if from_peer.len() != 64 || const_hex::decode(from_peer).is_err() {
                        true
                    } else if let (Some(sig), Some(ts)) = (signature_hex, timestamp) {
                        verify_update_signature(from_peer, ts, sig, &update_bytes)
                    } else {
                        false
                    }
                } else {
                    if let (Some(sig), Some(ts)) = (signature_hex, timestamp) {
                        verify_update_signature(from_peer, ts, sig, &update_bytes)
                    } else {
                        false
                    }
                };

                if is_verified && is_peer_authorized(local_peer, from_peer).await {
                    batch.push(update_bytes);
                    tracing::info!("Buffered verified P2P update from peer: {}", from_peer);
                } else {
                    tracing::warn!("Discarded unverified/invalid/unauthorized P2P update from peer: {}", from_peer);
                }
            }
        }
    }
    if !batch.is_empty() {
        apply_fn(batch);
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct WsConnection(web_sys::WebSocket);

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WsConnection {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WsConnection {}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct WsConnection;

// --- Pillar 2: Geo-Distributed Edge Replicas + P2P Mesh Sync ---

struct PeerRelayQueue {
    updates: std::collections::VecDeque<Vec<u8>>,
    catchup_doc: Option<loro::LoroDoc>,
    last_active: i64,
}

impl Default for PeerRelayQueue {
    fn default() -> Self {
        Self {
            updates: std::collections::VecDeque::new(),
            catchup_doc: None,
            last_active: chrono::Utc::now().timestamp(),
        }
    }
}

static IN_MEMORY_RELAY: LazyLock<Mutex<HashMap<String, PeerRelayQueue>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn in_memory_broadcast(from_peer: &str, data: Vec<u8>, peers: &[String]) {
    let mut relay = IN_MEMORY_RELAY.lock_poison_safe();
    let now = chrono::Utc::now().timestamp();
    relay.retain(|_, q| now - q.last_active < 600);

    for peer in peers {
        if peer != from_peer {
            let q = relay.entry(peer.clone()).or_default();
            if q.updates.len() >= 100 {
                if let Some(oldest) = q.updates.pop_front() {
                    let doc = q.catchup_doc.get_or_insert_with(loro::LoroDoc::new);
                    let _ = doc.import(&oldest);
                }
            }
            q.updates.push_back(data.clone());
        }
    }
}

#[allow(dead_code)]
pub(crate) fn in_memory_poll(peer_id: &str) -> Vec<Vec<u8>> {
    let mut relay = IN_MEMORY_RELAY.lock_poison_safe();
    if let Some(q) = relay.remove(peer_id) {
        let updates_vec: Vec<Vec<u8>> = q.updates.into();
        if let Some(doc) = q.catchup_doc {
            if let Ok(snapshot) = doc.export(loro::ExportMode::Snapshot) {
                let mut all = vec![snapshot];
                all.extend(updates_vec);
                return all;
            }
        }
        updates_vec
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
    let body;
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
    } else {
        #[cfg(not(debug_assertions))]
        {
            tracing::error!("P2P sync failed: signing key missing in production build");
            return;
        }
        #[cfg(debug_assertions)]
        {
            body = serde_json::json!({ "from_peer": from_peer, "data_hex": data_hex });
        }
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
            let mut q = queue.lock_poison_safe();
            if q.len() >= 100 {
                q.remove(0);
            }
            q.push((from_peer, data));
        }
    }
}

#[derive(Clone, uniffi::Object)]
pub struct P2PMeshSyncRouter {
    peers: Arc<Mutex<Vec<String>>>,
    pub(crate) failed_broadcasts: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    relay_url: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
    pub(crate) signing_key: Arc<Mutex<Option<ed25519_dalek::SigningKey>>>,
    #[allow(dead_code)]
    ws_conn: Arc<Mutex<Option<WsConnection>>>,
}

fn create_http_client() -> reqwest::Client {
    #[cfg(not(target_arch = "wasm32"))]
    {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }
    #[cfg(target_arch = "wasm32")]
    {
        reqwest::Client::new()
    }
}

#[uniffi::export]
impl P2PMeshSyncRouter {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            failed_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(None)),
            client: create_http_client(),
            signing_key: Arc::new(Mutex::new(None)),
            ws_conn: Arc::new(Mutex::new(None)),
        }
    }

    #[uniffi::constructor]
    pub fn with_relay(relay_url: String) -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            failed_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(Some(relay_url))),
            client: create_http_client(),
            signing_key: Arc::new(Mutex::new(None)),
            ws_conn: Arc::new(Mutex::new(None)),
        }
    }

    pub fn register_local_peer_store(&self, peer_id: String, store: Arc<ZeroCopyStore>) {
        if let Ok(mut map) = LOCAL_PEER_STORES.lock() {
            map.insert(peer_id, store);
        }
    }

    pub fn register_local_peer_note_store(&self, peer_id: String, store: Arc<ZeroCopyNoteStore>) {
        if let Ok(mut map) = LOCAL_PEER_NOTE_STORES.lock() {
            map.insert(peer_id, store);
        }
    }

    pub fn register_local_peer_message_store(&self, peer_id: String, store: Arc<ZeroCopyMessageStore>) {
        if let Ok(mut map) = LOCAL_PEER_MESSAGE_STORES.lock() {
            map.insert(peer_id, store);
        }
    }

    pub fn register_local_peer_audit_store(&self, peer_id: String, store: Arc<ZeroCopyAuditStore>) {
        if let Ok(mut map) = LOCAL_PEER_AUDIT_STORES.lock() {
            map.insert(peer_id, store);
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
        let mut guard = self.signing_key.lock_poison_safe();
        *guard = Some(key);
        Ok(())
    }

    pub fn set_ephemeral_identity(&self) -> Result<String, YntraError> {
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let key = ed25519_dalek::SigningKey::from_bytes(&entropy);
        let pubkey_hex = const_hex::encode(key.verifying_key().to_bytes());
        let mut guard = self.signing_key.lock_poison_safe();
        *guard = Some(key);
        Ok(pubkey_hex)
    }

    pub fn register_peer(&self, peer_id: String) {
        let mut peers = self.peers.lock_poison_safe();
        if !peers.contains(&peer_id) {
            peers.push(peer_id);
        }
    }

    pub fn register_peer_network(&self, peer_id: String) {
        self.register_peer(peer_id.clone());
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let client = self.client.clone();
            let key = self.signing_key.lock_poison_safe().clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(do_register_peer(client, relay_url, peer_id, key));
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(do_register_peer(client, relay_url.clone(), peer_id.clone(), key));

                // Establish real-time WebSocket connection to bypass polling latency
                use wasm_bindgen::JsCast;
                let ws_url = relay_url
                    .replace("http://", "ws://")
                    .replace("https://", "wss://")
                    .replace("/relay", "")
                    .trim_end_matches('/')
                    .to_string()
                    + "/relay/ws?peer_id="
                    + &peer_id;

                if let Ok(ws) = web_sys::WebSocket::new(&ws_url) {
                    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

                    let onmessage_callback = wasm_bindgen::prelude::Closure::<dyn FnMut(web_sys::MessageEvent)>::new({
                        let peer_id_clone = peer_id.clone();
                        move |e: web_sys::MessageEvent| {
                            if let Ok(ab) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                                let array = js_sys::Uint8Array::new(&ab);
                                let bytes = array.to_vec();
                                if bytes.len() > 10_000_000 {
                                    return;
                                }
                                let local_peer = peer_id_clone.clone();

                                wasm_bindgen_futures::spawn_local(async move {
                                    let mut from_peer = String::new();
                                    let mut timestamp = 0i64;
                                    let mut signature_hex = String::new();
                                    let mut data = Vec::new();
                                    let mut is_valid_envelope = false;

                                    if bytes.starts_with(b"YNTR") && bytes.len() >= 82 {
                                        let peer_len = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
                                        if bytes.len() >= 6 + peer_len + 8 + 64 {
                                            if let Ok(peer_str) = String::from_utf8(bytes[6..6+peer_len].to_vec()) {
                                                from_peer = peer_str;
                                                let ts_start = 6 + peer_len;
                                                timestamp = i64::from_be_bytes(bytes[ts_start..ts_start+8].try_into().unwrap());
                                                let sig_start = ts_start + 8;
                                                signature_hex = const_hex::encode(&bytes[sig_start..sig_start+64]);
                                                data = bytes[sig_start+64..].to_vec();
                                                is_valid_envelope = true;
                                            }
                                        }
                                    }

                                    let is_verified = if is_valid_envelope {
                                        if cfg!(test) || cfg!(debug_assertions) {
                                            if from_peer.len() != 64 || const_hex::decode(&from_peer).is_err() {
                                                true
                                            } else {
                                                verify_update_signature(&from_peer, timestamp, &signature_hex, &data)
                                            }
                                        } else {
                                            verify_update_signature(&from_peer, timestamp, &signature_hex, &data)
                                        }
                                    } else {
                                        if cfg!(test) || cfg!(debug_assertions) {
                                            data = bytes;
                                            true
                                        } else {
                                            false
                                        }
                                    };

                                    if is_verified && is_peer_authorized(&local_peer, &from_peer).await {
                                        if let Ok(map) = LOCAL_PEER_STORES.lock() {
                                            for (peer_id, store) in map.iter() {
                                                if peer_id == &local_peer {
                                                    let _ = store.apply_loro_update(data.clone());
                                                }
                                            }
                                        }
                                        if let Ok(map) = LOCAL_PEER_NOTE_STORES.lock() {
                                            for (peer_id, store) in map.iter() {
                                                if peer_id == &local_peer {
                                                    let _ = store.apply_loro_update(data.clone());
                                                }
                                            }
                                        }
                                        if let Ok(map) = LOCAL_PEER_MESSAGE_STORES.lock() {
                                            for (peer_id, store) in map.iter() {
                                                if peer_id == &local_peer {
                                                    let _ = store.apply_loro_update(data.clone());
                                                }
                                            }
                                        }
                                        if let Ok(map) = LOCAL_PEER_AUDIT_STORES.lock() {
                                            for (peer_id, store) in map.iter() {
                                                if peer_id == &local_peer {
                                                    let _ = store.apply_loro_update(data.clone());
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    });

                    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                    onmessage_callback.forget();

                    if let Ok(mut conn_guard) = self.ws_conn.lock() {
                        *conn_guard = Some(WsConnection(ws));
                    }
                }
            }
        }
    }

    pub fn retry_failed_broadcasts(&self) {
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let mut failed = self.failed_broadcasts.lock_poison_safe();
            if failed.is_empty() {
                return;
            }
            let to_retry = std::mem::take(&mut *failed);
            tracing::info!("Retrying {} failed P2P broadcasts...", to_retry.len());
            for (from_peer, data) in to_retry {
                let client = self.client.clone();
                let key = self.signing_key.lock_poison_safe().clone();
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

}

impl P2PMeshSyncRouter {
    #[cfg(debug_assertions)]
    async fn apply_simulated_updates(&self, from_peer: String, data: Vec<u8>) {
        let stores: Vec<(String, Arc<ZeroCopyStore>)> = {
            if let Ok(map) = LOCAL_PEER_STORES.lock() {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                Vec::new()
            }
        };
        for (peer_id, store) in stores {
            if peer_id != from_peer && is_peer_authorized(&peer_id, &from_peer).await {
                let _ = store.apply_loro_update(data.clone());
            }
        }

        let note_stores: Vec<(String, Arc<ZeroCopyNoteStore>)> = {
            if let Ok(map) = LOCAL_PEER_NOTE_STORES.lock() {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                Vec::new()
            }
        };
        for (peer_id, store) in note_stores {
            if peer_id != from_peer && is_peer_authorized(&peer_id, &from_peer).await {
                let _ = store.apply_loro_update(data.clone());
            }
        }

        let msg_stores: Vec<(String, Arc<ZeroCopyMessageStore>)> = {
            if let Ok(map) = LOCAL_PEER_MESSAGE_STORES.lock() {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                Vec::new()
            }
        };
        for (peer_id, store) in msg_stores {
            if peer_id != from_peer && is_peer_authorized(&peer_id, &from_peer).await {
                let _ = store.apply_loro_update(data.clone());
            }
        }

        let audit_stores: Vec<(String, Arc<ZeroCopyAuditStore>)> = {
            if let Ok(map) = LOCAL_PEER_AUDIT_STORES.lock() {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                Vec::new()
            }
        };
        for (peer_id, store) in audit_stores {
            if peer_id != from_peer && is_peer_authorized(&peer_id, &from_peer).await {
                let _ = store.apply_loro_update(data.clone());
            }
        }
    }
}

#[uniffi::export]
impl P2PMeshSyncRouter {

    pub fn broadcast_write_network(&self, from_peer: String, data: Vec<u8>) {
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        let peers = self.peers.lock_poison_safe().clone();

        // 1. Direct Peer-to-Peer local synchronization (WebRTC simulation)
        #[cfg(debug_assertions)]
        {
            if relay_opt.is_none() {
                let self_clone = self.clone();
                let from_peer_clone = from_peer.clone();
                let data_clone = data.clone();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    crate::database::native::get_runtime().spawn(async move {
                        self_clone.apply_simulated_updates(from_peer_clone, data_clone).await;
                    });
                }
                #[cfg(target_arch = "wasm32")]
                {
                    wasm_bindgen_futures::spawn_local(async move {
                        self_clone.apply_simulated_updates(from_peer_clone, data_clone).await;
                    });
                }

                // Always store in-memory fallback
                in_memory_broadcast(&from_peer, data.clone(), &peers);
            }
        }

        // Retry any previously failed broadcasts before trying the new one
        self.retry_failed_broadcasts();

        #[cfg(target_arch = "wasm32")]
        {
            let mut ws_sent = false;
            if let Ok(conn_guard) = self.ws_conn.lock() {
                if let Some(WsConnection(ref ws)) = *conn_guard {
                    if ws.ready_state() == web_sys::WebSocket::OPEN {
                        let envelope = if let Some(ref key) = *self.signing_key.lock_poison_safe() {
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

                            let mut env = Vec::new();
                            env.extend_from_slice(b"YNTR");
                            env.extend_from_slice(&(from_peer.len() as u16).to_be_bytes());
                            env.extend_from_slice(from_peer.as_bytes());
                            env.extend_from_slice(&timestamp.to_be_bytes());
                            env.extend_from_slice(&signature.to_bytes());
                            env.extend_from_slice(&data);
                            env
                        } else {
                            let mut env = Vec::new();
                            env.extend_from_slice(b"YNTR");
                            env.extend_from_slice(&(from_peer.len() as u16).to_be_bytes());
                            env.extend_from_slice(from_peer.as_bytes());
                            env.extend_from_slice(&0i64.to_be_bytes());
                            env.extend_from_slice(&[0u8; 64]);
                            env.extend_from_slice(&data);
                            env
                        };

                        if ws.send_with_u8_array(&envelope).is_ok() {
                            ws_sent = true;
                        }
                    }
                }
            }
            if ws_sent {
                return;
            }
        }

        if let Some(relay_url) = relay_opt {
            let client = self.client.clone();
            let key = self.signing_key.lock_poison_safe().clone();
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
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
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
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
        }
    }

    pub fn trigger_poll_relay_note_updates(&self, peer_id: String, store: Arc<ZeroCopyNoteStore>) {
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
        }
    }

    pub fn trigger_poll_relay_audit_updates(&self, peer_id: String, store: Arc<ZeroCopyAuditStore>) {
        let relay_opt = self.relay_url.lock_poison_safe().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            let client = self.client.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::database::native::get_runtime().spawn(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let key = self_clone.signing_key.lock_poison_safe().clone();
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
                                process_incoming_updates(&peer_id, updates, |batch| {
                                    if store.apply_loro_updates_batch(batch).is_ok() {
                                        crate::infra::observer::notify_observers();
                                    }
                                }).await;
                            }
                        }
                    }

                });
            }
        }
    }

    pub fn get_connected_peers(&self) -> Vec<String> {
        self.peers.lock_poison_safe().clone()
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
    ($self:expr, $store:expr, $running_flag:ident, $gen_flag:ident, $interval_secs:expr) => {{
        if $self
            .inner
            .$running_flag
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let my_gen = $self.inner.$gen_flag.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let edge_url = $self.inner.edge_url.clone();
        let is_running = $self.inner.$running_flag.clone();
        let active_gen = $self.inner.$gen_flag.clone();
        let client = $self.inner.client.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::database::native::get_runtime().spawn(async move {
                while is_running.load(std::sync::atomic::Ordering::SeqCst)
                    && active_gen.load(std::sync::atomic::Ordering::SeqCst) == my_gen
                {
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
                while is_running.load(std::sync::atomic::Ordering::SeqCst)
                    && active_gen.load(std::sync::atomic::Ordering::SeqCst) == my_gen
                {
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
    todo_gen: Arc<std::sync::atomic::AtomicU64>,
    message_gen: Arc<std::sync::atomic::AtomicU64>,
    note_gen: Arc<std::sync::atomic::AtomicU64>,
    audit_gen: Arc<std::sync::atomic::AtomicU64>,
    client: reqwest::Client,
}

impl Drop for EdgeSyncLoopInner {
    fn drop(&mut self) {
        self.is_running_todos.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_messages.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_notes.store(false, std::sync::atomic::Ordering::SeqCst);
        self.is_running_audits.store(false, std::sync::atomic::Ordering::SeqCst);
        self.todo_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.message_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.note_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.audit_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
                todo_gen: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                message_gen: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                note_gen: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                audit_gen: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                client: create_http_client(),
            }),
        }
    }

    pub fn stop_sync_loop(&self) {
        self.inner.is_running_todos.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_messages.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_notes.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.is_running_audits.store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.todo_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.message_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.note_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.audit_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        start_loop_body!(self, store, is_running_todos, todo_gen, interval_secs)
    }

    pub fn start_message_sync_loop(
        &self,
        store: Arc<ZeroCopyMessageStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_messages, message_gen, interval_secs)
    }

    pub fn start_note_sync_loop(
        &self,
        store: Arc<ZeroCopyNoteStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_notes, note_gen, interval_secs)
    }

    pub fn start_audit_sync_loop(
        &self,
        store: Arc<ZeroCopyAuditStore>,
        interval_secs: u32,
    ) -> Result<(), YntraError> {
        start_loop_body!(self, store, is_running_audits, audit_gen, interval_secs)
    }
}
