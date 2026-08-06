use super::crypto::ZkCryptoTrust;
use super::stores::{ZeroCopyAuditStore, ZeroCopyMessageStore, ZeroCopyNoteStore, ZeroCopyStore};
use super::sync::{EdgeSyncLoop, P2PMeshSyncRouter, in_memory_poll};
use crate::models::{AuditLogEntry, DailyNote, MessageItem, TodoItem};
use std::sync::Arc;

#[test]
fn test_zero_copy_store_read_write_zero_copy() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir
        .join("zero_copy_test_store.db")
        .to_string_lossy()
        .to_string();

    // Clean old test file
    let _ = std::fs::remove_file(&file_path);

    let store = ZeroCopyStore::new(file_path.clone()).unwrap();

    let todo1 = TodoItem {
        id: "todo_1".to_string(),
        workspace_id: "ws_abc".to_string(),
        text: "Implement Zero-Copy Mmap".to_string(),
        completed: false,
        updated_at: 123456789,
        sync_status: "pending".to_string(),
    };

    let todo2 = TodoItem {
        id: "todo_2".to_string(),
        workspace_id: "ws_abc".to_string(),
        text: "P2P WebRTC Fallback Sync".to_string(),
        completed: true,
        updated_at: 987654321,
        sync_status: "synced".to_string(),
    };

    store
        .write_todos(vec![todo1.clone(), todo2.clone()])
        .unwrap();

    // Read via zero-copy search in mmap
    let read1 = store
        .read_todo_zero_copy("todo_1".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read1.id, todo1.id);
    assert_eq!(read1.text, todo1.text);
    assert_eq!(read1.completed, todo1.completed);
    assert_eq!(read1.updated_at, todo1.updated_at);
    assert_eq!(read1.sync_status, todo1.sync_status);

    let read2 = store
        .read_todo_zero_copy("todo_2".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read2.id, todo2.id);
    assert_eq!(read2.completed, todo2.completed);
    assert_eq!(read2.updated_at, todo2.updated_at);

    let read_none = store
        .read_todo_zero_copy("todo_nonexistent".to_string())
        .unwrap();
    assert!(read_none.is_none());

    // Verify in-memory cache consistent get_todos_count & get_todo_at
    assert_eq!(store.get_todos_count().unwrap(), 2);
    assert_eq!(store.get_todo_at(0).unwrap().unwrap().id, todo1.id);
    assert_eq!(store.get_todo_at(1).unwrap().unwrap().id, todo2.id);
    assert!(store.get_todo_at(2).unwrap().is_none());

    // Clean up test file
    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn test_zk_envelope_encryption_and_proof() {
    let trust = ZkCryptoTrust::new();
    let passkey_seed = "my_super_secure_passkey_hardware_seed".to_string();
    let sensitive_data = "Workspace Secret Financial Details".to_string();

    // Derive user public key from passkey_seed for verification in tests
    let seed_zeroed = zeroize::Zeroizing::new(passkey_seed.clone());
    let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
    key_hasher.update(seed_zeroed.as_bytes());
    let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    key_hasher.finalize_xof().fill(&mut *private_key_bytes);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

    // Test encryption/decryption
    let ciphertext = trust
        .encrypt_workspace_field(passkey_seed.clone(), sensitive_data.clone())
        .unwrap();
    let decrypted = trust
        .decrypt_workspace_field(passkey_seed.clone(), ciphertext.clone())
        .unwrap();
    assert_eq!(decrypted, sensitive_data);

    // Test compliance proof
    let proof = trust
        .generate_compliance_proof(
            passkey_seed.clone(),
            ciphertext.clone(),
            "user_123".to_string(),
            "Admin".to_string(),
        )
        .unwrap();
    let ciphertext_bytes = match const_hex::decode(&ciphertext) {
        Ok(b) => b,
        Err(_) => ciphertext.as_bytes().to_vec(),
    };
    let data_hash = blake3::hash(&ciphertext_bytes);
    let data_hash_hex = const_hex::encode(data_hash.as_bytes());
    let is_valid = trust
        .verify_compliance_proof(
            proof,
            "user_123".to_string(),
            "Admin".to_string(),
            data_hash_hex,
            public_key_hex.clone(),
        )
        .unwrap();
    assert!(is_valid);

    // Test role proof (ZK role validation without raw data/secrets)
    let role_proof = trust
        .generate_role_proof(
            passkey_seed.clone(),
            "user_123".to_string(),
            "Admin".to_string(),
        )
        .unwrap();
    let is_role_valid = trust.verify_proof(
        role_proof.clone(),
        "user_123".to_string(),
        "Admin".to_string(),
        public_key_hex.clone(),
    );
    assert!(is_role_valid);

    // Mismatched role should fail validation
    let is_mismatched_role_valid = trust.verify_proof(
        role_proof.clone(),
        "user_123".to_string(),
        "Member".to_string(),
        public_key_hex.clone(),
    );
    assert!(!is_mismatched_role_valid);

    // Mismatched user is ignored in V3 (true zero-knowledge proof of role membership)
    let is_mismatched_user_valid = trust.verify_proof(
        role_proof.clone(),
        "user_456".to_string(),
        "Admin".to_string(),
        public_key_hex.clone(),
    );
    assert!(is_mismatched_user_valid);

    let invalid_role_proof = "not_a_valid_proof_hex_string_too_short".to_string();
    assert!(!trust.verify_proof(
        invalid_role_proof,
        "user_123".to_string(),
        "Admin".to_string(),
        public_key_hex.clone()
    ));
}

