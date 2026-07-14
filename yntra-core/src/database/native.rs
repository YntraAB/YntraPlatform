#![cfg(not(target_arch = "wasm32"))]

use crate::YntraError;
use std::sync::{Mutex, OnceLock};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static DATABASE: OnceLock<libsql::Database> = OnceLock::new();
static POOL: OnceLock<Mutex<std::collections::VecDeque<libsql::Connection>>> = OnceLock::new();

static DATABASE_DIR: OnceLock<String> = OnceLock::new();

#[uniffi::export]
pub fn set_database_directory(dir_path: String) -> Result<(), YntraError> {
    DATABASE_DIR
        .set(dir_path)
        .map_err(|_| YntraError::CryptoError("Database directory already initialized".to_string()))
}

fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn resolve_default_database_dir() -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        // Local development: redirect to target/local_runtime/
        if let Some(ws_root) = find_workspace_root() {
            ws_root.join("target").join("local_runtime")
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join("target")
                .join("local_runtime")
        }
    } else {
        // Production: platform-specific AppData paths
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
                std::path::PathBuf::from(local_appdata).join("YntraPlatform")
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                std::path::PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("YntraPlatform")
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
                std::path::PathBuf::from(xdg_data).join("yntraplatform")
            } else if let Ok(home) = std::env::var("HOME") {
                std::path::PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("yntraplatform")
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        }
    }
}

pub fn get_database_path(filename: &str) -> String {
    let dir = if let Some(dir) = DATABASE_DIR.get() {
        std::path::PathBuf::from(dir)
    } else {
        resolve_default_database_dir()
    };

    // Ensure the resolved directory exists
    let _ = std::fs::create_dir_all(&dir);

    dir.join(filename).to_string_lossy().to_string()
}

#[cfg(test)]
static KEEP_ALIVE_CONN: OnceLock<libsql::Connection> = OnceLock::new();

pub fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .thread_name("yntra-tokio-db")
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

pub fn block_on<F: std::future::Future + 'static>(future: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send + 'static,
{
    if let Ok(_handle) = tokio::runtime::Handle::try_current() {
        // We are already inside a Tokio runtime. To prevent deadlocks on single-threaded
        // runtimes (such as WASM or certain UI runners), we spawn the future on our
        // dedicated multithreaded runtime and wait for the result via an mpsc channel.
        let (tx, rx) = std::sync::mpsc::channel();
        let rt = get_runtime();
        rt.spawn(async move {
            let res = future.await;
            let _ = tx.send(res);
        });
        rx.recv()
            .expect("Failed to receive output from block_on task")
    } else {
        // No runtime is currently active. We can directly block_on the dedicated runtime.
        let rt = get_runtime();
        rt.block_on(future)
    }
}

pub fn get_database() -> &'static libsql::Database {
    DATABASE.get_or_init(|| {
        block_on(async {
            let db_path = if cfg!(test) {
                "file:memdb1?mode=memory&cache=shared".to_string()
            } else {
                get_database_path("yntra_local.db")
            };
            let credentials = if let Some(creds) = super::sync::get_configured_credentials() {
                Some(creds)
            } else if let (Ok(url), Ok(token)) = (std::env::var("LIBSQL_URL"), std::env::var("LIBSQL_AUTH_TOKEN")) {
                Some((url, token))
            } else {
                None
            };
            let is_replica = credentials.is_some();

            let db = if let Some((url, token)) = credentials {
                libsql::Builder::new_remote_replica(&db_path, url, token)
                    .build()
                    .await
                    .expect("Failed to build remote replica database")
            } else {
                libsql::Builder::new_local(&db_path)
                    .build()
                    .await
                    .expect("Failed to build local database")
            };

            
            // Try sync once if using replica
            if is_replica {
                let _ = db.sync().await;
            }

            #[cfg(test)]
            {
                let keep_alive = db.connect().expect("Failed to create keep-alive connection");
                let _ = KEEP_ALIVE_CONN.set(keep_alive);
            }
            
            let raw_conn = db.connect().expect("Failed to connect to libSQL database for schema setup");
            let _ = raw_conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;").await;
            let conn = DbConnection {
                inner: Some(raw_conn),
                in_transaction: std::sync::atomic::AtomicBool::new(false),
                _permit: None,
            };
            super::schema::setup_schema(&conn).await.expect("Failed to initialize database schema");
            
            db
        })
     })
}

static SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

fn get_semaphore() -> &'static tokio::sync::Semaphore {
    SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(16))
}

pub async fn acquire_connection() -> Result<DbConnection, YntraError> {
    let sem = get_semaphore();
    let permit = match tokio::time::timeout(std::time::Duration::from_secs(1), sem.acquire()).await
    {
        Ok(Ok(p)) => p,
        _ => {
            return Err(YntraError::DbError(
                "Database connection pool exhausted".to_string(),
            ));
        }
    };

    let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));

    // Try to pop a connection from the pool and return it immediately
    // (A SELECT 1 query is redundant for local SQLite connections)
    let conn_opt = {
        let mut conns = pool.lock().unwrap();
        conns.pop_front()
    };

    if let Some(conn) = conn_opt {
        return Ok(DbConnection {
            inner: Some(conn),
            in_transaction: std::sync::atomic::AtomicBool::new(false),
            _permit: Some(permit),
        });
    }

    let db = get_database();
    let conn = db
        .connect()
        .map_err(|e| YntraError::DbError(e.to_string()))?;
    let _ = conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;").await;
    Ok(DbConnection {
        inner: Some(conn),
        in_transaction: std::sync::atomic::AtomicBool::new(false),
        _permit: Some(permit),
    })
}

pub struct DbConnection {
    pub inner: Option<libsql::Connection>,
    pub in_transaction: std::sync::atomic::AtomicBool,
    pub _permit: Option<tokio::sync::SemaphorePermit<'static>>,
}

impl Drop for DbConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.inner.take() {
            let was_in_tx = self
                .in_transaction
                .load(std::sync::atomic::Ordering::SeqCst);
            let permit = self._permit.take();
            if was_in_tx {
                let rt = get_runtime();
                rt.spawn(async move {
                    // Try to rollback the active transaction. If it fails, discard connection.
                    if let Err(e) = conn.execute("ROLLBACK", ()).await {
                        tracing::warn!("Failed to rollback database connection on drop: {:?}. Discarding connection.", e);
                        return;
                    }
                    
                    let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
                    if let Ok(mut conns) = pool.lock() {
                        conns.push_back(conn);
                    }
                    drop(permit);
                });
            } else {
                let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
                if let Ok(mut conns) = pool.lock() {
                    conns.push_back(conn);
                }
                drop(permit);
            }
        }
    }
}

impl DbConnection {
    fn get_conn(&self) -> Result<&libsql::Connection, YntraError> {
        self.inner
            .as_ref()
            .ok_or_else(|| YntraError::DbError("Connection already closed".to_string()))
    }

    pub async fn begin_transaction(&self) -> Result<(), YntraError> {
        self.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;
        self.in_transaction
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub async fn commit(&self) -> Result<(), YntraError> {
        self.execute("COMMIT", ()).await?;
        self.in_transaction
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub async fn rollback(&self) -> Result<(), YntraError> {
        self.execute("ROLLBACK", ()).await?;
        self.in_transaction
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub async fn execute<P: libsql::params::IntoParams + Send>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<u64, YntraError> {
        let conn = self.get_conn()?;
        if let Some(in_tx) = super::check_transaction_sql(sql) {
            self.in_transaction
                .store(in_tx, std::sync::atomic::Ordering::SeqCst);
        }
        let res = conn
            .execute(sql, params)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()));
        if res.is_ok() {
            let is_rollback = sql.trim_start().len() >= 8
                && sql.trim_start()[..8].eq_ignore_ascii_case("ROLLBACK");
            if is_rollback {
                crate::infra::observer::discard_observers_dirty_state();
            } else {
                super::track_write(sql);
                if !self
                    .in_transaction
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    crate::infra::observer::notify_observers();
                }
            }
        }
        res
    }

