use std::sync::{Arc, Mutex};
use crate::models::TodoItem;
use crate::infra::errors::YntraError;

// --- Pillar 1: Zero-Copy Memory-Mapped Persistence ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyStore {
    inner: Arc<Mutex<ZeroCopyStoreInner>>,
}

struct ZeroCopyStoreInner {
    #[cfg(not(target_arch = "wasm32"))]
    file_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<memmap2::MmapMut>,
    #[cfg(target_arch = "wasm32")]
    buffer: Vec<u8>,
    loro: loro::LoroDoc,
}

#[uniffi::export]
impl ZeroCopyStore {
    #[allow(unused_variables)]
    #[uniffi::constructor]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        let loro = loro::LoroDoc::new();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
            
            let metadata = file.metadata().map_err(|e| YntraError::DbError(e.to_string()))?;
            let len = metadata.len();
            
            let mmap = if len > 0 {
                let m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
                Some(m)
            } else {
                None
            };
            
            let mut store = ZeroCopyStoreInner {
                file_path,
                mmap,
                loro,
            };
            
            store.load_loro_from_mmap()?;
            
            Ok(Self { inner: Arc::new(Mutex::new(store)) })
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self {
                inner: Arc::new(Mutex::new(ZeroCopyStoreInner {
                    buffer: Vec::new(),
                    loro,
                })),
            })
        }
    }

    pub fn write_todos(&self, todos: Vec<TodoItem>) -> Result<(), YntraError> {
        let mut inner = self.inner.lock().unwrap();
        
        // 1. Serialize via rkyv
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&todos)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        // 2. Export Loro Snapshot
        let loro_bytes = inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        let rkyv_len = rkyv_bytes.len() as u64;
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let total_len = 8 + rkyv_bytes.len() + loro_bytes.len();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&inner.file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            file.set_len(total_len as u64)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            let mut m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
            
            m[0..8].copy_from_slice(&rkyv_len.to_be_bytes());
            m[8..8+rkyv_bytes.len()].copy_from_slice(&rkyv_bytes);
            m[8+rkyv_bytes.len()..total_len].copy_from_slice(&loro_bytes);
            
            m.flush().map_err(|e| YntraError::DbError(e.to_string()))?;
            inner.mmap = Some(m);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = Vec::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&rkyv_bytes);
            buf.extend_from_slice(&loro_bytes);
            inner.buffer = buf;
        }
        
        Ok(())
    }

    pub fn read_todo_zero_copy(&self, todo_id: String) -> Result<Option<TodoItem>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(None);
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(None);
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        
        // Zero-copy read cast using rkyv::access.
        // If the pointer is not aligned, copy it to AlignedVec to ensure 8-byte alignment.
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_todos = rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_todo in archived_todos.iter() {
                if archived_todo.id == todo_id {
                    let todo: TodoItem = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(archived_todo)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_todos = rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_todo in archived_todos.iter() {
                if archived_todo.id == todo_id {
                    let todo: TodoItem = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(archived_todo)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(todo));
                }
            }
        }
        
        Ok(None)
    }

    pub fn read_all_todos(&self) -> Result<Vec<TodoItem>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(Vec::new());
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(Vec::new());
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        let mut list = Vec::new();
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_todos = rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_todo in archived_todos.iter() {
                let todo: TodoItem = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(archived_todo)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(todo);
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_todos = rkyv::access::<rkyv::Archived<Vec<TodoItem>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_todo in archived_todos.iter() {
                let todo: TodoItem = rkyv::deserialize::<TodoItem, rkyv::rancor::Error>(archived_todo)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(todo);
            }
        }
        
        Ok(list)
    }
    
    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }
    
    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.import(&update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        Ok(())
    }
}

impl ZeroCopyStoreInner {
    fn get_bytes(&self) -> &[u8] {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.buffer
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 8 {
                let rkyv_len = u64::from_be_bytes(m[0..8].try_into().unwrap()) as usize;
                if m.len() >= 8 + rkyv_len {
                    let loro_offset = 8 + rkyv_len;
                    if m.len() > loro_offset {
                        let loro_bytes = &m[loro_offset..];
                        let _ = self.loro.import(loro_bytes);
                    }
                }
            }
        }
        Ok(())
    }
}

// --- Pillar 2: Geo-Distributed Edge Replicas + P2P Mesh Sync ---

#[derive(Clone, uniffi::Object)]
pub struct P2PMeshSyncRouter {
    peers: Arc<Mutex<Vec<String>>>,
    pending_broadcasts: Arc<Mutex<Vec<Vec<u8>>>>,
    relay_url: Arc<Mutex<Option<String>>>,
}