#[test]
fn test_break_glass_emergency_recovery_and_zkp_audit_trail() {
    let trust = ZkCryptoTrust::new();
    let passkey_seed = "doctor_hardware_passkey_seed_99".to_string();
    let escrow_seed = "institutional_hospital_cmo_master_escrow_seed_101".to_string();
    let sensitive_medical_data =
        "Patient John Doe: Severe Penicillin Allergy, Anaphylaxis Hazard".to_string();

    // 1. Derive institutional escrow public key
    let (escrow_sk, escrow_vk) = super::crypto::derive_escrow_keypair_from_seed(&escrow_seed).unwrap();
    let escrow_pubkey_hex = const_hex::encode(escrow_vk.to_bytes());

    // 2. Encrypt medical field with dual-recipient envelope (Passkey + Institutional Escrow)
    let envelope_ciphertext = trust
        .encrypt_workspace_field_with_escrow(
            passkey_seed.clone(),
            sensitive_medical_data.clone(),
            escrow_pubkey_hex.clone(),
        )
        .unwrap();

    assert!(envelope_ciphertext.starts_with("zero_copy_escrow_v1:"));

    // 3. Normal Path: Physician on duty decrypts record via Passkey seed
    let normal_decrypted = trust
        .decrypt_workspace_field(passkey_seed.clone(), envelope_ciphertext.clone())
        .unwrap();
    assert_eq!(normal_decrypted, sensitive_medical_data);

    // 4. Emergency Path: Physician is off-duty / lost hardware key. ICU staff performs Break-Glass emergency recovery!
    let operator_id = "dr_smith_icu_duty".to_string();
    let patient_id = "patient_john_doe_88".to_string();
    let emergency_reason = "Code Blue ICU Emergency - Severe Allergy Verification".to_string();

    let break_glass_res = trust
        .decrypt_workspace_field_break_glass(
            escrow_seed.clone(),
            envelope_ciphertext.clone(),
            operator_id.clone(),
            patient_id.clone(),
            emergency_reason.clone(),
        )
        .unwrap();

    assert_eq!(break_glass_res.plaintext, sensitive_medical_data);
    assert_eq!(break_glass_res.operator_id, operator_id);
    assert_eq!(break_glass_res.emergency_reason, emergency_reason);
    assert!(break_glass_res.audit_proof_hex.starts_with("ZKP_BREAK_GLASS_AUDIT_V1:"));

    // Extract DEK hash from DEK unwrapped inside break glass for verification test
    let parts: Vec<&str> = envelope_ciphertext.split(':').collect();
    let payload_ciphertext_bytes = const_hex::decode(parts[4]).unwrap();
    let payload_nonce_bytes = const_hex::decode(parts[3]).unwrap();

    // 5. Verify ZKP Break-Glass Audit Proof
    // Re-extract DEK hash via escrow unwrapping to test standalone audit proof verifier
    let break_glass_wrap_bytes = const_hex::decode(parts[2]).unwrap();
    let ephem_pub_bytes = &break_glass_wrap_bytes[0..32];
    let escrow_wrap_nonce_bytes = &break_glass_wrap_bytes[32..56];
    let enc_escrow_dek = &break_glass_wrap_bytes[56..];

    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    hasher.update(&escrow_sk.to_bytes());
    let hash = hasher.finalize();
    let mut scalar_bytes = zeroize::Zeroizing::new([0u8; 32]);
    scalar_bytes.copy_from_slice(&hash[0..32]);
    scalar_bytes[0] &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;
    let escrow_scalar = curve25519_dalek::scalar::Scalar::from_bytes_mod_order(*scalar_bytes);

    let ephem_pub_point = curve25519_dalek::edwards::CompressedEdwardsY(ephem_pub_bytes.try_into().unwrap())
        .decompress()
        .unwrap();
    let dh_point = escrow_scalar * ephem_pub_point;
    let dh_bytes = dh_point.compress().to_bytes();

    let escrow_context_str =
        crate::infra::crypto::CryptoDomain::BreakGlassEnvelopeEncryption.get_context(1).unwrap();
    let mut escrow_key_hasher = blake3::Hasher::new_derive_key(escrow_context_str);
    escrow_key_hasher.update(&dh_bytes);
    let mut escrow_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    escrow_key_hasher.finalize_xof().fill(&mut *escrow_key_bytes);

    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};
    let escrow_wrap_cipher = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&*escrow_key_bytes));
    let dek_bytes = escrow_wrap_cipher
        .decrypt(XNonce::from_slice(escrow_wrap_nonce_bytes), enc_escrow_dek)
        .unwrap();

    let cipher_dek = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&dek_bytes));
    let decrypted_payload_bytes = cipher_dek
        .decrypt(
            XNonce::from_slice(&payload_nonce_bytes),
            payload_ciphertext_bytes.as_slice(),
        )
        .unwrap();
    assert_eq!(
        String::from_utf8(decrypted_payload_bytes).unwrap(),
        sensitive_medical_data
    );

    let dek_hash = blake3::hash(&dek_bytes);
    let dek_hash_hex = const_hex::encode(dek_hash.as_bytes());

    let is_audit_valid = trust
        .verify_break_glass_audit_proof(
            break_glass_res.audit_proof_hex.clone(),
            operator_id.clone(),
            patient_id.clone(),
            emergency_reason.clone(),
            dek_hash_hex.clone(),
            escrow_pubkey_hex.clone(),
        )
        .unwrap();

    assert!(is_audit_valid, "Valid Break-Glass ZKP Audit Proof was rejected!");

    // 6. Test tampered emergency reason fails audit verification
    let is_tampered_valid = trust
        .verify_break_glass_audit_proof(
            break_glass_res.audit_proof_hex,
            operator_id,
            patient_id,
            "Unauthorized Routine Check".to_string(),
            dek_hash_hex,
            escrow_pubkey_hex,
        )
        .unwrap();

    assert!(!is_tampered_valid, "Tampered Break-Glass Audit Proof was wrongfully accepted!");
}

#[test]
fn test_threshold_escrow_and_pre_decryption_gatekeeping() {
    let trust = ZkCryptoTrust::new();
    let passkey_seed = "physician_passkey_seed_77".to_string();
    let master_escrow_seed = "hospital_cmo_master_key_999".to_string();
    let medical_record = "ICU Emergency Record: Severe Latex Allergy, Intubation Notes".to_string();

    // 1. Generate 2-of-3 threshold escrow shares (CMO, Attending Physician, Compliance Officer)
    let shares = trust.generate_threshold_escrow_shares(master_escrow_seed.clone(), 2, 3).unwrap();
    assert_eq!(shares.len(), 3);

    // Reconstruct master seed using 2 shares (Share 1 and Share 3)
    let subset_shares = vec![shares[0].clone(), shares[2].clone()];
    let reconstructed_seed_hex = trust.combine_threshold_escrow_shares(subset_shares.clone()).unwrap();

    // Derive public key from reconstructed seed
    let (_, escrow_vk) = super::crypto::derive_escrow_keypair_from_seed(&reconstructed_seed_hex).unwrap();
    let escrow_pubkey_hex = const_hex::encode(escrow_vk.to_bytes());

    // Also derive public key directly from original seed
    let (_, orig_vk) = super::crypto::derive_escrow_keypair_from_seed(&master_escrow_seed).unwrap();
    let orig_pubkey_hex = const_hex::encode(orig_vk.to_bytes());

    assert_eq!(escrow_pubkey_hex, orig_pubkey_hex, "Shamir threshold reconstruction failed to yield identical master key!");

    // 2. Encrypt record with threshold escrow public key
    let ciphertext = trust
        .encrypt_workspace_field_with_escrow(passkey_seed.clone(), medical_record.clone(), escrow_pubkey_hex)
        .unwrap();

    // 3. Test Pre-Decryption Gatekeeping failure on empty parameters
    let gatekeep_res = trust.decrypt_workspace_field_break_glass_threshold(
        subset_shares.clone(),
        ciphertext.clone(),
        "".to_string(),
        "patient_44".to_string(),
        "ICU Emergency".to_string(),
    );
    assert!(gatekeep_res.is_err(), "Pre-decryption gatekeeping failed to block empty operator_id!");

    // 4. Test Successful Threshold Break-Glass Decryption (2-of-3 shares)
    let break_glass_res = trust
        .decrypt_workspace_field_break_glass_threshold(
            subset_shares,
            ciphertext.clone(),
            "dr_jones_icu".to_string(),
            "patient_44".to_string(),
            "Code Red Emergency Admission".to_string(),
        )
        .unwrap();

    assert_eq!(break_glass_res.plaintext, medical_record);
    assert_eq!(break_glass_res.operator_id, "dr_jones_icu");

    // 5. Test Workspace-Isolated Escrow Derivation
    let ws_ciphertext = trust
        .encrypt_workspace_field_for_workspace(
            passkey_seed.clone(),
            medical_record.clone(),
            "workspace_hospital_general".to_string(),
        )
        .unwrap();

    let ws_decrypted = trust.decrypt_workspace_field(passkey_seed, ws_ciphertext).unwrap();
    assert_eq!(ws_decrypted, medical_record);
}



