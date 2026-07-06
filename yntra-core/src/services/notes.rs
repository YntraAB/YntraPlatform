use crate::database;
use crate::observer::notify_observers;
use crate::{DailyNote, EditHistoryEntry, YntraError};

fn decode_content(raw_content: &str) -> String {
    if raw_content.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(&raw_content[5..]) {
            let doc = loro::LoroDoc::new();
            if doc.import(&bytes).is_ok() {
                return doc.get_text("content").to_string();
            }
        }
    }
    raw_content.to_string()
}

fn apply_diff_to_loro(text: &loro::LoroText, old_str: &str, new_str: &str) -> Result<(), YntraError> {
    let old_chars: Vec<char> = old_str.chars().collect();
    let new_chars: Vec<char> = new_str.chars().collect();
    
    let diffs = diff::slice(&old_chars, &new_chars);
    
    let mut pos = 0;
    let mut i = 0;
    while i < diffs.len() {
        match diffs[i] {
            diff::Result::Both(_, _) => {
                pos += 1;
                i += 1;
            }
            diff::Result::Left(_) => {
                let mut del_count = 0;
                while i < diffs.len() {
                    if let diff::Result::Left(_) = diffs[i] {
                        del_count += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }
                text.delete(pos, del_count).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
            diff::Result::Right(_) => {
                let mut ins_str = String::new();
                while i < diffs.len() {
                    if let diff::Result::Right(c) = diffs[i] {
                        ins_str.push(*c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                let ins_len = ins_str.chars().count();
                text.insert(pos, &ins_str).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                pos += ins_len;
            }
        }
    }
    Ok(())
}


#[uniffi::export]
pub async fn get_notes(requester_user_id: String, team_id: Option<String>) -> Result<Vec<DailyNote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    let (query, params) = if auth.role == "platform_admin" {
        match team_id {
            Some(tid) => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE team_id = ?1 ORDER BY created_at DESC".to_string(),
                crate::params![tid],
            ),
            None => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes ORDER BY created_at DESC".to_string(),
                crate::params![],
            ),
        }
    } else if auth.role == "admin" {
        match team_id {
            Some(tid) => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE team_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC".to_string(),
                crate::params![tid, ws_id],
            ),
            None => (
                "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE workspace_id = ?1 ORDER BY created_at DESC".to_string(),
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
                    "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE team_id = ?1 ORDER BY created_at DESC".to_string(),
                    crate::params![tid],
                )
            }
            None => (
                "SELECT n.id, n.workspace_id, n.team_id, n.author_id, n.subject, n.content, n.edit_history, n.created_at, n.updated_at, n.sync_status
                 FROM notes n
                 JOIN team_members tm ON n.team_id = tm.team_id
                 WHERE tm.user_id = ?1
                 ORDER BY n.created_at DESC".to_string(),
                 crate::params![requester_user_id],
            ),
        }
    };

    let mut stmt = conn.prepare(&query).await?;
    let list = stmt.query_map(params, |row| {
        let raw_content: String = row.get(5)?;
        Ok(DailyNote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            team_id: row.get(2)?,
            author_id: row.get(3)?,
            subject: row.get(4)?,
            content: decode_content(&raw_content),
            edit_history: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        })
    }).await?;

    Ok(list)
}