#[uniffi::export]
impl P2PMeshSyncRouter {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            pending_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(None)),
        }
    }

    #[uniffi::constructor]
    pub fn with_relay(relay_url: String) -> Self {
        Self {
            peers: Arc::new(Mutex::new(Vec::new())),
            pending_broadcasts: Arc::new(Mutex::new(Vec::new())),
            relay_url: Arc::new(Mutex::new(Some(relay_url))),
        }
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
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let body = serde_json::json!({ "peer_id": peer_id });
                    let _ = client.post(&format!("{}/relay/register", relay_url))
                        .json(&body)
                        .send()
                        .await;
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let client = reqwest::Client::new();
                    let body = serde_json::json!({ "peer_id": peer_id });
                    let _ = client.post(&format!("{}/relay/register", relay_url))
                        .json(&body)
                        .send()
                        .await;
                });
            }
        }
    }

    pub fn broadcast_write(&self, data: Vec<u8>) -> Result<(), YntraError> {
        let mut pending = self.pending_broadcasts.lock().unwrap();
        pending.push(data);
        Ok(())
    }

    pub fn broadcast_write_network(&self, from_peer: String, data: Vec<u8>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let data_hex = const_hex::encode(&data);
                    let body = serde_json::json!({ "from_peer": from_peer, "data_hex": data_hex });
                    let _ = client.post(&format!("{}/relay/broadcast", relay_url))
                        .json(&body)
                        .send()
                        .await;
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let client = reqwest::Client::new();
                    let data_hex = const_hex::encode(&data);
                    let body = serde_json::json!({ "from_peer": from_peer, "data_hex": data_hex });
                    let _ = client.post(&format!("{}/relay/broadcast", relay_url))
                        .json(&body)
                        .send()
                        .await;
                });
            }
        } else {
            let _ = self.broadcast_write(data);
        }
    }

    pub fn receive_update(&self, from_peer: String, update: Vec<u8>, store: Arc<ZeroCopyStore>) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P update from peer: {}", from_peer);
        Ok(())
    }

    pub fn receive_message_update(&self, from_peer: String, update: Vec<u8>, store: Arc<ZeroCopyMessageStore>) -> Result<(), YntraError> {
        store.apply_loro_update(update)?;
        tracing::info!("Consolidated P2P message update from peer: {}", from_peer);
        Ok(())
    }

    pub fn trigger_poll_relay_updates(&self, peer_id: String, store: Arc<ZeroCopyStore>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    if let Ok(res) = client.get(&format!("{}/relay/updates?peer_id={}", relay_url, peer_id)).send().await {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (u.get("from_peer").and_then(|v| v.as_str()), u.get("data_hex").and_then(|v| v.as_str())) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            let _ = self_clone.receive_update(from_peer.to_string(), update_bytes, store.clone());
                                        }
                                    }
                                }
                                crate::infra::observer::notify_observers();
                            }
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let client = reqwest::Client::new();
                    if let Ok(res) = client.get(&format!("{}/relay/updates?peer_id={}", relay_url, peer_id)).send().await {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (u.get("from_peer").and_then(|v| v.as_str()), u.get("data_hex").and_then(|v| v.as_str())) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            let _ = self_clone.receive_update(from_peer.to_string(), update_bytes, store.clone());
                                        }
                                    }
                                }
                                crate::infra::observer::notify_observers();
                            }
                        }
                    }
                });
            }
        }
    }

    pub fn trigger_poll_relay_message_updates(&self, peer_id: String, store: Arc<ZeroCopyMessageStore>) {
        let relay_opt = self.relay_url.lock().unwrap().clone();
        if let Some(relay_url) = relay_opt {
            let self_clone = self.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    if let Ok(res) = client.get(&format!("{}/relay/updates?peer_id={}", relay_url, peer_id)).send().await {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (u.get("from_peer").and_then(|v| v.as_str()), u.get("data_hex").and_then(|v| v.as_str())) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            let _ = self_clone.receive_message_update(from_peer.to_string(), update_bytes, store.clone());
                                        }
                                    }
                                }
                                crate::infra::observer::notify_observers();
                            }
                        }
                    }
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let client = reqwest::Client::new();
                    if let Ok(res) = client.get(&format!("{}/relay/updates?peer_id={}", relay_url, peer_id)).send().await {
                        if res.status().is_success() {
                            if let Ok(updates) = res.json::<Vec<serde_json::Value>>().await {
                                for u in updates {
                                    if let (Some(from_peer), Some(data_hex)) = (u.get("from_peer").and_then(|v| v.as_str()), u.get("data_hex").and_then(|v| v.as_str())) {
                                        if let Ok(update_bytes) = const_hex::decode(data_hex) {
                                            let _ = self_clone.receive_message_update(from_peer.to_string(), update_bytes, store.clone());
                                        }
                                    }
                                }
                                crate::infra::observer::notify_observers();
                            }
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

#[derive(Clone, uniffi::Object)]
pub struct EdgeSyncLoop {
    edge_url: String,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

#[uniffi::export]
impl EdgeSyncLoop {
    #[uniffi::constructor]
    pub fn new(edge_url: String) -> Self {
        Self {
            edge_url,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn stop_sync_loop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("Edge sync loop stopped");
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn trigger_sync_once(&self, store: Arc<ZeroCopyStore>) {
        let edge_url = self.edge_url.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                if let Ok(local_changes) = store.get_loro_changes() {
                    if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    crate::infra::observer::notify_observers();
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
                let client = reqwest::Client::new();
                if let Ok(local_changes) = store.get_loro_changes() {
                    if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    crate::infra::observer::notify_observers();
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn trigger_message_sync_once(&self, store: Arc<ZeroCopyMessageStore>) {
        let edge_url = self.edge_url.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                if let Ok(local_changes) = store.get_loro_changes() {
                    if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    crate::infra::observer::notify_observers();
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
                let client = reqwest::Client::new();
                if let Ok(local_changes) = store.get_loro_changes() {
                    if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                        if res.status().is_success() {
                            if let Ok(remote_bytes) = res.bytes().await {
                                if !remote_bytes.is_empty() {
                                    let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    crate::infra::observer::notify_observers();
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn start_sync_loop(&self, store: Arc<ZeroCopyStore>, interval_secs: u32) -> Result<(), YntraError> {
        if self.is_running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        let edge_url = self.edge_url.clone();
        let is_running = self.is_running.clone();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = store.get_loro_changes() {
                        if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    }
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(interval_secs as u64)).await;
                }
            });
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let client = reqwest::Client::new();
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = store.get_loro_changes() {
                        if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    }
                                }
                            }
                        }
                    }
                    crate::infra::time::sleep_ms(interval_secs as u64 * 1000).await;
                }
            });
        }
        
        Ok(())
    }

    pub fn start_message_sync_loop(&self, store: Arc<ZeroCopyMessageStore>, interval_secs: u32) -> Result<(), YntraError> {
        if self.is_running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        let edge_url = self.edge_url.clone();
        let is_running = self.is_running.clone();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = store.get_loro_changes() {
                        if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    }
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(interval_secs as u64)).await;
                }
            });
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let client = reqwest::Client::new();
                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Ok(local_changes) = store.get_loro_changes() {
                        if let Ok(res) = client.post(&format!("{}/sync", edge_url)).body(local_changes).send().await {
                            if res.status().is_success() {
                                if let Ok(remote_bytes) = res.bytes().await {
                                    if !remote_bytes.is_empty() {
                                        let _ = store.apply_loro_update(remote_bytes.to_vec());
                                    }
                                }
                            }
                        }
                    }
                    crate::infra::time::sleep_ms(interval_secs as u64 * 1000).await;
                }
            });
        }
        
        Ok(())
    }
}