#[test]
fn test_zkp_schema_proof_hijacking_prevention() {
    let trust = ZkCryptoTrust::new();
    let passkey_seed_1 = "my_secure_seed_1".to_string();
    let passkey_seed_2 = "my_secure_seed_2".to_string();

    // Derive public keys for verification
    let pk_hex_2 = trust.derive_public_key(passkey_seed_2.clone()).unwrap();

    // 1. Generate valid proof 1 (representing a valid transaction/document)
    let sensitive_data_1 = "Valid content".to_string();
    let ciphertext_1 = trust
        .encrypt_workspace_field(passkey_seed_1.clone(), sensitive_data_1)
        .unwrap();
    let proof_1_hex = trust
        .generate_compliance_proof(
            passkey_seed_1.clone(),
            ciphertext_1.clone(),
            "user_1".to_string(),
            "user".to_string(),
        )
        .unwrap();

    let proof_1_bytes = const_hex::decode(&proof_1_hex).unwrap();
    assert_eq!(proof_1_bytes.len(), 269);
    // Extracted schema proof fields from proof 1
    let schema_c_bytes = &proof_1_bytes[173..205];
    let schema_e_bytes = &proof_1_bytes[205..237];
    let schema_s_bytes = &proof_1_bytes[237..269];

    // 2. Generate another valid proof 2 (representing a different transaction/document)
    let sensitive_data_2 = "Different valid content".to_string();
    let ciphertext_2 = trust
        .encrypt_workspace_field(passkey_seed_2.clone(), sensitive_data_2)
        .unwrap();
    let proof_2_hex = trust
        .generate_compliance_proof(
            passkey_seed_2.clone(),
            ciphertext_2.clone(),
            "user_2".to_string(),
            "user".to_string(),
        )
        .unwrap();

    let proof_2_bytes = const_hex::decode(&proof_2_hex).unwrap();

    // 3. Construct a forged proof payload:
    // It uses all elements of proof 2 (so its signature matches ciphertext 2)
    // but replaces the schema validation ZKP fields with the ones hijacked from proof 1!
    let mut proof_forged_bytes = proof_2_bytes.clone();
    proof_forged_bytes[173..205].copy_from_slice(schema_c_bytes);
    proof_forged_bytes[205..237].copy_from_slice(schema_e_bytes);
    proof_forged_bytes[237..269].copy_from_slice(schema_s_bytes);

    let proof_forged_hex = const_hex::encode(&proof_forged_bytes);

    // 4. Try to verify the forged proof against ciphertext 2's data hash.
    // If our schema ZKP binding fix works, it must fail because the schema ZKP was bound
    // to ciphertext 1's commitment, not ciphertext 2's!
    let ciphertext_2_bytes = match const_hex::decode(&ciphertext_2) {
        Ok(b) => b,
        Err(_) => ciphertext_2.as_bytes().to_vec(),
    };
    let data_hash_2 = blake3::hash(&ciphertext_2_bytes);
    let data_hash_2_hex = const_hex::encode(data_hash_2.as_bytes());

    let is_valid = trust
        .verify_compliance_proof(
            proof_forged_hex,
            "user_2".to_string(),
            "user".to_string(),
            data_hash_2_hex,
            pk_hex_2,
        )
        .unwrap();

    assert!(
        !is_valid,
        "ZKP Schema Proof Hijacking succeeded! The system accepted a transposed schema proof!"
    );
}

#[test]
fn test_ring_signatures_aos() {
    // --- Test Ring Signatures (AOS ZKP) ---
    let passkey_seed_1 = "seed_1_secret".to_string();
    let passkey_seed_2 = "seed_2_secret".to_string();
    let passkey_seed_3 = "seed_3_secret".to_string();

    let trust = ZkCryptoTrust::new();
    let pk_1 = trust.derive_public_key(passkey_seed_1.clone()).unwrap();
    let pk_2 = trust.derive_public_key(passkey_seed_2.clone()).unwrap();
    let pk_3 = trust.derive_public_key(passkey_seed_3.clone()).unwrap();

    let ring = vec![pk_1.clone(), pk_2.clone(), pk_3.clone()];
    let data = "Zero-Knowledge anonymous write payload".to_string();
    let hex_data = const_hex::encode(data.as_bytes());
    let data_hash = blake3::hash(data.as_bytes());
    let data_hash_hex = const_hex::encode(data_hash.as_bytes());

    // Sign using seed 2 (Peer 2)
    let ring_proof = trust
        .generate_ring_compliance_proof(passkey_seed_2.clone(), hex_data.clone(), ring.clone())
        .unwrap();

    // Verify it against the ring
    let is_ring_valid = trust
        .verify_ring_compliance_proof(ring_proof.clone(), data_hash_hex.clone(), ring.clone())
        .unwrap();
    assert!(is_ring_valid);

    // Verify it using verify_compliance_proof (combining with commas)
    let ring_comb = ring.join(",");
    let is_comb_valid = trust
        .verify_compliance_proof(
            ring_proof.clone(),
            "".to_string(),
            "".to_string(),
            data_hash_hex.clone(),
            ring_comb.clone(),
        )
        .unwrap();
    assert!(is_comb_valid);

    // Signing using a seed not in the ring should fail
    let passkey_seed_foreign = "foreign_seed".to_string();
    let ring_proof_foreign = trust.generate_ring_compliance_proof(
        passkey_seed_foreign.clone(),
        hex_data.clone(),
        ring.clone(),
    );
    assert!(ring_proof_foreign.is_err());

    // --- Test Schema Validity ZKP ---
    // If the data is empty (invalid schema constraint), generation still creates a mock proof, but verification must fail.
    let empty_data_hex = const_hex::encode("".as_bytes());
    let empty_data_hash = blake3::hash("".as_bytes());
    let empty_data_hash_hex = const_hex::encode(empty_data_hash.as_bytes());
    let invalid_schema_proof = trust
        .generate_ring_compliance_proof(passkey_seed_2.clone(), empty_data_hex, ring.clone())
        .unwrap();
    let is_invalid_schema_verified = trust
        .verify_ring_compliance_proof(invalid_schema_proof, empty_data_hash_hex, ring.clone())
        .unwrap();
    assert!(!is_invalid_schema_verified);

    // --- Test Zero-Knowledge Ring Role Membership Proofs ---
    let role_ring = ring.clone();
    let user_id = "user_role_test_123".to_string();
    let role = "workspace_member".to_string();

    let ring_role_proof = trust
        .generate_ring_role_proof(
            passkey_seed_2.clone(),
            user_id.clone(),
            role.clone(),
            role_ring.clone(),
        )
        .unwrap();

    let is_role_ring_verified = trust.verify_proof(
        ring_role_proof.clone(),
        user_id.clone(),
        role.clone(),
        role_ring.join(","),
    );
    assert!(is_role_ring_verified);

    // Mismatched user ID is ignored in V3 anonymous ring role proofs
    assert!(trust.verify_proof(
        ring_role_proof.clone(),
        "different_user".to_string(),
        role.clone(),
        role_ring.join(",")
    ));
    assert!(!trust.verify_proof(
        ring_role_proof.clone(),
        user_id.clone(),
        "different_role".to_string(),
        role_ring.join(",")
    ));

    // Signing using a seed not in the role ring should fail
    let ring_role_proof_foreign = trust.generate_ring_role_proof(
        passkey_seed_foreign,
        user_id.clone(),
        role.clone(),
        role_ring.clone(),
    );
    assert!(ring_role_proof_foreign.is_err());

    // --- Test SOTA Groth16 zk-SNARK Verification over BN254 ---
    use ark_bn254::{Bn254, Fr};
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
    use ark_serialize::CanonicalSerialize;
    use ark_snark::{CircuitSpecificSetupSNARK, SNARK};

    #[derive(Clone)]
    struct SimpleCircuit {
        x: Option<Fr>,
        y: Option<Fr>,
        z: Option<Fr>,
    }

    impl ConstraintSynthesizer<Fr> for SimpleCircuit {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
            let x_val =
                cs.new_witness_variable(|| self.x.ok_or(SynthesisError::AssignmentMissing))?;
            let y_val =
                cs.new_witness_variable(|| self.y.ok_or(SynthesisError::AssignmentMissing))?;
            let z_val =
                cs.new_input_variable(|| self.z.ok_or(SynthesisError::AssignmentMissing))?;
            cs.enforce_constraint(
                ark_relations::r1cs::LinearCombination::from(x_val),
                ark_relations::r1cs::LinearCombination::from(y_val),
                ark_relations::r1cs::LinearCombination::from(z_val),
            )?;
            Ok(())
        }
    }

    use ark_std::rand::{SeedableRng, rngs::StdRng};
    let mut rng = StdRng::seed_from_u64(42u64);
    let empty_circuit = SimpleCircuit {
        x: None,
        y: None,
        z: None,
    };
    let (pk, vk) = Groth16::<Bn254>::setup(empty_circuit, &mut rng).unwrap();

    let x_scalar = Fr::from(3u32);
    let y_scalar = Fr::from(4u32);
    let z_scalar = Fr::from(12u32);
    let circuit = SimpleCircuit {
        x: Some(x_scalar),
        y: Some(y_scalar),
        z: Some(z_scalar),
    };
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();

    let mut proof_bytes = Vec::new();
    proof.serialize_compressed(&mut proof_bytes).unwrap();
    let proof_hex = const_hex::encode(&proof_bytes);

    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes).unwrap();
    let vk_hex = const_hex::encode(&vk_bytes);

    let mut z_bytes = Vec::new();
    z_scalar.serialize_compressed(&mut z_bytes).unwrap();
    let z_hex = const_hex::encode(&z_bytes);

    let public_inputs_hex = vec![z_hex.clone()];
    let is_snark_valid = trust
        .verify_groth16_proof(proof_hex.clone(), public_inputs_hex.clone(), vk_hex.clone())
        .unwrap();
    assert!(is_snark_valid);

    let invalid_z_scalar = Fr::from(13u32);
    let mut invalid_z_bytes = Vec::new();
    invalid_z_scalar
        .serialize_compressed(&mut invalid_z_bytes)
        .unwrap();
    let invalid_z_hex = const_hex::encode(&invalid_z_bytes);
    let invalid_public_inputs_hex = vec![invalid_z_hex];
    let is_invalid_snark_valid = trust
        .verify_groth16_proof(proof_hex, invalid_public_inputs_hex, vk_hex)
        .unwrap();
    assert!(!is_invalid_snark_valid);
}

