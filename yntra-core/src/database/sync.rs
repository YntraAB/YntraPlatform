use std::sync::{Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use crate::YntraError;

// Database Credentials Configuration Injection
#[derive(Clone)]
struct DbConfig {
    url: String,
    token: String,
}

static DB_CONFIG: OnceLock<Mutex<Option<DbConfig>>> = OnceLock::new();

#[uniffi::export]
pub fn configure_database_sync(url: String, token: String) {
    if let Ok(mut lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        *lock = Some(DbConfig { url, token });
    }
}

pub fn get_configured_credentials() -> Option<(String, String)> {
    if let Ok(lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        lock.as_ref().map(|cfg| (cfg.url.clone(), cfg.token.clone()))
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = yntra_sync_db, catch)]
    async fn js_sync_db(url: &str, token: &str) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[uniffi::export]
pub async fn sync_database() -> Result<(), YntraError> {
    // Check dynamic config first
    let config = if let Ok(lock) = DB_CONFIG.get_or_init(|| Mutex::new(None)).lock() {
        lock.clone()
    } else {
        None
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let has_sync_env = std::env::var("LIBSQL_URL").is_ok();
        if config.is_some() || has_sync_env {
            let db = super::native::get_database();
            db.sync().await.map_err(|e| YntraError::SyncError(e.to_string()))?;
            let _ = crate::services::notes::merge_unmerged_notes().await;
            crate::infra::observer::notify_observers();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let (url, token) = if let Some(cfg) = config {
            (Some(cfg.url), Some(cfg.token))
        } else if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let url = storage.get_item("LIBSQL_URL").ok().flatten();
                let token = storage.get_item("LIBSQL_AUTH_TOKEN").ok().flatten();
                (url, token)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        if let (Some(url), Some(token)) = (url, token) {
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
                    let msg = e.as_string().unwrap_or_else(|| "Unknown JS sync error".to_string());
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
        let _guard = rt.enter();
        tokio::spawn(async move {
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
        start_background_sync(10);
        assert!(SYNC_RUNNING.load(Ordering::SeqCst));
        stop_background_sync();
        assert!(SYNC_CANCELLED.load(Ordering::SeqCst));
    }
}
