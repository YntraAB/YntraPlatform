use super::stores::{ZeroCopyStore, ZeroCopyMessageStore, ZeroCopyAuditStore, ZeroCopyNoteStore};
use super::sync::{P2PMeshSyncRouter, in_memory_poll};
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

    // Test encryption/decryption
    let ciphertext = trust
        .encrypt_workspace_field(passkey_seed.clone(), sensitive_data.clone())
        .unwrap();
    let decrypted = trust
        .decrypt_workspace_field(passkey_seed, ciphertext.clone())
        .unwrap();
    assert_eq!(decrypted, sensitive_data);

    // Test compliance proof
    let proof = trust
        .generate_compliance_proof(
            ciphertext.clone(),
            "user_123".to_string(),
            "Admin".to_string(),
        )
        .unwrap();
    let is_valid = trust
        .verify_compliance_proof(
            proof,
            "user_123".to_string(),
            "Admin".to_string(),
            ciphertext,
        )
        .unwrap();
    assert!(is_valid);

    // Test role proof (ZK role validation without raw data/secrets)
    let role_proof = trust
        .generate_role_proof("user_123".to_string(), "Admin".to_string())
        .unwrap();
    let is_role_valid = trust.verify_proof(role_proof.clone(), "user_123".to_string(), "Admin".to_string());
    assert!(is_role_valid);

    // Mismatched role should fail validation
    let is_mismatched_role_valid = trust.verify_proof(role_proof.clone(), "user_123".to_string(), "Member".to_string());
    assert!(!is_mismatched_role_valid);

    // Mismatched user should fail validation
    let is_mismatched_user_valid = trust.verify_proof(role_proof.clone(), "user_456".to_string(), "Admin".to_string());
    assert!(!is_mismatched_user_valid);

    let invalid_role_proof = "not_a_valid_proof_hex_string_too_short".to_string();
    assert!(!trust.verify_proof(invalid_role_proof, "user_123".to_string(), "Admin".to_string()));
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
