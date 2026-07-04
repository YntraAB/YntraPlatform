#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{DailyNote, EditHistoryEntry, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_notes(requester_user_id: String, team_id: Option<String>) -> Result<Vec<DailyNote>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let user_role: String = conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "user".to_string());

        let is_admin = user_role == "admin" || user_role == "platform_admin";

        let (query, params) = if is_admin {
            match team_id {
                Some(tid) => (
                    "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes WHERE team_id = ?1 ORDER BY created_at DESC".to_string(),
                    vec![tid],
                ),
                None => (
                    "SELECT id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status FROM notes ORDER BY created_at DESC".to_string(),
                    vec![],
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
                        vec![tid],
                    )
                }
                None => (
                    "SELECT n.id, n.workspace_id, n.team_id, n.author_id, n.subject, n.content, n.edit_history, n.created_at, n.updated_at, n.sync_status
                     FROM notes n
                     JOIN team_members tm ON n.team_id = tm.team_id
                     WHERE tm.user_id = ?1
                     ORDER BY n.created_at DESC".to_string(),
                    vec![requester_user_id.clone()],
                ),
            }
        };

        let mut stmt = conn.prepare(&query).await?;
        let list = stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
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
                sync_status: row.get(9)?,
            })
        }).await?;

        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = requester_user_id;
        let store = wasm_store::get_store().lock().unwrap();
        if let Some(tid) = team_id {
            Ok(store
                .notes
                .iter()
                .filter(|n| n.team_id == tid)
                .cloned()
                .collect())
        } else {
            Ok(store.notes.clone())
        }
    }
}

#[uniffi::export]
pub async fn add_note(
    workspace_id: String,
    team_id: String,
    author_id: String,
    subject: String,
    content: String,
) -> Result<DailyNote, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = DailyNote {
        id: id.clone(),
        workspace_id,
        team_id,
        author_id: Some(author_id),
        subject,
        content,
        edit_history: "[]".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, edit_history, created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', ?7, ?8, 'pending')",
            crate::params![
                &item.id,
                &item.workspace_id,
                &item.team_id,
                &item.author_id,
                &item.subject,
                &item.content,
                &item.created_at,
                &item.updated_at
            ],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.notes.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn update_note(
    note_id: String,
    edited_by_name: String,
    subject: String,
    content: String,
) -> Result<DailyNote, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

        // 2. Compute history entry
        let mut history: Vec<EditHistoryEntry> = serde_json::from_str(&old_note.edit_history)
            .unwrap_or_default();
        
        let mut changed = false;
        let mut entry = EditHistoryEntry {
            editedBy: edited_by_name,
            editedAt: crate::infra::time::get_current_time_str_hm(),
            oldSubject: None,
            newSubject: None,
            oldContent: None,
            newContent: None,
        };

        if old_note.subject != subject {
            entry.oldSubject = Some(old_note.subject.clone());
            entry.newSubject = Some(subject.clone());
            changed = true;
        }
        if old_note.content != content {
            entry.oldContent = Some(old_note.content.clone());
            entry.newContent = Some(content.clone());
            changed = true;
        }
        if changed {
            history.insert(0, entry);
        }

        let edit_history_str = serde_json::to_string(&history)
            .map_err(|e| YntraError::DbError(e.to_string()))?;

        // 3. Update database
        conn.execute(
            "UPDATE notes SET subject = ?1, content = ?2, edit_history = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
            crate::params![&subject, &content, &edit_history_str, &now_ms, &note_id],
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

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(note) = store.notes.iter_mut().find(|n| n.id == note_id) {
            let mut history: Vec<EditHistoryEntry> = serde_json::from_str(&note.edit_history)
                .unwrap_or_default();
            
            let mut changed = false;
            let mut entry = EditHistoryEntry {
                editedBy: edited_by_name,
                editedAt: crate::infra::time::get_current_time_str_hm(),
                oldSubject: None,
                newSubject: None,
                oldContent: None,
                newContent: None,
            };

            if note.subject != subject {
                entry.oldSubject = Some(note.subject.clone());
                entry.newSubject = Some(subject.clone());
                changed = true;
            }
            if note.content != content {
                entry.oldContent = Some(note.content.clone());
                entry.newContent = Some(content.clone());
                changed = true;
            }
            if changed {
                history.insert(0, entry);
            }

            let edit_history_str = serde_json::to_string(&history)
                .map_err(|e| YntraError::DbError(e.to_string()))?;

            note.subject = subject.clone();
            note.content = content.clone();
            note.edit_history = edit_history_str.clone();
            note.updated_at = now_ms;
            note.sync_status = "pending".to_string();

            let updated_note = note.clone();
            drop(store);
            notify_observers();
            Ok(updated_note)
        } else {
            Err(YntraError::NotFoundError(format!("Note not found: {}", note_id)))
        }
    }
}

#[uniffi::export]
pub async fn delete_note(requester_user_id: String, note_id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        let note_row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT workspace_id, author_id FROM notes WHERE id = ?1",
            crate::params![&note_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        if let Some((note_ws_id, author_id)) = note_row {
            let requester_row: Option<(String, Option<String>)> = conn.query_row(
                "SELECT role, workspace_id FROM users WHERE id = ?1",
                crate::params![&requester_user_id],
                |r| Ok((r.get(0)?, r.get(1)?))
            ).await.ok();

            let (req_role, req_ws_id) = match requester_row {
                Some((role, Some(ws_id))) => (role, ws_id),
                _ => return Err(YntraError::AuthError("Requester user not found or invalid workspace".to_string())),
            };

            if note_ws_id != req_ws_id {
                return Err(YntraError::AuthError("Access denied: note is in a different workspace".to_string()));
            }

            let is_author = author_id.as_deref() == Some(&requester_user_id);
            let is_admin = req_role == "admin" || req_role == "platform_admin";

            if !is_author && !is_admin {
                return Err(YntraError::AuthError("Access denied: only the author or an administrator can delete this note".to_string()));
            }
        } else {
            return Err(YntraError::NotFoundError(format!("Note not found: {}", note_id)));
        }

        conn.execute("DELETE FROM notes WHERE id = ?1", crate::params![&note_id]).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let req_role = requester.role.as_str();
        let req_ws = requester.workspace_id.clone().unwrap_or_default();

        if let Some(pos) = store.notes.iter().position(|n| n.id == note_id) {
            let note = &store.notes[pos];
            if note.workspace_id != req_ws {
                return Err(YntraError::AuthError("Access denied: note is in a different workspace".to_string()));
            }
            let is_author = note.author_id.as_deref() == Some(&requester_user_id);
            let is_admin = req_role == "admin" || req_role == "platform_admin";

            if !is_author && !is_admin {
                return Err(YntraError::AuthError("Access denied".to_string()));
            }

            store.notes.remove(pos);
            drop(store);
            notify_observers();
            Ok(())
        } else {
            Err(YntraError::NotFoundError(format!("Note not found: {}", note_id)))
        }
    }
}
