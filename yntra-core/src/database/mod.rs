#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{acquire_connection, DbConnection, Statement, Row, Rows};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::{acquire_connection, DbConnection, Statement, Row, Rows};

pub mod schema;
pub use schema::setup_schema;

#[cfg(not(target_arch = "wasm32"))]
pub static DB_TEST_LOCK: DbTestLock = DbTestLock {
    inner: std::sync::OnceLock::new(),
};

#[cfg(not(target_arch = "wasm32"))]
pub struct DbTestLock {
    inner: std::sync::OnceLock<std::sync::Mutex<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DbTestLock {
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, std::sync::PoisonError<std::sync::MutexGuard<'_, ()>>> {
        let mutex = self.inner.get_or_init(|| std::sync::Mutex::new(()));
        match mutex.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                Ok(poisoned.into_inner())
            }
        }
    }
}