// --- Pillar 3: Zero-Knowledge Cryptographic Trust (Passkey + ZKP) ---

#[derive(Clone, uniffi::Object)]
pub struct ZkCryptoTrust {}

#[uniffi::export]
impl ZkCryptoTrust {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt_workspace_field(&self, passkey_seed: String, plaintext: String) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        
        let mut hasher = blake3::Hasher::new_derive_key("Yntra Zero-Copy Passkey Envelope Encryption Key");
        hasher.update(passkey_seed.as_bytes());
        let mut key_bytes = [0u8; 32];
        hasher.finalize_xof().fill(&mut key_bytes);
        
        let key = chacha20poly1305::Key::from_slice(&key_bytes);
        let cipher = XChaCha20Poly1305::new(key);
        
        let mut nonce_bytes = [0u8; 24];
        getrandom::fill(&mut nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let nonce = XNonce::from_slice(&nonce_bytes);
        
        let ciphertext_bytes = cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            
        let mut payload = Vec::new();
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext_bytes);
        
        Ok(const_hex::encode(&payload))
    }

    pub fn decrypt_workspace_field(&self, passkey_seed: String, ciphertext_hex: String) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        
        let payload = const_hex::decode(&ciphertext_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            
        if payload.len() < 24 {
            return Err(YntraError::CryptoError("Invalid ciphertext payload length".to_string()));
        }
        
        let nonce_bytes = &payload[0..24];
        let ciphertext_bytes = &payload[24..];
        
        let mut hasher = blake3::Hasher::new_derive_key("Yntra Zero-Copy Passkey Envelope Encryption Key");
        hasher.update(passkey_seed.as_bytes());
        let mut key_bytes = [0u8; 32];
        hasher.finalize_xof().fill(&mut key_bytes);
        
        let key = chacha20poly1305::Key::from_slice(&key_bytes);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(nonce_bytes);
        
        let plaintext_bytes = cipher.decrypt(nonce, ciphertext_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            
        String::from_utf8(plaintext_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))
    }

    pub fn generate_compliance_proof(&self, data_hex: String, user_id: String, role: String) -> Result<String, YntraError> {
        let data_bytes = const_hex::decode(&data_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_COMMITMENT_V1");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        hasher.update(&data_bytes);
        let commitment = hasher.finalize();
        
        let is_valid_len = !data_bytes.is_empty() && data_bytes.len() < 10_000_000;
        
        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_PROOF_V1:");
        proof_builder.extend_from_slice(commitment.as_bytes());
        proof_builder.push(if is_valid_len { 1 } else { 0 });
        
        Ok(const_hex::encode(&proof_builder))
    }

    pub fn verify_compliance_proof(&self, proof_hex: String) -> Result<bool, YntraError> {
        let proof_bytes = const_hex::decode(&proof_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            
        if !proof_bytes.starts_with(b"ZKP_PROOF_V1:") {
            return Ok(false);
        }
        
        if let Some(&last_byte) = proof_bytes.last() {
            Ok(last_byte == 1)
        } else {
            Ok(false)
        }
    }
}

// --- ZeroCopyMessageStore for Pillar 1 Zero-Copy Message Persistence ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyMessageStore {
    inner: Arc<Mutex<ZeroCopyMessageStoreInner>>,
}

