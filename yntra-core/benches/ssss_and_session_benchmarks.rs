use criterion::{black_box, criterion_group, criterion_main, Criterion};
use yntra_core::infra::crypto::ssss::{split_secret, reconstruct_secret, derive_webauthn_prf_key};
use yntra_core::infra::crypto::keychain::{encrypt_workspace_key_with_password, decrypt_workspace_key_with_password};
use yntra_core::infra::crypto::ephemeral_session::{
    register_ephemeral_session_token, validate_active_session_token, clear_active_session_token,
};

fn bench_shamir_secret_sharing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Shamir Secret Sharing (SSSS)");
    let master_secret = [0x42u8; 32];

    for &(k, n) in &[(2, 3), (3, 5), (5, 10)] {
        group.bench_function(format!("split_secret_{}_of_{}", k, n), |b| {
            b.iter(|| {
                let shares = split_secret(black_box(&master_secret), k, n).unwrap();
                black_box(shares);
            });
        });

        let shares = split_secret(&master_secret, k, n).unwrap();
        let subset_shares = shares[0..k].to_vec();

        group.bench_function(format!("reconstruct_secret_{}_of_{}", k, n), |b| {
            b.iter(|| {
                let recovered = reconstruct_secret(black_box(&subset_shares), black_box(k)).unwrap();
                black_box(recovered);
            });
        });
    }

    group.finish();
}

fn bench_webauthn_prf_and_keychain(c: &mut Criterion) {
    let mut group = c.benchmark_group("WebAuthn PRF & Keychain Password Wrapping");

    let credential_id = "cred_webauthn_100";
    let client_salt = [0x55u8; 32];
    let hmac_output = [0x77u8; 32];
    let master_key = [0x55u8; 32];
    let pass = "CorrectHorseBatteryStaple2026!";

    group.bench_function("derive_webauthn_prf_key", |b| {
        b.iter(|| {
            let derived = derive_webauthn_prf_key(
                black_box(credential_id),
                black_box(&client_salt),
                black_box(&hmac_output),
            ).unwrap();
            black_box(derived);
        });
    });

    group.bench_function("encrypt_workspace_key_with_password", |b| {
        b.iter(|| {
            let enc = encrypt_workspace_key_with_password(
                black_box(pass.to_string()),
                black_box(master_key.to_vec()),
            ).unwrap();
            black_box(enc);
        });
    });

    let encrypted_payload = encrypt_workspace_key_with_password(pass.to_string(), master_key.to_vec()).unwrap();
    group.bench_function("decrypt_workspace_key_with_password", |b| {
        b.iter(|| {
            let dec = decrypt_workspace_key_with_password(
                black_box(pass.to_string()),
                black_box(&encrypted_payload),
            ).unwrap();
            black_box(dec);
        });
    });

    group.finish();
}

fn bench_ephemeral_session_token_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ephemeral Session Token Engine");

    let key_hex = const_hex::encode([0x88u8; 32]);
    let workspace_id = "ws-bench-ephemeral-100";

    register_ephemeral_session_token(
        "tok-100".to_string(),
        "usr-100".to_string(),
        workspace_id.to_string(),
        86400,
        key_hex.clone(),
    )
    .unwrap();

    group.bench_function("validate_active_session_token", |b| {
        b.iter(|| {
            let res = validate_active_session_token(black_box(workspace_id));
            black_box(res).unwrap();
        });
    });

    group.bench_function("register_ephemeral_session_token", |b| {
        b.iter(|| {
            let res = register_ephemeral_session_token(
                black_box("tok-bench".to_string()),
                black_box("usr-bench".to_string()),
                black_box(workspace_id.to_string()),
                black_box(3600),
                black_box(key_hex.clone()),
            );
            black_box(res).unwrap();
        });
    });

    clear_active_session_token();
    group.finish();
}

criterion_group!(
    benches,
    bench_shamir_secret_sharing,
    bench_webauthn_prf_and_keychain,
    bench_ephemeral_session_token_validation
);
criterion_main!(benches);
