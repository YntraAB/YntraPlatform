use crate::database;
use crate::infra::errors::YntraError;
use crate::observer::notify_observers;
use crate::services::notes::crdt::{apply_diff_to_loro, get_merged_loro_doc, parse_loro_state};
use crate::services::notes::crypto::verify_zkp_if_encrypted;
use crate::services::notes::store::get_note_store;
use crate::{DailyNote, EditHistoryEntry};

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
    verify_zkp_if_encrypted(
        &conn,
        &content,
        &auth.user_id,
        &auth.role,
        &workspace_id,
        &team_id,
    )
    .await?;
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
    note_store.upsert_note(item.clone())?;

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

        conn.execute(
            "INSERT INTO notes_fts (id, subject, content_plain) VALUES (?1, ?2, ?3)",
            crate::params![&item.id, &item.subject, &content],
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

        conn.execute(
            "DELETE FROM notes_fts WHERE id = ?1",
            crate::params![&note_id],
        ).await?;

        conn.execute(
            "INSERT INTO notes_fts (id, subject, content_plain) VALUES (?1, ?2, ?3)",
            crate::params![&note_id, &subject, &content],
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
            let _ = note_store.upsert_note(note.clone());

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
        conn.execute(
            "DELETE FROM notes_fts WHERE id = ?1",
            crate::params![&note_id],
        )
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
pub async fn get_notes_rkyv(
    requester_user_id: String,
    team_id: Option<String>,
) -> Result<Vec<u8>, YntraError> {
    let notes = get_notes(requester_user_id, team_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&notes)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}