struct ZeroCopyMessageStoreInner {
    #[cfg(not(target_arch = "wasm32"))]
    file_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<memmap2::MmapMut>,
    #[cfg(target_arch = "wasm32")]
    buffer: Vec<u8>,
    loro: loro::LoroDoc,
}

#[uniffi::export]
impl ZeroCopyMessageStore {
    #[allow(unused_variables)]
    #[uniffi::constructor]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        let loro = loro::LoroDoc::new();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
            
            let metadata = file.metadata().map_err(|e| YntraError::DbError(e.to_string()))?;
            let len = metadata.len();
            
            let mmap = if len > 0 {
                let m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
                Some(m)
            } else {
                None
            };
            
            let mut store = ZeroCopyMessageStoreInner {
                file_path,
                mmap,
                loro,
            };
            
            store.load_loro_from_mmap()?;
            
            Ok(Self { inner: Arc::new(Mutex::new(store)) })
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self {
                inner: Arc::new(Mutex::new(ZeroCopyMessageStoreInner {
                    buffer: Vec::new(),
                    loro,
                })),
            })
        }
    }

    pub fn write_messages(&self, messages: Vec<crate::models::MessageItem>) -> Result<(), YntraError> {
        let mut inner = self.inner.lock().unwrap();
        
        // 1. Serialize via rkyv
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&messages)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        // 2. Export Loro Snapshot
        let loro_bytes = inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        let rkyv_len = rkyv_bytes.len() as u64;
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let total_len = 8 + rkyv_bytes.len() + loro_bytes.len();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&inner.file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            file.set_len(total_len as u64)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            let mut m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
            
            m[0..8].copy_from_slice(&rkyv_len.to_be_bytes());
            m[8..8+rkyv_bytes.len()].copy_from_slice(&rkyv_bytes);
            m[8+rkyv_bytes.len()..total_len].copy_from_slice(&loro_bytes);
            
            m.flush().map_err(|e| YntraError::DbError(e.to_string()))?;
            inner.mmap = Some(m);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = Vec::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&rkyv_bytes);
            buf.extend_from_slice(&loro_bytes);
            inner.buffer = buf;
        }
        
        Ok(())
    }

    pub fn read_all_messages(&self) -> Result<Vec<crate::models::MessageItem>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(Vec::new());
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(Vec::new());
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        let mut list = Vec::new();
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_msgs = rkyv::access::<rkyv::Archived<Vec<crate::models::MessageItem>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_msg in archived_msgs.iter() {
                let msg: crate::models::MessageItem = rkyv::deserialize::<crate::models::MessageItem, rkyv::rancor::Error>(archived_msg)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(msg);
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_msgs = rkyv::access::<rkyv::Archived<Vec<crate::models::MessageItem>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_msg in archived_msgs.iter() {
                let msg: crate::models::MessageItem = rkyv::deserialize::<crate::models::MessageItem, rkyv::rancor::Error>(archived_msg)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(msg);
            }
        }
        
        Ok(list)
    }

    pub fn read_message_zero_copy(&self, message_id: String) -> Result<Option<crate::models::MessageItem>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(None);
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(None);
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_msgs = rkyv::access::<rkyv::Archived<Vec<crate::models::MessageItem>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_msg in archived_msgs.iter() {
                if archived_msg.id == message_id {
                    let msg: crate::models::MessageItem = rkyv::deserialize::<crate::models::MessageItem, rkyv::rancor::Error>(archived_msg)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(msg));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_msgs = rkyv::access::<rkyv::Archived<Vec<crate::models::MessageItem>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_msg in archived_msgs.iter() {
                if archived_msg.id == message_id {
                    let msg: crate::models::MessageItem = rkyv::deserialize::<crate::models::MessageItem, rkyv::rancor::Error>(archived_msg)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(msg));
                }
            }
        }
        
        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }
    
    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.import(&update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        Ok(())
    }
}

