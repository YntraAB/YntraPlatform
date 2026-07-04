use crate::YntraError;
use std::sync::{Mutex, OnceLock};

// Reactive Database Observer callback trait
#[uniffi::export(callback_interface)]
pub trait DatabaseObserver: Send + Sync {
    fn on_database_changed(&self);
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
    if let Ok(observers) = get_observers().lock() {
        for observer in observers.iter() {
            observer.on_database_changed();
        }
    }
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
    Ok(())
}