    pub async fn execute_batch(&self, sql: &str) -> Result<(), YntraError> {
        let conn = self.get_conn()?;
        for stmt in super::parser::split_sql_statements(sql) {
            if let Some(in_tx) = super::check_transaction_sql(stmt) {
                self.in_transaction
                    .store(in_tx, std::sync::atomic::Ordering::SeqCst);
            }
        }
        conn.execute_batch(sql)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;

        let is_rollback =
            sql.trim_start().len() >= 8 && sql.trim_start()[..8].eq_ignore_ascii_case("ROLLBACK");
        if is_rollback {
            crate::infra::observer::discard_observers_dirty_state();
        } else {
            super::track_write_batch(sql);
            if !self
                .in_transaction
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                crate::infra::observer::notify_observers();
            }
        }
        Ok(())
    }

    pub async fn prepare(&self, sql: &str) -> Result<Statement, YntraError> {
        let conn = self.get_conn()?;
        let stmt = conn
            .prepare(sql)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        Ok(Statement { inner: stmt })
    }

    pub async fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, YntraError>
    where
        P: libsql::params::IntoParams + Send,
        F: FnOnce(&Row) -> Result<T, YntraError> + Send,
        T: Send,
    {
        let conn = self.get_conn()?;
        let mut stmt = conn
            .prepare(sql)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        let mut rows = stmt
            .query(params)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?
        {
            let wrapped_row = Row { inner: row };
            f(&wrapped_row)
        } else {
            Err(YntraError::NoRowsReturned)
        }
    }
}

pub struct Statement {
    inner: libsql::Statement,
}

impl Statement {
    pub async fn query<P: libsql::params::IntoParams + Send>(
        &mut self,
        params: P,
    ) -> Result<Rows, YntraError> {
        let rows = self
            .inner
            .query(params)
            .await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        Ok(Rows { inner: rows })
    }

    pub async fn query_map<T, F, P>(&mut self, params: P, mut f: F) -> Result<Vec<T>, YntraError>
    where
        P: libsql::params::IntoParams + Send,
        F: FnMut(&Row) -> Result<T, YntraError> + Send,
        T: Send,
    {
        let mut rows = self.query(params).await?;
        let mut list = Vec::new();
        while let Some(row) = rows.next().await? {
            list.push(f(&row)?);
        }
        Ok(list)
    }
}

pub struct Rows {
    inner: libsql::Rows,
}

impl Rows {
    pub async fn next(&mut self) -> Result<Option<Row>, YntraError> {
        match self.inner.next().await {
            Ok(Some(row)) => Ok(Some(Row { inner: row })),
            Ok(None) => Ok(None),
            Err(e) => Err(YntraError::DbError(e.to_string())),
        }
    }
}

pub struct Row {
    inner: libsql::Row,
}

impl Row {
    pub fn get<T: FromLibsqlRow>(&self, idx: i32) -> Result<T, YntraError> {
        T::get_from_row(&self.inner, idx)
    }
}

pub trait FromLibsqlRow: Sized {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError>;
}

impl FromLibsqlRow for String {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<String>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for i64 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<i64>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for i32 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        match row.get::<i64>(idx) {
            Ok(val) => i32::try_from(val).map_err(|_| {
                YntraError::DbError(format!(
                    "Integer overflow: value {} at index {} exceeds 32-bit range",
                    val, idx
                ))
            }),
            Err(e) => Err(YntraError::DbError(e.to_string())),
        }
    }
}

impl FromLibsqlRow for f64 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<f64>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for bool {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<bool>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<String> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<String>>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<i64> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<i64>>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<i32> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        match row.get::<Option<i64>>(idx) {
            Ok(val) => match val {
                Some(v) => {
                    let converted = i32::try_from(v).map_err(|_| {
                        YntraError::DbError(format!(
                            "Integer overflow: value {} at index {} exceeds 32-bit range",
                            v, idx
                        ))
                    })?;
                    Ok(Some(converted))
                }
                None => Ok(None),
            },
            Err(e) => Err(YntraError::DbError(e.to_string())),
        }
    }
}

