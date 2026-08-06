use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;
use yntra_core::database;
use yntra_core::services::event_bus::{dispatch_core_event, register_event_rule};

fn bench_event_rule_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("Event Bus Rule Registration");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-eb-bench', 'Event Bus WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-eb-admin', 'ws-eb-bench', 'admin@eb.io', 'hash', 'platform_admin')",
                (),
            )
            .await;
    });

    group.bench_function("register_event_rule", |b| {
        b.to_async(&rt).iter(|| async {
            let res = register_event_rule(
                black_box("u-eb-admin".to_string()),
                black_box("ws-eb-bench".to_string()),
                black_box("time".to_string()),
                black_box("jobs".to_string()),
                black_box("TimeLogSubmitted".to_string()),
                black_box("UpdateJobCosting".to_string()),
                None,
            )
            .await;
            let _ = black_box(res);
        });
    });

    group.finish();
}

fn bench_event_dispatch_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("Event Bus Core Dispatch");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-eb-dispatch', 'Dispatch WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-eb-admin-2', 'ws-eb-dispatch', 'admin2@eb.io', 'hash', 'platform_admin')",
                (),
            )
            .await;

        let _ = register_event_rule(
            "u-eb-admin-2".to_string(),
            "ws-eb-dispatch".to_string(),
            "time".to_string(),
            "jobs".to_string(),
            "TimeLogSubmitted".to_string(),
            "UpdateJobCosting".to_string(),
            None,
        )
        .await;
    });

    for payload_size in [1, 5, 20].iter() {
        let payload = format!(r#"{{"hours": {}, "job_id": "job_{}"}}"#, payload_size, payload_size);
        group.bench_with_input(
            BenchmarkId::new("dispatch_core_event", payload_size),
            payload_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let res = dispatch_core_event(
                        black_box("u-eb-admin-2".to_string()),
                        black_box("ws-eb-dispatch".to_string()),
                        black_box("time".to_string()),
                        black_box("TimeLogSubmitted".to_string()),
                        black_box(payload.clone()),
                    )
                    .await;
                    let _ = black_box(res);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_event_rule_registration,
    bench_event_dispatch_pipeline
);
criterion_main!(benches);
