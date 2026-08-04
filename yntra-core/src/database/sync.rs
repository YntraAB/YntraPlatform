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
static SYNC_ACTIVE_USER_ID: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[allow(dead_code)]
fn get_sync_http_client() -> &'static reqwest::Client {
    static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct MobileSyncResult {
    pub success: bool,
    pub synced_rows_count: u32,
    pub pending_sync_queue_count: u32,
    pub error_message: Option<String>,
    pub timestamp_ms: i64,
}

#[uniffi::export]
pub async fn get_mobile_sync_queue_summary(workspace_id: String) -> Result<u32, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let mut total_pending = 0u32;

    let tables = vec!["time_reports", "messages", "notes", "todos", "reports", "job_tickets"];
    for table in tables {
        let query = format!("SELECT count(*) FROM {} WHERE workspace_id = ?1 AND sync_status = 'pending'", table);
        let count: i64 = conn
            .query_row(&query, crate::params![&workspace_id], |r| r.get(0))
            .await
            .unwrap_or(0);
        total_pending += count as u32;
    }

    Ok(total_pending)
}

#[uniffi::export]
pub async fn perform_os_background_sync(workspace_id: String) -> Result<MobileSyncResult, YntraError> {
    let timestamp_ms = crate::infra::time::get_current_time_ms();
    let sync_res = sync_database().await;

    let pending_queue_count = get_mobile_sync_queue_summary(workspace_id.clone()).await.unwrap_or(0);
    crate::infra::observer::notify_sync_status_changed(pending_queue_count);

    match sync_res {
        Ok(_) => Ok(MobileSyncResult {
            success: true,
            synced_rows_count: 0,
            pending_sync_queue_count: pending_queue_count,
            error_message: None,
            timestamp_ms,
        }),
        Err(e) => Ok(MobileSyncResult {
            success: false,
            synced_rows_count: 0,
            pending_sync_queue_count: pending_queue_count,
            error_message: Some(e.to_string()),
            timestamp_ms,
        }),
    }
}

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

#[uniffi::export]
pub fn set_sync_active_user_id(user_id: Option<String>) {
    if let Ok(mut lock) = SYNC_ACTIVE_USER_ID.get_or_init(|| Mutex::new(None)).lock() {
        *lock = user_id;
    }
}