impl FromLibsqlRow for Option<f64> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<f64>>(idx)
            .map_err(|e| YntraError::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_auto_rollback_on_drop() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        // 1. Establish initial data
        let conn = acquire_connection().await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-tx-drop', 'Tx Drop WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('user-tx-drop-1', 'ws-tx-drop', 'user1@tx.io', 'user')", ()).await.unwrap();

        // 2. Start a transaction on a separate scope and drop it mid-transaction
        {
            let conn_tx = acquire_connection().await.unwrap();
            conn_tx.begin_transaction().await.unwrap();
            conn_tx.execute("INSERT INTO users (id, workspace_id, email, role) VALUES ('user-tx-drop-2', 'ws-tx-drop', 'user2@tx.io', 'user')", ()).await.unwrap();
            // Drop conn_tx here without committing. It should auto-rollback.
        }

        // Give the background task time to finalize dropping the connection and release locks
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 3. Acquire a new connection and verify that user-2 was rolled back, but user-1 exists
        let conn_new = acquire_connection().await.unwrap();
        let user1_exists = conn_new
            .query_row(
                "SELECT COUNT(*) FROM users WHERE id = 'user-tx-drop-1'",
                (),
                |r| r.get::<i64>(0),
            )
            .await
            .unwrap();
        assert_eq!(user1_exists, 1);

        let user2_exists = conn_new
            .query_row(
                "SELECT COUNT(*) FROM users WHERE id = 'user-tx-drop-2'",
                (),
                |r| r.get::<i64>(0),
            )
            .await
            .unwrap();
        assert_eq!(user2_exists, 0);

        // Cleanup
        conn_new
            .execute("DELETE FROM users WHERE workspace_id = 'ws-tx-drop'", ())
            .await
            .unwrap();
        conn_new
            .execute("DELETE FROM workspaces WHERE id = 'ws-tx-drop'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_connection_pool_limits() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        // Acquire 16 connections (this should consume all semaphore permits)
        let mut connections = Vec::new();
        for _ in 0..16 {
            let conn = acquire_connection().await;
            assert!(conn.is_ok());
            connections.push(conn.unwrap());
        }

        // The 17th acquisition should time out and return a pool exhaustion error
        let conn_17 = acquire_connection().await;
        assert!(conn_17.is_err());
        if let Err(YntraError::DbError(msg)) = conn_17 {
            assert!(
                msg.contains("Database connection pool exhausted"),
                "Got unexpected error msg: {}",
                msg
            );
        } else {
            panic!("Expected DbError for connection pool exhaustion");
        }

        // Drop one connection to free a permit
        connections.pop();

        // Now we should be able to acquire a connection successfully again
        let conn_retry = acquire_connection().await;
        assert!(conn_retry.is_ok());
    }

    #[test]
    fn test_dynamic_database_directory_resolution() {
        // Test resolution with no directory configured
        let path1 = get_database_path("test_file.db");
        let expected_default = resolve_default_database_dir().join("test_file.db");
        assert_eq!(path1, expected_default.to_string_lossy().to_string());

        // Set the directory if not already set
        if DATABASE_DIR.get().is_none() {
            let set_res = set_database_directory("/tmp/yntra_test_sandbox".to_string());
            assert!(set_res.is_ok());
        }

        // Test resolution with directory configured
        let path2 = get_database_path("test_file.db");
        let configured_dir = DATABASE_DIR.get().unwrap();
        let expected = std::path::PathBuf::from(configured_dir).join("test_file.db");
        assert_eq!(path2, expected.to_string_lossy().to_string());

        // Attempting to set directory again should fail
        let set_res2 = set_database_directory("/another/path".to_string());
        assert!(set_res2.is_err());
    }
}