#[test]
fn test_zero_copy_message_store_read_write_zero_copy() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir
        .join("zero_copy_test_message_store.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&file_path);

    let store = ZeroCopyMessageStore::new(file_path.clone()).unwrap();

    let msg1 = MessageItem {
        id: "msg_1".to_string(),
        workspace_id: "ws_abc".to_string(),
        sender_id: Some("u_1".to_string()),
        receiver_id: Some("u_2".to_string()),
        target_team_id: None,
        subject: Some("Hello".to_string()),
        body: Some("World".to_string()),
        is_read: false,
        created_at: "2026-07-12T12:00:00Z".to_string(),
        updated_at: 123456789,
        sync_status: "pending".to_string(),
    };

    let msg2 = MessageItem {
        id: "msg_2".to_string(),
        workspace_id: "ws_abc".to_string(),
        sender_id: Some("u_2".to_string()),
        receiver_id: Some("u_1".to_string()),
        target_team_id: None,
        subject: Some("Reply".to_string()),
        body: Some("Got it".to_string()),
        is_read: true,
        created_at: "2026-07-12T12:05:00Z".to_string(),
        updated_at: 987654321,
        sync_status: "synced".to_string(),
    };

    store
        .write_messages(vec![msg1.clone(), msg2.clone()])
        .unwrap();

    let read1 = store
        .read_message_zero_copy("msg_1".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read1.id, msg1.id);
    assert_eq!(read1.subject, msg1.subject);
    assert_eq!(read1.body, msg1.body);

    let read2 = store
        .read_message_zero_copy("msg_2".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read2.id, msg2.id);
    assert_eq!(read2.is_read, msg2.is_read);

    let read_none = store
        .read_message_zero_copy("msg_nonexistent".to_string())
        .unwrap();
    assert!(read_none.is_none());

    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn test_zero_copy_audit_store_read_write_zero_copy() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir
        .join("zero_copy_test_audit_store.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&file_path);

    let store = ZeroCopyAuditStore::new(file_path.clone()).unwrap();

    let entry1 = AuditLogEntry {
        id: "entry_1".to_string(),
        workspace_id: "workspace-1".to_string(),
        actor_id: "actor_1".to_string(),
        target_client_id: Some("client_1".to_string()),
        action_type: "create".to_string(),
        timestamp: 123456789,
        prev_hash: "hash_0".to_string(),
        curr_hash: "hash_1".to_string(),
        seq: 1,
        signature: Some("sig_1".to_string()),
    };

    let entry2 = AuditLogEntry {
        id: "entry_2".to_string(),
        workspace_id: "workspace-1".to_string(),
        actor_id: "actor_2".to_string(),
        target_client_id: None,
        action_type: "update".to_string(),
        timestamp: 987654321,
        prev_hash: "hash_1".to_string(),
        curr_hash: "hash_2".to_string(),
        seq: 2,
        signature: None,
    };

    store
        .write_audit_logs(vec![entry1.clone(), entry2.clone()])
        .unwrap();

    let read1 = store
        .read_audit_zero_copy("entry_1".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read1.id, entry1.id);
    assert_eq!(read1.action_type, entry1.action_type);
    assert_eq!(read1.signature, entry1.signature);

    let read2 = store
        .read_audit_zero_copy("entry_2".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read2.id, entry2.id);
    assert_eq!(read2.action_type, entry2.action_type);

    let read_none = store
        .read_audit_zero_copy("entry_nonexistent".to_string())
        .unwrap();
    assert!(read_none.is_none());

    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn test_zero_copy_note_store_read_write_zero_copy() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir
        .join("zero_copy_test_note_store.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&file_path);

    let store = ZeroCopyNoteStore::new(file_path.clone()).unwrap();

    let note1 = DailyNote {
        id: "note_1".to_string(),
        workspace_id: "ws_abc".to_string(),
        team_id: "team_1".to_string(),
        author_id: Some("u_1".to_string()),
        subject: "Meeting Notes".to_string(),
        content: "We discussed zero-copy persistence".to_string(),
        edit_history: "[]".to_string(),
        created_at: "2026-07-12".to_string(),
        updated_at: 123456789,
        sync_status: "pending".to_string(),
    };

    store.write_notes(vec![note1.clone()]).unwrap();

    let read1 = store
        .read_note_zero_copy("note_1".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(read1.id, note1.id);
    assert_eq!(read1.subject, note1.subject);
    assert_eq!(read1.content, note1.content);

    let all = store.read_all_notes().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "note_1");

    let read_none = store
        .read_note_zero_copy("note_nonexistent".to_string())
        .unwrap();
    assert!(read_none.is_none());

    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn test_p2p_mesh_note_sync_in_memory_fallback() {
    let temp_dir = std::env::temp_dir();
    let path_a = temp_dir
        .join("test_note_sync_a.db")
        .to_string_lossy()
        .to_string();
    let path_b = temp_dir
        .join("test_note_sync_b.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    let store_a = Arc::new(ZeroCopyNoteStore::new(path_a.clone()).unwrap());
    let store_b = Arc::new(ZeroCopyNoteStore::new(path_b.clone()).unwrap());

    let uid_str = uuid::Uuid::new_v4().to_string();
    let peer_a = format!("peer_fb_a_{}", uid_str);
    let peer_b = format!("peer_fb_b_{}", uid_str);

    let router = P2PMeshSyncRouter::new();
    router.set_compliance_mode(crate::database::zero_copy::sync::ComplianceMode::UnrestrictedLocalP2P);
    router.register_peer(peer_a.clone());
    router.register_peer(peer_b.clone());

    let note = DailyNote {
        id: "note_x".to_string(),
        workspace_id: "ws_abc".to_string(),
        team_id: "team_1".to_string(),
        author_id: Some(peer_a.clone()),
        subject: "ZK Sync Test".to_string(),
        content: "Encrypted data here".to_string(),
        edit_history: "[]".to_string(),
        created_at: "2026-07-12".to_string(),
        updated_at: 1000,
        sync_status: "pending".to_string(),
    };

    store_a.write_notes(vec![note.clone()]).unwrap();

    let changes = store_a.get_loro_changes().unwrap();
    router.broadcast_write_network(peer_a, changes);

    let updates = in_memory_poll(&peer_b);
    assert_eq!(updates.len(), 1);

    store_b.apply_loro_update(updates[0].clone()).unwrap();

    let notes_b = store_b.read_all_notes().unwrap();
    assert_eq!(notes_b.len(), 1);
    assert_eq!(notes_b[0].id, "note_x");
    assert_eq!(notes_b[0].subject, "ZK Sync Test");

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

#[test]
fn test_p2p_mesh_sync_router_identity() {
    let router = P2PMeshSyncRouter::new();
    // 1. Ephemeral identity should be generated successfully
    let pubkey_hex = router.set_ephemeral_identity().unwrap();
    assert_eq!(pubkey_hex.len(), 64); // hex encoded 32-byte public key is 64 chars

    // 2. Setting a valid 32-byte private key hex should succeed
    let valid_privkey_hex = const_hex::encode(&[3u8; 32]);
    assert!(router.set_identity(valid_privkey_hex).is_ok());

    // 3. Setting an invalid key length should fail
    let invalid_privkey_hex = const_hex::encode(&[3u8; 31]);
    assert!(router.set_identity(invalid_privkey_hex).is_err());
}

#[test]
fn test_loro_crdt_dynamic_reconciliation_and_merge() {
    let temp_dir = std::env::temp_dir();
    let path_a = temp_dir
        .join("test_crdt_merge_a.db")
        .to_string_lossy()
        .to_string();
    let path_b = temp_dir
        .join("test_crdt_merge_b.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    let store_a = ZeroCopyStore::new(path_a.clone()).unwrap();
    let store_b = ZeroCopyStore::new(path_b.clone()).unwrap();

    // Peer A writes Todo 1
    let todo1 = TodoItem {
        id: "todo_1".to_string(),
        workspace_id: "ws_abc".to_string(),
        text: "Todo A".to_string(),
        completed: false,
        updated_at: 100,
        sync_status: "pending".to_string(),
    };
    store_a.write_todos(vec![todo1.clone()]).unwrap();

    // Peer B writes Todo 2 concurrently
    let todo2 = TodoItem {
        id: "todo_2".to_string(),
        workspace_id: "ws_abc".to_string(),
        text: "Todo B".to_string(),
        completed: true,
        updated_at: 200,
        sync_status: "pending".to_string(),
    };
    store_b.write_todos(vec![todo2.clone()]).unwrap();

    // Merge updates
    let changes_a = store_a.get_loro_changes().unwrap();
    let changes_b = store_b.get_loro_changes().unwrap();

    // Apply A's changes to B, and B's changes to A
    store_b.apply_loro_update(changes_a).unwrap();
    store_a.apply_loro_update(changes_b).unwrap();

    // Read all from B and A - they should contain BOTH Todo 1 and Todo 2 merged conflict-free!
    let todos_a = store_a.read_all_todos().unwrap();
    let todos_b = store_b.read_all_todos().unwrap();

    assert_eq!(todos_a.len(), 2);
    assert_eq!(todos_b.len(), 2);

    assert!(todos_a.iter().any(|t| t.id == "todo_1"));
    assert!(todos_a.iter().any(|t| t.id == "todo_2"));

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

#[test]
fn test_edge_sync_loop_clone_drop_safety() {
    let sync_loop = EdgeSyncLoop::new("http://localhost:8080".to_string());

    let temp_dir = std::env::temp_dir();
    let path = temp_dir
        .join("test_loop_drop.db")
        .to_string_lossy()
        .to_string();
    let _ = std::fs::remove_file(&path);
    let store = Arc::new(ZeroCopyStore::new(path.clone()).unwrap());

    sync_loop.start_sync_loop(store, 10).unwrap();
    assert!(sync_loop.is_running());

    // Clone the loop handle
    {
        let clone = sync_loop.clone();
        assert!(clone.is_running());
        // Drop the clone here
    }

    // After dropping the clone, the original loop should STILL be running!
    assert!(sync_loop.is_running());

    // Stop it explicitly
    sync_loop.stop_sync_loop();
    assert!(!sync_loop.is_running());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_loro_crdt_batching() {
    let temp_dir = std::env::temp_dir();
    let path_main = temp_dir
        .join("test_batch_main.db")
        .to_string_lossy()
        .to_string();
    let path_peer_a = temp_dir
        .join("test_batch_peer_a.db")
        .to_string_lossy()
        .to_string();
    let path_peer_b = temp_dir
        .join("test_batch_peer_b.db")
        .to_string_lossy()
        .to_string();

    let _ = std::fs::remove_file(&path_main);
    let _ = std::fs::remove_file(&path_peer_a);
    let _ = std::fs::remove_file(&path_peer_b);

    let store_main = ZeroCopyStore::new(path_main.clone()).unwrap();
    let store_peer_a = ZeroCopyStore::new(path_peer_a.clone()).unwrap();
    let store_peer_b = ZeroCopyStore::new(path_peer_b.clone()).unwrap();

    // Peer A writes Todo 1
    let todo1 = TodoItem {
        id: "todo_1".to_string(),
        workspace_id: "ws_123".to_string(),
        text: "Todo 1".to_string(),
        completed: false,
        updated_at: 100,
        sync_status: "pending".to_string(),
    };
    store_peer_a.write_todos(vec![todo1]).unwrap();
    let change1 = store_peer_a.get_loro_changes().unwrap();

    // Peer B writes Todo 2
    let todo2 = TodoItem {
        id: "todo_2".to_string(),
        workspace_id: "ws_123".to_string(),
        text: "Todo 2".to_string(),
        completed: true,
        updated_at: 200,
        sync_status: "pending".to_string(),
    };
    store_peer_b.write_todos(vec![todo2]).unwrap();
    let change2 = store_peer_b.get_loro_changes().unwrap();

    // Now apply both changes to the main store in a single batch
    store_main
        .apply_loro_updates_batch(vec![change1, change2])
        .unwrap();

    let todos = store_main.read_all_todos().unwrap();
    assert_eq!(todos.len(), 2);
    assert!(todos.iter().any(|t| t.id == "todo_1"));
    assert!(todos.iter().any(|t| t.id == "todo_2"));

    let _ = std::fs::remove_file(&path_main);
    let _ = std::fs::remove_file(&path_peer_a);
    let _ = std::fs::remove_file(&path_peer_b);
}

#[test]
fn test_edge_sync_loop_note_and_audit_loops() {
    let sync_loop = EdgeSyncLoop::new("http://localhost:8080".to_string());

    let temp_dir = std::env::temp_dir();
    let path_note = temp_dir
        .join("test_note_loop.db")
        .to_string_lossy()
        .to_string();
    let path_audit = temp_dir
        .join("test_audit_loop.db")
        .to_string_lossy()
        .to_string();
    let _ = std::fs::remove_file(&path_note);
    let _ = std::fs::remove_file(&path_audit);

    let note_store = Arc::new(ZeroCopyNoteStore::new(path_note.clone()).unwrap());
    let audit_store = Arc::new(ZeroCopyAuditStore::new(path_audit.clone()).unwrap());

    sync_loop.start_note_sync_loop(note_store, 10).unwrap();
    sync_loop.start_audit_sync_loop(audit_store, 10).unwrap();
    assert!(sync_loop.is_running());

    sync_loop.stop_sync_loop();
    assert!(!sync_loop.is_running());

    let _ = std::fs::remove_file(&path_note);
    let _ = std::fs::remove_file(&path_audit);
}

#[test]
fn test_in_memory_relay_queue_bounding() {
    let uid_str = uuid::Uuid::new_v4().to_string();
    let peer_a = format!("peer_q_a_{}", uid_str);
    let peer_b = format!("peer_q_b_{}", uid_str);

    let router = P2PMeshSyncRouter::new();
    router.set_compliance_mode(crate::database::zero_copy::sync::ComplianceMode::UnrestrictedLocalP2P);
    router.register_peer(peer_a.clone());
    router.register_peer(peer_b.clone());

    // Generate 150 valid Loro snapshot updates
    let doc_a = loro::LoroDoc::new();
    let text_a = doc_a.get_text("content");
    let mut updates_list = Vec::new();
    for i in 0..150 {
        text_a.insert(0, &format!("char_{} ", i)).unwrap();
        let snapshot = doc_a.export(loro::ExportMode::Snapshot).unwrap();
        updates_list.push(snapshot);
    }

    // Broadcast all 150 updates
    for u in updates_list {
        router.broadcast_write_network(peer_a.clone(), u);
    }

    // Since the queue is hard-limited to 100 entries, but has a catch-up snapshot prepended:
    let polled = in_memory_poll(&peer_b);
    // 1 catch-up snapshot + 100 updates = 101 polled items
    assert_eq!(polled.len(), 101);

    // Import all polled items into peer_b's doc
    let doc_b = loro::LoroDoc::new();
    for p in polled {
        doc_b.import(&p).unwrap();
    }

    // Verify both docs have exactly the same content
    let text_b = doc_b.get_text("content");
    assert_eq!(text_b.to_string(), text_a.to_string());
}

#[test]
fn test_failed_broadcast_retry_queue() {
    let router = P2PMeshSyncRouter::with_relay("http://invalid-url-domain-xyz.xyz".to_string());

    // Manually push a failed broadcast to the queue
    {
        let mut failed = router.failed_broadcasts.lock().unwrap();
        failed.push(("peer_a".to_string(), vec![1, 2, 3]));
    }

    // Verify it is in the queue
    {
        let failed = router.failed_broadcasts.lock().unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].1, vec![1, 2, 3]);
    }

    // Calling retry should drain it and attempt to send
    router.retry_failed_broadcasts();

    // The queue should be drained
    {
        let failed = router.failed_broadcasts.lock().unwrap();
        assert!(failed.is_empty());
    }
}

#[test]
fn test_p2p_update_envelope_and_signature() {
    let router = P2PMeshSyncRouter::new();
    let pubkey_hex = router.set_ephemeral_identity().unwrap();

    let data = vec![1, 2, 3, 4, 5];
    let timestamp = chrono::Utc::now().timestamp_millis();

    // 1. Generate signature
    let sig_hex = {
        let key_guard = router.signing_key.lock().unwrap();
        let key = key_guard.as_ref().unwrap();

        let mut msg = Vec::new();
        msg.extend_from_slice(b"broadcast:");
        msg.extend_from_slice(pubkey_hex.as_bytes());
        msg.extend_from_slice(b":");
        msg.extend_from_slice(&timestamp.to_be_bytes());
        msg.extend_from_slice(b":");
        msg.extend_from_slice(&data);

        use ed25519_dalek::Signer;
        let sig = key.sign(&msg);
        const_hex::encode(sig.to_bytes())
    };

    // 2. Verify signature with valid params
    let is_valid = super::sync::verify_update_signature(&pubkey_hex, timestamp, &sig_hex, &data);
    assert!(is_valid);

    // 3. Verify signature fails with wrong data
    let is_valid_wrong_data =
        super::sync::verify_update_signature(&pubkey_hex, timestamp, &sig_hex, &[9, 9, 9]);
    assert!(!is_valid_wrong_data);

    // 4. Verify signature fails with expired/drifted timestamp (e.g. 1 hour ago)
    let old_timestamp = timestamp - 3600 * 1000;
    let is_valid_drift =
        super::sync::verify_update_signature(&pubkey_hex, old_timestamp, &sig_hex, &data);
    assert!(!is_valid_drift);
}

#[tokio::test]
async fn test_p2p_sync_workspace_access_control() {
    let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
    let conn = crate::database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test', 'Sync WS', '[]', '{}')", ()).await.unwrap();

    let peer_a = "000000000000000000000000000000000000000000000000000000000000001a";
    let peer_b = "000000000000000000000000000000000000000000000000000000000000001b";
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test', 'a@yntra.se')", crate::params![peer_a]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test', 'b@yntra.se')", crate::params![peer_b]).await.unwrap();

    let peer_c = "000000000000000000000000000000000000000000000000000000000000001c";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test-diff', 'Diff WS', '[]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test-diff', 'c@yntra.se')", crate::params![peer_c]).await.unwrap();

    let auth_ab = super::sync::is_peer_authorized(peer_a, peer_b).await;
    assert!(auth_ab);

    let auth_ac = super::sync::is_peer_authorized(peer_a, peer_c).await;
    assert!(!auth_ac);

    conn.execute(
        "DELETE FROM users WHERE id IN (?1, ?2, ?3)",
        crate::params![peer_a, peer_b, peer_c],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM workspaces WHERE id IN ('ws-sync-test', 'ws-sync-test-diff')",
        (),
    )
    .await
    .unwrap();
}

#[test]
fn test_peer_store_sanitization() {
    // Valid name
    let res1 = super::create_peer_store("valid_peer_123".to_string());
    assert!(res1.is_ok());

    let res2 = super::create_peer_note_store("valid-peer_notes".to_string());
    assert!(res2.is_ok());

    // Invalid name with directory traversal
    let res3 = super::create_peer_store("../attacker".to_string());
    assert!(res3.is_err());

    let res4 = super::create_peer_note_store("/absolute/path".to_string());
    assert!(res4.is_err());

    let res5 = super::create_peer_store("peer*invalid".to_string());
    assert!(res5.is_err());
}

#[test]
fn test_pool_poisoning_recovery() {
    let pool = std::sync::Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::<String>::new(),
    ));

    // Poison the pool lock
    let pool_clone = pool.clone();
    let handle = std::thread::spawn(move || {
        let _guard = pool_clone.lock().unwrap();
        panic!("intentional panic to poison lock");
    });
    let _ = handle.join();

    // Now the lock is poisoned
    assert!(pool.is_poisoned());

    // Recover using unwrap_or_else
    let mut guard = pool.lock().unwrap_or_else(|e| {
        let mut inner = e.into_inner();
        inner.clear();
        inner
    });

    // Guard can be used successfully
    guard.push_back("recovered".to_string());
    assert_eq!(guard.pop_front().as_deref(), Some("recovered"));
}

#[test]
fn test_zkp_schema_proof_hijacking_variants() {
    let trust = ZkCryptoTrust::new();
    let passkey_seed_1 = "my_secure_seed_1".to_string();
    let passkey_seed_2 = "my_secure_seed_2".to_string();

    let pk_hex_2 = trust.derive_public_key(passkey_seed_2.clone()).unwrap();

    let sensitive_data_1 = "Valid content".to_string();
    let ciphertext_1 = trust
        .encrypt_workspace_field(passkey_seed_1.clone(), sensitive_data_1)
        .unwrap();
    let proof_1_hex = trust
        .generate_compliance_proof(
            passkey_seed_1.clone(),
            ciphertext_1.clone(),
            "user_1".to_string(),
            "user".to_string(),
        )
        .unwrap();

    let proof_1_bytes = const_hex::decode(&proof_1_hex).unwrap();
    let schema_c_bytes = &proof_1_bytes[173..205];
    let schema_e_bytes = &proof_1_bytes[205..237];
    let schema_s_bytes = &proof_1_bytes[237..269];

    let sensitive_data_2 = "Different valid content".to_string();
    let ciphertext_2 = trust
        .encrypt_workspace_field(passkey_seed_2.clone(), sensitive_data_2)
        .unwrap();
    let proof_2_hex = trust
        .generate_compliance_proof(
            passkey_seed_2.clone(),
            ciphertext_2.clone(),
            "user_2".to_string(),
            "user".to_string(),
        )
        .unwrap();

    let proof_2_bytes = const_hex::decode(&proof_2_hex).unwrap();
    let ciphertext_2_bytes = match const_hex::decode(&ciphertext_2) {
        Ok(b) => b,
        Err(_) => ciphertext_2.as_bytes().to_vec(),
    };
    let data_hash_2 = blake3::hash(&ciphertext_2_bytes);
    let data_hash_2_hex = const_hex::encode(data_hash_2.as_bytes());

    // Variant 1: Hijack only schema_c
    let mut proof_forged_c = proof_2_bytes.clone();
    proof_forged_c[173..205].copy_from_slice(schema_c_bytes);
    let is_valid_c = trust
        .verify_compliance_proof(
            const_hex::encode(&proof_forged_c),
            "user_2".to_string(),
            "user".to_string(),
            data_hash_2_hex.clone(),
            pk_hex_2.clone(),
        )
        .unwrap();
    assert!(
        !is_valid_c,
        "ZKP Schema Proof Hijacking succeeded with transposed schema_c!"
    );

    // Variant 2: Hijack only schema_e
    let mut proof_forged_e = proof_2_bytes.clone();
    proof_forged_e[205..237].copy_from_slice(schema_e_bytes);
    let is_valid_e = trust
        .verify_compliance_proof(
            const_hex::encode(&proof_forged_e),
            "user_2".to_string(),
            "user".to_string(),
            data_hash_2_hex.clone(),
            pk_hex_2.clone(),
        )
        .unwrap();
    assert!(
        !is_valid_e,
        "ZKP Schema Proof Hijacking succeeded with transposed schema_e!"
    );

    // Variant 3: Hijack only schema_s
    let mut proof_forged_s = proof_2_bytes.clone();
    proof_forged_s[237..269].copy_from_slice(schema_s_bytes);
    let is_valid_s = trust
        .verify_compliance_proof(
            const_hex::encode(&proof_forged_s),
            "user_2".to_string(),
            "user".to_string(),
            data_hash_2_hex.clone(),
            pk_hex_2.clone(),
        )
        .unwrap();
    assert!(
        !is_valid_s,
        "ZKP Schema Proof Hijacking succeeded with transposed schema_s!"
    );
}

#[test]
fn test_memory_zeroization_empirical() {
    use zeroize::Zeroize;

    let mut secret = "my_super_secret_password_12345".to_string();
    let ptr = secret.as_ptr();
    let len = secret.len();

    // Inspect initial memory to confirm secret is present
    let initial_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(initial_bytes, b"my_super_secret_password_12345");

    // Zeroize the secret
    secret.zeroize();

    // After zeroize(), the memory must be overwritten with zeroes
    let zeroized_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let all_zeroes = zeroized_bytes.iter().all(|&b| b == 0);
    assert!(
        all_zeroes,
        "Memory was not zeroized! Bytes found: {:?}",
        zeroized_bytes
    );
}

#[test]
fn test_zeroizing_wrapper_empirical() {
    use zeroize::Zeroizing;

    let ptr;
    let len;
    {
        let secret = Zeroizing::new("another_secret_password_to_check".to_string());
        ptr = secret.as_ptr();
        len = secret.len();

        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert_eq!(bytes, b"another_secret_password_to_check");
    }

    // After Zeroizing wrapper goes out of scope, the memory must be cleared
    let bytes_after = unsafe { std::slice::from_raw_parts(ptr, len) };
    let all_zeroes = bytes_after.iter().all(|&b| b == 0);
    assert!(
        all_zeroes,
        "Zeroizing wrapper failed to clear memory! Bytes found: {:?}",
        bytes_after
    );
}

#[test]
fn test_security_patches_zeroization_empirical() {
    use crate::ZkCryptoTrust;
    use crate::infra::crypto::decrypt_workspace_key_with_password;
    use crate::infra::crypto::encrypt_workspace_key_with_password;

    let trust = ZkCryptoTrust::new();

    // 1. Verify derive_public_key zeroizes passkey_seed (Success path)
    {
        let passkey_seed = "seed_for_derive_public_key_zeroization_check".to_string();
        let ptr = passkey_seed.as_ptr();
        let len = passkey_seed.len();

        let res = trust.derive_public_key(passkey_seed);
        assert!(res.is_ok());

        let bytes_after = unsafe { std::slice::from_raw_parts(ptr, len) };
        let all_zeroes = bytes_after.iter().all(|&b| b == 0);
        assert!(
            all_zeroes,
            "derive_public_key failed to zeroize passkey_seed! Bytes: {:?}",
            bytes_after
        );
    }

    // 2. Verify generate_role_proof zeroizes passkey_seed (Success path)
    {
        let passkey_seed = "seed_for_generate_role_proof_zeroization_check".to_string();
        let ptr = passkey_seed.as_ptr();
        let len = passkey_seed.len();

        let res =
            trust.generate_role_proof(passkey_seed, "user_123".to_string(), "Admin".to_string());
        assert!(res.is_ok());

        let bytes_after = unsafe { std::slice::from_raw_parts(ptr, len) };
        let all_zeroes = bytes_after.iter().all(|&b| b == 0);
        assert!(
            all_zeroes,
            "generate_role_proof failed to zeroize passkey_seed! Bytes: {:?}",
            bytes_after
        );
    }

    // 3. Verify encrypt_workspace_key_with_password zeroizes password and workspace_key (Success path)
    {
        let password = "password_for_encrypt_workspace_key".to_string();
        let pwd_ptr = password.as_ptr();
        let pwd_len = password.len();

        let workspace_key = vec![42u8; 32];
        let key_ptr = workspace_key.as_ptr();
        let key_len = workspace_key.len();

        let res = encrypt_workspace_key_with_password(password, workspace_key);
        assert!(res.is_ok());

        let pwd_bytes_after = unsafe { std::slice::from_raw_parts(pwd_ptr, pwd_len) };
        assert!(
            pwd_bytes_after.iter().all(|&b| b == 0),
            "encrypt_workspace_key_with_password failed to zeroize password!"
        );

        let key_bytes_after = unsafe { std::slice::from_raw_parts(key_ptr, key_len) };
        assert!(
            key_bytes_after.iter().all(|&b| b == 0),
            "encrypt_workspace_key_with_password failed to zeroize workspace_key!"
        );
    }

    // 4. Verify decrypt_workspace_key_with_password zeroizes password (Early return / Error path)
    {
        let password = "password_for_decrypt_workspace_key_error".to_string();
        let pwd_ptr = password.as_ptr();
        let pwd_len = password.len();

        // Trigger early return by passing invalid envelope format
        let res = decrypt_workspace_key_with_password(password, "invalid_envelope");
        assert!(res.is_err());

        let pwd_bytes_after = unsafe { std::slice::from_raw_parts(pwd_ptr, pwd_len) };
        assert!(
            pwd_bytes_after.iter().all(|&b| b == 0),
            "decrypt_workspace_key_with_password failed to zeroize password on early return!"
        );
    }
}

#[test]
fn test_panic_zeroization_empirical() {
    use zeroize::Zeroizing;
    let secret = "secret_to_be_zeroized_on_panic".to_string();
    let ptr = secret.as_ptr();
    let len = secret.len();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _secret_zeroed = Zeroizing::new(secret);
        panic!("simulated panic");
    }));
    assert!(result.is_err());

    let bytes_after = unsafe { std::slice::from_raw_parts(ptr, len) };
    let all_zeroes = bytes_after.iter().all(|&b| b == 0);
    assert!(
        all_zeroes,
        "Zeroizing failed to clear memory on panic/unwinding!"
    );
}

