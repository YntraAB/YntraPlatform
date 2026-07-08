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

static MODIFIED_TABLES: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

pub fn set_last_modified_table(table: &str) {
    if let Ok(mut tables) = MODIFIED_TABLES.get_or_init(|| Mutex::new(std::collections::HashSet::new())).lock() {
        tables.insert(table.to_string());
    }
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
                let raw_name = words[idx + 1].split('(').next().unwrap_or("");
                let name = raw_name.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
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
    let tables = if let Ok(mut lock) = MODIFIED_TABLES.get_or_init(|| Mutex::new(std::collections::HashSet::new())).lock() {
        std::mem::take(&mut *lock)
    } else {
        std::collections::HashSet::new()
    };

    if let Ok(observers) = get_observers().lock() {
        if tables.is_empty() {
            for observer in observers.iter() {
                observer.on_database_changed();
            }
        } else {
            for observer in observers.iter() {
                for table in &tables {
                    observer.on_table_changed(table.clone());
                }
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
                                    notify_observers();
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
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
use crate::infra::time::sleep_ms;

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
                sleep_ms((interval_secs * 1000) as u64).await;
                let _ = sync_database();
            }
        });
    }
}

pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    for c in sql.chars() {
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ';' if !in_single_quote && !in_double_quote => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    statements.push(trimmed.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(trimmed.to_string());
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql_statements() {
        let sql = "INSERT INTO messages (body) VALUES ('hello; world'); UPDATE todos SET text = 'a;b'; DELETE FROM notes";
        let res = split_sql_statements(sql);
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], "INSERT INTO messages (body) VALUES ('hello; world')");
        assert_eq!(res[1], "UPDATE todos SET text = 'a;b'");
        assert_eq!(res[2], "DELETE FROM notes");
    }

    #[test]
    fn test_extract_table_name_inserts() {
        assert_eq!(extract_table_name("INSERT INTO todos (id, text) VALUES (1, 'hello')"), Some("todos".to_string()));
        assert_eq!(extract_table_name("INSERT INTO [todos] (id) VALUES (1)"), Some("todos".to_string()));
        assert_eq!(extract_table_name("INSERT INTO `todos` VALUES (1)"), Some("todos".to_string()));
        assert_eq!(extract_table_name("  INSERT   INTO   \"todos\" ..."), Some("todos".to_string()));
    }

    #[test]
    fn test_extract_table_name_updates() {
        assert_eq!(extract_table_name("UPDATE users SET name = 'Alice'"), Some("users".to_string()));
        assert_eq!(extract_table_name("UPDATE [users] SET x = 1"), Some("users".to_string()));
        assert_eq!(extract_table_name("UPDATE `users` SET x = 1"), Some("users".to_string()));
    }

    #[test]
    fn test_extract_table_name_deletes() {
        assert_eq!(extract_table_name("DELETE FROM messages WHERE id = 1"), Some("messages".to_string()));
        assert_eq!(extract_table_name("DELETE FROM [messages]"), Some("messages".to_string()));
    }

    #[test]
    fn test_extract_table_name_invalid_or_select() {
        assert_eq!(extract_table_name("SELECT * FROM todos"), None);
        assert_eq!(extract_table_name("INSERT INTO"), None);
        assert_eq!(extract_table_name("UPDATE"), None);
        assert_eq!(extract_table_name("DELETE FROM"), None);
        assert_eq!(extract_table_name(""), None);
    }
}