pub fn get_sync_active_user_id() -> Option<String> {
    if let Ok(lock) = SYNC_ACTIVE_USER_ID.get_or_init(|| Mutex::new(None)).lock() {
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
    let active_user_id = get_sync_active_user_id();
    let role: Option<String> = if let Some(ref target_uid) = active_user_id {
        conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            crate::params![target_uid],
            |r| r.get(0),
        )
        .await
        .ok()
    } else {
        conn.query_row(
            "SELECT role FROM users LIMIT 1",
            (),
            |r| r.get(0),
        )
        .await
        .ok()
    };

    if let Some(role) = role {
        let r_lower = role.to_lowercase();
        return r_lower == "student"
            || r_lower == "role-school-student"
            || r_lower == "parent"
            || r_lower == "role-school-parent";
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
async fn sync_database_row_level(url: String, token: String) -> Result<(), YntraError> {
    let conn = crate::database::acquire_connection().await?;

    // 1. Get logged-in user ID, role, and workspace ID
    let active_user_id = get_sync_active_user_id();
    let (user_id, role, _workspace_id): (String, String, String) = if let Some(target_uid) = active_user_id {
        match conn
            .query_row(
                "SELECT id, role, workspace_id FROM users WHERE id = ?1",
                crate::params![&target_uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .await
        {
            Ok(res) => res,
            Err(_) => return Ok(()),
        }
    } else {
        match conn
            .query_row("SELECT id, role, workspace_id FROM users LIMIT 1", (), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .await
        {
            Ok(res) => res,
            Err(_) => return Ok(()), // No user logged in, nothing to sync
        }
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
        "job_tickets",
        "move_quotes",
        "move_inventory",
        "move_invoices",
        "move_signatures",
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

        // Now execute modifications and updates in optimized chunks with bulk status updates
        for chunk in pending_writes.chunks(50) {
            let mut synced_ids = Vec::new();
            for (row_id, sql, params_json) in chunk {
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
                    let req = get_sync_http_client()
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
                            sql.clone(),
                            params_json.clone(),
                        )
                        .await?;
                }
                synced_ids.push(row_id.clone());
            }

            // Bulk mark local rows as synced in a single SQL query per chunk
            if !synced_ids.is_empty() {
                let placeholders = (1..=synced_ids.len())
                    .map(|i| format!("?{}", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                let update_sql = format!(
                    "UPDATE {} SET sync_status = 'synced' WHERE id IN ({})",
                    table, placeholders
                );
                let params: Vec<libsql::Value> = synced_ids
                    .into_iter()
                    .map(libsql::Value::Text)
                    .collect();
                conn.execute(&update_sql, params).await?;
            }
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
            let req = get_sync_http_client()
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
        let is_remote_replica = url.starts_with("http://") || url.starts_with("https://");

        if is_unprivileged || !is_remote_replica {
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
            let db = super::native::get_database_async().await?;
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

#[uniffi::export]
pub async fn get_pending_sync_count() -> Result<i64, YntraError> {
    let conn = super::acquire_connection().await?;
    let mut total_pending: i64 = 0;

    let tables = ["todos", "notes", "reports", "time_reports", "messages"];
    for table in tables {
        let query = format!("SELECT COUNT(*) FROM {} WHERE sync_status = 'pending'", table);
        if let Ok(count) = conn.query_row(&query, (), |row| row.get::<i64>(0)).await {
            total_pending += count;
        }
    }

    Ok(total_pending)
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct SyncQueueSummaryRecord {

    pub table_name: String,
    pub pending_count: u32,
    pub last_updated_at: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct PendingBlobUpload {
    pub hash_pointer: String,
    pub job_ticket_id: String,
    pub media_type: String,
    pub original_size_bytes: i64,
    pub compressed_size_bytes: i64,
    pub upload_status: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct SyncConflictRecord {
    pub table_name: String,
    pub record_id: String,
    pub local_version_json: String,
    pub remote_version_json: String,
    pub updated_at: i64,
}

#[uniffi::export]
pub async fn get_sync_queue_breakdown(workspace_id: String) -> Result<Vec<SyncQueueSummaryRecord>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let mut list = Vec::new();

    let tables = vec!["time_reports", "messages", "notes", "todos", "reports", "job_tickets"];
    for table in tables {
        let query = format!("SELECT COUNT(*), COALESCE(MAX(updated_at), 0) FROM {} WHERE workspace_id = ?1 AND sync_status = 'pending'", table);
        let row_res: Result<(i64, i64), _> = conn
            .query_row(&query, crate::params![&workspace_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .await;

        if let Ok((count, last_updated)) = row_res {

            if count > 0 {
                list.push(SyncQueueSummaryRecord {
                    table_name: table.to_string(),
                    pending_count: count as u32,
                    last_updated_at: last_updated,
                });
            }
        }
    }

    Ok(list)
}

#[uniffi::export]
pub async fn get_pending_blob_uploads(workspace_id: String) -> Result<Vec<PendingBlobUpload>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let mut stmt = conn.prepare(
        "SELECT hash, job_ticket_id, media_type, original_size_bytes, compressed_size_bytes, upload_status FROM offline_media_blobs WHERE workspace_id = ?1 AND upload_status != 'synced'"
    ).await?;

    let list = stmt
        .query_map(crate::params![&workspace_id], |r| {
            Ok(PendingBlobUpload {
                hash_pointer: r.get(0)?,
                job_ticket_id: r.get(1)?,
                media_type: r.get(2)?,
                original_size_bytes: r.get(3)?,
                compressed_size_bytes: r.get(4)?,
                upload_status: r.get(5)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_sync_conflicts(workspace_id: String) -> Result<Vec<SyncConflictRecord>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let mut list = Vec::new();

    // Query notes with unmerged Loro CRDT edits as conflicts
    let mut stmt = conn.prepare(
        "SELECT id, subject, content, updated_at FROM notes WHERE workspace_id = ?1 AND content LIKE 'loro:%'"
    ).await?;

    let mut rows = stmt.query(crate::params![&workspace_id]).await?;
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let subj: String = row.get(1)?;
        let content: String = row.get(2)?;
        let updated: i64 = row.get(3)?;

        let local_json = serde_json::json!({ "subject": subj, "content": content }).to_string();
        let remote_json = serde_json::json!({ "subject": subj, "content": "Server Loro Delta State" }).to_string();

        list.push(SyncConflictRecord {
            table_name: "notes".to_string(),
            record_id: id,
            local_version_json: local_json,
            remote_version_json: remote_json,
            updated_at: updated,
        });
    }

    Ok(list)
}

#[uniffi::export]
pub async fn resolve_sync_conflict(
    requester_user_id: String,
    workspace_id: String,
    table_name: String,
    record_id: String,
    resolution_choice: String,
    custom_resolved_json: Option<String>,
) -> Result<bool, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    if table_name == "notes" {
        if let Some(custom_json) = custom_resolved_json {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&custom_json) {
                let content = obj.get("content").and_then(|v| v.as_str()).unwrap_or_default();
                conn.execute(
                    "UPDATE notes SET content = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3 AND workspace_id = ?4",
                    crate::params![content, now_ms, &record_id, &workspace_id],
                ).await?;
            }
        } else if resolution_choice == "keep_local" {
            conn.execute(
                "UPDATE notes SET sync_status = 'synced', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
                crate::params![now_ms, &record_id, &workspace_id],
            ).await?;
        }
    }

    crate::infra::observer::notify_observers();
    Ok(true)
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

    #[test]
    fn test_sync_active_user_id_configuration() {
        set_sync_active_user_id(Some("user-target-123".to_string()));
        assert_eq!(get_sync_active_user_id(), Some("user-target-123".to_string()));
        set_sync_active_user_id(None);
        assert_eq!(get_sync_active_user_id(), None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_mobile_os_background_sync_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await?;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-mob-sync', 'Mobile WS', '[]', '{}')", ()).await?;
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-mob-1', 'ws-mob-sync', 'mob@sync.io', 'admin')", ()).await?;

        conn.execute(
            "INSERT OR REPLACE INTO time_reports (id, workspace_id, user_id, date, hours, note, status, created_at, updated_at, sync_status) VALUES ('tr-mob-1', 'ws-mob-sync', 'u-mob-1', '2026-08-04', 8.0, 'test', 'approved', '2026-08-04', 1000, 'pending')",
            (),
        ).await?;

        let pending_count = get_mobile_sync_queue_summary("ws-mob-sync".to_string()).await?;
        assert!(pending_count >= 1, "Pending sync queue should report at least 1 pending item");

        let res = perform_os_background_sync("ws-mob-sync".to_string()).await?;
        assert_eq!(res.pending_sync_queue_count, pending_count);
        assert!(res.timestamp_ms > 0);

        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-mob-sync'", ()).await?;
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-mob-sync'", ()).await?;
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-mob-sync'", ()).await?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_sync_queue_breakdown_and_conflict_resolution() -> Result<(), Box<dyn std::error::Error>> {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await?;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-ux-sync', 'UX WS', '[]', '{}')", ()).await?;
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-ux-1', 'ws-ux-sync', 'ux@sync.io', 'platform_admin')", ()).await?;
        conn.execute("INSERT OR REPLACE INTO teams (id, workspace_id, name) VALUES ('t-1', 'ws-ux-sync', 'UX Team')", ()).await?;


        conn.execute(
            "INSERT OR REPLACE INTO time_reports (id, workspace_id, user_id, date, hours, note, status, created_at, updated_at, sync_status) VALUES ('tr-ux-1', 'ws-ux-sync', 'u-ux-1', '2026-08-04', 4.0, 'ux test', 'approved', '2026-08-04', 2000, 'pending')",
            (),
        ).await?;

        // 1. Verify breakdown
        let breakdown = get_sync_queue_breakdown("ws-ux-sync".to_string()).await?;
        assert_eq!(breakdown.len(), 1);
        assert_eq!(breakdown[0].table_name, "time_reports");
        assert_eq!(breakdown[0].pending_count, 1);

        // 2. Insert Loro conflict note & verify conflict detection
        conn.execute(
            "INSERT OR REPLACE INTO notes (id, workspace_id, team_id, subject, content, edit_history, created_at, updated_at, sync_status) VALUES ('n-ux-1', 'ws-ux-sync', 't-1', 'UX Note', 'loro:1:deadbeef', '[]', '2026-08-04', 3000, 'pending')",
            (),
        ).await?;

        let conflicts = get_sync_conflicts("ws-ux-sync".to_string()).await?;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].table_name, "notes");

        // 3. Resolve conflict
        let ok = resolve_sync_conflict("u-ux-1".to_string(), "ws-ux-sync".to_string(), "notes".to_string(), "n-ux-1".to_string(), "keep_local".to_string(), None).await?;
        assert!(ok);

        let status: String = conn.query_row("SELECT sync_status FROM notes WHERE id = 'n-ux-1'", (), |r| r.get(0)).await?;
        assert_eq!(status, "synced");

        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-ux-sync'", ()).await?;
        conn.execute("DELETE FROM notes WHERE workspace_id = 'ws-ux-sync'", ()).await?;
        conn.execute("DELETE FROM teams WHERE workspace_id = 'ws-ux-sync'", ()).await?;
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-ux-sync'", ()).await?;
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-ux-sync'", ()).await?;
        Ok(())

    }
}

