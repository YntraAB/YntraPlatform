use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use yntra_core::infra::auth::{
    get_auth_context_cache, insert_auth_context_cache, invalidate_auth_context_cache_for_user,
    validate_id, AuthContext,
};
use yntra_core::infra::crypto::ephemeral_session::{
    register_ephemeral_session_token, validate_active_session_token,
};
use yntra_core::services::auth::totp::{generate_totp_secret, verify_user_totp};

fn bench_auth_identifier_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Auth & ID Validation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("validate_id_valid_user_id", |b| {
        b.iter(|| {
            let res = validate_id(black_box("user-admin-uuid-1234-5678"), black_box("user_id"));
            black_box(res).unwrap();
        });
    });

    group.bench_function("validate_id_valid_workspace_urn", |b| {
        b.iter(|| {
            let res = validate_id(
                black_box("urn:yntra:workspace:care:ws-999"),
                black_box("workspace_id"),
            );
            black_box(res).unwrap();
        });
    });

    group.finish();
}

fn bench_totp_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("TOTP Secret & Token Operations");
    group.throughput(Throughput::Elements(1));

    group.bench_function("generate_totp_secret", |b| {
        b.iter(|| {
            let secret = generate_totp_secret();
            black_box(secret);
        });
    });

    let secret = generate_totp_secret();

    group.bench_function("verify_user_totp_invalid_code", |b| {
        b.iter(|| {
            let is_valid =
                verify_user_totp(black_box(secret.clone()), black_box("000000".to_string()));
            black_box(is_valid);
        });
    });

    group.finish();
}

fn bench_auth_context_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("Auth Context Cache Operations");
    group.throughput(Throughput::Elements(1));

    let auth = AuthContext {
        user_id: "user-cache-test-123".to_string(),
        role: "ADMIN".to_string(),
        workspace_id: "ws-cache-test-456".to_string(),
        is_admin: true,
        workspace_settings: None,
    };

    group.bench_function("insert_and_get_auth_cache", |b| {
        b.iter(|| {
            insert_auth_context_cache("user-cache-test-123", black_box(auth.clone()));
            let cached = get_auth_context_cache("user-cache-test-123");
            black_box(cached).unwrap();
        });
    });

    group.bench_function("invalidate_user_auth_cache", |b| {
        b.iter(|| {
            invalidate_auth_context_cache_for_user(black_box("user-cache-test-123"));
        });
    });

    group.finish();
}

fn bench_ephemeral_session_tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ephemeral Session Token Management");
    group.throughput(Throughput::Elements(1));
    let key_hex = "00".repeat(32);

    group.bench_function("register_ephemeral_session_token", |b| {
        b.iter(|| {
            let token = register_ephemeral_session_token(
                black_box("token_abc_123".to_string()),
                black_box("user_doctor_45".to_string()),
                black_box("ws_care_hospital".to_string()),
                black_box(3600),
                black_box(key_hex.clone()),
            );
            black_box(token).unwrap();
        });
    });

    let _ = register_ephemeral_session_token(
        "token_abc_123".to_string(),
        "user_doctor_45".to_string(),
        "ws_care_hospital".to_string(),
        3600,
        key_hex,
    );

    group.bench_function("validate_active_session_token", |b| {
        b.iter(|| {
            let active = validate_active_session_token(black_box("ws_care_hospital"));
            black_box(active).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_auth_identifier_validation,
    bench_totp_operations,
    bench_auth_context_cache,
    bench_ephemeral_session_tokens
);
criterion_main!(benches);
