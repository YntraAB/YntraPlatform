// wasm.rs
// WebAssembly database adapter for yntra-core using JS worker bridge

use crate::YntraError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasm_bindgen::prelude::*;

// A wrapper to make WASM futures Send.
// Safety: Since WebAssembly (wasm32-unknown-unknown) is single-threaded in the browser,
// implementing Send for JS-bound futures is safe and required to satisfy generic bounds.
pub struct SendFuture<F> {
    inner: F,
    thread_id: std::thread::ThreadId,
}

impl<F> SendFuture<F> {
    pub fn new(inner: F) -> Self {
        Self {
            inner,
            thread_id: std::thread::current().id(),
        }
    }
}

#[cfg(target_feature = "atomics")]
unsafe impl<F: Send> Send for SendFuture<F> {}

#[cfg(not(target_feature = "atomics"))]
unsafe impl<F> Send for SendFuture<F> {}

impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut_self = unsafe { self.get_unchecked_mut() };
        if std::thread::current().id() != mut_self.thread_id {
            panic!("Safety violation: SendFuture polled on a different thread under WASM.");
        }
        let inner = unsafe { Pin::new_unchecked(&mut mut_self.inner) };
        inner.poll(cx)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_execute_sql, catch)]
    async fn js_execute_sql_internal(
        query_type: &str,
        sql: &str,
        params: JsValue,
    ) -> Result<JsValue, JsValue>;
}

async fn js_execute_sql(
    query_type: &str,
    sql: &str,
    params: JsValue,
) -> Result<JsValue, YntraError> {
    let fut = js_execute_sql_internal(query_type, sql, params);
    let send_fut = SendFuture::new(fut);
    match send_fut.await {
        Ok(js_val) => Ok(js_val),
        Err(js_err) => {
            let err_msg = js_err
                .as_string()
                .unwrap_or_else(|| "Unknown JavaScript error during SQL execution".to_string());
            Err(YntraError::DbError(err_msg))
        }
    }
}

#[derive(serde::Deserialize)]
struct ExecuteResult {
    #[serde(rename = "rowsAffected", default)]
    rows_affected: u64,
}

pub trait IntoWasmParams {
    fn into_wasm_params(self) -> serde_json::Value;
}

impl IntoWasmParams for Vec<serde_json::Value> {
    fn into_wasm_params(self) -> serde_json::Value {
        serde_json::Value::Array(self)
    }
}

impl IntoWasmParams for () {
    fn into_wasm_params(self) -> serde_json::Value {
        serde_json::Value::Array(vec![])
    }
}

impl IntoWasmParams for serde_json::Value {
    fn into_wasm_params(self) -> serde_json::Value {
        self
    }
}

static DB_LOCK: std::sync::OnceLock<futures_util::lock::Mutex<()>> = std::sync::OnceLock::new();

fn get_db_lock() -> &'static futures_util::lock::Mutex<()> {
    DB_LOCK.get_or_init(|| futures_util::lock::Mutex::new(()))
}

pub async fn acquire_connection() -> Result<DbConnection, YntraError> {
    let guard = get_db_lock().lock().await;
    Ok(DbConnection {
        in_transaction: std::sync::atomic::AtomicBool::new(false),
        _guard: Some(guard),
    })
}

pub struct DbConnection {
    pub in_transaction: std::sync::atomic::AtomicBool,
    pub _guard: Option<futures_util::lock::MutexGuard<'static, ()>>,
}

impl Drop for DbConnection {
    fn drop(&mut self) {
        let guard = self._guard.take();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = js_execute_sql("execute", "ROLLBACK", wasm_bindgen::JsValue::null()).await;
            drop(guard);
        });
    }
}

impl DbConnection {
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

