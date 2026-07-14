pub mod crypto;
pub mod engine;
pub mod stores;
pub mod sync;
#[cfg(test)]
mod tests;

pub use crypto::ZkCryptoTrust;
pub use stores::{
    create_peer_note_store, create_peer_store, ZeroCopyAuditStore, ZeroCopyMessageStore,
    ZeroCopyNoteStore, ZeroCopyStore,
};
pub use sync::{EdgeSyncLoop, P2PMeshSyncRouter};

pub(crate) trait MutexExt<T> {
    fn lock_poison_safe(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for std::sync::Mutex<T> {
    fn lock_poison_safe(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}
