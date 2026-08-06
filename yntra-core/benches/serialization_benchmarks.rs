use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rkyv::access;
use rkyv::api::high::to_bytes;
use yntra_core::models::audit::AuditLogEntry;
use yntra_core::models::team::TodoItem;

fn bench_rkyv_vs_serde_json_audit_logs(c: &mut Criterion) {
    let mut group = c.benchmark_group("AuditLogEntry Serialization Scaling");

    for size in [10, 100, 1000, 5000].iter() {
        let entries: Vec<AuditLogEntry> = (0..*size)
            .map(|i| AuditLogEntry {
                id: format!("audit-entry-id-{}", i),
                workspace_id: "ws-test-workspace-999".to_string(),
                actor_id: "actor-user-456".to_string(),
                target_client_id: Some("target-client-789".to_string()),
                action_type: "PATIENT_RECORD_READ_PHI".to_string(),
                timestamp: 1770000000 + i as i64,
                prev_hash: "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_string(),
                curr_hash: "f9e8d7c6b5a432100987654321fedcba".to_string(),
                seq: i as i64,
                signature: Some("ed25519_sig_bytes_hex_encoded_1234567890".to_string()),
            })
            .collect();

        group.throughput(Throughput::Elements(*size as u64));

        // serde_json serialize
        group.bench_with_input(
            BenchmarkId::new("serde_json_serialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let json_str = serde_json::to_string(black_box(&entries)).unwrap();
                    black_box(json_str);
                });
            },
        );

        let json_bytes = serde_json::to_string(&entries).unwrap();
        // serde_json deserialize
        group.bench_with_input(
            BenchmarkId::new("serde_json_deserialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let deserialized: Vec<AuditLogEntry> =
                        serde_json::from_str(black_box(&json_bytes)).unwrap();
                    black_box(deserialized);
                });
            },
        );

        // rkyv serialize
        group.bench_with_input(
            BenchmarkId::new("rkyv_serialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let bytes = to_bytes::<rkyv::rancor::Error>(black_box(&entries)).unwrap();
                    black_box(bytes);
                });
            },
        );

        let rkyv_bytes = to_bytes::<rkyv::rancor::Error>(&entries).unwrap();
        // rkyv zero-copy access
        group.bench_with_input(
            BenchmarkId::new("rkyv_zero_copy_access", size),
            size,
            |b, _| {
                b.iter(|| {
                    let archived = access::<
                        rkyv::Archived<Vec<AuditLogEntry>>,
                        rkyv::rancor::Error,
                    >(black_box(&rkyv_bytes))
                    .unwrap();
                    black_box(archived);
                });
            },
        );
    }

    group.finish();
}

fn bench_rkyv_vs_serde_json_todo_items(c: &mut Criterion) {
    let mut group = c.benchmark_group("TodoItem Serialization Scaling");

    for size in [10, 100, 1000].iter() {
        let items: Vec<TodoItem> = (0..*size)
            .map(|i| TodoItem {
                id: format!("todo-id-{}", i),
                workspace_id: "ws-care-111".to_string(),
                text: format!("Follow up on lab report for patient {}", i),
                completed: i % 2 == 0,
                updated_at: 1770000000 + i as i64,
                sync_status: "SYNCED".to_string(),
            })
            .collect();

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("serde_json_serialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let json_str = serde_json::to_string(black_box(&items)).unwrap();
                    black_box(json_str);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rkyv_serialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let bytes = to_bytes::<rkyv::rancor::Error>(black_box(&items)).unwrap();
                    black_box(bytes);
                });
            },
        );

        let rkyv_bytes = to_bytes::<rkyv::rancor::Error>(&items).unwrap();
        group.bench_with_input(
            BenchmarkId::new("rkyv_zero_copy_access", size),
            size,
            |b, _| {
                b.iter(|| {
                    let archived = access::<
                        rkyv::Archived<Vec<TodoItem>>,
                        rkyv::rancor::Error,
                    >(black_box(&rkyv_bytes))
                    .unwrap();
                    black_box(archived);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rkyv_vs_serde_json_audit_logs,
    bench_rkyv_vs_serde_json_todo_items
);
criterion_main!(benches);