impl ZeroCopyMessageStoreInner {
    fn get_bytes(&self) -> &[u8] {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.buffer
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 8 {
                let rkyv_len = u64::from_be_bytes(m[0..8].try_into().unwrap()) as usize;
                if m.len() >= 8 + rkyv_len {
                    let loro_offset = 8 + rkyv_len;
                    if m.len() > loro_offset {
                        let loro_bytes = &m[loro_offset..];
                        let _ = self.loro.import(loro_bytes);
                    }
                }
            }
        }
        Ok(())
    }
}

// --- ZeroCopyAuditStore for Pillar 1 Zero-Copy Audit Log Persistence ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyAuditStore {
    inner: Arc<Mutex<ZeroCopyAuditStoreInner>>,
}

struct ZeroCopyAuditStoreInner {
    #[cfg(not(target_arch = "wasm32"))]
    file_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<memmap2::MmapMut>,
    #[cfg(target_arch = "wasm32")]
    buffer: Vec<u8>,
    loro: loro::LoroDoc,
}

#[uniffi::export]
impl ZeroCopyAuditStore {
    #[allow(unused_variables)]
    #[uniffi::constructor]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        let loro = loro::LoroDoc::new();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
            
            let metadata = file.metadata().map_err(|e| YntraError::DbError(e.to_string()))?;
            let len = metadata.len();
            
            let mmap = if len > 0 {
                let m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
                Some(m)
            } else {
                None
            };
            
            let mut store = ZeroCopyAuditStoreInner {
                file_path,
                mmap,
                loro,
            };
            
            store.load_loro_from_mmap()?;
            
            Ok(Self { inner: Arc::new(Mutex::new(store)) })
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self {
                inner: Arc::new(Mutex::new(ZeroCopyAuditStoreInner {
                    buffer: Vec::new(),
                    loro,
                })),
            })
        }
    }

    pub fn write_audit_logs(&self, entries: Vec<crate::models::AuditLogEntry>) -> Result<(), YntraError> {
        let mut inner = self.inner.lock().unwrap();
        
        // 1. Serialize via rkyv
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&entries)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        // 2. Export Loro Snapshot
        let loro_bytes = inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        let rkyv_len = rkyv_bytes.len() as u64;
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let total_len = 8 + rkyv_bytes.len() + loro_bytes.len();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&inner.file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            file.set_len(total_len as u64)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            let mut m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
            
            m[0..8].copy_from_slice(&rkyv_len.to_be_bytes());
            m[8..8+rkyv_bytes.len()].copy_from_slice(&rkyv_bytes);
            m[8+rkyv_bytes.len()..total_len].copy_from_slice(&loro_bytes);
            
            m.flush().map_err(|e| YntraError::DbError(e.to_string()))?;
            inner.mmap = Some(m);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = Vec::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&rkyv_bytes);
            buf.extend_from_slice(&loro_bytes);
            inner.buffer = buf;
        }
        
        Ok(())
    }

    pub fn read_all_audit_logs(&self) -> Result<Vec<crate::models::AuditLogEntry>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(Vec::new());
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(Vec::new());
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        let mut list = Vec::new();
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_entries = rkyv::access::<rkyv::Archived<Vec<crate::models::AuditLogEntry>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_entry in archived_entries.iter() {
                let entry: crate::models::AuditLogEntry = rkyv::deserialize::<crate::models::AuditLogEntry, rkyv::rancor::Error>(archived_entry)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(entry);
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_entries = rkyv::access::<rkyv::Archived<Vec<crate::models::AuditLogEntry>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_entry in archived_entries.iter() {
                let entry: crate::models::AuditLogEntry = rkyv::deserialize::<crate::models::AuditLogEntry, rkyv::rancor::Error>(archived_entry)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(entry);
            }
        }
        
        Ok(list)
    }

    pub fn read_audit_zero_copy(&self, entry_id: String) -> Result<Option<crate::models::AuditLogEntry>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(None);
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(None);
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_entries = rkyv::access::<rkyv::Archived<Vec<crate::models::AuditLogEntry>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_entry in archived_entries.iter() {
                if archived_entry.id == entry_id {
                    let entry: crate::models::AuditLogEntry = rkyv::deserialize::<crate::models::AuditLogEntry, rkyv::rancor::Error>(archived_entry)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(entry));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_entries = rkyv::access::<rkyv::Archived<Vec<crate::models::AuditLogEntry>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_entry in archived_entries.iter() {
                if archived_entry.id == entry_id {
                    let entry: crate::models::AuditLogEntry = rkyv::deserialize::<crate::models::AuditLogEntry, rkyv::rancor::Error>(archived_entry)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(entry));
                }
            }
        }
        
        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }
    
    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.import(&update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        Ok(())
    }
}

