use std::sync::{Arc, Mutex};

// Define the observer callback interface that Swift and Kotlin will implement
#[uniffi::export(callback_interface)]
pub trait DatabaseObserver: Send + Sync {
    fn on_table_changed(&self, table: String);
}

// Global thread-safe list of active observers
lazy_static::lazy_static! {
    static ref OBSERVERS: Mutex<Vec<Arc<dyn DatabaseObserver>>> = Mutex::new(Vec::new());
}

// Export registration function to the FFI boundary
#[uniffi::export]
pub fn register_observer(observer: Arc<dyn DatabaseObserver>) {
    let mut observers = OBSERVERS.lock().unwrap();
    observers.push(observer);
}

// Trigger this internal function on database changes to notify all listeners
pub fn notify_observers(table_name: &str) {
    let observers = OBSERVERS.lock().unwrap();
    for obs in observers.iter() {
        obs.on_table_changed(table_name.to_string());
    }
}
