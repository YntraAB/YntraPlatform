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

pub mod parser;
pub mod sync;

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

pub fn track_write(sql: &str) {
    if let Some(table) = self::parser::extract_table_name(sql) {
        crate::infra::observer::set_last_modified_table(&table);
    }
}

pub fn track_write_batch(sql: &str) {
    for stmt in self::parser::split_sql_statements(sql) {
        if let Some(table) = self::parser::extract_table_name(&stmt) {
            crate::infra::observer::set_last_modified_table(&table);
        }
    }
}