#[test]
fn test_loro_snapshot_compaction_prevents_log_bloat() {
    let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

    let name_a = "note_compaction_test_a".to_string();
    let name_b = "note_compaction_test_b".to_string();

    let store_a = super::create_peer_note_store(name_a.clone()).unwrap();

    // Perform 20 sequential updates to the same note
    for i in 0..20 {
        let note = DailyNote {
            id: "note_compact_1".to_string(),
            workspace_id: "ws_abc".to_string(),
            team_id: "team_1".to_string(),
            author_id: Some("user_1".to_string()),
            subject: format!("Subject V{}", i),
            content: format!("Content iteration {}", i),
            edit_history: "[]".to_string(),
            created_at: "2026-07-26".to_string(),
            updated_at: 1000 + i,
            sync_status: "synced".to_string(),
        };
        store_a.write_notes(vec![note]).unwrap();
    }

    let snapshot_bytes = store_a.get_loro_changes().unwrap();
    assert!(!snapshot_bytes.is_empty());

    // Import snapshot into a fresh store instance and verify state integrity
    let store_b = super::create_peer_note_store(name_b.clone()).unwrap();
    store_b.apply_loro_update(snapshot_bytes).unwrap();

    let notes_b = store_b.read_all_notes().unwrap();
    assert_eq!(notes_b.len(), 1);
    assert_eq!(notes_b[0].subject, "Subject V19");
}