    /// Executes a SQL write statement.
    /// Note: Parameters and results are passed directly as JsValue objects over the WASM/JS boundary
    /// avoiding intermediate JSON string serialization.
    pub async fn execute<P: IntoWasmParams>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<u64, YntraError> {
        let tx_state_update = super::check_transaction_sql(sql);
        let params_wasm = params.into_wasm_params();
        let params_val = serde_wasm_bindgen::to_value(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_val = js_execute_sql("execute", sql, params_val).await?;
        let res: ExecuteResult = serde_wasm_bindgen::from_value(result_val)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;

        if let Some(in_tx) = tx_state_update {
            self.in_transaction
                .store(in_tx, std::sync::atomic::Ordering::SeqCst);
        }

        let is_rollback =
            sql.trim_start().len() >= 8 && sql.trim_start()[..8].eq_ignore_ascii_case("ROLLBACK");
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
        Ok(res.rows_affected)
    }

    /// Executes a batch of SQL statements.
    pub async fn execute_batch(&self, sql: &str) -> Result<(), YntraError> {
        let mut last_tx_state = None;
        for stmt in super::parser::split_sql_statements(sql) {
            if let Some(in_tx) = super::check_transaction_sql(stmt) {
                last_tx_state = Some(in_tx);
            }
        }
        js_execute_sql("execute_batch", sql, JsValue::UNDEFINED).await?;

        if let Some(in_tx) = last_tx_state {
            self.in_transaction
                .store(in_tx, std::sync::atomic::Ordering::SeqCst);
        }

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
        Ok(Statement {
            sql: sql.to_string(),
        })
    }

    pub async fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, YntraError>
    where
        P: IntoWasmParams,
        F: FnOnce(&Row) -> Result<T, YntraError> + Send,
        T: Send,
    {
        let params_wasm = params.into_wasm_params();
        let params_val = serde_wasm_bindgen::to_value(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_val = js_execute_sql("query", sql, params_val).await?;
        let array = js_sys::Array::from(&result_val);
        if array.length() == 0 {
            return Err(YntraError::NoRowsReturned);
        }
        let row = Row {
            value: array.get(0),
        };
        f(&row)
    }
}

pub struct Statement {
    pub sql: String,
}

impl Statement {
    pub async fn query<P: IntoWasmParams>(&mut self, params: P) -> Result<Rows, YntraError> {
        let params_wasm = params.into_wasm_params();
        let params_val = serde_wasm_bindgen::to_value(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_val = js_execute_sql("query", &self.sql, params_val).await?;
        let array = js_sys::Array::from(&result_val);
        Ok(Rows {
            rows: array,
            index: 0,
        })
    }

    pub async fn query_map<T, F, P>(&mut self, params: P, mut f: F) -> Result<Vec<T>, YntraError>
    where
        P: IntoWasmParams,
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
    rows: js_sys::Array,
    index: usize,
}

impl Rows {
    pub async fn next(&mut self) -> Result<Option<Row>, YntraError> {
        if self.index < self.rows.length() as usize {
            let val = self.rows.get(self.index as u32);
            self.index += 1;
            Ok(Some(Row { value: val }))
        } else {
            Ok(None)
        }
    }
}

pub struct Row {
    value: JsValue,
}

impl Row {
    pub fn get<T: serde::de::DeserializeOwned>(&self, idx: i32) -> Result<T, YntraError> {
        let array = js_sys::Array::from(&self.value);
        if idx < 0 || idx >= array.length() as i32 {
            return Err(YntraError::DbError(format!(
                "Column index out of bounds: {}",
                idx
            )));
        }
        let val = array.get(idx as u32);
        serde_wasm_bindgen::from_value(val).map_err(|e| {
            YntraError::DbError(format!(
                "Failed to deserialize column at index {}: {}",
                idx, e
            ))
        })
    }
}

pub fn params_from_iter<I, T>(iter: I) -> Vec<serde_json::Value>
where
    I: IntoIterator<Item = T>,
    T: crate::rusqlite::ToWasmValue,
{
    iter.into_iter().map(|x| x.to_value()).collect()
}
