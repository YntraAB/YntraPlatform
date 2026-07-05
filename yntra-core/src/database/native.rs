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
            let db = if let (Ok(url), Ok(token)) = (std::env::var("LIBSQL_URL"), std::env::var("LIBSQL_AUTH_TOKEN")) {
                libsql::Builder::new_remote_replica("yntra_local.db", url, token)
                    .build()
                    .await
                    .expect("Failed to build remote replica database")
            } else {
                libsql::Builder::new_local("yntra_local.db")
                    .build()
                    .await
                    .expect("Failed to build local database")
            };
            
            // Try sync once if using replica
            if std::env::var("LIBSQL_URL").is_ok() {
                let _ = db.sync().await;
            }
            
            let conn = DbConnection { inner: Some(db.connect().expect("Failed to connect to libSQL database for schema setup")) };
            super::schema::setup_schema(&conn).await.expect("Failed to initialize database schema");
            
            db
        })
    })
}

pub async fn acquire_connection() -> Result<DbConnection, YntraError> {
    let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
    {
        let mut conns = pool.lock().unwrap();
        if let Some(conn) = conns.pop_front() {
            return Ok(DbConnection { inner: Some(conn) });
        }
    }
    let db = get_database();
    let conn = db.connect().map_err(|e| YntraError::DbError(e.to_string()))?;
    Ok(DbConnection { inner: Some(conn) })
}

pub struct DbConnection {
    pub inner: Option<libsql::Connection>,
}

impl Drop for DbConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.inner.take() {
            let pool = POOL.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
            if let Ok(mut conns) = pool.lock() {
                conns.push_back(conn);
            }
        }
    }
}

impl DbConnection {
    pub async fn execute<P: libsql::params::IntoParams + Send>(&self, sql: &str, params: P) -> Result<u64, YntraError> {
        let res = self.inner.as_ref().unwrap().execute(sql, params).await
            .map_err(|e| YntraError::DbError(e.to_string()));
        if res.is_ok() {
            if let Some(table) = crate::infra::observer::extract_table_name(sql) {
                crate::infra::observer::set_last_modified_table(&table);
            }
        }
        res
    }

    pub async fn execute_batch(&self, sql: &str) -> Result<(), YntraError> {
        let res = self.inner.as_ref().unwrap().execute_batch(sql).await
            .map_err(|e| YntraError::DbError(e.to_string()));
        if res.is_ok() {
            for stmt in sql.split(';') {
                if let Some(table) = crate::infra::observer::extract_table_name(stmt) {
                    crate::infra::observer::set_last_modified_table(&table);
                }
            }
        }
        res
    }

    pub async fn prepare(&self, sql: &str) -> Result<Statement, YntraError> {
        let stmt = self.inner.as_ref().unwrap().prepare(sql).await
            .map_err(|e| YntraError::DbError(e.to_string()))?;
        Ok(Statement { inner: stmt })
    }

    pub async fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, YntraError>
    where
        P: libsql::params::IntoParams + Send,
        F: FnOnce(&Row) -> Result<T, YntraError> + Send,
        T: Send,
    {
        let mut stmt = self.inner.as_ref().unwrap().prepare(sql).await.map_err(|e| YntraError::DbError(e.to_string()))?;
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
