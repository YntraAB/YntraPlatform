use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;
use yntra_core::database;
use yntra_core::services::billing::{
    check_feature_tier_gate, get_tier_seat_price, get_workspace_subscription,
};

fn bench_tier_pricing_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Billing Tier Price Resolution");

    group.bench_function("get_tier_seat_price_enterprise", |b| {
        b.iter(|| {
            let price = get_tier_seat_price(black_box("enterprise"));
            black_box(price);
        });
    });

    group.bench_function("get_tier_seat_price_pro", |b| {
        b.iter(|| {
            let price = get_tier_seat_price(black_box("pro"));
            black_box(price);
        });
    });

    group.finish();
}

fn bench_subscription_resolution_and_gate_checking(c: &mut Criterion) {
    let mut group = c.benchmark_group("Workspace Subscription & Feature Gate Checks");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-bill-bench', 'Bill WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-bill-admin', 'ws-bill-bench', 'admin@bill.io', 'hash', 'platform_admin')",
                (),
            )
            .await;
    });

    group.bench_function("get_workspace_subscription", |b| {
        b.to_async(&rt).iter(|| async {
            let sub = get_workspace_subscription(
                black_box("u-bill-admin".to_string()),
                black_box("ws-bill-bench".to_string()),
            )
            .await;
            let _ = black_box(sub);
        });
    });

    for feature_id in ["zk_auth", "p2p_mesh", "custom_crdt"].iter() {
        group.bench_with_input(
            BenchmarkId::new("check_feature_tier_gate", feature_id),
            feature_id,
            |b, &fid| {
                b.to_async(&rt).iter(|| async {
                    let gate = check_feature_tier_gate(
                        black_box("u-bill-admin".to_string()),
                        black_box("ws-bill-bench".to_string()),
                        black_box(fid.to_string()),
                    )
                    .await;
                    let _ = black_box(gate);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_tier_pricing_calculations,
    bench_subscription_resolution_and_gate_checking
);
criterion_main!(benches);
