use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::services::time_integrity::TimeIntegrityEngine;

fn bench_time_integrity_monotonic_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Clock & Timestamp Monotonicity Verification");

    group.bench_function("verify_timestamp_integrity_monotonic", |b| {
        b.iter(|| {
            let record = TimeIntegrityEngine::verify_timestamp_integrity(
                black_box("device_alpha_99"),
                black_box(1770000000100i64),
                black_box(1770000000000i64),
                black_box(42),
            );
            black_box(record);
        });
    });

    group.bench_function("verify_timestamp_integrity_skew_flagged", |b| {
        b.iter(|| {
            let record = TimeIntegrityEngine::verify_timestamp_integrity_with_skew_tolerance(
                black_box("device_beta_88"),
                black_box(1770000000000i64),
                black_box(1770000200000i64), // 200s backward skew > 120s default
                black_box(100),
                black_box(120_000),
            );
            black_box(record);
        });
    });

    group.finish();
}

fn bench_batch_vector_clock_sequence_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Batch Vector Clock Event Sequence Processing");

    for batch_size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("process_vector_clock_sequence", batch_size),
            batch_size,
            |b, &size| {
                b.iter(|| {
                    let mut last_ts = 1770000000000i64;
                    let mut last_seq = 0u64;

                    for i in 0..size {
                        let claimed_ts = last_ts + (i as i64 * 10);
                        let record = TimeIntegrityEngine::verify_timestamp_integrity(
                            black_box("device_batch_node"),
                            black_box(claimed_ts),
                            black_box(last_ts),
                            black_box(last_seq),
                        );
                        last_ts = record.physical_timestamp_ms;
                        last_seq = record.logical_sequence;
                        black_box(record);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_time_integrity_monotonic_verification,
    bench_batch_vector_clock_sequence_processing
);
criterion_main!(benches);
