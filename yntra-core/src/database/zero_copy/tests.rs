use super::stores::{ZeroCopyStore, ZeroCopyMessageStore, ZeroCopyAuditStore, ZeroCopyNoteStore};
use super::sync::{EdgeSyncLoop, P2PMeshSyncRouter, in_memory_poll};
use super::crypto::ZkCryptoTrust;
use crate::models::{TodoItem, MessageItem, AuditLogEntry, DailyNote};
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
    let ciphertext_bytes = const_hex::decode(&ciphertext).unwrap();
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
        .generate_role_proof(passkey_seed.clone(), "user_123".to_string(), "Admin".to_string())
        .unwrap();
    let is_role_valid = trust.verify_proof(role_proof.clone(), "user_123".to_string(), "Admin".to_string(), public_key_hex.clone());
    assert!(is_role_valid);

    // Mismatched role should fail validation
    let is_mismatched_role_valid = trust.verify_proof(role_proof.clone(), "user_123".to_string(), "Member".to_string(), public_key_hex.clone());
    assert!(!is_mismatched_role_valid);

    // Mismatched user is ignored in V3 (true zero-knowledge proof of role membership)
    let is_mismatched_user_valid = trust.verify_proof(role_proof.clone(), "user_456".to_string(), "Admin".to_string(), public_key_hex.clone());
    assert!(is_mismatched_user_valid);

    let invalid_role_proof = "not_a_valid_proof_hex_string_too_short".to_string();
    assert!(!trust.verify_proof(invalid_role_proof, "user_123".to_string(), "Admin".to_string(), public_key_hex.clone()));

    // --- Test Ring Signatures (AOS ZKP) ---
    let passkey_seed_1 = "seed_1_secret".to_string();
    let passkey_seed_2 = "seed_2_secret".to_string();
    let passkey_seed_3 = "seed_3_secret".to_string();

    let pk_1 = trust.derive_public_key(passkey_seed_1.clone()).unwrap();
    let pk_2 = trust.derive_public_key(passkey_seed_2.clone()).unwrap();
    let pk_3 = trust.derive_public_key(passkey_seed_3.clone()).unwrap();

    let ring = vec![pk_1.clone(), pk_2.clone(), pk_3.clone()];
    let data = "Zero-Knowledge anonymous write payload".to_string();
    let hex_data = const_hex::encode(data.as_bytes());
    let data_hash = blake3::hash(data.as_bytes());
    let data_hash_hex = const_hex::encode(data_hash.as_bytes());

    // Sign using seed 2 (Peer 2)
    let ring_proof = trust.generate_ring_compliance_proof(
        passkey_seed_2.clone(),
        hex_data.clone(),
        ring.clone(),
    ).unwrap();

    // Verify it against the ring
    let is_ring_valid = trust.verify_ring_compliance_proof(
        ring_proof.clone(),
        data_hash_hex.clone(),
        ring.clone(),
    ).unwrap();
    assert!(is_ring_valid);

    // Verify it using verify_compliance_proof (combining with commas)
    let ring_comb = ring.join(",");
    let is_comb_valid = trust.verify_compliance_proof(
        ring_proof.clone(),
        "".to_string(),
        "".to_string(),
        data_hash_hex.clone(),
        ring_comb.clone(),
    ).unwrap();
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
    let invalid_schema_proof = trust.generate_ring_compliance_proof(
        passkey_seed_2.clone(),
        empty_data_hex,
        ring.clone(),
    ).unwrap();
    let is_invalid_schema_verified = trust.verify_ring_compliance_proof(
        invalid_schema_proof,
        empty_data_hash_hex,
        ring.clone(),
    ).unwrap();
    assert!(!is_invalid_schema_verified);

    // --- Test Zero-Knowledge Ring Role Membership Proofs ---
    let role_ring = ring.clone();
    let user_id = "user_role_test_123".to_string();
    let role = "workspace_member".to_string();

    let ring_role_proof = trust.generate_ring_role_proof(
        passkey_seed_2.clone(),
        user_id.clone(),
        role.clone(),
        role_ring.clone(),
    ).unwrap();

    let is_role_ring_verified = trust.verify_proof(
        ring_role_proof.clone(),
        user_id.clone(),
        role.clone(),
        role_ring.join(","),
    );
    assert!(is_role_ring_verified);

    // Mismatched user ID is ignored in V3 anonymous ring role proofs
    assert!(trust.verify_proof(ring_role_proof.clone(), "different_user".to_string(), role.clone(), role_ring.join(",")));
    assert!(!trust.verify_proof(ring_role_proof.clone(), user_id.clone(), "different_role".to_string(), role_ring.join(",")));

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
    use ark_snark::{SNARK, CircuitSpecificSetupSNARK};

    #[derive(Clone)]
    struct SimpleCircuit {
        x: Option<Fr>,
        y: Option<Fr>,
        z: Option<Fr>,
    }

    impl ConstraintSynthesizer<Fr> for SimpleCircuit {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
            let x_val = cs.new_witness_variable(|| self.x.ok_or(SynthesisError::AssignmentMissing))?;
            let y_val = cs.new_witness_variable(|| self.y.ok_or(SynthesisError::AssignmentMissing))?;
            let z_val = cs.new_input_variable(|| self.z.ok_or(SynthesisError::AssignmentMissing))?;
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
    let empty_circuit = SimpleCircuit { x: None, y: None, z: None };
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
    let is_snark_valid = trust.verify_groth16_proof(
        proof_hex.clone(),
        public_inputs_hex.clone(),
        vk_hex.clone(),
    ).unwrap();
    assert!(is_snark_valid);

    let invalid_z_scalar = Fr::from(13u32);
    let mut invalid_z_bytes = Vec::new();
    invalid_z_scalar.serialize_compressed(&mut invalid_z_bytes).unwrap();
    let invalid_z_hex = const_hex::encode(&invalid_z_bytes);
    let invalid_public_inputs_hex = vec![invalid_z_hex];
    let is_invalid_snark_valid = trust.verify_groth16_proof(
        proof_hex,
        invalid_public_inputs_hex,
        vk_hex,
    ).unwrap();
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

    let router = P2PMeshSyncRouter::new();
    router.register_peer("peer_a".to_string());
    router.register_peer("peer_b".to_string());

    let note = DailyNote {
        id: "note_x".to_string(),
        workspace_id: "ws_abc".to_string(),
        team_id: "team_1".to_string(),
        author_id: Some("peer_a".to_string()),
        subject: "ZK Sync Test".to_string(),
        content: "Encrypted data here".to_string(),
        edit_history: "[]".to_string(),
        created_at: "2026-07-12".to_string(),
        updated_at: 1000,
        sync_status: "pending".to_string(),
    };

    store_a.write_notes(vec![note.clone()]).unwrap();

    let changes = store_a.get_loro_changes().unwrap();
    router.broadcast_write_network("peer_a".to_string(), changes);

    let updates = in_memory_poll("peer_b");
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
    let path_main = temp_dir.join("test_batch_main.db").to_string_lossy().to_string();
    let path_peer_a = temp_dir.join("test_batch_peer_a.db").to_string_lossy().to_string();
    let path_peer_b = temp_dir.join("test_batch_peer_b.db").to_string_lossy().to_string();

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
    store_main.apply_loro_updates_batch(vec![change1, change2]).unwrap();

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
    let path_note = temp_dir.join("test_note_loop.db").to_string_lossy().to_string();
    let path_audit = temp_dir.join("test_audit_loop.db").to_string_lossy().to_string();
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
    let router = P2PMeshSyncRouter::new();
    router.register_peer("peer_a".to_string());
    router.register_peer("peer_b".to_string());

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
        router.broadcast_write_network("peer_a".to_string(), u);
    }

    // Since the queue is hard-limited to 100 entries, but has a catch-up snapshot prepended:
    let polled = in_memory_poll("peer_b");
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
    let is_valid_wrong_data = super::sync::verify_update_signature(&pubkey_hex, timestamp, &sig_hex, &[9, 9, 9]);
    assert!(!is_valid_wrong_data);

    // 4. Verify signature fails with expired/drifted timestamp (e.g. 1 hour ago)
    let old_timestamp = timestamp - 3600 * 1000;
    let is_valid_drift = super::sync::verify_update_signature(&pubkey_hex, old_timestamp, &sig_hex, &data);
    assert!(!is_valid_drift);
}

#[tokio::test]
async fn test_p2p_sync_workspace_access_control() {
    let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
    let conn = crate::database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test', 'Sync WS', '[]', '{}')", ()).await.unwrap();

    let peer_a = "000000000000000000000000000000000000000000000000000000000000001a";
    let peer_b = "000000000000000000000000000000000000000000000000000000000000001b";
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test', 'a@yntra.io')", crate::params![peer_a]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test', 'b@yntra.io')", crate::params![peer_b]).await.unwrap();

    let peer_c = "000000000000000000000000000000000000000000000000000000000000001c";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test-diff', 'Diff WS', '[]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email) VALUES (?1, 'ws-sync-test-diff', 'c@yntra.io')", crate::params![peer_c]).await.unwrap();

    let auth_ab = super::sync::is_peer_authorized(peer_a, peer_b).await;
    assert!(auth_ab);

    let auth_ac = super::sync::is_peer_authorized(peer_a, peer_c).await;
    assert!(!auth_ac);

    conn.execute("DELETE FROM users WHERE id IN (?1, ?2, ?3)", crate::params![peer_a, peer_b, peer_c]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id IN ('ws-sync-test', 'ws-sync-test-diff')", ()).await.unwrap();
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
    let pool = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::<String>::new()));
    
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



