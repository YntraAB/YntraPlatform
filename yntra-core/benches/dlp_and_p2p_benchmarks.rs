use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::database::zero_copy::{
    inspect_payload_dlp_bytes, DlpPolicy, P2PMeshSyncRouter,
};

fn bench_dlp_payload_inspection_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("DLP Payload Content Inspection");
    let policy = DlpPolicy::default();

    // Test payload sizes: 64B, 1KB, 64KB, 1MB
    let payload_sizes = [64, 1024, 65536, 1048576];

    for size in payload_sizes.iter() {
        // Construct realistic test payload with occasional PHI/SSN strings embedded
        let mut base_text = "Patient record update: Patient SSN 000-12-3456 diagnosis ICD10-J20.9. ".repeat(size / 70 + 1);
        base_text.truncate(*size);
        let payload_bytes = base_text.into_bytes();

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("inspect_payload_dlp_bytes", size),
            size,
            |b, _| {
                b.iter(|| {
                    let res = inspect_payload_dlp_bytes(
                        black_box(&payload_bytes),
                        black_box(&policy),
                    );
                    black_box(res);
                });
            },
        );
    }

    group.finish();
}

fn bench_p2p_mesh_sync_router(c: &mut Criterion) {
    let mut group = c.benchmark_group("P2P Mesh Sync Router");

    let router = P2PMeshSyncRouter::new();
    let remote_peer = "peer_node_beta_9876543210fedcba9876543210fedcba9876543210fedcba9876".to_string();

    group.bench_function("register_peer_connection", |b| {
        b.iter(|| {
            let res = router.register_peer(black_box(remote_peer.clone()));
            black_box(res);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dlp_payload_inspection_throughput,
    bench_p2p_mesh_sync_router
);
criterion_main!(benches);
