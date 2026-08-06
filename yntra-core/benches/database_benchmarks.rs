use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;
use yntra_core::database::parser::clean_sql;
use yntra_core::infra::observer::{
    clear_observers, notify_observers, reconcile_foreground_state, register_observer,
    set_last_modified_record, DatabaseObserver,
};

struct MockObserver {
    count: Arc<AtomicUsize>,
}

impl DatabaseObserver for MockObserver {
    fn on_database_changed(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

fn bench_sql_cleaning_and_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("SQL Parser & Sanitizer");
    let sql_input = r#"
        -- Query for active client records with encrypted fields
        SELECT id, workspace_id, actor_id, target_client_id, action_type, timestamp, curr_hash
        FROM audit_logs
        WHERE workspace_id = 'ws_prod_001' AND timestamp > 1700000000 -- Filter by epoch
        ORDER BY timestamp DESC LIMIT 500;
    "#;

    group.bench_function("clean_sql_with_comments", |b| {
        b.iter(|| {
            let cleaned = clean_sql(black_box(sql_input));
            black_box(cleaned);
        });
    });

    let cleaned_sql = clean_sql(sql_input);
    let dialect = SQLiteDialect {};

    group.bench_function("sqlparser_parse_sqlite_ast", |b| {
        b.iter(|| {
            let ast = Parser::parse_sql(black_box(&dialect), black_box(&cleaned_sql));
            black_box(ast).unwrap();
        });
    });

    group.finish();
}

fn bench_database_observer_notifications(c: &mut Criterion) {
    let mut group = c.benchmark_group("DatabaseObserver Broadcasts");

    let counter = Arc::new(AtomicUsize::new(0));
    clear_observers();
    register_observer(Box::new(MockObserver {
        count: counter.clone(),
    }));

    group.bench_function("set_last_modified_record_and_notify", |b| {
        b.iter(|| {
            set_last_modified_record(black_box("audit_logs"), black_box("record_999"));
            notify_observers();
        });
    });

    group.bench_function("reconcile_foreground_state", |b| {
        b.iter(|| {
            let report = reconcile_foreground_state(black_box(0));
            black_box(report);
        });
    });

    clear_observers();
    group.finish();
}

fn bench_in_memory_database_crud_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("libSQL In-Memory Database Execution");
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let conn = yntra_core::database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS bench_items (id TEXT PRIMARY KEY, value TEXT, ts INTEGER);",
                (),
            )
            .await;
    });

    group.bench_function("insert_record", |b| {
        b.to_async(&rt).iter(|| async {
            let conn = yntra_core::database::acquire_connection().await.unwrap();
            let id = format!("item_{}", uuid::Uuid::new_v4());
            let _ = conn
                .execute(
                    "INSERT INTO bench_items (id, value, ts) VALUES (?1, ?2, ?3)",
                    yntra_core::params![id, "bench_test_value", 1770000000i64],
                )
                .await;
        });
    });

    group.bench_function("query_row_single", |b| {
        b.to_async(&rt).iter(|| async {
            let conn = yntra_core::database::acquire_connection().await.unwrap();
            let val = conn
                .query_row(
                    "SELECT id FROM bench_items LIMIT 1",
                    (),
                    |r| r.get::<String>(0),
                )
                .await;
            let _ = black_box(val);
        });
    });

    group.finish();
}

fn bench_database_batch_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("libSQL Transactional Batch Commits");
    let rt = Runtime::new().unwrap();

    for batch_size in [100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_insert_transaction", batch_size),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    let conn = yntra_core::database::acquire_connection().await.unwrap();
                    conn.begin_transaction().await.unwrap();
                    for i in 0..size {
                        let id = format!("batch_{}_{}", size, i);
                        conn.execute(
                            "INSERT OR REPLACE INTO bench_items (id, value, ts) VALUES (?1, ?2, ?3)",
                            yntra_core::params![id, "batch_payload", i as i64],
                        )
                        .await
                        .unwrap();
                    }
                    conn.commit().await.unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sql_cleaning_and_parsing,
    bench_database_observer_notifications,
    bench_in_memory_database_crud_operations,
    bench_database_batch_transactions
);
criterion_main!(benches);
