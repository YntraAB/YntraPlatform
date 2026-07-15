use crate::database;
use crate::observer::notify_observers;
use crate::{DailyNote, EditHistoryEntry, YntraError};

pub mod crdt;

use crdt::{apply_diff_to_loro, get_merged_loro_doc, parse_loro_state};

pub async fn verify_zkp_if_encrypted(
    conn: &database::DbConnection,
    content: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    team_id: &str,
) -> Result<(), YntraError> {
    if content.starts_with("zero_copy_enc:") {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() != 3 {
            return Err(YntraError::CryptoError(
                "Invalid encrypted payload format".to_string(),
            ));
        }
        let proof = parts[1];
        let ciphertext = parts[2];
        let trust = crate::ZkCryptoTrust::new();
        let ciphertext_bytes = const_hex::decode(ciphertext)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let data_hash = blake3::hash(&ciphertext_bytes);
        let data_hash_hex = const_hex::encode(data_hash.as_bytes());

        let public_key_hex: String;

        let is_ring = if let Ok(proof_bytes) = const_hex::decode(proof) {
            proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:")
        } else {
            false
        };

        if is_ring {
            // Query all candidate authorized users' public keys to form the ring
            let mut stmt_candidates = conn.prepare(
                "SELECT id FROM users WHERE id = ?1 \
                 UNION \
                 SELECT u.id FROM team_members tm JOIN users u ON tm.user_id = u.id WHERE tm.team_id = ?2 \
                 UNION \
                 SELECT id FROM users WHERE role = 'platform_admin' OR (role = 'admin' AND workspace_id = ?3)"
            ).await?;
            let mut rows = stmt_candidates.query(crate::params![user_id, team_id, workspace_id]).await?;
            let mut pks = Vec::new();
            while let Some(row) = rows.next().await? {
                let uid: String = row.get(0)?;
                let metadata_str: Option<String> = conn
                    .query_row(
                        "SELECT metadata FROM users WHERE id = ?1",
                        crate::params![&uid],
                        |r| r.get(0),
                    )
                    .await
                    .ok()
                    .flatten();
                if let Some(ref meta) = metadata_str {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                        let pk = val.get("public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                val.get("siths_public_key")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            });
                        if let Some(p) = pk {
                            pks.push(p);
                        }
                    }
                }
            }
            pks.sort();
            pks.dedup();
            public_key_hex = pks.join(",");
        } else {
            // Standard single key verification
            let metadata_str: Option<String> = conn
                .query_row(
                    "SELECT metadata FROM users WHERE id = ?1",
                    crate::params![user_id],
                    |r| r.get(0),
                )
                .await
                .ok()
                .flatten();

            public_key_hex = if let Some(ref meta) = metadata_str {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                    val.get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            val.get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            } else {
                None
            }
            .unwrap_or_default();
        }

        if !trust
            .verify_compliance_proof(
                proof.to_string(),
                user_id.to_string(),
                role.to_string(),
                data_hash_hex,
                public_key_hex,
            )
            .unwrap_or(false)
        {
            return Err(YntraError::CryptoError(
                "Validation failed: Zero-Knowledge compliance proof is invalid".to_string(),
            ));
        }
    }
    Ok(())
}

use crate::ZeroCopyNoteStore;
use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};

