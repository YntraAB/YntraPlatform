use crate::YntraError;
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Configuration for simulated network chaos conditions
#[derive(Clone, Debug)]
pub struct ChaosConfig {
    pub dropout_rate: f64,       // Packet loss probability (0.0 = no loss, 1.0 = 100% loss)
    pub min_latency_ms: u64,     // Minimum network latency injection in ms
    pub max_latency_ms: u64,     // Maximum network latency injection in ms
    pub enable_disconnections: bool, // Simulates link dropouts and recovery cycles
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            dropout_rate: 0.25,        // 25% dropout rate by default in chaos mode
            min_latency_ms: 20,
            max_latency_ms: 250,
            enable_disconnections: true,
        }
    }
}

/// Simulated Network Proxy for inject chaos into P2P mesh CRDT synchronization
pub struct ChaosNetworkProxy {
    config: ChaosConfig,
    connected: AtomicBool,
    peer_queues: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    packets_sent: AtomicU64,
    packets_dropped: AtomicU64,
}

impl ChaosNetworkProxy {
    pub fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            connected: AtomicBool::new(true),
            peer_queues: Arc::new(Mutex::new(HashMap::new())),
            packets_sent: AtomicU64::new(0),
            packets_dropped: AtomicU64::new(0),
        }
    }

    pub fn register_peer(&self, peer_id: &str) {
        let mut queues = self.peer_queues.lock().unwrap_or_else(|e| e.into_inner());
        queues.entry(peer_id.to_string()).or_default();
    }

    pub fn set_connection_status(&self, is_connected: bool) {
        self.connected.store(is_connected, Ordering::SeqCst);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Transmit a sync delta packet with chaos simulation (dropout check & latency delay)
    pub async fn transmit(&self, _from_peer: &str, to_peer: &str, payload: Vec<u8>) -> bool {
        if !self.is_connected() {
            self.packets_dropped.fetch_add(1, Ordering::SeqCst);
            return false;
        }

        static SEED_COUNTER: AtomicU64 = AtomicU64::new(101);
        let seq = SEED_COUNTER.fetch_add(37, Ordering::Relaxed);
        let pseudo_rand = ((seq * 2654435761) % 1000) as f64 / 1000.0;

        if pseudo_rand < self.config.dropout_rate {
            self.packets_dropped.fetch_add(1, Ordering::SeqCst);
            return false;
        }

        // Simulate network latency jitter
        let latency_span = self.config.max_latency_ms.saturating_sub(self.config.min_latency_ms);
        let jitter = if latency_span > 0 {
            (seq % latency_span) + self.config.min_latency_ms
        } else {
            self.config.min_latency_ms
        };

        #[cfg(not(target_arch = "wasm32"))]
        if jitter > 0 {
            tokio::time::sleep(Duration::from_millis(jitter)).await;
        }

        let mut queues = self.peer_queues.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(queue) = queues.get_mut(to_peer) {
            queue.push(payload);
            self.packets_sent.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            self.packets_dropped.fetch_add(1, Ordering::SeqCst);
            false
        }
    }

    /// Broadcast delta snapshot updates to all registered peers under chaos simulation
    pub async fn broadcast(&self, from_peer: &str, payload: Vec<u8>) -> u64 {
        let peers: Vec<String> = {
            let queues = self.peer_queues.lock().unwrap_or_else(|e| e.into_inner());
            queues.keys().cloned().collect()
        };

        let mut delivered = 0;
        for peer in peers {
            if peer != from_peer {
                if self.transmit(from_peer, &peer, payload.clone()).await {
                    delivered += 1;
                }
            }
        }
        delivered
    }

    /// Flush and drain all queued packets for target peer
    pub fn drain_queue(&self, peer_id: &str) -> Vec<Vec<u8>> {
        let mut queues = self.peer_queues.lock().unwrap_or_else(|e| e.into_inner());
        queues.get_mut(peer_id).map(|q| std::mem::take(q)).unwrap_or_default()
    }

    pub fn stats(&self) -> (u64, u64) {
        (
            self.packets_sent.load(Ordering::SeqCst),
            self.packets_dropped.load(Ordering::SeqCst),
        )
    }
}

