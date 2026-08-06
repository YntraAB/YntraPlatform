use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::database::zero_copy::ZkCryptoTrust;
use yntra_core::services::auth::hardware::{
    verify_fda_part11_intent_signature, FdaPart11Signature,
};

fn bench_zk_schema_compliance_proofs(c: &mut Criterion) {
    let mut group = c.benchmark_group("ZK Schema Compliance Proofs");
    group.throughput(Throughput::Elements(1));
    let zk = ZkCryptoTrust::new();

    let passkey_seed = "usr_passkey_seed_master_phrase_999!".to_string();
    let sample_payload = "41424344454631323334353637383930".to_string(); // Hex encoded ASCII
    let user_id = "usr_doctor_alice".to_string();
    let role = "DOCTOR".to_string();

    let public_key_hex = zk.derive_public_key(passkey_seed.clone()).unwrap();
    let data_hash_hex = blake3::hash(sample_payload.as_bytes()).to_string();

    group.bench_function("generate_compliance_proof", |b| {
        b.iter(|| {
            let proof = zk.generate_compliance_proof(
                black_box(passkey_seed.clone()),
                black_box(sample_payload.clone()),
                black_box(user_id.clone()),
                black_box(role.clone()),
            );
            black_box(proof).unwrap();
        });
    });

    let proof_hex = zk
        .generate_compliance_proof(
            passkey_seed.clone(),
            sample_payload.clone(),
            user_id.clone(),
            role.clone(),
        )
        .unwrap();

    group.bench_function("verify_compliance_proof", |b| {
        b.iter(|| {
            let res = zk.verify_compliance_proof(
                black_box(proof_hex.clone()),
                black_box(user_id.clone()),
                black_box(role.clone()),
                black_box(data_hash_hex.clone()),
                black_box(public_key_hex.clone()),
            );
            black_box(res).unwrap();
        });
    });

    group.finish();
}

