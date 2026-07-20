use crate::YntraError;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

// Database Credentials Configuration Injection
#[derive(Clone)]
struct DbConfig {
    url: String,
    token: String,
}

static DB_CONFIG: OnceLock<Mutex<Option<DbConfig>>> = OnceLock::new();
static SYNC_ROLE_PROOF: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[uniffi::export]
pub fn configure_database_sync(url: String, token: String) {
    if let Ok(mut lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        *lock = Some(DbConfig { url, token });
    }
}

pub fn get_configured_credentials() -> Option<(String, String)> {
    if let Ok(lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        lock.as_ref()
            .map(|cfg| (cfg.url.clone(), cfg.token.clone()))
    } else {
        None
    }
}

#[uniffi::export]
pub fn set_sync_role_proof(proof: Option<String>) {
    if let Ok(mut lock) = SYNC_ROLE_PROOF.get_or_init(|| Mutex::new(None)).lock() {
        *lock = proof;
    }
}

pub fn get_sync_role_proof() -> Option<String> {
    if let Ok(lock) = SYNC_ROLE_PROOF.get_or_init(|| Mutex::new(None)).lock() {
        lock.clone()
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_sync_db, catch)]
    async fn js_sync_db(
        url: &str,
        token: &str,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(not(target_arch = "wasm32"))]
fn row_get_json_value(row: &crate::database::Row, idx: usize) -> Result<serde_json::Value, YntraError> {
    let val = row.get_value(idx as i32)?;
    let json_val = match val {
        libsql::Value::Null => serde_json::Value::Null,
        libsql::Value::Integer(n) => serde_json::Value::Number(serde_json::value::Number::from(n)),
        libsql::Value::Real(f) => {
            if let Some(num) = serde_json::value::Number::from_f64(f) {
                serde_json::Value::Number(num)
            } else {
                serde_json::Value::Null
            }
        }
        libsql::Value::Text(s) => serde_json::Value::String(s),
        libsql::Value::Blob(_) => serde_json::Value::Null,
    };
    Ok(json_val)
}