#[test]
fn test_compact_history_garbage_collection() {
    let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

    let name = "store_compaction_unit_test".to_string();
    let store = super::create_peer_note_store(name).unwrap();

    for i in 0..50 {
        let note = DailyNote {
            id: "note_compact_test".to_string(),
            workspace_id: "ws_compact".to_string(),
            team_id: "team_compact".to_string(),
            author_id: Some("author_1".to_string()),
            subject: format!("Compaction Iteration {}", i),
            content: format!("Content Payload Version {}", i),
            edit_history: "[]".to_string(),
            created_at: "2026-07-27".to_string(),
            updated_at: 1000 + i,
            sync_status: "synced".to_string(),
        };
        store.upsert_note(note).unwrap();
    }

    let before_notes = store.read_all_notes().unwrap();
    assert_eq!(before_notes.len(), 1);
    assert_eq!(before_notes[0].subject, "Compaction Iteration 49");

    // Compact history
    store.compact_history().unwrap();

    let after_notes = store.read_all_notes().unwrap();
    assert_eq!(after_notes.len(), 1);
    assert_eq!(after_notes[0].subject, "Compaction Iteration 49");
}

#[tokio::test]
async fn test_adaptive_transport_mesh_fallback_and_lan() {
    use super::sync::{
        clear_lan_peer_endpoints, get_adaptive_transport_status, get_registered_lan_endpoints,
        register_lan_peer_endpoint, broadcast_multi_layer_update,
    };

    clear_lan_peer_endpoints().unwrap();

    let ws_id = "ws-mesh-test".to_string();
    let peer_id = "peer-alpha".to_string();

    // 1. Initial status -> WebRtcMesh active
    let st1 = get_adaptive_transport_status(ws_id.clone(), peer_id.clone(), None).unwrap();
    assert_eq!(st1.active_transport, "WebRtcMesh");
    assert!(st1.web_rtc_connected);
    assert!(!st1.lan_socket_connected);

    // 2. Register local LAN subnet endpoint
    register_lan_peer_endpoint(peer_id.clone(), "192.168.1.150:9090".to_string()).unwrap();
    let lan_eps = get_registered_lan_endpoints().unwrap();
    assert_eq!(lan_eps.len(), 1);
    assert_eq!(lan_eps[0].lan_address, "192.168.1.150:9090");

    // 3. Simulate WebRTC firewall block -> Fallback to LocalLanSocket
    let st2 = get_adaptive_transport_status(ws_id.clone(), peer_id.clone(), Some(true)).unwrap();
    assert_eq!(st2.active_transport, "LocalLanSocket");
    assert!(!st2.web_rtc_connected);
    assert!(st2.lan_socket_connected);
    assert_eq!(st2.lan_ip_address.unwrap(), "192.168.1.150:9090");

    // 4. Test multi-layer update broadcast
    let payload_hex = const_hex::encode(b"test_crdt_delta");
    let b_res = broadcast_multi_layer_update(
        ws_id.clone(),
        peer_id.clone(),
        payload_hex.clone(),
        Some("LocalLanSocket".to_string()),
    )
    .await
    .unwrap();

    assert!(b_res.contains("LocalLanSocket"));
}

