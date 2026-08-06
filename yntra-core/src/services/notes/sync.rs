use crate::DailyNote;
use crate::database;
use crate::infra::errors::YntraError;
use crate::observer::notify_observers;
use crate::services::notes::crdt::{get_merged_loro_doc, parse_loro_state};
use crate::services::notes::store::get_note_store;

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
        let ws_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0)
        ).await.unwrap_or(0);
        if ws_exists == 0 {
            conn.execute(
                "INSERT INTO workspaces (id, name, created_at, updated_at, sync_status)
                 VALUES (?1, 'Remote Workspace', ?2, ?2, 'synced')",
                crate::params![&workspace_id, &now_ms],
            ).await?;
        }

        let remote_email = format!("remote-{}@yntra.se", workspace_id);
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

        if plain_text.starts_with("zero_copy_escrow_v1:") || plain_text.starts_with("zero_copy_v1:") || plain_text.starts_with("zero_copy_enc:") {
            let (proof, ciphertext) = if plain_text.starts_with("zero_copy_enc:") {
                let rest = &plain_text["zero_copy_enc:".len()..];
                match rest.split_once(':') {
                    Some((p, c)) => (p, c),
                    None => return Err(YntraError::CryptoError("Invalid encrypted payload format".to_string())),
                }
            } else {
                let parts: Vec<&str> = plain_text.split(':').collect();
                if parts.len() == 5 {
                    (parts[1], parts[4])
                } else if parts.len() == 3 {
                    (parts[1], parts[2])
                } else {
                    return Err(YntraError::CryptoError("Invalid encrypted payload format".to_string()));
                }
            };
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
            let ciphertext_bytes = match const_hex::decode(ciphertext) {
                Ok(b) => b,
                Err(_) => ciphertext.as_bytes().to_vec(),
            };
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
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.content, MAX(u.seq) \
         FROM notes n \
         JOIN note_updates u ON u.note_id = n.id \
         GROUP BY n.id",
        )
        .await?;

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