#[cfg(not(target_arch = "wasm32"))]
async fn execute_with_json_params(
    conn: &crate::database::DbConnection,
    sql: &str,
    params: Vec<serde_json::Value>,
) -> Result<(), YntraError> {
    let mut libsql_params = Vec::new();
    for v in params {
        let l_val = match v {
            serde_json::Value::Null => libsql::Value::Null,
            serde_json::Value::Bool(b) => libsql::Value::Integer(if b { 1 } else { 0 }),
            serde_json::Value::Number(num) => {
                if let Some(i) = num.as_i64() {
                    libsql::Value::Integer(i)
                } else if let Some(f) = num.as_f64() {
                    libsql::Value::Real(f)
                } else {
                    libsql::Value::Null
                }
            }
            serde_json::Value::String(s) => libsql::Value::Text(s),
            _ => libsql::Value::Null,
        };
        libsql_params.push(l_val);
    }
    conn.execute(sql, libsql_params).await?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn check_is_unprivileged() -> bool {
    let conn = match crate::database::acquire_connection().await {
        Ok(c) => c,
        Err(_) => return false,
    };
    let mut stmt = match conn.prepare("SELECT role FROM users").await {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut rows = match stmt.query(()).await {
        Ok(r) => r,
        Err(_) => return false,
    };
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(role) = row.get::<String>(0) {
            let r_lower = role.to_lowercase();
            if r_lower == "student" || r_lower == "role-school-student" || r_lower == "parent" || r_lower == "role-school-parent" {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
async fn sync_database_row_level(url: String, token: String) -> Result<(), YntraError> {
    let conn = crate::database::acquire_connection().await?;

    // 1. Get logged-in user ID, role, and workspace ID
    let (user_id, role, _workspace_id): (String, String, String) = match conn
        .query_row("SELECT id, role, workspace_id FROM users LIMIT 1", (), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .await
    {
        Ok(res) => res,
        Err(_) => return Ok(()), // No user logged in, nothing to sync
    };

    // Get ZK role proof from memory config
    let role_proof = get_sync_role_proof();

    let is_remote = url.starts_with("http://") || url.starts_with("https://");

    // 2. Define the tables to sync
    let tables = vec![
        "student_profiles",
        "attendance_records",
        "health_incidents",
        "term_grades",
        "timetable_slots",
        "courses",
        "school_invoices",
        "submissions",
        "health_records",
        "report_cards",
        "library_books",
        "library_lending_logs",
        "school_payments",
    ];

    // 3. UPLOAD PHASE
    for &table in &tables {
        // We also need the column names of the table to build the INSERT statement dynamically
        let mut columns = Vec::new();
        let mut info_stmt = conn.prepare(&format!("PRAGMA table_info({})", table)).await?;
        let mut info_rows = info_stmt.query(()).await?;
        while let Some(row) = info_rows.next().await? {
            let col_name: String = row.get(1)?;
            columns.push(col_name);
        }

        // Query rows where sync_status = 'pending' and collect them
        let mut pending_writes = Vec::new();
        {
            let mut query_stmt = conn
                .prepare(&format!("SELECT * FROM {} WHERE sync_status = 'pending'", table))
                .await?;
            let mut rows = query_stmt.query(()).await?;

            while let Some(row) = rows.next().await? {
                let mut row_id = String::new();
                if let Some(id_idx) = columns.iter().position(|c| c == "id") {
                    row_id = row.get(id_idx as i32)?;
                }

                let mut params = Vec::new();
                let mut placeholders = Vec::new();
                let mut insert_cols = Vec::new();

                for (i, col) in columns.iter().enumerate() {
                    if col == "sync_status" {
                        continue; // Skip sync_status in remote database
                    }
                    let json_val = row_get_json_value(&row, i)?;

                    insert_cols.push(format!("`{}`", col));
                    placeholders.push(format!("?{}", params.len() + 1));
                    params.push(json_val);
                }

                let sql = format!(
                    "INSERT OR REPLACE INTO `{}` ({}) VALUES ({})",
                    table,
                    insert_cols.join(", "),
                    placeholders.join(", ")
                );
                let params_json = serde_json::Value::Array(params).to_string();
                pending_writes.push((row_id, sql, params_json));
            }
        } // query_stmt and rows are dropped here, releasing database read lock!

        // Now execute modifications and updates on a clean database state
        for (row_id, sql, params_json) in pending_writes {
            // Execute the write remotely or locally
            if is_remote {
                let endpoint = format!("{}/api/sync/execute", url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "requester_user_id": user_id,
                    "role": role,
                    "role_proof": role_proof,
                    "sql": sql,
                    "params_json": params_json,
                });
                let req = reqwest::Client::new()
                    .post(&endpoint)
                    .json(&body);
                let req = if !token.is_empty() {
                    req.header("Authorization", format!("Bearer {}", token))
                } else {
                    req
                };
                let res = req.send().await.map_err(|e| YntraError::SyncError(e.to_string()))?;
                if !res.status().is_success() {
                    return Err(YntraError::SyncError(format!(
                        "Remote execute write failed: status {}",
                        res.status()
                    )));
                }
            } else {
                let coordinator = crate::RemoteSyncCoordinator::new();
                coordinator
                    .verify_and_execute_write(
                        user_id.clone(),
                        role.clone(),
                        role_proof.clone(),
                        sql,
                        params_json,
                    )
                    .await?;
            }

            // Mark local row as synced
            conn.execute(
                &format!("UPDATE {} SET sync_status = 'synced' WHERE id = ?1", table),
                crate::params![&row_id],
            )
            .await?;
        }
    }

    // --- DOWNLOAD PHASE ---
    for &table in &tables {
        let payload = if is_remote {
            let endpoint = format!("{}/api/sync/payload", url.trim_end_matches('/'));
            let body = serde_json::json!({
                "requester_user_id": user_id,
                "role": role,
                "role_proof": role_proof,
                "table_name": table,
            });
            let req = reqwest::Client::new()
                .post(&endpoint)
                .json(&body);
            let req = if !token.is_empty() {
                req.header("Authorization", format!("Bearer {}", token))
            } else {
                req
            };
            let res = req.send().await.map_err(|e| YntraError::SyncError(e.to_string()))?;
            if !res.status().is_success() {
                return Err(YntraError::SyncError(format!(
                    "Remote fetch partitioned payload failed: status {}",
                    res.status()
                )));
            }
            res.text().await.map_err(|e| YntraError::SyncError(e.to_string()))?
        } else {
            let coordinator = crate::RemoteSyncCoordinator::new();
            coordinator
                .generate_partitioned_sync_payload(
                    user_id.clone(),
                    role.clone(),
                    role_proof.clone(),
                    table.to_string(),
                )
                .await?
        };

        let rows_json: Vec<serde_json::Value> = serde_json::from_str(&payload)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;

        // Merge each row into the local database
        for r_val in rows_json {
            let obj = r_val.as_object().ok_or_else(|| {
                YntraError::SerializationError("Sync row is not a JSON object".to_string())
            })?;

            // Fetch columns to check if the table has sync_status
            let mut has_sync_status = false;
            let mut info_stmt = conn.prepare(&format!("PRAGMA table_info({})", table)).await?;
            let mut info_rows = info_stmt.query(()).await?;
            while let Some(row) = info_rows.next().await? {
                let name: String = row.get(1)?;
                if name == "sync_status" {
                    has_sync_status = true;
                    break;
                }
            }

            let mut insert_cols = Vec::new();
            let mut placeholders = Vec::new();
            let mut params = Vec::new();

            for (k, v) in obj {
                insert_cols.push(format!("`{}`", k));
                placeholders.push(format!("?{}", params.len() + 1));
                params.push(v.clone());
            }

            if has_sync_status {
                insert_cols.push("`sync_status`".to_string());
                placeholders.push(format!("?{}", params.len() + 1));
                params.push(serde_json::Value::String("synced".to_string()));
            }

            let sql = format!(
                "INSERT OR REPLACE INTO `{}` ({}) VALUES ({})",
                table,
                insert_cols.join(", "),
                placeholders.join(", ")
            );

            execute_with_json_params(&conn, &sql, params).await?;
        }
    }

    Ok(())
}

#[uniffi::export]
pub async fn sync_database() -> Result<(), YntraError> {
    // Check dynamic config first
    let config = if let Ok(lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        lock.clone()
    } else {
        None
    };

    let (url, token) = if let Some(cfg) = config {
        (cfg.url, cfg.token)
    } else if let (Ok(url), Ok(token)) = (std::env::var("LIBSQL_URL"), std::env::var("LIBSQL_AUTH_TOKEN")) {
        (url, token)
    } else {
        ("".to_string(), "".to_string())
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let is_unprivileged = check_is_unprivileged().await;

        if is_unprivileged {
            if !url.is_empty() {
                sync_database_row_level(url, token).await?;
                let _ = crate::services::notes::merge_unmerged_notes().await;
                crate::infra::observer::notify_observers();
            }
            return Ok(());
        }
    }

    if !url.is_empty() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let db = super::native::get_database();
            match tokio::time::timeout(std::time::Duration::from_secs(15), db.sync()).await {
                Ok(sync_res) => {
                    sync_res.map_err(|e| YntraError::SyncError(e.to_string()))?;
                }
                Err(_) => {
                    return Err(YntraError::SyncError("Database sync timed out".to_string()));
                }
            }
            let _ = crate::services::notes::merge_unmerged_notes().await;
            crate::infra::observer::notify_observers();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let fut = js_sync_db(&url, &token);
            let send_fut = crate::database::wasm::SendFuture::new(fut);
            match send_fut.await {
                Ok(val) => {
                    let has_changes = serde_wasm_bindgen::from_value::<serde_json::Value>(val)
                        .ok()
                        .and_then(|obj| obj.get("hasChanges").and_then(|v| v.as_bool()))
                        .unwrap_or(true);
                    if has_changes {
                        let _ = crate::services::notes::merge_unmerged_notes().await;
                        crate::infra::observer::notify_observers();
                    }
                }
                Err(e) => {
                    let msg = e
                        .as_string()
                        .unwrap_or_else(|| "Unknown JS sync error".to_string());
                    return Err(YntraError::SyncError(msg));
                }
            }
        }
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
use crate::infra::time::sleep_ms;

static SYNC_RUNNING: AtomicBool = AtomicBool::new(false);
static SYNC_CANCELLED: AtomicBool = AtomicBool::new(false);

#[uniffi::export]
pub fn start_background_sync(interval_secs: u32) {
    if SYNC_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    SYNC_CANCELLED.store(false, Ordering::SeqCst);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = super::native::get_runtime();
        rt.spawn(async move {
            let base_interval = interval_secs as u64;
            let mut consecutive_failures = 0;
            loop {
                let current_interval = if consecutive_failures > 0 {
                    (base_interval * 2_u64.pow(consecutive_failures)).min(600)
                } else {
                    base_interval
                };

                tokio::time::sleep(std::time::Duration::from_secs(current_interval)).await;

                if SYNC_CANCELLED.load(Ordering::SeqCst) {
                    SYNC_RUNNING.store(false, Ordering::SeqCst);
                    break;
                }

                match sync_database().await {
                    Ok(_) => consecutive_failures = 0,
                    Err(_) => consecutive_failures = (consecutive_failures + 1).min(5),
                }
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let base_interval = interval_secs as u64;
            let mut consecutive_failures = 0;
            loop {
                let current_interval = if consecutive_failures > 0 {
                    (base_interval * 2_u64.pow(consecutive_failures)).min(600)
                } else {
                    base_interval
                };

                sleep_ms(current_interval * 1000).await;

                if SYNC_CANCELLED.load(Ordering::SeqCst) {
                    SYNC_RUNNING.store(false, Ordering::SeqCst);
                    break;
                }

                match sync_database().await {
                    Ok(_) => consecutive_failures = 0,
                    Err(_) => consecutive_failures = (consecutive_failures + 1).min(5),
                }
            }
        });
    }
}

#[uniffi::export]
pub fn stop_background_sync() {
    SYNC_CANCELLED.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_loop_cancellation() {
        start_background_sync(0);
        assert!(SYNC_RUNNING.load(Ordering::SeqCst));
        stop_background_sync();
        assert!(SYNC_CANCELLED.load(Ordering::SeqCst));

        let mut ok = false;
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if !SYNC_RUNNING.load(Ordering::SeqCst) {
                ok = true;
                break;
            }
        }
        assert!(ok, "Sync loop did not stop running after cancellation");
    }
}