#[test]
fn test_hipaa_ferpa_dlp_payload_inspection() {
    use super::sync::{DlpPolicy, inspect_payload_dlp_bytes};

    let policy = DlpPolicy::default();

    // 1. Clean payload
    let clean_data = b"Normal collaborative task title: Buy medical supplies";
    let res_clean = inspect_payload_dlp_bytes(clean_data, &policy);
    assert!(!res_clean.is_violation);
    assert_eq!(res_clean.classification, "CLEAN");

    // 2. HIPAA PHI SSN detection
    let ssn_data = b"Patient record: Jane Doe, SSN: 123-45-6789";
    let res_ssn = inspect_payload_dlp_bytes(ssn_data, &policy);
    assert!(res_ssn.is_violation);
    assert_eq!(res_ssn.classification, "PHI_DETECTED");
    assert!(res_ssn.matched_patterns.contains(&"PHI_SSN_PATTERN".to_string()));

    // 3. HIPAA Medical Record Number & ICD-10 Diagnosis code detection
    let phi_data = b"Clinical notes: Patient MRN-9876543, ICD-10 Diagnosis: Diabetes Mellitus";
    let res_phi = inspect_payload_dlp_bytes(phi_data, &policy);
    assert!(res_phi.is_violation);
    assert_eq!(res_phi.classification, "PHI_DETECTED");
    assert!(res_phi.matched_patterns.contains(&"PHI_MEDICAL_RECORD_PATTERN".to_string()));

    // 4. FERPA Student ID & Transcript detection
    let ferpa_data = b"Student ID: SID-554433221, Student Transcript Cumulative GPA: 3.9";
    let res_ferpa = inspect_payload_dlp_bytes(ferpa_data, &policy);
    assert!(res_ferpa.is_violation);
    assert_eq!(res_ferpa.classification, "FERPA_DETECTED");
    assert!(res_ferpa.matched_patterns.contains(&"FERPA_STUDENT_RECORD_PATTERN".to_string()));

    // 5. Custom Keyword match
    let mut custom_policy = DlpPolicy::default();
    custom_policy.custom_keywords.push("CONFIDENTIAL_PROJECT_X".to_string());
    let custom_data = b"Top secret document containing CONFIDENTIAL_PROJECT_X specs";
    let res_custom = inspect_payload_dlp_bytes(custom_data, &custom_policy);
    assert!(res_custom.is_violation);
    assert!(res_custom.matched_patterns.iter().any(|p| p.contains("CONFIDENTIAL_PROJECT_X")));
}