static NOTE_STORES: LazyLock<Mutex<HashMap<String, ZeroCopyNoteStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_arch = "wasm32")]
fn get_note_store_path(workspace_id: &str) -> String {
    format!("yntra_zero_copy_notes_{}.db", workspace_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_note_store_path(workspace_id: &str) -> String {
    if cfg!(test) {
        std::env::temp_dir()
            .join(format!("yntra_zero_copy_notes_{}.db", workspace_id))
            .to_string_lossy()
            .to_string()
    } else {
        crate::database::native::get_database_path(&format!("yntra_zero_copy_notes_{}.db", workspace_id))
    }
}

pub fn get_note_store(workspace_id: &str) -> ZeroCopyNoteStore {
    let mut stores = NOTE_STORES.lock().unwrap();
    stores
        .entry(workspace_id.to_string())
        .or_insert_with(|| {
            let path = get_note_store_path(workspace_id);
            ZeroCopyNoteStore::new(path).expect("Failed to initialize ZeroCopyNoteStore")
        })
        .clone()
}

pub async fn load_notes_from_opfs_internal(workspace_id: &str) -> Result<(), YntraError> {
    let store = get_note_store(workspace_id);
    store.load_from_opfs().await?;
    Ok(())
}

#[uniffi::export]
pub async fn load_notes_from_opfs(workspace_id: String) -> Result<(), YntraError> {
    load_notes_from_opfs_internal(&workspace_id).await
}

#[uniffi::export]
pub async fn get_notes(
    requester_user_id: String,
    team_id: Option<String>,
) -> Result<Vec<DailyNote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    let (query, params) = if auth.role == "platform_admin" {
        match team_id {
            Some(tid) => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status, content_plain, \
                 (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = notes.id) FROM notes WHERE team_id = ?1 ORDER BY created_at DESC".to_string(),
                crate::params![tid],
            ),
            None => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status, content_plain, \
                 (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = notes.id) FROM notes ORDER BY created_at DESC".to_string(),
                crate::params![],
            ),
        }
    } else if auth.role == "admin" {
        match team_id {
            Some(tid) => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status, content_plain, \
                 (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = notes.id) FROM notes WHERE team_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC".to_string(),
                crate::params![tid, ws_id],
            ),
            None => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status, content_plain, \
                 (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = notes.id) FROM notes WHERE workspace_id = ?1 ORDER BY created_at DESC".to_string(),
                crate::params![ws_id],
            ),
        }
    } else {
        match team_id {
            Some(tid) => {
                let is_member: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                    crate::params![&tid, &requester_user_id],
                    |r| r.get(0)
                ).await.unwrap_or(0);
                if is_member == 0 {
                    return Err(YntraError::AuthError("Access denied: you are not a member of this team".to_string()));
                }
                (
                    "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status, content_plain, \
                     (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = notes.id) FROM notes WHERE team_id = ?1 ORDER BY created_at DESC".to_string(),
                    crate::params![tid],
                )
            }
            None => (
                "SELECT n.id, n.workspace_id, n.team_id, n.author_id, n.subject, n.content, n.edit_history, n.created_at, n.updated_at, n.sync_status, n.content_plain, \
                 (SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_updates.note_id = n.id) \
                 FROM notes n \
                 JOIN team_members tm ON n.team_id = tm.team_id \
                 WHERE tm.user_id = ?1 \
                 ORDER BY n.created_at DESC".to_string(),
                 crate::params![requester_user_id],
            ),
        }
    };

    let mut stmt = conn.prepare(&query).await?;
    let mut rows = stmt.query(params).await?;

    let mut raw_notes = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let workspace_id: String = row.get(1)?;
        let team_id: String = row.get(2)?;
        let author_id: Option<String> = row.get(3)?;
        let subject: String = row.get(4)?;
        let base_content: String = row.get(5)?;
        let edit_history: String = row.get(6)?;
        let created_at: String = row.get(7)?;
        let updated_at: i64 = row.get(8)?;
        let sync_status: String = row.get(9)?;
        let content_plain: Option<String> = row.get(10)?;
        let max_seq: i64 = row.get(11)?;

        raw_notes.push((
            id,
            workspace_id,
            team_id,
            author_id,
            subject,
            base_content,
            edit_history,
            created_at,
            updated_at,
            sync_status,
            content_plain,
            max_seq,
        ));
    }

    if raw_notes.is_empty() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let mut repairs = Vec::new();
    let mut stmt_updates = conn.prepare(
        "SELECT seq, update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
    ).await?;

    for (
        id,
        workspace_id,
        team_id,
        author_id,
        subject,
        base_content,
        edit_history,
        created_at,
        updated_at,
        sync_status,
        content_plain,
        max_seq,
    ) in raw_notes
    {
        let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);

        let has_unmerged = max_seq > last_merged_seq;

        let content = if let Some(plain) = content_plain.filter(|_| !has_unmerged) {
            plain
        } else {
            let doc = loro::LoroDoc::new();
            if base_content.starts_with("loro:") {
                if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
                    doc.import(&bytes)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
            } else {
                doc.get_text("content")
                    .insert(0, hex_or_plain)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }

            let mut final_max_seq = last_merged_seq;
            if has_unmerged {
                let mut rows_updates = stmt_updates
                    .query(crate::params![&id, last_merged_seq])
                    .await?;
                while let Some(row_up) = rows_updates.next().await? {
                    let seq: i64 = row_up.get(0)?;
                    let update_data_hex: String = row_up.get(1)?;
                    if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
                        doc.import(&bytes)
                            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    }
                    if seq > final_max_seq {
                        final_max_seq = seq;
                    }
                }
            }

            let plain = doc.get_text("content").to_string();

            // Cache merged state to avoid future LoroDoc execution on next read
            let snapshot_bytes = doc
                .export(loro::ExportMode::Snapshot)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            let loro_content = format!(
                "loro:{}:{}",
                final_max_seq,
                crate::infra::crypto::hex_encode(&snapshot_bytes)
            );

            repairs.push((loro_content, plain.clone(), id.clone()));

            plain
        };

        list.push(DailyNote {
            id,
            workspace_id,
            team_id,
            author_id,
            subject,
            content,
            edit_history,
            created_at,
            updated_at,
            sync_status,
        });
    }

    if !repairs.is_empty() {
        let _ = conn.begin_transaction().await;
        for (loro_content, plain, id) in repairs {
            let _ = conn.execute(
                "/* read_repair */ UPDATE notes SET content = ?1, content_plain = ?2 WHERE id = ?3",
                crate::params![loro_content, plain, id],
            ).await;
        }
        let _ = conn.commit().await;
    }

    Ok(list)
}

