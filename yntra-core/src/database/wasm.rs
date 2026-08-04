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

#[cfg(target_arch = "wasm32")]
pub async fn yield_to_browser_event_loop() {
    let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::NULL);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
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

pub(crate) async fn js_execute_sql(
    query_type: &str,
    sql: &str,
    params: JsValue,
) -> Result<JsValue, YntraError> {
    let fut = js_execute_sql_internal(query_type, sql, params);
    let send_fut = SendFuture::new(fut);
    match send_fut.await {
        Ok(js_val) => Ok(js_val),
        Err(js_err) => {
            let err_msg = if let Some(s) = js_err.as_string() {
                s
            } else {
                let err_obj = js_sys::Error::from(js_err);
                String::from(err_obj.to_string())
            };
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
        let in_tx = self
            .in_transaction
            .load(std::sync::atomic::Ordering::SeqCst);
        wasm_bindgen_futures::spawn_local(async move {
            if in_tx {
                let _ = js_execute_sql("execute", "ROLLBACK", wasm_bindgen::JsValue::null()).await;
            }
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
        let mut contains_begin = false;
        let mut last_tx_state = None;
        for stmt in super::parser::split_sql_statements(sql) {
            if let Some(in_tx) = super::check_transaction_sql(stmt) {
                if in_tx {
                    contains_begin = true;
                }
                last_tx_state = Some(in_tx);
            }
        }

        if contains_begin {
            self.in_transaction
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        let res = js_execute_sql("execute_batch", sql, JsValue::UNDEFINED).await;

        if res.is_ok() {
            if let Some(in_tx) = last_tx_state {
                self.in_transaction
                    .store(in_tx, std::sync::atomic::Ordering::SeqCst);
            }
        }

        let is_rollback =
            sql.trim_start().len() >= 8 && sql.trim_start()[..8].eq_ignore_ascii_case("ROLLBACK");
        if is_rollback {
            crate::infra::observer::discard_observers_dirty_state();
        } else if res.is_ok() {
            super::track_write_batch(sql);
            if !self
                .in_transaction
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                crate::infra::observer::notify_observers();
            }
        }

        res.map(|_| ())
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

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub fn get_pool_metrics() -> crate::models::DbPoolMetrics {
    crate::models::DbPoolMetrics::default()
}

#[cfg(target_arch = "wasm32")]
#[uniffi::export]
pub async fn check_opfs_storage_quota() -> Result<crate::models::OpfsStorageQuota, YntraError> {
    let fut = async move {
        if let Some(window) = web_sys::window() {
            let storage = window.navigator().storage();
            let estimate_key = wasm_bindgen::JsValue::from_str("estimate");
            let js_estimate_fn = js_sys::Reflect::get(&storage, &estimate_key).ok();

            if let Some(estimate_fn) =
                js_estimate_fn.and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            {
                let promise_val = estimate_fn
                    .call0(&storage)
                    .map_err(|e| YntraError::DbError(format!("{:?}", e)))?;
                let promise = js_sys::Promise::from(promise_val);
                let result_val = wasm_bindgen_futures::JsFuture::from(promise)
                    .await
                    .map_err(|e| YntraError::DbError(format!("{:?}", e)))?;

                let quota_key = wasm_bindgen::JsValue::from_str("quota");
                let usage_key = wasm_bindgen::JsValue::from_str("usage");

                let quota = js_sys::Reflect::get(&result_val, &quota_key)
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(100_000_000.0) as u64;

                let usage = js_sys::Reflect::get(&result_val, &usage_key)
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;

                let remaining = quota.saturating_sub(usage);
                let percent = if quota > 0 {
                    (usage as f64 / quota as f64) * 100.0
                } else {
                    0.0
                };
                let is_low = percent >= 90.0 || remaining < 5_000_000;

                return Ok(crate::models::OpfsStorageQuota {
                    quota_bytes: quota,
                    usage_bytes: usage,
                    remaining_bytes: remaining,
                    usage_percent: percent,
                    is_storage_low: is_low,
                });
            }
        }

        Ok(crate::models::OpfsStorageQuota {
            quota_bytes: 100_000_000,
            usage_bytes: 0,
            remaining_bytes: 100_000_000,
            usage_percent: 0.0,
            is_storage_low: false,
        })
    };
    SendFuture::new(fut).await
}

#[cfg(target_arch = "wasm32")]
pub async fn ensure_storage_quota(required_bytes: u64) -> Result<(), YntraError> {
    let quota_info = check_opfs_storage_quota().await?;
    if quota_info.is_storage_low {
        tracing::warn!(
            "TELEMETRY ALERT: OPFS Storage Low! Usage is at {:.1}% ({} / {} bytes used, {} bytes remaining).",
            quota_info.usage_percent,
            quota_info.usage_bytes,
            quota_info.quota_bytes,
            quota_info.remaining_bytes
        );
    }
    if quota_info.remaining_bytes < required_bytes {
        tracing::error!(
            "OPFS Storage Quota Exceeded: Requested {} bytes but only {} bytes remaining (usage: {:.1}%).",
            required_bytes,
            quota_info.remaining_bytes,
            quota_info.usage_percent
        );
        return Err(YntraError::DbError(format!(
            "Storage quota exceeded: required {} bytes but only {} bytes remaining",
            required_bytes, quota_info.remaining_bytes
        )));
    }
    Ok(())
}