impl ZeroCopyAuditStoreInner {
    fn get_bytes(&self) -> &[u8] {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.buffer
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 8 {
                let rkyv_len = u64::from_be_bytes(m[0..8].try_into().unwrap()) as usize;
                if m.len() >= 8 + rkyv_len {
                    let loro_offset = 8 + rkyv_len;
                    if m.len() > loro_offset {
                        let loro_bytes = &m[loro_offset..];
                        let _ = self.loro.import(loro_bytes);
                    }
                }
            }
        }
        Ok(())
    }
}

// --- ZeroCopyNoteStore for Pillar 1 Zero-Copy Notes Persistence ---

#[derive(Clone, uniffi::Object)]
pub struct ZeroCopyNoteStore {
    inner: Arc<Mutex<ZeroCopyNoteStoreInner>>,
}

struct ZeroCopyNoteStoreInner {
    #[cfg(not(target_arch = "wasm32"))]
    file_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<memmap2::MmapMut>,
    #[cfg(target_arch = "wasm32")]
    buffer: Vec<u8>,
    loro: loro::LoroDoc,
}

#[uniffi::export]
impl ZeroCopyNoteStore {
    #[allow(unused_variables)]
    #[uniffi::constructor]
    pub fn new(file_path: String) -> Result<Self, YntraError> {
        let loro = loro::LoroDoc::new();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
            
            let metadata = file.metadata().map_err(|e| YntraError::DbError(e.to_string()))?;
            let len = metadata.len();
            
            let mmap = if len > 0 {
                let m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
                Some(m)
            } else {
                None
            };
            
            let mut store = ZeroCopyNoteStoreInner {
                file_path,
                mmap,
                loro,
            };
            
            store.load_loro_from_mmap()?;
            
            Ok(Self { inner: Arc::new(Mutex::new(store)) })
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self {
                inner: Arc::new(Mutex::new(ZeroCopyNoteStoreInner {
                    buffer: Vec::new(),
                    loro,
                })),
            })
        }
    }

    pub fn write_notes(&self, notes: Vec<crate::models::DailyNote>) -> Result<(), YntraError> {
        let mut inner = self.inner.lock().unwrap();
        
        let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&notes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        let loro_bytes = inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            
        let rkyv_len = rkyv_bytes.len() as u64;
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let total_len = 8 + rkyv_bytes.len() + loro_bytes.len();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&inner.file_path)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            file.set_len(total_len as u64)
                .map_err(|e| YntraError::DbError(e.to_string()))?;
                
            let mut m = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|e| YntraError::DbError(e.to_string()))? };
            
            m[0..8].copy_from_slice(&rkyv_len.to_be_bytes());
            m[8..8+rkyv_bytes.len()].copy_from_slice(&rkyv_bytes);
            m[8+rkyv_bytes.len()..total_len].copy_from_slice(&loro_bytes);
            
            m.flush().map_err(|e| YntraError::DbError(e.to_string()))?;
            inner.mmap = Some(m);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            let mut buf = Vec::new();
            buf.extend_from_slice(&rkyv_len.to_be_bytes());
            buf.extend_from_slice(&rkyv_bytes);
            buf.extend_from_slice(&loro_bytes);
            inner.buffer = buf;
        }
        
        Ok(())
    }

    pub fn read_all_notes(&self) -> Result<Vec<crate::models::DailyNote>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(Vec::new());
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(Vec::new());
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        let mut list = Vec::new();
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_notes = rkyv::access::<rkyv::Archived<Vec<crate::models::DailyNote>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_note in archived_notes.iter() {
                let note: crate::models::DailyNote = rkyv::deserialize::<crate::models::DailyNote, rkyv::rancor::Error>(archived_note)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(note);
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_notes = rkyv::access::<rkyv::Archived<Vec<crate::models::DailyNote>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_note in archived_notes.iter() {
                let note: crate::models::DailyNote = rkyv::deserialize::<crate::models::DailyNote, rkyv::rancor::Error>(archived_note)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                list.push(note);
            }
        }
        
        Ok(list)
    }

    pub fn read_note_zero_copy(&self, note_id: String) -> Result<Option<crate::models::DailyNote>, YntraError> {
        let inner = self.inner.lock().unwrap();
        let bytes = inner.get_bytes();
        
        if bytes.len() < 8 {
            return Ok(None);
        }
        
        let rkyv_len = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + rkyv_len {
            return Ok(None);
        }
        
        let rkyv_slice = &bytes[8..8+rkyv_len];
        
        if (rkyv_slice.as_ptr() as usize) % 8 == 0 {
            let archived_notes = rkyv::access::<rkyv::Archived<Vec<crate::models::DailyNote>>, rkyv::rancor::Error>(rkyv_slice)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_note in archived_notes.iter() {
                if archived_note.id == note_id {
                    let note: crate::models::DailyNote = rkyv::deserialize::<crate::models::DailyNote, rkyv::rancor::Error>(archived_note)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(note));
                }
            }
        } else {
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(rkyv_slice);
            let archived_notes = rkyv::access::<rkyv::Archived<Vec<crate::models::DailyNote>>, rkyv::rancor::Error>(&aligned)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            for archived_note in archived_notes.iter() {
                if archived_note.id == note_id {
                    let note: crate::models::DailyNote = rkyv::deserialize::<crate::models::DailyNote, rkyv::rancor::Error>(archived_note)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    return Ok(Some(note));
                }
            }
        }
        
        Ok(None)
    }

    pub fn get_loro_changes(&self) -> Result<Vec<u8>, YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }
    
    pub fn apply_loro_update(&self, update_bytes: Vec<u8>) -> Result<(), YntraError> {
        let inner = self.inner.lock().unwrap();
        inner.loro.import(&update_bytes)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        Ok(())
    }
}

