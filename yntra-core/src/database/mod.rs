#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{DbConnection, Row, Rows, Statement, acquire_connection};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::{DbConnection, Row, Rows, Statement, acquire_connection};

pub mod schema;
pub use schema::setup_schema;

pub mod parser;
pub mod sync;
pub mod zero_copy;
pub use zero_copy::{
    EdgeSyncLoop, P2PMeshSyncRouter, ZeroCopyAuditStore, ZeroCopyMessageStore, ZeroCopyNoteStore,
    ZeroCopyStore, ZkCryptoTrust,
};

pub mod proxy;
pub use proxy::RemoteSyncCoordinator;

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
    pub fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ()>, std::sync::PoisonError<std::sync::MutexGuard<'_, ()>>>
    {
        let mutex = self.inner.get_or_init(|| std::sync::Mutex::new(()));
        match mutex.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => Ok(poisoned.into_inner()),
        }
    }
}

fn contains_word_ignore_ascii_case(sql: &str, word: &str) -> bool {
    let bytes = sql.as_bytes();
    let needle = word.as_bytes();
    if bytes.len() < needle.len() {
        return false;
    }
    for i in 0..=(bytes.len() - needle.len()) {
        if bytes[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            return true;
        }
    }
    false
}

pub fn track_write(sql: &str) {
    if let Some(table) = self::parser::extract_table_name(sql) {
        if table == "users" || table == "workspaces" {
            crate::infra::auth::invalidate_auth_context_cache_for_sql(sql, &table);
        }
        crate::infra::observer::set_last_modified_table(&table);
    } else {
        if self::parser::has_write_keyword(sql) {
            let has_users = contains_word_ignore_ascii_case(sql, "users");
            let has_workspaces = contains_word_ignore_ascii_case(sql, "workspaces");
            if has_users || has_workspaces {
                tracing::warn!("SQL write parser failed to extract table name. Invalidating entire auth context cache to ensure security.");
                crate::infra::auth::invalidate_auth_context_cache();
            }
        }
    }
}

pub fn track_write_batch(sql: &str) {
    for stmt in self::parser::split_sql_statements(sql) {
        if let Some(table) = self::parser::extract_table_name(stmt) {
            if table == "users" || table == "workspaces" {
                crate::infra::auth::invalidate_auth_context_cache_for_sql(stmt, &table);
            }
            crate::infra::observer::set_last_modified_table(&table);
        } else {
            if self::parser::has_write_keyword(stmt) {
                let has_users = contains_word_ignore_ascii_case(stmt, "users");
                let has_workspaces = contains_word_ignore_ascii_case(stmt, "workspaces");
                if has_users || has_workspaces {
                    tracing::warn!("SQL batch write parser failed to extract table name. Invalidating entire auth context cache to ensure security.");
                    crate::infra::auth::invalidate_auth_context_cache();
                }
            }
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
        assert_eq!(
            check_transaction_sql("-- test\nBEGIN IMMEDIATE TRANSACTION;"),
            Some(true)
        );
        assert_eq!(check_transaction_sql("/* comment */ COMMIT;"), Some(false));
        assert_eq!(
            check_transaction_sql("   -- comment\n   ROLLBACK;"),
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_track_write_parser_fallback() {
        let _lock = DB_TEST_LOCK.lock().unwrap();
        let ws_id = "test-fallback-ws".to_string();
        let user_id = "test-fallback-user".to_string();
        crate::infra::auth::insert_auth_context_cache(&user_id, crate::infra::auth::AuthContext {
            user_id: user_id.clone(),
            workspace_id: ws_id.clone(),
            role: "admin".to_string(),
            is_admin: true,
            workspace_settings: None,
        });

        // Ensure it is cached
        assert!(crate::infra::auth::get_auth_context_cache(&user_id).is_some());

        // Run a query with leading semicolon to trigger parsing failure but containing USERS
        track_write("; UPDATE users SET name = 'fail'");

        // Cache must be invalidated (cleared entirely)
        assert!(crate::infra::auth::get_auth_context_cache(&user_id).is_none());
    }
}
