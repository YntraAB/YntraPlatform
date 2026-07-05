// wasm.rs
// WebAssembly database adapter for yntra-core using JS worker bridge

use crate::YntraError;
use wasm_bindgen::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// A wrapper to make WASM futures Send.
// Safety: Since WebAssembly (wasm32-unknown-unknown) is single-threaded in the browser,
// implementing Send for JS-bound futures is safe and required to satisfy generic bounds.
pub struct SendFuture<F> {
    inner: F,
}

impl<F> SendFuture<F> {
    pub fn new(inner: F) -> Self {
        Self { inner }
    }
}

unsafe impl<F> Send for SendFuture<F> {}

impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut_self = unsafe { self.get_unchecked_mut() };
        let inner = unsafe { Pin::new_unchecked(&mut mut_self.inner) };
        inner.poll(cx)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_execute_sql, catch)]
    async fn js_execute_sql_internal(query_type: &str, sql: &str, params_json: &str) -> Result<JsValue, JsValue>;
}

async fn js_execute_sql(query_type: &str, sql: &str, params_json: &str) -> Result<String, YntraError> {
    let fut = js_execute_sql_internal(query_type, sql, params_json);
    let send_fut = SendFuture::new(fut);
    match send_fut.await {
        Ok(js_val) => {
            if let Some(s) = js_val.as_string() {
                Ok(s)
            } else {
                Err(YntraError::DbError("JS database call did not return a string".to_string()))
            }
        }
        Err(js_err) => {
            let err_msg = js_err.as_string()
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
    fn into_wasm_params(self) -> Vec<serde_json::Value>;
}

impl IntoWasmParams for Vec<serde_json::Value> {
    fn into_wasm_params(self) -> Vec<serde_json::Value> {
        self
    }
}

impl IntoWasmParams for () {
    fn into_wasm_params(self) -> Vec<serde_json::Value> {
        vec![]
    }
}

pub async fn acquire_connection() -> Result<DbConnection, YntraError> {
    Ok(DbConnection)
}

pub struct DbConnection;

impl DbConnection {
    pub async fn execute<P: IntoWasmParams>(&self, sql: &str, params: P) -> Result<u64, YntraError> {
        let params_wasm = params.into_wasm_params();
        let params_str = serde_json::to_string(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_str = js_execute_sql("execute", sql, &params_str).await?;
        let res: ExecuteResult = serde_json::from_str(&result_str)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        if let Some(table) = crate::infra::observer::extract_table_name(sql) {
            crate::infra::observer::set_last_modified_table(&table);
        }
        Ok(res.rows_affected)
    }

    pub async fn execute_batch(&self, sql: &str) -> Result<(), YntraError> {
        js_execute_sql("execute_batch", sql, "[]").await?;
        for stmt in sql.split(';') {
            if let Some(table) = crate::infra::observer::extract_table_name(stmt) {
                crate::infra::observer::set_last_modified_table(&table);
            }
        }
        Ok(())
    }

    pub async fn prepare(&self, sql: &str) -> Result<Statement, YntraError> {
        Ok(Statement { sql: sql.to_string() })
    }

    pub async fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, YntraError>
    where
        P: IntoWasmParams,
        F: FnOnce(&Row) -> Result<T, YntraError> + Send,
        T: Send,
    {
        let params_wasm = params.into_wasm_params();
        let params_str = serde_json::to_string(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_str = js_execute_sql("query", sql, &params_str).await?;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&result_str)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        if rows.is_empty() {
            return Err(YntraError::DbError("No row returned".to_string()));
        }
        let row = Row { value: rows[0].clone() };
        f(&row)
    }
}

pub struct Statement {
    pub sql: String,
}

impl Statement {
    pub async fn query<P: IntoWasmParams>(&mut self, params: P) -> Result<Rows, YntraError> {
        let params_wasm = params.into_wasm_params();
        let params_str = serde_json::to_string(&params_wasm)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let result_str = js_execute_sql("query", &self.sql, &params_str).await?;
        let rows_val: Vec<serde_json::Value> = serde_json::from_str(&result_str)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        Ok(Rows {
            rows: rows_val,
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
    rows: Vec<serde_json::Value>,
    index: usize,
}

impl Rows {
    pub async fn next(&mut self) -> Result<Option<Row>, YntraError> {
        if self.index < self.rows.len() {
            let val = self.rows[self.index].clone();
            self.index += 1;
            Ok(Some(Row { value: val }))
        } else {
            Ok(None)
        }
    }
}

pub struct Row {
    value: serde_json::Value,
}

impl Row {
    pub fn get<T: serde::de::DeserializeOwned>(&self, idx: i32) -> Result<T, YntraError> {
        if let Some(arr) = self.value.as_array() {
            let val = arr.get(idx as usize)
                .ok_or_else(|| YntraError::DbError(format!("Column index out of bounds: {}", idx)))?;
            serde_json::from_value(val.clone())
                .map_err(|e| YntraError::DbError(format!("Failed to deserialize column at index {}: {}", idx, e)))
        } else {
            Err(YntraError::DbError("Row value is not a JSON array".to_string()))
        }
    }
}

pub fn params_from_iter<I, T>(iter: I) -> Vec<serde_json::Value>
where
    I: IntoIterator<Item = T>,
    T: crate::rusqlite::ToWasmValue,
{
    iter.into_iter().map(|x| x.to_value()).collect()
}