#[tokio::test]
async fn test_compliance_mode_governance_and_dlp_audit_logging() {
    use super::sync::{ComplianceMode, DlpPolicy, P2PMeshSyncRouter};

    let router = P2PMeshSyncRouter::with_compliance(
        None,
        ComplianceMode::StrictServerOnly,
        DlpPolicy::default(),
    );

    assert_eq!(router.get_compliance_mode(), ComplianceMode::StrictServerOnly);

    // Test mode mutation
    router.set_compliance_mode(ComplianceMode::AuditedProxyRelay);
    assert_eq!(router.get_compliance_mode(), ComplianceMode::AuditedProxyRelay);

    // Test DLP inspection method on router
    let phi_payload = b"Protected Health Information: Patient MRN-112233".to_vec();
    let dlp_res = router.inspect_payload_dlp(phi_payload.clone());
    assert!(dlp_res.is_violation);
    assert_eq!(dlp_res.classification, "PHI_DETECTED");

    // Test broadcast write under AuditedProxyRelay with PHI payload -> Blocked & audit logged
    router.broadcast_write_network("peer-nurse-tablet".to_string(), phi_payload);
}

#[test]
fn test_loro_crdt_doc_payload_expansion_dlp() {
    use super::sync::{DlpPolicy, inspect_payload_dlp_bytes};
    use loro::{ExportMode, LoroDoc};

    // Construct a Loro CRDT document with binary encoded PHI note content
    let doc = LoroDoc::new();
    let map = doc.get_map("db");
    let note_map = map.insert_container("note_1", loro::LoroMap::new()).unwrap();
    note_map.insert("id", "note_1").unwrap();
    note_map.insert("content", "Clinical Evaluation: Patient Jane Doe MRN-9988771").unwrap();

    let binary_crdt_snapshot = doc.export(ExportMode::Snapshot).unwrap();

    let policy = DlpPolicy::default();
    let res = inspect_payload_dlp_bytes(&binary_crdt_snapshot, &policy);

    assert!(res.is_violation);
    assert_eq!(res.classification, "PHI_DETECTED");
    assert!(res.matched_patterns.contains(&"PHI_MEDICAL_RECORD_PATTERN".to_string()));
}

#[test]
fn test_ed25519_signed_sync_audit_events_and_tenant_scoping() {
    use super::sync::{ComplianceMode, DlpPolicy, P2PMeshSyncRouter};

    let router = P2PMeshSyncRouter::with_compliance(
        None,
        ComplianceMode::AuditedLocalP2P,
        DlpPolicy::default(),
    );

    let pubkey_hex = router.set_ephemeral_identity().unwrap();
    assert_eq!(pubkey_hex.len(), 64);

    let custom_tenant = "hospital-tenant-42".to_string();
    router.set_workspace_id(custom_tenant.clone());
    assert_eq!(router.get_workspace_id(), custom_tenant);

    let clean_crdt = b"Standard checklist update: Checked room temperature".to_vec();
    router.broadcast_write_network("peer-doctor-tablet".to_string(), clean_crdt);

    let audit_store = crate::services::audit::get_audit_store(&custom_tenant);
    let logs = audit_store.read_all_audit_logs().unwrap();
    let sync_log = logs.iter().find(|e| e.action_type.contains("P2P_SYNC_BROADCAST"));

    assert!(sync_log.is_some());
    let entry = sync_log.unwrap();
    assert_eq!(entry.workspace_id, custom_tenant);
    assert!(entry.signature.is_some());
}



