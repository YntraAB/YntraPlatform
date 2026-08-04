use super::crdt::*;
use super::crud::*;
use super::search::*;
use super::store::*;
use super::sync::*;
use crate::YntraError;
use crate::ZkCryptoTrust;
use crate::database;

#[tokio::test]
async fn test_note_zkp_compliance_verification() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, team, user, and member relations
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-notes-test', 'Notes WS', '[]', '{}')", ()).await.unwrap();

    let seed = "super_secure_seed".to_string();
    let seed_zeroed = zeroize::Zeroizing::new(seed.clone());
    let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
    key_hasher.update(seed_zeroed.as_bytes());
    let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    key_hasher.finalize_xof().fill(&mut *private_key_bytes);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
    let metadata = serde_json::json!({ "public_key": public_key_hex }).to_string();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-notes-user', 'ws-notes-test', 'notes@user.com', 'user', ?1)", crate::params![metadata]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO teams (id, workspace_id, name) VALUES ('team-notes-test', 'ws-notes-test', 'Notes Team')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO team_members (team_id, user_id, workspace_id) VALUES ('team-notes-test', 'u-notes-user', 'ws-notes-test')", ()).await.unwrap();

    // Clear notes tables
    let _ = conn
        .execute("DELETE FROM notes WHERE id LIKE 'test-note-%'", ())
        .await;
    let _ = conn
        .execute(
            "DELETE FROM note_updates WHERE note_id LIKE 'test-note-%'",
            (),
        )
        .await;

    let requester = "u-notes-user".to_string();
    let workspace = "ws-notes-test".to_string();
    let team = "team-notes-test".to_string();
    let author = "u-notes-user".to_string();
    let subject = "Security Audit Notes".to_string();

    // 1. Plaintext content -> Succeeds
    let note_plain = add_note(
        requester.clone(),
        workspace.clone(),
        team.clone(),
        author.clone(),
        subject.clone(),
        "Plaintext unencrypted note".to_string(),
    )
    .await;
    assert!(note_plain.is_ok());

    // 2. Encrypted content with VALID compliance proof -> Succeeds
    let trust = ZkCryptoTrust::new();
    let sensitive_info = "Sensitive database credential".to_string();
    let ciphertext = trust
        .encrypt_workspace_field(seed.clone(), sensitive_info)
        .unwrap();
    let valid_proof = trust
        .generate_compliance_proof(
            seed.clone(),
            ciphertext.clone(),
            author.clone(),
            "user".to_string(),
        )
        .unwrap();
    let valid_content = format!("zero_copy_enc:{}:{}", valid_proof, ciphertext);

    let note_valid_enc = add_note(
        requester.clone(),
        workspace.clone(),
        team.clone(),
        author.clone(),
        subject.clone(),
        valid_content.clone(),
    )
    .await;
    assert!(note_valid_enc.is_ok());

    // 3. Encrypted content with INVALID compliance proof -> Fails with CryptoError
    let invalid_proof_bytes = b"ZKP_PROOF_V1:mock_invalid_commitment_bytes\x00";
    let invalid_proof_hex = const_hex::encode(invalid_proof_bytes);
    let invalid_content = format!("zero_copy_enc:{}:{}", invalid_proof_hex, ciphertext);

    let note_invalid_enc = add_note(
        requester.clone(),
        workspace.clone(),
        team.clone(),
        author.clone(),
        subject.clone(),
        invalid_content.clone(),
    )
    .await;
    assert!(note_invalid_enc.is_err());
    match note_invalid_enc {
        Err(YntraError::CryptoError(msg)) => {
            assert!(msg.contains("Zero-Knowledge compliance proof is invalid"))
        }
        _ => panic!("Expected CryptoError when saving note with invalid ZK compliance proof"),
    }

    // 4. Update note with INVALID proof -> Fails
    let note_valid = note_valid_enc.unwrap();
    let update_res = update_note(
        requester.clone(),
        note_valid.id.clone(),
        "User Name".to_string(),
        "Updated Subject".to_string(),
        invalid_content.clone(),
    )
    .await;
    assert!(update_res.is_err());

    // 5. Update note with VALID proof -> Succeeds
    let update_ok = update_note(
        requester.clone(),
        note_valid.id.clone(),
        "User Name".to_string(),
        "Updated Subject".to_string(),
        valid_content.clone(),
    )
    .await;
    assert!(update_ok.is_ok());
}

