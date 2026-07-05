use crate::YntraError;
use std::sync::{Mutex, OnceLock};

// Reactive Database Observer callback trait
#[uniffi::export(callback_interface)]
pub trait DatabaseObserver: Send + Sync {
    fn on_database_changed(&self);
    fn on_table_changed(&self, _table: String) {
        self.on_database_changed();
    }
}

thread_local! {
    static LAST_MODIFIED_TABLE: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

pub fn set_last_modified_table(table: &str) {
    LAST_MODIFIED_TABLE.with(|val| {
        *val.borrow_mut() = Some(table.to_string());
    });
}

pub fn extract_table_name(sql: &str) -> Option<String> {
    let sql_upper = sql.to_uppercase();
    let words: Vec<&str> = sql_upper.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
    
    match words[0] {
        "INSERT" => {
            let idx = words.iter().position(|&w| w == "INTO")?;
            if idx + 1 < words.len() {
                let name = words[idx + 1].trim_matches(|c| c == '(' || c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                return Some(name.to_lowercase());
            }
        }
        "UPDATE" => {
            if words.len() > 1 {
                let name = words[1].trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                return Some(name.to_lowercase());
            }
        }
        "DELETE" => {
            let idx = words.iter().position(|&w| w == "FROM")?;
            if idx + 1 < words.len() {
                let name = words[idx + 1].trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                return Some(name.to_lowercase());
            }
        }
        _ => {}
    }
    None
}

static OBSERVERS: OnceLock<Mutex<Vec<Box<dyn DatabaseObserver>>>> = OnceLock::new();

fn get_observers() -> &'static Mutex<Vec<Box<dyn DatabaseObserver>>> {
    OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

#[uniffi::export]
pub fn register_observer(observer: Box<dyn DatabaseObserver>) {
    if let Ok(mut observers) = get_observers().lock() {
        observers.push(observer);
    }
}

#[uniffi::export]
pub fn clear_observers() {
    if let Ok(mut observers) = get_observers().lock() {
        observers.clear();
    }
}

pub fn notify_observers() {
    let table_opt = LAST_MODIFIED_TABLE.with(|val| {
        val.borrow_mut().take()
    });

    if let Ok(observers) = get_observers().lock() {
        for observer in observers.iter() {
            if let Some(ref table) = table_opt {
                observer.on_table_changed(table.clone());
            } else {
                observer.on_database_changed();
            }
        }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::var("LIBSQL_URL").is_ok() {
            let db = crate::database::native::get_database();
            crate::database::native::block_on(async {
                db.sync().await
            }).map_err(|e| YntraError::SyncError(e.to_string()))?;
            notify_observers();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let url = storage.get_item("LIBSQL_URL").ok().flatten();
                let token = storage.get_item("LIBSQL_AUTH_TOKEN").ok().flatten();
                if let (Some(url), Some(token)) = (url, token) {
                    wasm_bindgen_futures::spawn_local(async move {
                        match js_sync_db(&url, &token).await {
                            Ok(_) => {
                                notify_observers();
                            }
                            Err(e) => {
                                let msg = e.as_string().unwrap_or_else(|| "Unknown JS sync error".to_string());
                                tracing::error!("WASM database sync failed: {}", msg);
                            }
                        }
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[uniffi::export]
pub fn start_background_sync(interval_secs: u32) {
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = crate::database::native::get_runtime();
        let _guard = rt.enter();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
            loop {
                interval.tick().await;
                let _ = sync_database();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                sleep_ms(interval_secs * 1000).await;
                let _ = sync_database();
            }
        });
    }
}