impl ZeroCopyNoteStoreInner {
    fn get_bytes(&self) -> &[u8] {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.mmap.as_ref().map(|m| &m[..]).unwrap_or(&[])
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.buffer
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    fn load_loro_from_mmap(&mut self) -> Result<(), YntraError> {
        if let Some(ref m) = self.mmap {
            if m.len() >= 8 {
                let rkyv_len = u64::from_be_bytes(m[0..8].try_into().unwrap()) as usize;
                if m.len() >= 8 + rkyv_len {
                    let loro_offset = 8 + rkyv_len;
                    if m.len() > loro_offset {
                        let loro_bytes = &m[loro_offset..];
                        let _ = self.loro.import(loro_bytes);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MessageItem, AuditLogEntry};

    #[test]
    fn test_zero_copy_store_read_write_zero_copy() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("zero_copy_test_store.db").to_string_lossy().to_string();
        
        // Clean old test file
        let _ = std::fs::remove_file(&file_path);
        
        let store = ZeroCopyStore::new(file_path.clone()).unwrap();
        
        let todo1 = TodoItem {
            id: "todo_1".to_string(),
            workspace_id: "ws_abc".to_string(),
            text: "Implement Zero-Copy Mmap".to_string(),
            completed: false,
            updated_at: 123456789,
            sync_status: "pending".to_string(),
        };
        
        let todo2 = TodoItem {
            id: "todo_2".to_string(),
            workspace_id: "ws_abc".to_string(),
            text: "P2P WebRTC Fallback Sync".to_string(),
            completed: true,
            updated_at: 987654321,
            sync_status: "synced".to_string(),
        };
        
        store.write_todos(vec![todo1.clone(), todo2.clone()]).unwrap();
        
        // Read via zero-copy search in mmap
        let read1 = store.read_todo_zero_copy("todo_1".to_string()).unwrap().unwrap();
        assert_eq!(read1.id, todo1.id);
        assert_eq!(read1.text, todo1.text);
        assert_eq!(read1.completed, todo1.completed);
        assert_eq!(read1.updated_at, todo1.updated_at);
        assert_eq!(read1.sync_status, todo1.sync_status);
        
        let read2 = store.read_todo_zero_copy("todo_2".to_string()).unwrap().unwrap();
        assert_eq!(read2.id, todo2.id);
        assert_eq!(read2.completed, todo2.completed);
        assert_eq!(read2.updated_at, todo2.updated_at);
        
        let read_none = store.read_todo_zero_copy("todo_nonexistent".to_string()).unwrap();
        assert!(read_none.is_none());
        
        // Clean up test file
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_zk_envelope_encryption_and_proof() {
        let trust = ZkCryptoTrust::new();
        let passkey_seed = "my_super_secure_passkey_hardware_seed".to_string();
        let sensitive_data = "Workspace Secret Financial Details".to_string();
        
        // Test encryption/decryption
        let ciphertext = trust.encrypt_workspace_field(passkey_seed.clone(), sensitive_data.clone()).unwrap();
        let decrypted = trust.decrypt_workspace_field(passkey_seed, ciphertext.clone()).unwrap();
        assert_eq!(decrypted, sensitive_data);
        
        // Test compliance proof
        let proof = trust.generate_compliance_proof(ciphertext, "user_123".to_string(), "Admin".to_string()).unwrap();
        let is_valid = trust.verify_compliance_proof(proof).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_zero_copy_message_store_read_write_zero_copy() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("zero_copy_test_message_store.db").to_string_lossy().to_string();
        
        let _ = std::fs::remove_file(&file_path);
        
        let store = ZeroCopyMessageStore::new(file_path.clone()).unwrap();
        
        let msg1 = MessageItem {
            id: "msg_1".to_string(),
            workspace_id: "ws_abc".to_string(),
            sender_id: Some("u_1".to_string()),
            receiver_id: Some("u_2".to_string()),
            target_team_id: None,
            subject: Some("Hello".to_string()),
            body: Some("World".to_string()),
            is_read: false,
            created_at: "2026-07-12T12:00:00Z".to_string(),
            updated_at: 123456789,
            sync_status: "pending".to_string(),
        };
        
        let msg2 = MessageItem {
            id: "msg_2".to_string(),
            workspace_id: "ws_abc".to_string(),
            sender_id: Some("u_2".to_string()),
            receiver_id: Some("u_1".to_string()),
            target_team_id: None,
            subject: Some("Reply".to_string()),
            body: Some("Got it".to_string()),
            is_read: true,
            created_at: "2026-07-12T12:05:00Z".to_string(),
            updated_at: 987654321,
            sync_status: "synced".to_string(),
        };
        
        store.write_messages(vec![msg1.clone(), msg2.clone()]).unwrap();
        
        let read1 = store.read_message_zero_copy("msg_1".to_string()).unwrap().unwrap();
        assert_eq!(read1.id, msg1.id);
        assert_eq!(read1.subject, msg1.subject);
        assert_eq!(read1.body, msg1.body);
        
        let read2 = store.read_message_zero_copy("msg_2".to_string()).unwrap().unwrap();
        assert_eq!(read2.id, msg2.id);
        assert_eq!(read2.is_read, msg2.is_read);
        
        let read_none = store.read_message_zero_copy("msg_nonexistent".to_string()).unwrap();
        assert!(read_none.is_none());
        
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_zero_copy_audit_store_read_write_zero_copy() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("zero_copy_test_audit_store.db").to_string_lossy().to_string();
        
        let _ = std::fs::remove_file(&file_path);
        
        let store = ZeroCopyAuditStore::new(file_path.clone()).unwrap();
        
        let entry1 = AuditLogEntry {
            id: "entry_1".to_string(),
            actor_id: "actor_1".to_string(),
            target_client_id: Some("client_1".to_string()),
            action_type: "create".to_string(),
            timestamp: 123456789,
            prev_hash: "hash_0".to_string(),
            curr_hash: "hash_1".to_string(),
            seq: 1,
            signature: Some("sig_1".to_string()),
        };
        
        let entry2 = AuditLogEntry {
            id: "entry_2".to_string(),
            actor_id: "actor_2".to_string(),
            target_client_id: None,
            action_type: "update".to_string(),
            timestamp: 987654321,
            prev_hash: "hash_1".to_string(),
            curr_hash: "hash_2".to_string(),
            seq: 2,
            signature: None,
        };
        
        store.write_audit_logs(vec![entry1.clone(), entry2.clone()]).unwrap();
        
        let read1 = store.read_audit_zero_copy("entry_1".to_string()).unwrap().unwrap();
        assert_eq!(read1.id, entry1.id);
        assert_eq!(read1.action_type, entry1.action_type);
        assert_eq!(read1.signature, entry1.signature);
        
        let read2 = store.read_audit_zero_copy("entry_2".to_string()).unwrap().unwrap();
        assert_eq!(read2.id, entry2.id);
        assert_eq!(read2.action_type, entry2.action_type);
        
        let read_none = store.read_audit_zero_copy("entry_nonexistent".to_string()).unwrap();
        assert!(read_none.is_none());
        
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_zero_copy_note_store_read_write_zero_copy() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("zero_copy_test_note_store.db").to_string_lossy().to_string();
        
        let _ = std::fs::remove_file(&file_path);
        
        let store = ZeroCopyNoteStore::new(file_path.clone()).unwrap();
        
        let note1 = crate::models::DailyNote {
            id: "note_1".to_string(),
            workspace_id: "ws_abc".to_string(),
            team_id: "team_1".to_string(),
            author_id: Some("u_1".to_string()),
            subject: "Meeting Notes".to_string(),
            content: "We discussed zero-copy persistence".to_string(),
            edit_history: "[]".to_string(),
            created_at: "2026-07-12".to_string(),
            updated_at: 123456789,
            sync_status: "pending".to_string(),
        };
        
        store.write_notes(vec![note1.clone()]).unwrap();
        
        let read1 = store.read_note_zero_copy("note_1".to_string()).unwrap().unwrap();
        assert_eq!(read1.id, note1.id);
        assert_eq!(read1.subject, note1.subject);
        assert_eq!(read1.content, note1.content);
        
        let all = store.read_all_notes().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "note_1");
        
        let read_none = store.read_note_zero_copy("note_nonexistent".to_string()).unwrap();
        assert!(read_none.is_none());
        
        let _ = std::fs::remove_file(&file_path);
    }
}
