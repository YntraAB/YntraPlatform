#![cfg(not(target_arch = "wasm32"))]

use std::sync::{Mutex, OnceLock};
use crate::YntraError;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static DATABASE: OnceLock<libsql::Database> = OnceLock::new();
static POOL: OnceLock<Mutex<std::collections::VecDeque<libsql::Connection>>> = OnceLock::new();

pub fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

pub fn block_on<F: std::future::Future>(future: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    if let Ok(_handle) = tokio::runtime::Handle::try_current() {
        std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = get_runtime();
                rt.block_on(future)
            });
            handle.join().unwrap()
        })
    } else {
        let rt = get_runtime();
        rt.block_on(future)
    }
}

pub fn get_database() -> &'static libsql::Database {
    DATABASE.get_or_init(|| {
        block_on(async {
            let db_path = if cfg!(test) {
                "file:memdb1?mode=memory&cache=shared"
            } else {
                "yntra_local.db"
            };
            let db = if let (Ok(url), Ok(token)) = (std::env::var("LIBSQL_URL"), std::env::var("LIBSQL_AUTH_TOKEN")) {
                libsql::Builder::new_remote_replica(db_path, url, token)
                    .build()
                    .await
                    .expect("Failed to build remote replica database")
            } else {
                libsql::Builder::new_local(db_path)
                    .build()
                    .await
                    .expect("Failed to build local database")
            };
            
            // Try sync once if using replica
            if std::env::var("LIBSQL_URL").is_ok() {
                let _ = db.sync().await;
            }
            
            let raw_conn = db.connect().expect("Failed to connect to libSQL database for schema setup");
            let _ = raw_conn.execute("PRAGMA foreign_keys = ON", ()).await;
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
    let permit = match tokio::time::timeout(std::time::Duration::from_secs(1), sem.acquire()).await {
        Ok(Ok(p)) => p,
        _ => return Err(YntraError::DbError("Database connection pool exhausted".to_string())),
    };

    let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
    {
        let mut conns = pool.lock().unwrap();
        if let Some(conn) = conns.pop_front() {
            return Ok(DbConnection {
                inner: Some(conn),
                in_transaction: std::sync::atomic::AtomicBool::new(false),
                _permit: Some(permit),
            });
        }
    }

    let db = get_database();
    let conn = db.connect().map_err(|e| YntraError::DbError(e.to_string()))?;
    let _ = conn.execute("PRAGMA foreign_keys = ON", ()).await;
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
            if self.in_transaction.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = block_on(async {
                    let _ = conn.execute("ROLLBACK", ()).await;
                });
            }
            let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
            if let Ok(mut conns) = pool.lock() {
                conns.push_back(conn);
            }
        }
    }
}
 
 impl DbConnection {
     fn get_conn(&self) -> Result<&libsql::Connection, YntraError> {
         self.inner.as_ref().ok_or_else(|| YntraError::DbError("Connection already closed".to_string()))
     }

     pub async fn begin_transaction(&self) -> Result<(), YntraError> {
         self.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;
         self.in_transaction.store(true, std::sync::atomic::Ordering::SeqCst);
         Ok(())
     }

     pub async fn commit(&self) -> Result<(), YntraError> {
         self.execute("COMMIT", ()).await?;
         self.in_transaction.store(false, std::sync::atomic::Ordering::SeqCst);
         Ok(())
     }

     pub async fn rollback(&self) -> Result<(), YntraError> {
         self.execute("ROLLBACK", ()).await?;
         self.in_transaction.store(false, std::sync::atomic::Ordering::SeqCst);
         Ok(())
     }

     pub async fn execute<P: libsql::params::IntoParams + Send>(&self, sql: &str, params: P) -> Result<u64, YntraError> {
         let sql_upper = sql.to_uppercase();
         if sql_upper.contains("BEGIN") {
             self.in_transaction.store(true, std::sync::atomic::Ordering::SeqCst);
         }
         if sql_upper.contains("COMMIT") || sql_upper.contains("ROLLBACK") {
             self.in_transaction.store(false, std::sync::atomic::Ordering::SeqCst);
         }

         let conn = self.get_conn()?;
         let res = conn.execute(sql, params).await
             .map_err(|e| YntraError::DbError(e.to_string()));
         if res.is_ok() {
             if let Some(table) = crate::infra::observer::extract_table_name(sql) {
                 crate::infra::observer::set_last_modified_table(&table);
             }
         }
         res
     }
 
     pub async fn execute_batch(&self, sql: &str) -> Result<(), YntraError> {
        let sql_upper = sql.to_uppercase();
        if sql_upper.contains("BEGIN") {
            self.in_transaction.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if sql_upper.contains("COMMIT") || sql_upper.contains("ROLLBACK") {
            self.in_transaction.store(false, std::sync::atomic::Ordering::SeqCst);
        }

        let conn = self.get_conn()?;
        conn.execute_batch(sql).await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        for stmt in crate::infra::observer::split_sql_statements(sql) {
            if let Some(table) = crate::infra::observer::extract_table_name(&stmt) {
                crate::infra::observer::set_last_modified_table(&table);
            }
        }
        Ok(())
    }

    pub async fn prepare(&self, sql: &str) -> Result<Statement, YntraError> {
        let conn = self.get_conn()?;
        let stmt = conn.prepare(sql).await
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
        let mut stmt = conn.prepare(sql).await.map_err(|e| YntraError::DbError(e.to_string()))?;
        let mut rows = stmt.query(params).await.map_err(|e| YntraError::DbError(e.to_string()))?;
        if let Some(row) = rows.next().await.map_err(|e| YntraError::DbError(e.to_string()))? {
            let wrapped_row = Row { inner: row };
            f(&wrapped_row)
        } else {
            Err(YntraError::DbError("No row returned".to_string()))
        }
    }
}

pub struct Statement {
    inner: libsql::Statement,
}

impl Statement {
    pub async fn query<P: libsql::params::IntoParams + Send>(&mut self, params: P) -> Result<Rows, YntraError> {
        let rows = self.inner.query(params).await.map_err(|e| YntraError::DbError(e.to_string()))?;
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
        row.get::<String>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for i64 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<i64>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for i32 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<i32>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for f64 {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<f64>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for bool {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<bool>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<String> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<String>>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<i64> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<i64>>(idx).map_err(|e| YntraError::DbError(e.to_string()))
    }
}

impl FromLibsqlRow for Option<i32> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        match row.get::<Option<i64>>(idx) {
            Ok(val) => Ok(val.map(|v| v as i32)),
            Err(e) => Err(YntraError::DbError(e.to_string())),
        }
    }
}

impl FromLibsqlRow for Option<f64> {
    fn get_from_row(row: &libsql::Row, idx: i32) -> Result<Self, YntraError> {
        row.get::<Option<f64>>(idx).map_err(|e| YntraError::DbError(e.to_string()))
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

        // 3. Acquire a new connection and verify that user-2 was rolled back, but user-1 exists
        let conn_new = acquire_connection().await.unwrap();
        let user1_exists = conn_new.query_row(
            "SELECT COUNT(*) FROM users WHERE id = 'user-tx-drop-1'",
            (),
            |r| r.get::<i64>(0)
        ).await.unwrap_or(0);
        assert_eq!(user1_exists, 1);

        let user2_exists = conn_new.query_row(
            "SELECT COUNT(*) FROM users WHERE id = 'user-tx-drop-2'",
            (),
            |r| r.get::<i64>(0)
        ).await.unwrap_or(0);
        assert_eq!(user2_exists, 0);

        // Cleanup
        conn_new.execute("DELETE FROM users WHERE workspace_id = 'ws-tx-drop'", ()).await.unwrap();
        conn_new.execute("DELETE FROM workspaces WHERE id = 'ws-tx-drop'", ()).await.unwrap();
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
            assert!(msg.contains("Database connection pool exhausted"), "Got unexpected error msg: {}", msg);
        } else {
            panic!("Expected DbError for connection pool exhaustion");
        }

        // Drop one connection to free a permit
        connections.pop();

        // Now we should be able to acquire a connection successfully again
        let conn_retry = acquire_connection().await;
        assert!(conn_retry.is_ok());
    }
}
