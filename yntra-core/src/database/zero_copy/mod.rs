pub mod chaos;
pub mod crypto;
pub mod engine;
pub mod stores;
pub mod sync;
#[cfg(test)]
mod tests;

pub use chaos::{ChaosConfig, ChaosNetworkProxy, run_chaos_sync_load_test};
pub use crypto::{BreakGlassResult, ZkCryptoTrust};
pub use stores::{
    ZeroCopyAuditStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyStore,
    create_peer_note_store, create_peer_store,
};
pub use sync::{
    ComplianceMode, DlpInspectionResult, DlpPolicy, EdgeSyncLoop, P2PMeshSyncRouter,
    inspect_payload_dlp_bytes,
};

pub(crate) trait MutexExt<T> {
    fn lock_poison_safe(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for std::sync::Mutex<T> {
    fn lock_poison_safe(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}