/// Run automated multi-device CRDT merge conflict and chaos network load test
#[uniffi::export]
pub async fn run_chaos_sync_load_test(
    num_devices: u32,
    total_mutations: u32,
    dropout_rate: f64,
    max_latency_ms: u64,
) -> Result<String, YntraError> {
    let start_time = Instant::now();
    let num_nodes = num_devices.max(2) as usize;
    let mutations_per_device = (total_mutations / num_devices.max(1)).max(1) as usize;

    let chaos_config = ChaosConfig {
        dropout_rate,
        min_latency_ms: 5,
        max_latency_ms,
        enable_disconnections: true,
    };

    let proxy = Arc::new(ChaosNetworkProxy::new(chaos_config));

    // Register all virtual peer devices
    let mut peer_ids = Vec::with_capacity(num_nodes);
    let mut docs = Vec::with_capacity(num_nodes);

    for i in 0..num_nodes {
        let peer_id = format!("chaos_peer_{}_{}", i, uuid::Uuid::new_v4());
        proxy.register_peer(&peer_id);
        peer_ids.push(peer_id);
        docs.push(loro::LoroDoc::new());
    }

    // STAGE 1: Concurrent High-Load Mutations under Chaos Network Dropouts & Latency
    for m in 0..mutations_per_device {
        for (idx, doc) in docs.iter().enumerate() {
            let text = doc.get_text("chaos_shared_content");
            let peer_id = &peer_ids[idx];
            let entry = format!("[Dev_{} Mut_{}] ", idx, m);
            text.insert(0, &entry).map_err(|e| YntraError::DbError(e.to_string()))?;

            // Export delta snapshot and broadcast through Chaos Network Proxy
            let snapshot = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::DbError(e.to_string()))?;
            proxy.broadcast(peer_id, snapshot).await;
        }

        // Simulate intermittent network disconnection halfway through load test
        if m == mutations_per_device / 2 {
            proxy.set_connection_status(false);
        }
    }

    // STAGE 2: Re-enable network connection and perform state synchronization catch-up
    proxy.set_connection_status(true);

    // Drain all delayed chaos queues and apply remaining updates to every device
    for (idx, peer_id) in peer_ids.iter().enumerate() {
        let packets = proxy.drain_queue(peer_id);
        for packet in packets {
            let _ = docs[idx].import(&packet);
        }
    }

    // Full Mesh Sync Exchange to ensure 100% convergence across all virtual devices
    for source_idx in 0..num_nodes {
        let snapshot = docs[source_idx].export(loro::ExportMode::Snapshot).map_err(|e| YntraError::DbError(e.to_string()))?;
        for target_idx in 0..num_nodes {
            if source_idx != target_idx {
                docs[target_idx].import(&snapshot).map_err(|e| YntraError::DbError(e.to_string()))?;
            }
        }
    }

    // STAGE 3: Convergence & Invariant Verification
    let primary_text = docs[0].get_text("chaos_shared_content").to_string();
    let mut all_converged = true;

    for (idx, doc) in docs.iter().enumerate().skip(1) {
        let current_text = doc.get_text("chaos_shared_content").to_string();
        if current_text != primary_text {
            all_converged = false;
            tracing::error!("Chaos convergence mismatch on peer device {}", idx);
        }
    }

    let elapsed = start_time.elapsed().as_millis() as u64;
    let (sent, dropped) = proxy.stats();

    let report = serde_json::json!({
        "status": if all_converged { "PASSED" } else { "FAILED" },
        "converged": all_converged,
        "num_devices": num_nodes,
        "total_mutations": num_nodes * mutations_per_device,
        "packets_sent": sent,
        "packets_dropped": dropped,
        "dropout_rate": dropout_rate,
        "max_latency_ms": max_latency_ms,
        "convergence_time_ms": elapsed,
        "final_content_length": primary_text.len(),
    });

    if !all_converged {
        Err(YntraError::SyncError("Multi-device CRDT chaos convergence test failed: state diverged across peers".to_string()))
    } else {
        Ok(report.to_string())
    }
}
