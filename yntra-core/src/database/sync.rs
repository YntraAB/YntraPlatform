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
pub fn sync_database() -> Result<(), YntraError> {
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
            super::native::block_on(async {
                db.sync().await
            }).map_err(|e| YntraError::SyncError(e.to_string()))?;
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
            wasm_bindgen_futures::spawn_local(async move {
                match js_sync_db(&url, &token).await {
                    Ok(val) => {
                        let has_changes = if let Some(s) = val.as_string() {
                            if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&s) {
                                obj.get("hasChanges").and_then(|v| v.as_bool()).unwrap_or(true)
                            } else {
                                true
                            }
                        } else {
                            true
                        };
                        if has_changes {
                            crate::infra::observer::notify_observers();
                        }
                    }
                    Err(e) => {
                        let msg = e.as_string().unwrap_or_else(|| "Unknown JS sync error".to_string());
                        tracing::error!("WASM database sync failed: {}", msg);
                    }
                }
            });
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
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
            loop {
                interval.tick().await;
                if SYNC_CANCELLED.load(Ordering::SeqCst) {
                    SYNC_RUNNING.store(false, Ordering::SeqCst);
                    break;
                }
                let _ = sync_database();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                sleep_ms((interval_secs * 1000) as u64).await;
                if SYNC_CANCELLED.load(Ordering::SeqCst) {
                    SYNC_RUNNING.store(false, Ordering::SeqCst);
                    break;
                }
                let _ = sync_database();
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