#[tokio::test]
async fn test_delete_note_removes_from_cache() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, team, user, and member relations
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-notes-test-del', 'Notes WS', '[]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-notes-user-del', 'ws-notes-test-del', 'notes@user.com', 'user')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO teams (id, workspace_id, name) VALUES ('team-notes-test-del', 'ws-notes-test-del', 'Notes Team')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO team_members (team_id, user_id, workspace_id) VALUES ('team-notes-test-del', 'u-notes-user-del', 'ws-notes-test-del')", ()).await.unwrap();

    let note = add_note(
        "u-notes-user-del".to_string(),
        "ws-notes-test-del".to_string(),
        "team-notes-test-del".to_string(),
        "u-notes-user-del".to_string(),
        "Delete Test Subject".to_string(),
        "Plaintext note body".to_string(),
    )
    .await
    .unwrap();

    // Verify it was added to SQLite and ZeroCopyNoteStore
    let note_in_db: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE id = ?1",
            crate::params![&note.id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);
    assert_eq!(note_in_db, 1);

    let note_store = get_note_store("ws-notes-test-del");
    let cached_note = note_store.read_note_zero_copy(note.id.clone()).unwrap();
    assert!(cached_note.is_some());
    assert_eq!(cached_note.unwrap().subject, "Delete Test Subject");

    // Now call delete_note
    delete_note("u-notes-user-del".to_string(), note.id.clone())
        .await
        .unwrap();

    // Verify it was deleted from SQLite
    let note_in_db_post: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE id = ?1",
            crate::params![&note.id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);
    assert_eq!(note_in_db_post, 0);

    // Verify it was deleted from ZeroCopyNoteStore
    let cached_note_post = note_store.read_note_zero_copy(note.id.clone()).unwrap();
    assert!(cached_note_post.is_none());

    // Cleanup
    conn.execute(
        "DELETE FROM team_members WHERE workspace_id = 'ws-notes-test-del'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM teams WHERE workspace_id = 'ws-notes-test-del'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = 'ws-notes-test-del'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-notes-test-del'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_apply_note_loro_update_collaborative() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-notes-collab";
    let _ = conn
        .execute(
            "DELETE FROM team_members WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM teams WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM users WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM workspaces WHERE id = ?1",
            crate::params![ws_id],
        )
        .await;

    let seed = "collab_secure_seed".to_string();
    let seed_zeroed = zeroize::Zeroizing::new(seed.clone());
    let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
    key_hasher.update(seed_zeroed.as_bytes());
    let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    key_hasher.finalize_xof().fill(&mut *private_key_bytes);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
    let metadata = serde_json::json!({ "public_key": public_key_hex }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Notes Collab WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-notes-author', ?1, 'author@collab.com', 'user')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-notes-editor', ?1, 'editor@collab.com', 'user', ?2)", crate::params![ws_id, metadata]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO teams (id, workspace_id, name) VALUES ('team-collab', ?1, 'Collab Team')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO team_members (team_id, user_id, workspace_id) VALUES ('team-collab', 'u-notes-author', ?1)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO team_members (team_id, user_id, workspace_id) VALUES ('team-collab', 'u-notes-editor', ?1)", crate::params![ws_id]).await.unwrap();

    // Clear notes tables
    let _ = conn
        .execute(
            "DELETE FROM notes WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn.execute("DELETE FROM note_updates WHERE note_id IN (SELECT id FROM notes WHERE workspace_id = ?1)", crate::params![ws_id]).await;

    crate::infra::crypto::set_session_key(
        "collab-test-session-key".to_string().into_bytes(),
        ws_id.to_string(),
    );

    // 1. Author creates a note
    let note = add_note(
        "u-notes-author".to_string(),
        ws_id.to_string(),
        "team-collab".to_string(),
        "u-notes-author".to_string(),
        "Collab Note".to_string(),
        "Initial Content".to_string(),
    )
    .await
    .unwrap();

    // 2. Editor user creates a collaborative Loro update containing encrypted text with their own ZKP proof
    let original_loro_bytes = get_note_loro_state("u-notes-author".to_string(), note.id.clone())
        .await
        .unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&original_loro_bytes).unwrap();

    // Editor modifies the content and encrypts it
    let trust = ZkCryptoTrust::new();
    let seed = "collab_secure_seed".to_string();
    let new_text = "Sensitive editor data".to_string();
    let ciphertext = trust
        .encrypt_workspace_field(seed.clone(), new_text.clone())
        .unwrap();
    let editor_proof = trust
        .generate_compliance_proof(
            seed,
            ciphertext.clone(),
            "u-notes-editor".to_string(),
            "user".to_string(),
        )
        .unwrap();
    let encrypted_content = format!("zero_copy_enc:{}:{}", editor_proof, ciphertext);

    // Apply diff to editor's doc
    let editor_text = doc.get_text("content");
    let old_content = editor_text.to_string();
    apply_diff_to_loro(&editor_text, &old_content, &encrypted_content).unwrap();

    // Export the update bytes for editor's edit
    let update_bytes = doc.export(loro::ExportMode::Snapshot).unwrap();

    // 3. Apply the Loro update. This should succeed under the new collaborative validation logic!
    let apply_res =
        apply_note_loro_update("u-notes-editor".to_string(), note.id.clone(), update_bytes).await;
    assert!(
        apply_res.is_ok(),
        "apply_note_loro_update failed: {:?}",
        apply_res.err()
    );

    // Verify database projection is updated and decrypted content is accessible
    let projected_plain: String = conn
        .query_row(
            "SELECT content_plain FROM notes WHERE id = ?1",
            crate::params![&note.id],
            |r| r.get(0),
        )
        .await
        .unwrap();
    assert_eq!(projected_plain, encrypted_content);

    // Cleanup
    let _ = conn
        .execute(
            "DELETE FROM note_updates WHERE note_id = ?1",
            crate::params![&note.id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM notes WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    conn.execute(
        "DELETE FROM team_members WHERE workspace_id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM teams WHERE workspace_id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM workspaces WHERE id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_notes_fts_search() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-fts-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'FTS WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fts-user', ?1, 'fts@user.com', 'admin')", crate::params![ws_id]).await.unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO teams (id, workspace_id, name) VALUES ('team-fts', ?1, 'FTS Team')",
        crate::params![ws_id],
    )
    .await
    .unwrap();

    // Clear notes tables
    let _ = conn
        .execute(
            "DELETE FROM notes WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM notes_fts WHERE id IN (SELECT id FROM notes WHERE workspace_id = ?1)",
            crate::params![ws_id],
        )
        .await;

    // 1. Create notes
    let note1 = add_note(
        "u-fts-user".to_string(),
        ws_id.to_string(),
        "team-fts".to_string(),
        "u-fts-user".to_string(),
        "Math homework assignment".to_string(),
        "Remember to complete exercises 1 to 5".to_string(),
    )
    .await
    .unwrap();

    let note2 = add_note(
        "u-fts-user".to_string(),
        ws_id.to_string(),
        "team-fts".to_string(),
        "u-fts-user".to_string(),
        "Biology exam study".to_string(),
        "Study mitochondria and cellular respiration mechanisms".to_string(),
    )
    .await
    .unwrap();

    // 2. Perform search matches
    let res1 = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "homework".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res1.len(), 1);
    assert_eq!(res1[0].id, note1.id);

    let res2 = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "mitochondria".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res2.len(), 1);
    assert_eq!(res2[0].id, note2.id);
    // Verify snippet highlighting contains markdown bold '***'
    assert!(res2[0].content.contains("***mitochondria***"));

    // Verify query sanitization against special characters
    let res_sanitized = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "mitochondria : & OR *".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res_sanitized.len(), 1);
    assert_eq!(res_sanitized[0].id, note2.id);

    // 3. Update note and verify index changes
    let _updated = update_note(
        "u-fts-user".to_string(),
        note1.id.clone(),
        "FTS User".to_string(),
        "Math homework assignment".to_string(),
        "Remember to complete exercise 9 instead".to_string(),
    )
    .await
    .unwrap();

    // Search old content (should be gone/not match)
    let res3 = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "exercises".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res3.len(), 0);

    // Search new content
    let res4 = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "exercise 9".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res4.len(), 1);
    assert_eq!(res4[0].id, note1.id);

    // 4. Delete note and verify removal
    delete_note("u-fts-user".to_string(), note1.id.clone())
        .await
        .unwrap();
    let res5 = search_notes(
        "u-fts-user".to_string(),
        "team-fts".to_string(),
        "exercise 9".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(res5.len(), 0);

    // Cleanup
    let _ = conn
        .execute(
            "DELETE FROM note_updates WHERE note_id = ?1",
            crate::params![&note2.id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM notes WHERE workspace_id = ?1",
            crate::params![ws_id],
        )
        .await;
    let _ = conn
        .execute(
            "DELETE FROM notes_fts WHERE id = ?1",
            crate::params![&note2.id],
        )
        .await;
    conn.execute(
        "DELETE FROM teams WHERE workspace_id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM workspaces WHERE id = ?1",
        crate::params![ws_id],
    )
    .await
    .unwrap();
}
