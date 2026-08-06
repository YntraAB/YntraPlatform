use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::services::pre_sync_projection::PreSyncProjectionEngine;

fn bench_pre_sync_projection_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pre-Sync Role Projection Filtering");

    let sample_json = r#"{
        "id": "tr_1001",
        "employee_id": "emp_555",
        "hours": 8.5,
        "hourly_rate": 65.0,
        "pay_rate": 65.0,
        "notes": "Routine site maintenance"
    }"#;

    group.bench_function("apply_projection_worker_role", |b| {
        b.iter(|| {
            let res = PreSyncProjectionEngine::apply_projection(
                black_box("worker"),
                black_box("time_reports"),
                black_box(sample_json),
                black_box(&[]),
            );
            black_box(res);
        });
    });

    group.bench_function("apply_projection_admin_role", |b| {
        b.iter(|| {
            let res = PreSyncProjectionEngine::apply_projection(
                black_box("admin"),
                black_box("time_reports"),
                black_box(sample_json),
                black_box(&[]),
            );
            black_box(res);
        });
    });

    group.finish();
}

fn bench_pre_sync_write_back_sanitization(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pre-Sync Write-Back Sanitization");

    let client_payload = r#"{
        "id": "tr_1001",
        "employee_id": "emp_555",
        "hours": 9.0,
        "hourly_rate": "[RESTRICTED_PAY_RATE]",
        "notes": "Updated maintenance log"
    }"#;

    let server_authoritative = r#"{
        "id": "tr_1001",
        "employee_id": "emp_555",
        "hours": 8.5,
        "hourly_rate": 65.0,
        "notes": "Routine site maintenance"
    }"#;

    for batch_count in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*batch_count as u64));

        group.bench_with_input(
            BenchmarkId::new("sanitize_write_back_payload_batch", batch_count),
            batch_count,
            |b, &count| {
                b.iter(|| {
                    for _ in 0..count {
                        let res = PreSyncProjectionEngine::sanitize_write_back_payload(
                            black_box("field_worker"),
                            black_box("time_reports"),
                            black_box(client_payload),
                            black_box(server_authoritative),
                            black_box(&[]),
                        );
                        black_box(res);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_pre_sync_projection_filtering,
    bench_pre_sync_write_back_sanitization
);
criterion_main!(benches);