async fn get_merged_loro_doc(conn: &database::DbConnection, note_id: &str) -> Result<loro::LoroDoc, YntraError> {
    let doc = loro::LoroDoc::new();
    
    // 1. Fetch base note content snapshot
    let base_content: String = conn.query_row(
        "SELECT content FROM notes WHERE id = ?1",
        crate::params![note_id],
        |r| r.get(0)
    ).await.unwrap_or_else(|_| "".to_string());
    
    if base_content.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(&base_content[5..]) {
            doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else if !base_content.is_empty() {
        doc.get_text("content").insert(0, &base_content).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    
    // 2. Fetch and import all append-only updates
    let mut stmt = conn.prepare(
        "SELECT update_data FROM note_updates WHERE note_id = ?1 ORDER BY seq ASC, created_at ASC"
    ).await?;
    let mut rows = stmt.query(crate::params![note_id]).await?;
    while let Some(row) = rows.next().await? {
        let update_data_hex: String = row.get(0)?;
        if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
            doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    }
    
    Ok(doc)
}

#[uniffi::export]
pub async fn add_note(
    workspace_id: String,
    team_id: String,
    author_id: String,
    subject: String,
    content: String,
) -> Result<DailyNote, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &author_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
    let loro_content = format!("loro:{}", crate::infra::crypto::hex_encode(&loro_bytes));

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

    conn.execute(
        "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', ?7, ?8, 'pending')",
         crate::params![
             &item.id,
             &item.workspace_id,
             &item.team_id,
             &item.author_id,
             &item.subject,
             &loro_content, // Use the serialized Loro snapshot for database storage
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

    notify_observers();

    // Return the plaintext representation in memory
    item.content = content;
    Ok(item)
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
    let loro_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let loro_content = format!("loro:{}", crate::infra::crypto::hex_encode(&loro_bytes));

    // Append update to the event-sourced updates table
    let update_id = uuid::Uuid::new_v4().to_string();
    let next_seq: i64 = conn.query_row(
        "SELECT IFNULL(MAX(seq), 0) + 1 FROM note_updates WHERE note_id = ?1",
        crate::params![&note_id],
        |r| r.get(0)
    ).await.unwrap_or(1);

    let update_data_hex = crate::infra::crypto::hex_encode(&loro_bytes);
    conn.execute(
        "INSERT INTO note_updates (id, note_id, client_id, seq, update_data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        crate::params![&update_id, &note_id, &requester_user_id, &next_seq, &update_data_hex, &now_ms],
    ).await?;

    // 3. Update database cache projection
    conn.execute(
        "UPDATE notes SET subject = ?1, content = ?2, edit_history = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![&subject, &loro_content, &edit_history_str, &now_ms, &note_id],
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

    notify_observers();
    Ok(updated_note)
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

    conn.execute("DELETE FROM notes WHERE id = ?1", crate::params![&note_id]).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub fn merge_loro_notes(state1: String, state2: String) -> Result<String, YntraError> {
    let doc1 = loro::LoroDoc::new();
    if state1.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(&state1[5..]) {
            doc1.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc1.get_text("content").insert(0, &state1).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let doc2 = loro::LoroDoc::new();
    if state2.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(&state2[5..]) {
            doc2.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc2.get_text("content").insert(0, &state2).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    let bytes2 = doc2.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    doc1.import(&bytes2).map_err(|e| YntraError::SerializationError(e.to_string()))?;

    let merged_bytes = doc1.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(format!("loro:{}", crate::infra::crypto::hex_encode(&merged_bytes)))
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

    // 1. Append update to the event-sourced updates table
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

    // 2. Build fully merged state and update the database projection cache
    let doc = get_merged_loro_doc(&conn, &note_id).await?;
    let loro_bytes = doc.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    let loro_content = format!("loro:{}", crate::infra::crypto::hex_encode(&loro_bytes));

    conn.execute(
        "UPDATE notes SET content = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![&loro_content, &now_ms, &note_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_loro_merge() {
        // Create initial doc state
        let doc1 = loro::LoroDoc::new();
        let text1 = doc1.get_text("content");
        text1.insert(0, "Hello").unwrap();
        let state1 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap()));

        // Create concurrent update state from doc1's state
        let doc2 = loro::LoroDoc::new();
        let bytes1 = crate::infra::crypto::hex_decode(&state1[5..]).unwrap();
        doc2.import(&bytes1).unwrap();
        let text2 = doc2.get_text("content");
        text2.insert(5, " World").unwrap();
        let state2 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc2.export(loro::ExportMode::Snapshot).unwrap()));

        // Create another concurrent update state from doc1's state
        let doc3 = loro::LoroDoc::new();
        doc3.import(&bytes1).unwrap();
        let text3 = doc3.get_text("content");
        text3.insert(0, "CRDT ").unwrap();
        let state3 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc3.export(loro::ExportMode::Snapshot).unwrap()));

        // Merge state2 and state3
        let merged1 = merge_loro_notes(state2, state3).unwrap();
        let merged_bytes = crate::infra::crypto::hex_decode(&merged1[5..]).unwrap();

        // Load merged state into a final document
        let doc_final = loro::LoroDoc::new();
        doc_final.import(&merged_bytes).unwrap();
        let final_text = doc_final.get_text("content").to_string();

        // The text should contain edits from both users resolved conflict-free
        assert!(final_text.contains("World"));
        assert!(final_text.contains("CRDT"));
    }

    #[test]
    fn test_note_loro_merge_symmetric_plaintext() {
        let doc1 = loro::LoroDoc::new();
        let text1 = doc1.get_text("content");
        text1.insert(0, "LoroState").unwrap();
        let state1 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap()));

        let state2 = "PlaintextState".to_string();

        // Merge state1 (Loro) and state2 (Plaintext)
        let merged_1_2 = merge_loro_notes(state1.clone(), state2.clone()).unwrap();
        let merged_bytes_1_2 = crate::infra::crypto::hex_decode(&merged_1_2[5..]).unwrap();
        let doc_final_1_2 = loro::LoroDoc::new();
        doc_final_1_2.import(&merged_bytes_1_2).unwrap();
        let text_final_1_2 = doc_final_1_2.get_text("content").to_string();
        
        assert!(text_final_1_2.contains("LoroState"));
        assert!(text_final_1_2.contains("PlaintextState"));

        // Merge state2 (Plaintext) and state1 (Loro) - Should yield the exact same result symmetrically!
        let merged_2_1 = merge_loro_notes(state2, state1).unwrap();
        let merged_bytes_2_1 = crate::infra::crypto::hex_decode(&merged_2_1[5..]).unwrap();
        let doc_final_2_1 = loro::LoroDoc::new();
        doc_final_2_1.import(&merged_bytes_2_1).unwrap();
        let text_final_2_1 = doc_final_2_1.get_text("content").to_string();

        assert_eq!(text_final_1_2, text_final_2_1);
    }

    #[test]
    fn test_apply_diff_to_loro() {
        let doc = loro::LoroDoc::new();
        let text = doc.get_text("content");
        
        // Initial insert
        text.insert(0, "Hello World").unwrap();
        
        // Test insertion in middle
        apply_diff_to_loro(&text, "Hello World", "Hello CRDT World").unwrap();
        assert_eq!(text.to_string(), "Hello CRDT World");

        // Test deletion in middle
        apply_diff_to_loro(&text, "Hello CRDT World", "Hello World").unwrap();
        assert_eq!(text.to_string(), "Hello World");

        // Test replacement
        apply_diff_to_loro(&text, "Hello World", "Goodbye World").unwrap();
        assert_eq!(text.to_string(), "Goodbye World");

        // Test empty string handling
        apply_diff_to_loro(&text, "Goodbye World", "").unwrap();
        assert_eq!(text.to_string(), "");

        // Test restore from empty
        apply_diff_to_loro(&text, "", "Back again").unwrap();
        assert_eq!(text.to_string(), "Back again");
    }
}

