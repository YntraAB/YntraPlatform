use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;
use yntra_core::database;
use yntra_core::services::recovery::{
    derive_sso_vdi_fallback_key, generate_passkey_threshold_key_pair,
    reconstruct_master_vault_key_raw, register_threshold_key_node,
};

fn bench_key_derivation_and_pair_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Recovery Key Derivation & Keypair Gen");

    group.bench_function("generate_passkey_threshold_key_pair", |b| {
        b.iter(|| {
            let res = generate_passkey_threshold_key_pair();
            let _ = black_box(res);
        });
    });

    group.bench_function("derive_sso_vdi_fallback_key", |b| {
        b.iter(|| {
            let res = derive_sso_vdi_fallback_key(
                black_box("user_domain_sso_jwt_assertion_payload_header_sig".to_string()),
                black_box("org_kms_secret_key_bytes_payload".to_string()),
            );
            let _ = black_box(res);
        });
    });

    let shares = vec![
        "0102030405".to_string(),
        "0202030405".to_string(),
    ];

    group.bench_function("reconstruct_master_vault_key_raw", |b| {
        b.iter(|| {
            let res = reconstruct_master_vault_key_raw(black_box(shares.clone()), black_box(2));
            let _ = black_box(res);
        });
    });

    group.finish();
}

fn bench_threshold_key_node_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("Threshold Key Node Registration DB Engine");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rec-bench', 'Recovery WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-rec-admin', 'ws-rec-bench', 'admin@rec.io', 'hash', 'admin')",
                (),
            )
            .await;
    });

    for node_idx in [1, 5, 10].iter() {
        let pubkey_hex = format!("node_pubkey_hex_index_{}", node_idx);

        group.bench_with_input(
            BenchmarkId::new("register_threshold_key_node", node_idx),
            node_idx,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let res = register_threshold_key_node(
                        black_box("u-rec-admin".to_string()),
                        black_box("ws-rec-bench".to_string()),
                        black_box(pubkey_hex.clone()),
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
    bench_key_derivation_and_pair_generation,
    bench_threshold_key_node_registration
);
criterion_main!(benches);