fn bench_aos_ring_compliance_proofs(c: &mut Criterion) {
    let mut group = c.benchmark_group("AOS 1-out-of-N Ring Compliance Proofs");
    let zk = ZkCryptoTrust::new();

    let passkey_seed = "usr_ring_signer_seed_777".to_string();
    let signer_pk = zk.derive_public_key(passkey_seed.clone()).unwrap();
    let data_hex = "00112233445566778899aabbccddeeff".to_string();
    let data_hash_hex = blake3::hash(&const_hex::decode(&data_hex).unwrap()).to_string();

    for ring_size in [3, 5, 10].iter() {
        let mut ring_keys = vec![signer_pk.clone()];
        for i in 1..*ring_size {
            let dummy_seed = format!("dummy_peer_seed_{}", i);
            ring_keys.push(zk.derive_public_key(dummy_seed).unwrap());
        }

        group.bench_with_input(
            BenchmarkId::new("generate_ring_compliance_proof", ring_size),
            ring_size,
            |b, _| {
                b.iter(|| {
                    let proof = zk.generate_ring_compliance_proof(
                        black_box(passkey_seed.clone()),
                        black_box(data_hex.clone()),
                        black_box(ring_keys.clone()),
                    );
                    black_box(proof).unwrap();
                });
            },
        );

        let proof_hex = zk
            .generate_ring_compliance_proof(
                passkey_seed.clone(),
                data_hex.clone(),
                ring_keys.clone(),
            )
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("verify_ring_compliance_proof", ring_size),
            ring_size,
            |b, _| {
                b.iter(|| {
                    let res = zk.verify_ring_compliance_proof(
                        black_box(proof_hex.clone()),
                        black_box(data_hash_hex.clone()),
                        black_box(ring_keys.clone()),
                    );
                    black_box(res).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_passkey_escrow_envelope_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("Passkey Envelope Encryption & Escrow Wrap");
    let zk = ZkCryptoTrust::new();

    let passkey_seed = "passkey_seed_user_secret_001".to_string();
    let plaintext = "Sensitive Protected Health Information (PHI) Patient Record #998877".to_string();

    group.bench_function("encrypt_workspace_field", |b| {
        b.iter(|| {
            let enc = zk.encrypt_workspace_field(
                black_box(passkey_seed.clone()),
                black_box(plaintext.clone()),
            );
            black_box(enc).unwrap();
        });
    });

    let ciphertext_hex = zk
        .encrypt_workspace_field(passkey_seed.clone(), plaintext.clone())
        .unwrap();

    group.bench_function("decrypt_workspace_field", |b| {
        b.iter(|| {
            let dec = zk.decrypt_workspace_field(
                black_box(passkey_seed.clone()),
                black_box(ciphertext_hex.clone()),
            );
            black_box(dec).unwrap();
        });
    });

    group.finish();
}

fn bench_break_glass_emergency_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("Break-Glass Emergency Decryption & ZKP Audit");
    let zk = ZkCryptoTrust::new();

    let passkey_seed = "user_passkey_seed_break_glass".to_string();
    let plaintext = "CRITICAL PHI: Emergency Allergy Alert for ICU Patient 445".to_string();

    let ciphertext_hex = zk
        .encrypt_workspace_field(passkey_seed, plaintext)
        .unwrap();

    let escrow_private_seed = "YNTRA_DEFAULT_INSTITUTIONAL_ESCROW_MASTER_SEED".to_string();
    let operator_id = "op_dr_smith_er".to_string();
    let patient_id = "pat_icu_445".to_string();
    let emergency_reason = "Anaphylactic Shock ICU intervention".to_string();

    group.bench_function("decrypt_workspace_field_break_glass", |b| {
        b.iter(|| {
            let res = zk.decrypt_workspace_field_break_glass(
                black_box(escrow_private_seed.clone()),
                black_box(ciphertext_hex.clone()),
                black_box(operator_id.clone()),
                black_box(patient_id.clone()),
                black_box(emergency_reason.clone()),
            );
            black_box(res).unwrap();
        });
    });

    let break_glass_res = zk
        .decrypt_workspace_field_break_glass(
            escrow_private_seed.clone(),
            ciphertext_hex,
            operator_id.clone(),
            patient_id.clone(),
            emergency_reason.clone(),
        )
        .unwrap();

    let dek_hash_hex = const_hex::encode([42u8; 32]);
    let escrow_public_key_hex = "".to_string();

    group.bench_function("verify_break_glass_audit_proof", |b| {
        b.iter(|| {
            let res = zk.verify_break_glass_audit_proof(
                black_box(break_glass_res.audit_proof_hex.clone()),
                black_box(operator_id.clone()),
                black_box(patient_id.clone()),
                black_box(emergency_reason.clone()),
                black_box(dek_hash_hex.clone()),
                black_box(escrow_public_key_hex.clone()),
            );
            let _ = black_box(res);
        });
    });

    group.finish();
}

fn bench_threshold_shamir_secret_sharing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Threshold Shamir Secret Sharing");
    let zk = ZkCryptoTrust::new();

    let master_seed = "master_escrow_seed_shamir_3_of_5_test".to_string();

    group.bench_function("generate_threshold_escrow_shares_3_of_5", |b| {
        b.iter(|| {
            let shares = zk.generate_threshold_escrow_shares(
                black_box(master_seed.clone()),
                black_box(3),
                black_box(5),
            );
            black_box(shares).unwrap();
        });
    });

    let shares = zk
        .generate_threshold_escrow_shares(master_seed, 3, 5)
        .unwrap();
    let subset = shares[0..3].to_vec();

    group.bench_function("combine_threshold_escrow_shares_3_of_5", |b| {
        b.iter(|| {
            let recovered = zk.combine_threshold_escrow_shares(black_box(subset.clone()));
            black_box(recovered).unwrap();
        });
    });

    group.finish();
}

fn bench_fda_part11_intent_signatures(c: &mut Criterion) {
    let mut group = c.benchmark_group("FDA 21 CFR Part 11 Intent Signatures");

    let sig_payload = FdaPart11Signature {
        signature_id: "fda_sig_12345678".to_string(),
        signer_user_id: "user_dr_jones".to_string(),
        signer_printed_name: "Dr. Indiana Jones".to_string(),
        workspace_id: "ws_hospital_care".to_string(),
        target_record_id: "rec_presc_77".to_string(),
        manifested_intent: "APPROVED_PRESCRIPTION_DOSAGE".to_string(),
        timestamp_rfc3339: "2026-08-06T22:00:00Z".to_string(),
        ed25519_signature_hex: "00".repeat(64),
        public_key_hex: "00".repeat(32),
        is_valid: true,
    };

    group.bench_function("verify_fda_part11_intent_signature", |b| {
        b.iter(|| {
            let res = verify_fda_part11_intent_signature(black_box(sig_payload.clone()));
            let _ = black_box(res);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_zk_schema_compliance_proofs,
    bench_aos_ring_compliance_proofs,
    bench_passkey_escrow_envelope_encryption,
    bench_break_glass_emergency_decryption,
    bench_threshold_shamir_secret_sharing,
    bench_fda_part11_intent_signatures
);
criterion_main!(benches);