#[uniffi::export]
pub async fn get_note_by_id(
    requester_user_id: String,
    workspace_id: String,
    note_id: String,
) -> Result<Option<DailyNote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_note_store(&workspace_id);
    store.read_note_zero_copy(note_id)
}

#[uniffi::export]
pub async fn add_note(
    requester_user_id: String,
    workspace_id: String,
    team_id: String,
    author_id: String,
    subject: String,
    content: String,
) -> Result<DailyNote, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    verify_zkp_if_encrypted(&conn, &content, &auth.user_id, &auth.role, &workspace_id, &team_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }
    if auth.role != "platform_admin" && requester_user_id != author_id {
        return Err(YntraError::AuthError(
            "Access denied: cannot create note as another user".to_string(),
        ));
    }

    if auth.role != "admin" && auth.role != "platform_admin" {
        let is_member: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                crate::params![&team_id, &author_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
        if is_member == 0 {
            return Err(YntraError::AuthError(
                "Access denied: you are not a member of this team".to_string(),
            ));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();

    // Create Loro doc for the content
    let doc = loro::LoroDoc::new();
    let text = doc.get_text("content");
    text.insert(0, &content)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let loro_bytes = doc
        .export(loro::ExportMode::Snapshot)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let loro_content = format!("loro:0:{}", crate::infra::crypto::hex_encode(&loro_bytes));

    let mut item = DailyNote {
        id: id.clone(),
        workspace_id,
        team_id,
        author_id: Some(author_id),
        subject,
        content: content.clone(), // Return plaintext content for UI consistency
        edit_history: "[]".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    // Persist to ZeroCopyNoteStore (source of truth)
    let note_store = get_note_store(&item.workspace_id);
    let mut all_notes = note_store.read_all_notes().unwrap_or_default();
    all_notes.push(item.clone());
    note_store.write_notes(all_notes)?;

    conn.begin_transaction().await?;
    let res = async {
        conn.execute(
            "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, content_plain, edit_history, created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '[]', ?8, ?9, 'pending')",
             crate::params![
                 &item.id,
                 &item.workspace_id,
                 &item.team_id,
                 &item.author_id,
                 &item.subject,
                 &loro_content, // Use the serialized Loro snapshot for database storage
                 &content, // cached plain text
                 &item.created_at,
                 &item.updated_at
             ],
        ).await?;

        // Seed the event-sourced log with the initial snapshot update
        let update_id = uuid::Uuid::new_v4().to_string();
        let author_id_str = item.author_id.clone().unwrap_or_default();
        let update_data_hex = crate::infra::crypto::hex_encode(&loro_bytes);
        conn.execute(
            "INSERT INTO note_updates (id, note_id, client_id, seq, update_data, created_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            crate::params![&update_id, &item.id, &author_id_str, &update_data_hex, &now_ms],
        ).await?;

        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            // Return the plaintext representation in memory
            item.content = content;
            Ok(item)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn update_note(
    requester_user_id: String,
    note_id: String,
    edited_by_name: String,
    subject: String,
    content: String,
) -> Result<DailyNote, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.begin_transaction().await?;

    let res = async {
        // 1. Fetch the existing note
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE id = ?1"
        ).await?;
        let mut rows = stmt.query(crate::params![&note_id]).await?;
        let old_note = if let Some(row) = rows.next().await? {
            DailyNote {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                team_id: row.get(2)?,
                author_id: row.get(3)?,
                subject: row.get(4)?,
                content: row.get(5)?,
                edit_history: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            }
        } else {
            return Err(YntraError::NotFoundError(format!("Note not found: {}", note_id)));
        };

        let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
        let (ws_id, t_id): (String, String) = conn
            .query_row(
                "SELECT workspace_id, team_id FROM notes WHERE id = ?1",
                crate::params![note_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .await
            .unwrap_or_else(|_| (auth.workspace_id.clone(), String::new()));
        verify_zkp_if_encrypted(&conn, &content, &auth.user_id, &auth.role, &ws_id, &t_id).await?;
        if auth.role != "platform_admin" && auth.workspace_id != old_note.workspace_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }

        if auth.role != "admin" && auth.role != "platform_admin" && old_note.author_id.as_deref() != Some(&requester_user_id) {
            let is_member: i64 = conn.query_row(
                "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                crate::params![&old_note.team_id, &requester_user_id],
                |r| r.get(0)
            ).await.unwrap_or(0);
            if is_member == 0 {
                return Err(YntraError::AuthError("Access denied: you do not have permission to edit this note".to_string()));
            }
        }

        // 2. Build fully merged Loro document state
        let doc = get_merged_loro_doc(&conn, &note_id).await?;
        let old_content_plain = doc.get_text("content").to_string();
        let vv = doc.oplog_vv();

        // Compute history entry using plain content
        let mut history: Vec<EditHistoryEntry> = serde_json::from_str(&old_note.edit_history)
            .unwrap_or_default();
        
        let mut changed = false;
        let mut entry = EditHistoryEntry {
            edited_by: edited_by_name,
            edited_at: crate::infra::time::get_current_time_str_hm(),
            old_subject: None,
            new_subject: None,
            old_content: None,
            new_content: None,
        };

        if old_note.subject != subject {
            entry.old_subject = Some(old_note.subject.clone());
            entry.new_subject = Some(subject.clone());
            changed = true;
        }
        if old_content_plain != content {
            entry.old_content = Some(old_content_plain.clone());
            entry.new_content = Some(content.clone());
            changed = true;
        }
        if changed {
            history.insert(0, entry);
        }

        let edit_history_str = serde_json::to_string(&history)
            .map_err(|e| YntraError::DbError(e.to_string()))?;

        let text = doc.get_text("content");
        apply_diff_to_loro(&text, &old_content_plain, &content)?;
        
        let snapshot_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let incremental_bytes = doc.export(loro::ExportMode::updates(&vv)).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let update_data_hex = crate::infra::crypto::hex_encode(&incremental_bytes);

        // Append update to the event-sourced updates table
        let update_id = uuid::Uuid::new_v4().to_string();
        let next_seq: i64 = conn.query_row(
            "SELECT IFNULL(MAX(seq), 0) + 1 FROM note_updates WHERE note_id = ?1",
            crate::params![&note_id],
            |r| r.get(0)
        ).await.unwrap_or(1);

        conn.execute(
            "INSERT INTO note_updates (id, note_id, client_id, seq, update_data, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            crate::params![&update_id, &note_id, &requester_user_id, &next_seq, &update_data_hex, &now_ms],
        ).await?;

        // 3. Update database cache projection
        let loro_content = format!("loro:{}:{}", next_seq, crate::infra::crypto::hex_encode(&snapshot_bytes));
        conn.execute(
            "UPDATE notes SET subject = ?1, content = ?2, content_plain = ?3, edit_history = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
            crate::params![&subject, &loro_content, &content, &edit_history_str, &now_ms, &note_id],
        ).await?;

        let updated_note = DailyNote {
            id: old_note.id,
            workspace_id: old_note.workspace_id,
            team_id: old_note.team_id,
            author_id: old_note.author_id,
            subject,
            content,
            edit_history: edit_history_str,
            created_at: old_note.created_at,
            updated_at: now_ms,
            sync_status: "pending".to_string(),
        };

        Ok(updated_note)
    }.await;

    match res {
        Ok(note) => {
            // Update in ZeroCopyNoteStore (source of truth)
            let note_store = get_note_store(&note.workspace_id);
            let mut all_notes = note_store.read_all_notes().unwrap_or_default();
            let mut found = false;
            for n in all_notes.iter_mut() {
                if n.id == note.id {
                    n.subject = note.subject.clone();
                    n.content = note.content.clone();
                    n.edit_history = note.edit_history.clone();
                    n.updated_at = note.updated_at;
                    n.sync_status = note.sync_status.clone();
                    found = true;
                    break;
                }
            }
            if !found {
                all_notes.push(note.clone());
            }
            let _ = note_store.write_notes(all_notes);

            conn.commit().await?;
            notify_observers();
            Ok(note)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn delete_note(requester_user_id: String, note_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    let note_row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT workspace_id, author_id FROM notes WHERE id = ?1",
            crate::params![&note_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .ok();

    let note_ws_id = if let Some((note_ws_id, author_id)) = note_row {
        let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

        if auth.role != "platform_admin" && note_ws_id != auth.workspace_id {
            return Err(YntraError::AuthError(
                "Access denied: note is in a different workspace".to_string(),
            ));
        }

        let is_author = author_id.as_deref() == Some(&requester_user_id);
        if !is_author && !auth.is_admin {
            return Err(YntraError::AuthError(
                "Access denied: only the author or an administrator can delete this note"
                    .to_string(),
            ));
        }
        note_ws_id
    } else {
        return Err(YntraError::NotFoundError(format!(
            "Note not found: {}",
            note_id
        )));
    };

    conn.begin_transaction().await?;
    let res = async {
        conn.execute(
            "DELETE FROM note_updates WHERE note_id = ?1",
            crate::params![&note_id],
        )
        .await?;
        conn.execute("DELETE FROM notes WHERE id = ?1", crate::params![&note_id])
            .await?;
        Ok(())
    }
    .await;

    match res {
        Ok(_) => {
            conn.commit().await?;

            // Delete from ZeroCopyNoteStore
            let note_store = get_note_store(&note_ws_id);
            let mut all_notes = note_store.read_all_notes().unwrap_or_default();
            all_notes.retain(|n| n.id != note_id);
            let _ = note_store.write_notes(all_notes);

            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub fn merge_loro_notes(state1: String, state2: String) -> Result<String, YntraError> {
    let doc1 = loro::LoroDoc::new();
    let (seq1, hex_or_plain1) = parse_loro_state(&state1);
    if state1.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain1) {
            doc1.import(&bytes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc1.get_text("content")
            .insert(0, hex_or_plain1)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let doc2 = loro::LoroDoc::new();
    let (seq2, hex_or_plain2) = parse_loro_state(&state2);
    if state2.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain2) {
            doc2.import(&bytes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc2.get_text("content")
            .insert(0, hex_or_plain2)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let bytes2 = doc2
        .export(loro::ExportMode::Snapshot)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    doc1.import(&bytes2)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    let merged_bytes = doc1
        .export(loro::ExportMode::Snapshot)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let max_seq = seq1.max(seq2);
    if max_seq >= 0 {
        Ok(format!(
            "loro:{}:{}",
            max_seq,
            crate::infra::crypto::hex_encode(&merged_bytes)
        ))
    } else {
        Ok(format!(
            "loro:{}",
            crate::infra::crypto::hex_encode(&merged_bytes)
        ))
    }
}

#[uniffi::export]
pub async fn get_note_loro_state(
    requester_user_id: String,
    note_id: String,
) -> Result<Vec<u8>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (note_ws_id, team_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, team_id FROM notes WHERE id = ?1",
            crate::params![&note_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Note not found: {}", note_id)))?;

    if auth.role != "platform_admin" && note_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: note belongs to a different workspace".to_string(),
        ));
    }

    if auth.role != "admin" && auth.role != "platform_admin" {
        let is_member: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                crate::params![&team_id, &requester_user_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
        if is_member == 0 {
            return Err(YntraError::AuthError(
                "Access denied: you do not have permission to access this note".to_string(),
            ));
        }
    }

    let doc = get_merged_loro_doc(&conn, &note_id).await?;
    let bytes = doc
        .export(loro::ExportMode::Snapshot)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes)
}

#[uniffi::export]
pub async fn apply_note_loro_update(
    requester_user_id: String,
    note_id: String,
    update_bytes: Vec<u8>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (note_ws_id, team_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, team_id FROM notes WHERE id = ?1",
            crate::params![&note_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Note not found: {}", note_id)))?;

    if auth.role != "platform_admin" && note_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: note belongs to a different workspace".to_string(),
        ));
    }

    if auth.role != "admin" && auth.role != "platform_admin" {
        let is_member: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                crate::params![&team_id, &requester_user_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
        if is_member == 0 {
            return Err(YntraError::AuthError(
                "Access denied: you do not have permission to edit this note".to_string(),
            ));
        }
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    conn.begin_transaction().await?;

    let res = async {
        // 1. Build merged state using cached snapshot + remote update
        let doc = get_merged_loro_doc(&conn, &note_id).await?;
        doc.import(&update_bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        
        let loro_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;

        let (author_id, team_id, workspace_id): (String, String, String) = conn.query_row(
            "SELECT author_id, team_id, workspace_id FROM notes WHERE id = ?1",
            crate::params![&note_id],
            |r| {
                let author_id: Option<String> = r.get(0)?;
                let team_id: String = r.get(1)?;
                let workspace_id: String = r.get(2)?;
                Ok((author_id.unwrap_or_default(), team_id, workspace_id))
            }
        ).await.map_err(|_| YntraError::NotFoundError(format!("Note not found: {}", note_id)))?;

        // Ensure the dummy 'remote_<workspace_id>' user exists to satisfy the FOREIGN KEY constraint on note_updates
        let remote_client_id = format!("remote_{}", workspace_id);
        let remote_email = format!("remote-{}@yntra.io", workspace_id);
        let user_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE id = ?1",
            crate::params![&remote_client_id],
            |r| r.get(0)
        ).await.unwrap_or(0);
        if user_exists == 0 {
            conn.execute(
                "INSERT INTO users (id, workspace_id, email, role, preferences, updated_at, sync_status)
                 VALUES (?1, ?2, ?3, 'remote', '{}', ?4, 'synced')",
                crate::params![&remote_client_id, &workspace_id, &remote_email, &now_ms],
            ).await?;
        }

        // 2. Append update to the event-sourced updates table
        let update_id = uuid::Uuid::new_v4().to_string();
        let next_seq: i64 = conn.query_row(
            "SELECT IFNULL(MAX(seq), 0) + 1 FROM note_updates WHERE note_id = ?1",
            crate::params![&note_id],
            |r| r.get(0)
        ).await.unwrap_or(1);

        let update_data_hex = crate::infra::crypto::hex_encode(&update_bytes);
        conn.execute(
            "INSERT INTO note_updates (id, note_id, client_id, seq, update_data, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            crate::params![&update_id, &note_id, &remote_client_id, &next_seq, &update_data_hex, &now_ms],
        ).await?;

        // 3. Update the database projection cache
        let plain_text = doc.get_text("content").to_string();

        if plain_text.starts_with("zero_copy_enc:") {
            let parts: Vec<&str> = plain_text.split(':').collect();
            if parts.len() != 3 {
                return Err(YntraError::CryptoError("Invalid encrypted payload format".to_string()));
            }
            let proof = parts[1];
            let ciphertext = parts[2];
            let trust = crate::ZkCryptoTrust::new();

            // Fetch candidate users authorized to edit/update this note
            let mut stmt_candidates = conn.prepare(
                "SELECT id, role FROM users WHERE id = ?1 \
                 UNION \
                 SELECT u.id, u.role FROM team_members tm JOIN users u ON tm.user_id = u.id WHERE tm.team_id = ?2 \
                 UNION \
                 SELECT id, role FROM users WHERE role = 'platform_admin' OR (role = 'admin' AND workspace_id = ?3)"
            ).await?;
            let mut rows = stmt_candidates.query(crate::params![&author_id, &team_id, &workspace_id]).await?;
            let mut candidates = Vec::new();
            while let Some(row) = rows.next().await? {
                let uid: String = row.get(0)?;
                let urole: String = row.get(1)?;
                candidates.push((uid, urole));
            }

            let mut validated = false;
            let ciphertext_bytes = const_hex::decode(ciphertext)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let data_hash = blake3::hash(&ciphertext_bytes);
            let data_hash_hex = const_hex::encode(data_hash.as_bytes());

            let is_ring = if let Ok(proof_bytes) = const_hex::decode(proof) {
                proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:")
            } else {
                false
            };

            if is_ring {
                let mut pks = Vec::new();
                for (uid, _) in &candidates {
                    let metadata_str: Option<String> = conn
                        .query_row(
                            "SELECT metadata FROM users WHERE id = ?1",
                            crate::params![uid],
                            |r| r.get(0),
                        )
                        .await
                        .ok()
                        .flatten();
                    if let Some(ref meta) = metadata_str {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                            let pk = val.get("public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    val.get("siths_public_key")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                });
                            if let Some(p) = pk {
                                pks.push(p);
                            }
                        }
                    }
                }
                pks.sort();
                pks.dedup();
                let ring_pks = pks.join(",");
                if trust.verify_compliance_proof(
                    proof.to_string(),
                    String::new(),
                    String::new(),
                    data_hash_hex.clone(),
                    ring_pks,
                ).unwrap_or(false) {
                    validated = true;
                }
            } else {
                for (uid, urole) in candidates {
                    let metadata_str: Option<String> = conn
                        .query_row(
                            "SELECT metadata FROM users WHERE id = ?1",
                            crate::params![&uid],
                            |r| r.get(0),
                        )
                        .await
                        .ok()
                        .flatten();

                    let public_key_hex = if let Some(ref meta) = metadata_str {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                            val.get("public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    val.get("siths_public_key")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                    .unwrap_or_default();

                    if trust.verify_compliance_proof(
                        proof.to_string(),
                        uid,
                        urole,
                        data_hash_hex.clone(),
                        public_key_hex,
                    ).unwrap_or(false) {
                        validated = true;
                        break;
                    }
                }
            }

            if !validated {
                return Err(YntraError::CryptoError("Validation failed: Zero-Knowledge compliance proof is invalid for all authorized updaters".to_string()));
            }
        }
        let loro_content = format!("loro:{}:{}", next_seq, crate::infra::crypto::hex_encode(&loro_bytes));
        conn.execute(
            "UPDATE notes SET content = ?1, content_plain = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
            crate::params![&loro_content, &plain_text, &now_ms, &note_id],
        ).await?;

        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;

            // Reconstruct the updated DailyNote and write to ZeroCopyNoteStore (source of truth)
            if let Ok(mut stmt) = conn.prepare("SELECT id, workspace_id, team_id, author_id, subject, content_plain, edit_history, created_at, updated_at FROM notes WHERE id = ?1").await {
                if let Ok(mut rows) = stmt.query(crate::params![&note_id]).await {
                    if let Ok(Some(row)) = rows.next().await {
                        if let Ok(note) = (|| -> Result<DailyNote, YntraError> {
                            Ok(DailyNote {
                                id: row.get(0)?,
                                workspace_id: row.get(1)?,
                                team_id: row.get(2)?,
                                author_id: row.get(3)?,
                                subject: row.get(4)?,
                                content: row.get(5)?,
                                edit_history: row.get(6)?,
                                created_at: row.get(7)?,
                                updated_at: row.get(8)?,
                                sync_status: "pending".to_string(),
                            })
                        })() {
                            let note_store = get_note_store(&note.workspace_id);
                            let mut all_notes = note_store.read_all_notes().unwrap_or_default();
                            let mut found = false;
                            for n in all_notes.iter_mut() {
                                if n.id == note.id {
                                    *n = note.clone();
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                all_notes.push(note);
                            }
                            let _ = note_store.write_notes(all_notes);
                        }
                    }
                }
            }

            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

pub async fn merge_unmerged_notes() -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    // Find candidate notes that have at least one update in note_updates, fetching max_seq using a JOIN
    let mut stmt = conn.prepare(
        "SELECT n.id, n.content, MAX(u.seq) \
         FROM notes n \
         JOIN note_updates u ON u.note_id = n.id \
         GROUP BY n.id"
    ).await?;

    let mut rows = stmt.query(()).await?;
    let mut candidates = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        let max_seq: i64 = row.get(2)?;
        candidates.push((id, content, max_seq));
    }

    let mut repairs = Vec::new();
    let mut stmt_updates = conn.prepare(
        "SELECT seq, update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
    ).await?;

    for (id, base_content, max_seq) in candidates {
        let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);

        if max_seq > last_merged_seq {
            let doc = loro::LoroDoc::new();
            if base_content.starts_with("loro:") {
                if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
                    doc.import(&bytes)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
            } else {
                doc.get_text("content")
                    .insert(0, hex_or_plain)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }

            let mut rows_updates = stmt_updates
                .query(crate::params![&id, last_merged_seq])
                .await?;
            let mut final_max_seq = last_merged_seq;
            while let Some(row_up) = rows_updates.next().await? {
                let seq: i64 = row_up.get(0)?;
                let update_data_hex: String = row_up.get(1)?;
                if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
                    doc.import(&bytes)
                        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if seq > final_max_seq {
                    final_max_seq = seq;
                }
            }

            let plain = doc.get_text("content").to_string();
            let snapshot_bytes = doc
                .export(loro::ExportMode::Snapshot)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            let loro_content = format!(
                "loro:{}:{}",
                final_max_seq,
                crate::infra::crypto::hex_encode(&snapshot_bytes)
            );

            repairs.push((loro_content, plain, id));
        }
    }

    if !repairs.is_empty() {
        conn.begin_transaction().await?;
        for (loro_content, plain, id) in repairs {
            let _ = conn.execute(
                "/* background_repair */ UPDATE notes SET content = ?1, content_plain = ?2 WHERE id = ?3",
                crate::params![loro_content, plain, id],
            ).await;
        }
        conn.commit().await?;
    }

    Ok(())
}

#[uniffi::export]
pub async fn get_notes_rkyv(
    requester_user_id: String,
    team_id: Option<String>,
) -> Result<Vec<u8>, YntraError> {
    let notes = get_notes(requester_user_id, team_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&notes)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .generate_compliance_proof(seed.clone(), ciphertext.clone(), author.clone(), "user".to_string())
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
        let _ = conn.execute("DELETE FROM team_members WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM teams WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await;

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
        let _ = conn.execute("DELETE FROM notes WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM note_updates WHERE note_id IN (SELECT id FROM notes WHERE workspace_id = ?1)", crate::params![ws_id]).await;

        crate::infra::crypto::set_session_key("collab-test-session-key".to_string().into_bytes());

        // 1. Author creates a note
        let note = add_note(
            "u-notes-author".to_string(),
            ws_id.to_string(),
            "team-collab".to_string(),
            "u-notes-author".to_string(),
            "Collab Note".to_string(),
            "Initial Content".to_string(),
        ).await.unwrap();

        // 2. Editor user creates a collaborative Loro update containing encrypted text with their own ZKP proof
        let original_loro_bytes = get_note_loro_state("u-notes-author".to_string(), note.id.clone()).await.unwrap();
        let doc = loro::LoroDoc::new();
        doc.import(&original_loro_bytes).unwrap();

        // Editor modifies the content and encrypts it
        let trust = ZkCryptoTrust::new();
        let seed = "collab_secure_seed".to_string();
        let new_text = "Sensitive editor data".to_string();
        let ciphertext = trust.encrypt_workspace_field(seed.clone(), new_text.clone()).unwrap();
        let editor_proof = trust.generate_compliance_proof(seed, ciphertext.clone(), "u-notes-editor".to_string(), "user".to_string()).unwrap();
        let encrypted_content = format!("zero_copy_enc:{}:{}", editor_proof, ciphertext);

        // Apply diff to editor's doc
        let editor_text = doc.get_text("content");
        let old_content = editor_text.to_string();
        apply_diff_to_loro(&editor_text, &old_content, &encrypted_content).unwrap();

        // Export the update bytes for editor's edit
        let update_bytes = doc.export(loro::ExportMode::Snapshot).unwrap();

        // 3. Apply the Loro update. This should succeed under the new collaborative validation logic!
        let apply_res = apply_note_loro_update("u-notes-editor".to_string(), note.id.clone(), update_bytes).await;
        assert!(apply_res.is_ok(), "apply_note_loro_update failed: {:?}", apply_res.err());

        // Verify database projection is updated and decrypted content is accessible
        let projected_plain: String = conn.query_row(
            "SELECT content_plain FROM notes WHERE id = ?1",
            crate::params![&note.id],
            |r| r.get(0),
        ).await.unwrap();
        assert_eq!(projected_plain, encrypted_content);

        // Cleanup
        let _ = conn.execute("DELETE FROM note_updates WHERE note_id = ?1", crate::params![&note.id]).await;
        let _ = conn.execute("DELETE FROM notes WHERE workspace_id = ?1", crate::params![ws_id]).await;
        conn.execute("DELETE FROM team_members WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM teams WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }
}
