use std::sync::{Arc, Mutex, OnceLock};

// Reactive Database Observer callback trait
#[uniffi::export(callback_interface)]
pub trait DatabaseObserver: Send + Sync {
    fn on_database_changed(&self);
    fn on_table_changed(&self, _table: String) {
        self.on_database_changed();
    }
    fn on_record_changed(&self, _table: String, _id: String) {
        self.on_table_changed(_table);
    }
}

static MODIFIED_TABLES: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
static MODIFIED_RECORDS: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();

pub fn set_last_modified_table(table: &str) {
    if let Ok(mut tables) = MODIFIED_TABLES
        .get_or_init(|| Mutex::new(std::collections::HashSet::new()))
        .lock()
    {
        tables.insert(table.to_string());
    }
}

pub fn set_last_modified_record(table: &str, id: &str) {
    if let Ok(mut records) = MODIFIED_RECORDS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
    {
        records.push((table.to_string(), id.to_string()));
    }
}

// Store observers inside Arc to allow thread-safe, lock-free callback invocation
static OBSERVERS: OnceLock<Mutex<Vec<Arc<dyn DatabaseObserver>>>> = OnceLock::new();

fn get_observers() -> &'static Mutex<Vec<Arc<dyn DatabaseObserver>>> {
    OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

#[uniffi::export]
pub fn register_observer(observer: Box<dyn DatabaseObserver>) {
    if let Ok(mut observers) = get_observers().lock() {
        observers.push(Arc::from(observer));
    }
}

#[uniffi::export]
pub fn clear_observers() {
    if let Ok(mut observers) = get_observers().lock() {
        observers.clear();
    }
}

pub fn notify_observers() {
    let tables = if let Ok(mut lock) = MODIFIED_TABLES
        .get_or_init(|| Mutex::new(std::collections::HashSet::new()))
        .lock()
    {
        std::mem::take(&mut *lock)
    } else {
        std::collections::HashSet::new()
    };

    let records = if let Ok(mut lock) = MODIFIED_RECORDS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
    {
        std::mem::take(&mut *lock)
    } else {
        Vec::new()
    };

    // Clone the list of observers while holding the lock, then release it immediately
    // to prevent reentrancy deadlocks when invoking external FFI callback code.
    let observers = if let Ok(lock) = get_observers().lock() {
        lock.clone()
    } else {
        Vec::new()
    };

    if !records.is_empty() {
        for observer in &observers {
            for (table, id) in &records {
                observer.on_record_changed(table.clone(), id.clone());
            }
        }
    } else if !tables.is_empty() {
        for observer in &observers {
            for table in &tables {
                observer.on_table_changed(table.clone());
            }
        }
    } else {
        for observer in observers {
            observer.on_database_changed();
        }
    }
}

pub fn discard_observers_dirty_state() {
    if let Ok(mut lock) = MODIFIED_TABLES
        .get_or_init(|| Mutex::new(std::collections::HashSet::new()))
        .lock()
    {
        lock.clear();
    }
    if let Ok(mut lock) = MODIFIED_RECORDS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
    {
        lock.clear();
    }
}
