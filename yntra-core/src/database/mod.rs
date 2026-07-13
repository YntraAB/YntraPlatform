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
pub mod zero_copy;
pub use zero_copy::{ZeroCopyStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyAuditStore, P2PMeshSyncRouter, EdgeSyncLoop, ZkCryptoTrust};


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
        if table == "users" || table == "workspaces" {
            crate::infra::auth::invalidate_auth_context_cache();
        }
        crate::infra::observer::set_last_modified_table(&table);
    }
}

pub fn track_write_batch(sql: &str) {
    for stmt in self::parser::split_sql_statements(sql) {
        if let Some(table) = self::parser::extract_table_name(stmt) {
            if table == "users" || table == "workspaces" {
                crate::infra::auth::invalidate_auth_context_cache();
            }
            crate::infra::observer::set_last_modified_table(&table);
        }
    }
}

pub fn check_transaction_sql(sql: &str) -> Option<bool> {
    let cleaned_owned;
    let has_comments = sql.contains("/*") || sql.contains("--");
    let sql_trimmed = if has_comments {
        cleaned_owned = self::parser::clean_sql(sql);
        cleaned_owned.trim_start()
    } else {
        sql.trim_start()
    };
    if sql_trimmed.len() >= 5 {
        let prefix = &sql_trimmed[..5];
        if prefix.eq_ignore_ascii_case("BEGIN") {
            return Some(true);
        }
    }
    if sql_trimmed.len() >= 6 {
        let prefix = &sql_trimmed[..6];
        if prefix.eq_ignore_ascii_case("COMMIT") {
            return Some(false);
        }
    }
    if sql_trimmed.len() >= 8 {
        let prefix = &sql_trimmed[..8];
        if prefix.eq_ignore_ascii_case("ROLLBACK") {
            return Some(false);
        }
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_transaction_sql_comments() {
        assert_eq!(check_transaction_sql("-- test\nBEGIN IMMEDIATE TRANSACTION;"), Some(true));
        assert_eq!(check_transaction_sql("/* comment */ COMMIT;"), Some(false));
        assert_eq!(check_transaction_sql("   -- comment\n   ROLLBACK;"), Some(false));
    }
}


