use crate::database;
use crate::observer::notify_observers;
use crate::{DailyNote, EditHistoryEntry, YntraError};

pub mod crdt;

use crdt::{parse_loro_state, apply_diff_to_loro, get_merged_loro_doc};

#[uniffi::export]
pub async fn get_notes(requester_user_id: String, team_id: Option<String>) -> Result<Vec<DailyNote>, YntraError> {
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
        
        raw_notes.push((id, workspace_id, team_id, author_id, subject, base_content, edit_history, created_at, updated_at, sync_status, content_plain, max_seq));
    }

    if raw_notes.is_empty() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let mut repairs = Vec::new();
    for (id, workspace_id, team_id, author_id, subject, base_content, edit_history, created_at, updated_at, sync_status, content_plain, max_seq) in raw_notes {
        let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);
        
        let has_unmerged = max_seq > last_merged_seq;

        let content = if !has_unmerged && content_plain.is_some() {
            content_plain.unwrap()
        } else {
            let doc = loro::LoroDoc::new();
            if base_content.starts_with("loro:") {
                if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
                    doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
            } else {
                doc.get_text("content").insert(0, hex_or_plain).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }

            let mut final_max_seq = last_merged_seq;
            if has_unmerged {
                let mut stmt_updates = conn.prepare(
                    "SELECT seq, update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
                ).await?;
                let mut rows_updates = stmt_updates.query(crate::params![&id, last_merged_seq]).await?;
                while let Some(row_up) = rows_updates.next().await? {
                    let seq: i64 = row_up.get(0)?;
                    let update_data_hex: String = row_up.get(1)?;
                    if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
                        doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                    }
                    if seq > final_max_seq {
                        final_max_seq = seq;
                    }
                }
            }

            let plain = doc.get_text("content").to_string();

            // Cache merged state to avoid future LoroDoc execution on next read
            let snapshot_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            let loro_content = format!("loro:{}:{}", final_max_seq, crate::infra::crypto::hex_encode(&snapshot_bytes));
            
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
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }
    if auth.role != "platform_admin" && requester_user_id != author_id {
        return Err(YntraError::AuthError("Access denied: cannot create note as another user".to_string()));
    }

    if auth.role != "admin" && auth.role != "platform_admin" {
        let is_member: i64 = conn.query_row(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
            crate::params![&team_id, &author_id],
            |r| r.get(0)
        ).await.unwrap_or(0);
        if is_member == 0 {
            return Err(YntraError::AuthError("Access denied: you are not a member of this team".to_string()));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    
    // Create Loro doc for the content
    let doc = loro::LoroDoc::new();
    let text = doc.get_text("content");
    text.insert(0, &content).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let loro_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
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

    let note_row: Option<(String, Option<String>)> = conn.query_row(
        "SELECT workspace_id, author_id FROM notes WHERE id = ?1",
        crate::params![&note_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((note_ws_id, author_id)) = note_row {
        let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

        if auth.role != "platform_admin" && note_ws_id != auth.workspace_id {
            return Err(YntraError::AuthError("Access denied: note is in a different workspace".to_string()));
        }

        let is_author = author_id.as_deref() == Some(&requester_user_id);
        if !is_author && !auth.is_admin {
            return Err(YntraError::AuthError("Access denied: only the author or an administrator can delete this note".to_string()));
        }
    } else {
        return Err(YntraError::NotFoundError(format!("Note not found: {}", note_id)));
    }

    conn.begin_transaction().await?;
    let res = async {
        conn.execute("DELETE FROM note_updates WHERE note_id = ?1", crate::params![&note_id]).await?;
        conn.execute("DELETE FROM notes WHERE id = ?1", crate::params![&note_id]).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
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
            doc1.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc1.get_text("content").insert(0, hex_or_plain1).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let doc2 = loro::LoroDoc::new();
    let (seq2, hex_or_plain2) = parse_loro_state(&state2);
    if state2.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain2) {
            doc2.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc2.get_text("content").insert(0, hex_or_plain2).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let bytes2 = doc2.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    doc1.import(&bytes2).map_err(|e| YntraError::SerializationError(e.to_string()))?;

    let merged_bytes = doc1.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let max_seq = seq1.max(seq2);
    if max_seq >= 0 {
        Ok(format!("loro:{}:{}", max_seq, crate::infra::crypto::hex_encode(&merged_bytes)))
    } else {
        Ok(format!("loro:{}", crate::infra::crypto::hex_encode(&merged_bytes)))
    }
}

#[uniffi::export]
pub async fn get_note_loro_state(note_id: String) -> Result<Vec<u8>, YntraError> {
    let conn = database::acquire_connection().await?;
    let doc = get_merged_loro_doc(&conn, &note_id).await?;
    let bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes)
}

#[uniffi::export]
pub async fn apply_note_loro_update(note_id: String, update_bytes: Vec<u8>) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.begin_transaction().await?;

    let res = async {
        // 1. Build merged state using cached snapshot + remote update
        let doc = get_merged_loro_doc(&conn, &note_id).await?;
        doc.import(&update_bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        
        let loro_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;

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
             VALUES (?1, ?2, 'remote', ?3, ?4, ?5)",
            crate::params![&update_id, &note_id, &next_seq, &update_data_hex, &now_ms],
        ).await?;

        // 3. Update the database projection cache
        let plain_text = doc.get_text("content").to_string();
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
pub async fn merge_unmerged_notes() -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    
    // Find candidate notes that have at least one update in note_updates
    let mut stmt = conn.prepare(
        "SELECT id, content FROM notes WHERE EXISTS (SELECT 1 FROM note_updates WHERE note_updates.note_id = notes.id)"
    ).await?;
    
    let mut rows = stmt.query(()).await?;
    let mut candidates = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        candidates.push((id, content));
    }
    
    let mut repairs = Vec::new();
    for (id, base_content) in candidates {
        let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);
        
        let max_seq: i64 = conn.query_row(
            "SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_id = ?1",
            crate::params![&id],
            |r| r.get(0)
        ).await.unwrap_or(-1);
        
        if max_seq > last_merged_seq {
            let doc = loro::LoroDoc::new();
            if base_content.starts_with("loro:") {
                if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
                    doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
            } else {
                doc.get_text("content").insert(0, hex_or_plain).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
            
            let mut stmt_updates = conn.prepare(
                "SELECT seq, update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
            ).await?;
            let mut rows_updates = stmt_updates.query(crate::params![&id, last_merged_seq]).await?;
            let mut final_max_seq = last_merged_seq;
            while let Some(row_up) = rows_updates.next().await? {
                let seq: i64 = row_up.get(0)?;
                let update_data_hex: String = row_up.get(1)?;
                if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
                    doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                }
                if seq > final_max_seq {
                    final_max_seq = seq;
                }
            }
            
            let plain = doc.get_text("content").to_string();
            let snapshot_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            let loro_content = format!("loro:{}:{}", final_max_seq, crate::infra::crypto::hex_encode(&snapshot_bytes));
            
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
