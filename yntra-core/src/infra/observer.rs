use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

static TRANSACTION_SEQUENCE_NUMBER: AtomicU64 = AtomicU64::new(1);

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
    fn on_sync_status_changed(&self, _pending_count: u32) {}
    fn on_state_sequence_changed(&self, _seq: u64) {}
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct StateReconciliationReport {
    pub current_sequence: u64,
    pub last_acknowledged_sequence: u64,
    pub has_missed_events: bool,
    pub missed_events_count: u64,
}

#[uniffi::export]
pub fn get_current_state_sequence_number() -> u64 {
    TRANSACTION_SEQUENCE_NUMBER.load(Ordering::Relaxed)
}

#[uniffi::export]
pub fn reconcile_foreground_state(last_acknowledged_seq: u64) -> StateReconciliationReport {
    let current_seq = TRANSACTION_SEQUENCE_NUMBER.load(Ordering::Relaxed);
    let has_missed = current_seq > last_acknowledged_seq;
    let missed_count = current_seq.saturating_sub(last_acknowledged_seq);

    StateReconciliationReport {
        current_sequence: current_seq,
        last_acknowledged_sequence: last_acknowledged_seq,
        has_missed_events: has_missed,
        missed_events_count: missed_count,
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
static OBSERVERS: OnceLock<RwLock<Vec<Arc<dyn DatabaseObserver>>>> = OnceLock::new();

fn get_observers() -> &'static RwLock<Vec<Arc<dyn DatabaseObserver>>> {
    OBSERVERS.get_or_init(|| RwLock::new(Vec::new()))
}

#[uniffi::export]
pub fn register_observer(observer: Box<dyn DatabaseObserver>) {
    if let Ok(mut observers) = get_observers().write() {
        observers.push(Arc::from(observer));
    }
}

#[uniffi::export]
pub fn clear_observers() {
    if let Ok(mut observers) = get_observers().write() {
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

    // Clone the list of observers while holding the read lock, then release it immediately
    // to prevent reentrancy deadlocks when invoking external FFI callback code.
    let observers = if let Ok(lock) = get_observers().read() {
        lock.clone()
    } else {
        Vec::new()
    };

    if observers.is_empty() {
        return;
    }

    let dispatch_fn = move || {
        if !records.is_empty() || !tables.is_empty() {
            // 1. Deduplicate records and count updates per table
            let mut unique_records: std::collections::HashSet<(String, String)> =
                std::collections::HashSet::new();
            let mut record_counts_per_table: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            for (table, id) in records {
                if unique_records.insert((table.clone(), id)) {
                    *record_counts_per_table.entry(table).or_insert(0) += 1;
                }
            }

            // 2. Coalesce bulk updates (> 5 records per table) or existing table-level flags into table notifications
            let mut tables_to_notify = tables;
            for (table, count) in &record_counts_per_table {
                if *count > 5 {
                    tables_to_notify.insert(table.clone());
                }
            }

            // 3. Filter individual records: fire on_record_changed only if table is not already receiving a full table notification
            let filtered_records: Vec<(String, String)> = unique_records
                .into_iter()
                .filter(|(table, _)| !tables_to_notify.contains(table))
                .collect();

            // 4. Dispatch notifications to observers
            let seq = TRANSACTION_SEQUENCE_NUMBER.fetch_add(1, Ordering::SeqCst) + 1;
            for observer in &observers {
                observer.on_state_sequence_changed(seq);
                for (table, id) in &filtered_records {
                    observer.on_record_changed(table.clone(), id.clone());
                }
                for table in &tables_to_notify {
                    observer.on_table_changed(table.clone());
                }
            }
        } else {
            let seq = TRANSACTION_SEQUENCE_NUMBER.fetch_add(1, Ordering::SeqCst) + 1;
            for observer in observers {
                observer.on_state_sequence_changed(seq);
                observer.on_database_changed();
            }
        }

    };

    #[cfg(test)]
    {
        dispatch_fn();
    }

    #[cfg(not(test))]
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    dispatch_fn();
                });
            } else {
                let rt = crate::database::native::get_runtime();
                rt.spawn(async move {
                    dispatch_fn();
                });
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            dispatch_fn();
        }
    }
}

pub fn notify_sync_status_changed(pending_count: u32) {
    let observers = if let Ok(lock) = get_observers().read() {
        lock.clone()
    } else {
        Vec::new()
    };

    for observer in observers {
        observer.on_sync_status_changed(pending_count);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestObserver {
        record_called: AtomicBool,
        table_called: AtomicBool,
        db_called: AtomicBool,
    }

    impl DatabaseObserver for TestObserver {
        fn on_database_changed(&self) {
            self.db_called.store(true, Ordering::SeqCst);
        }
        fn on_table_changed(&self, _table: String) {
            self.table_called.store(true, Ordering::SeqCst);
        }
        fn on_record_changed(&self, _table: String, _id: String) {
            self.record_called.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_simultaneous_record_and_table_notifications() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        clear_observers();
        discard_observers_dirty_state();

        let obs = Arc::new(TestObserver {
            record_called: AtomicBool::new(false),
            table_called: AtomicBool::new(false),
            db_called: AtomicBool::new(false),
        });

        // Register observer manually
        if let Ok(mut observers) = get_observers().write() {
            observers.push(obs.clone());
        }

        // Set both a modified record and a modified table
        set_last_modified_record("todos", "todo-123");
        set_last_modified_table("users");

        notify_observers();

        assert!(
            obs.record_called.load(Ordering::SeqCst),
            "Record notification should be fired"
        );
        assert!(
            obs.table_called.load(Ordering::SeqCst),
            "Table notification should be fired even when records exist"
        );
        assert!(
            !obs.db_called.load(Ordering::SeqCst),
            "DB fallback notification should not fire when specific events exist"
        );

        clear_observers();
    }

    #[test]
    fn test_bulk_record_coalescing_and_deduplication() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        clear_observers();
        discard_observers_dirty_state();

        let record_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let table_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        struct CounterObserver {
            records: Arc<std::sync::atomic::AtomicUsize>,
            tables: Arc<std::sync::atomic::AtomicUsize>,
        }

        impl DatabaseObserver for CounterObserver {
            fn on_database_changed(&self) {}
            fn on_table_changed(&self, _table: String) {
                self.tables.fetch_add(1, Ordering::SeqCst);
            }
            fn on_record_changed(&self, _table: String, _id: String) {
                self.records.fetch_add(1, Ordering::SeqCst);
            }
        }

        let obs = Arc::new(CounterObserver {
            records: record_count.clone(),
            tables: table_count.clone(),
        });

        if let Ok(mut observers) = get_observers().write() {
            observers.push(obs.clone());
        }

        // Simulate 10 updates to the same table "messages" (above 5 threshold)
        for i in 0..10 {
            set_last_modified_record("messages", &format!("msg-{}", i));
        }

        notify_observers();

        // 10 records for "messages" should collapse into 1 table-level notification
        assert_eq!(
            record_count.load(Ordering::SeqCst),
            0,
            "Per-record notifications should be suppressed during bulk updates"
        );
        assert_eq!(
            table_count.load(Ordering::SeqCst),
            1,
            "Bulk updates should trigger exactly 1 table-level notification"
        );

        clear_observers();
    }
}
