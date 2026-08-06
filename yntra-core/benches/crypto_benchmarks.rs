use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use yntra_core::infra::crypto::{
    stretch_key_new, split_secret, reconstruct_secret, generate_workspace_keypair,
    generate_role_signature, verify_role_signature,
};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use blake3::Hasher;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier};

fn bench_argon2_key_stretching(c: &mut Criterion) {
    let mut group = c.benchmark_group("Argon2id Key Stretching");
    let input_key = b"super_secret_user_master_password_123!";

    group.bench_function("stretch_key_32_bytes", |b| {
        b.iter(|| {
            let res = stretch_key_new(black_box(input_key));
            black_box(res).unwrap();
        })
    });
    group.finish();
}

fn bench_blake3_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("BLAKE3 Hashing Throughput");
    for size in [64, 1024, 65536, 1048576].iter() {
        let payload = vec![0u8; *size];
        group.throughput(criterion::Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut hasher = Hasher::new();
                hasher.update(black_box(&payload));
                black_box(hasher.finalize());
            });
        });
    }
    group.finish();
}

fn bench_xchacha20poly1305_aead(c: &mut Criterion) {
    let mut group = c.benchmark_group("XChaCha20Poly1305 AEAD");
    let key = [0u8; 32];
    let nonce_bytes = [1u8; 24];
    let cipher = XChaCha20Poly1305::new(&key.into());
    let nonce = XNonce::from_slice(&nonce_bytes);

    for size in [256, 4096, 65536].iter() {
        let payload = vec![42u8; *size];
        let ciphertext = cipher.encrypt(nonce, payload.as_slice()).unwrap();

        group.throughput(criterion::Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("encrypt", size), size, |b, _| {
            b.iter(|| {
                let res = cipher.encrypt(black_box(nonce), black_box(payload.as_slice()));
                black_box(res).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("decrypt", size), size, |b, _| {
            b.iter(|| {
                let res = cipher.decrypt(black_box(nonce), black_box(ciphertext.as_slice()));
                black_box(res).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_ed25519_signatures(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ed25519 Signatures");
    let key_bytes = [7u8; 32];
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let verifying_key: VerifyingKey = signing_key.verifying_key();
    let message = b"Yntra Platform Transaction Authorization Payload";
    let signature = signing_key.sign(message);

    group.bench_function("ed25519_sign", |b| {
        b.iter(|| {
            let sig = signing_key.sign(black_box(message));
            black_box(sig);
        });
    });

    group.bench_function("ed25519_verify", |b| {
        b.iter(|| {
            let res = verifying_key.verify(black_box(message), black_box(&signature));
            black_box(res).unwrap();
        });
    });

    let workspace_kp = generate_workspace_keypair().unwrap();
    let role_sig = generate_role_signature(
        &workspace_kp.private_key(),
        "user_123",
        "ADMIN",
        "ws_999",
    ).unwrap();

    group.bench_function("yntra_role_signature_verify", |b| {
        b.iter(|| {
            let res = verify_role_signature(
                black_box("user_123"),
                black_box("ADMIN"),
                black_box("ws_999"),
                black_box(&role_sig),
                black_box(&workspace_kp.public_key()),
            );
            black_box(res);
        });
    });

    group.finish();
}

fn bench_shamir_secret_sharing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Shamir Secret Sharing (SSSS)");
    let secret = b"super_secret_escrow_master_encryption_key_32_bytes!!";

    group.bench_function("split_secret_3_of_5", |b| {
        b.iter(|| {
            let shares = split_secret(black_box(secret), black_box(3), black_box(5));
            black_box(shares).unwrap();
        });
    });

    let shares = split_secret(secret, 3, 5).unwrap();
    let sub_shares = shares[0..3].to_vec();

    group.bench_function("reconstruct_secret_3_of_5", |b| {
        b.iter(|| {
            let recovered = reconstruct_secret(black_box(&sub_shares), black_box(3));
            black_box(recovered).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_argon2_key_stretching,
    bench_blake3_hashing,
    bench_xchacha20poly1305_aead,
    bench_ed25519_signatures,
    bench_shamir_secret_sharing
);
criterion_main!(benches);
