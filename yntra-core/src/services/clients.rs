#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{ClientProfile, JournalEntry, MedicationItem, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_clients(requester_user_id: String) -> Result<Vec<ClientProfile>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let user_row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT role, full_name FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (role, full_name) = match user_row {
            Some((r, f)) => (r, f),
            None => return Err(YntraError::AuthError("User not found".to_string())),
        };

        let is_admin = role == "admin" || role == "platform_admin";

        let (query, params) = if is_admin {
            (
                "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients".to_string(),
                vec![],
            )
        } else if role == "client" {
            if let Some(name) = full_name {
                let parts: Vec<&str> = name.split_whitespace().collect();
                if parts.len() >= 2 {
                    (
                        "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients WHERE first_name = ?1 AND last_name = ?2".to_string(),
                        vec![parts[0].to_string(), parts[1].to_string()],
                    )
                } else {
                    (
                        "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients WHERE 1 = 0".to_string(),
                        vec![],
                    )
                }
            } else {
                (
                    "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients WHERE 1 = 0".to_string(),
                    vec![],
                )
            }
        } else {
            (
                "SELECT c.id, c.workspace_id, c.team_id, c.first_name, c.last_name, c.personal_number, c.care_level, c.message_settings, c.created_at, c.updated_at, c.sync_status
                 FROM clients c
                 JOIN team_members tm ON c.team_id = tm.team_id
                 WHERE tm.user_id = ?1".to_string(),
                vec![requester_user_id.clone()],
            )
        };

        let mut stmt = conn.prepare(&query).await?;
        let list = stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
            let ws_id: String = row.get(1)?;
            let raw_pnum: Option<String> = row.get(5)?;
            Ok(ClientProfile {
                id: row.get(0)?,
                workspace_id: ws_id.clone(),
                team_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                personal_number: crate::infra::crypto::decrypt_opt_field(raw_pnum, Some(ws_id)),
                care_level: row.get(6)?,
                message_settings: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                sync_status: row.get(10)?,
            })
        }).await?;

        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id);
        let role = requester.map(|u| u.role.clone()).unwrap_or_else(|| "user".to_string());
        
        if role == "admin" || role == "platform_admin" {
            Ok(store.clients.clone())
        } else if role == "client" {
            let full_name = requester.and_then(|u| u.full_name.clone()).unwrap_or_default();
            Ok(store.clients.iter().filter(|c| {
                full_name.contains(&c.first_name) && full_name.contains(&c.last_name)
            }).cloned().collect())
        } else {
            Ok(store.clients.clone())
        }
    }
}

#[uniffi::export]
pub async fn get_medications(client_id: String, actor_id: String) -> Result<Vec<MedicationItem>, YntraError> {
    crate::log_action(actor_id.clone(), Some(client_id.clone()), "read_medications".to_string()).await?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        // 1. Fetch client details to check workspace and team assignment
        let client = {
            let mut stmt = conn.prepare("SELECT workspace_id, team_id FROM clients WHERE id = ?1").await?;
            let mut rows = stmt.query(crate::params![&client_id]).await?;
            if let Some(row) = rows.next().await? {
                let ws_id: String = row.get(0)?;
                let team_id: Option<String> = row.get(1)?;
                (ws_id, team_id)
            } else {
                return Err(YntraError::NotFoundError("Client not found".to_string()));
            }
        };

        // 2. Perform team access check (inre sekretess)
        let is_authorized = {
            let mut admin_stmt = conn.prepare("SELECT role FROM users WHERE id = ?1").await?;
            let mut admin_rows = admin_stmt.query(crate::params![&actor_id]).await?;
            let is_admin = if let Some(row) = admin_rows.next().await? {
                let role: String = row.get(0)?;
                role == "platform_admin"
            } else {
                false
            };

            if is_admin {
                true
            } else if let Some(tid) = &client.1 {
                let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
                let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
                member_rows.next().await?.is_some()
            } else {
                true
            }
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
        }

        let ws_id = client.0;

        // 3. Query medications and decrypt sensitive fields
        let mut stmt = conn.prepare("SELECT id, client_id, name, dosage, frequency, instructions, created_at, workspace_id, updated_at, sync_status FROM client_medications WHERE client_id = ?1").await?;

        let list = stmt.query_map([client_id], |row| {
            let ws_id_clone = ws_id.clone();
            let raw_dosage: Option<String> = row.get(3)?;
            let raw_freq: Option<String> = row.get(4)?;
            let raw_instr: Option<String> = row.get(5)?;
            Ok(MedicationItem {
                id: row.get(0)?,
                client_id: row.get(1)?,
                name: row.get(2)?,
                dosage: crate::infra::crypto::decrypt_opt_field(raw_dosage, Some(ws_id_clone.clone())),
                frequency: crate::infra::crypto::decrypt_opt_field(raw_freq, Some(ws_id_clone.clone())),
                instructions: crate::infra::crypto::decrypt_opt_field(raw_instr, Some(ws_id_clone)),
                created_at: row.get(6)?,
                workspace_id: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        }).await?;

        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let client = store.clients.iter().find(|c| c.id == client_id).ok_or_else(|| YntraError::NotFoundError("Client not found".to_string()))?;

        let is_authorized = {
            let user = store.users.iter().find(|u| u.id == actor_id);
            if let Some(u) = user {
                u.role == "platform_admin" || client.team_id.is_none()
            } else {
                false
            }
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
        }

        Ok(store
            .medications
            .iter()
            .filter(|m| m.client_id == client_id)
            .cloned()
            .collect())
    }
}

#[uniffi::export]
pub async fn get_journals(client_id: String, actor_id: String) -> Result<Vec<JournalEntry>, YntraError> {
    crate::log_action(actor_id.clone(), Some(client_id.clone()), "read_journals".to_string()).await?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        // 1. Fetch client details to check workspace and team assignment
        let client = {
            let mut stmt = conn.prepare("SELECT workspace_id, team_id FROM clients WHERE id = ?1").await?;
            let mut rows = stmt.query(crate::params![&client_id]).await?;
            if let Some(row) = rows.next().await? {
                let ws_id: String = row.get(0)?;
                let team_id: Option<String> = row.get(1)?;
                (ws_id, team_id)
            } else {
                return Err(YntraError::NotFoundError("Client not found".to_string()));
            }
        };

        // 2. Perform team access check (inre sekretess)
        let is_authorized = {
            let mut admin_stmt = conn.prepare("SELECT role FROM users WHERE id = ?1").await?;
            let mut admin_rows = admin_stmt.query(crate::params![&actor_id]).await?;
            let is_admin = if let Some(row) = admin_rows.next().await? {
                let role: String = row.get(0)?;
                role == "platform_admin"
            } else {
                false
            };

            if is_admin {
                true
            } else if let Some(tid) = &client.1 {
                let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
                let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
                member_rows.next().await?.is_some()
            } else {
                true
            }
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
        }

        let ws_id = client.0;

        // 3. Query journals and decrypt sensitive content field
        let mut stmt = conn.prepare("SELECT id, client_id, author_id, content, created_at, workspace_id, updated_at, sync_status FROM client_journals WHERE client_id = ?1 ORDER BY created_at DESC").await?;

        let list = stmt.query_map([client_id], |row| {
            let ws_id_clone = ws_id.clone();
            let raw_content: String = row.get(3)?;
            Ok(JournalEntry {
                id: row.get(0)?,
                client_id: row.get(1)?,
                author_id: row.get(2)?,
                content: crate::infra::crypto::decrypt_field(&raw_content, &ws_id_clone),
                created_at: row.get(4)?,
                workspace_id: row.get(5)?,
                updated_at: row.get(6)?,
                sync_status: row.get(7)?,
            })
        }).await?;

        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let client = store.clients.iter().find(|c| c.id == client_id).ok_or_else(|| YntraError::NotFoundError("Client not found".to_string()))?;

        let is_authorized = {
            let user = store.users.iter().find(|u| u.id == actor_id);
            if let Some(u) = user {
                u.role == "platform_admin" || client.team_id.is_none()
            } else {
                false
            }
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
        }

        Ok(store
            .journals
            .iter()
            .filter(|j| j.client_id == client_id)
            .cloned()
            .collect())
    }
}

#[uniffi::export]
pub async fn add_journal_entry(
    workspace_id: String,
    client_id: String,
    author_id: String,
    content: String,
) -> Result<JournalEntry, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let author_row: Option<(String, String)> = conn.query_row(
            "SELECT role, workspace_id FROM users WHERE id = ?1",
            crate::params![&author_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (author_role, author_ws) = match author_row {
            Some((role, ws)) => (role, ws),
            None => return Err(YntraError::AuthError("Author not found".to_string())),
        };

        if author_ws != workspace_id {
            return Err(YntraError::AuthError("Access denied: author belongs to a different workspace".to_string()));
        }

        let client_row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT workspace_id, team_id FROM clients WHERE id = ?1",
            crate::params![&client_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (client_ws, client_team) = match client_row {
            Some((ws, team)) => (ws, team),
            None => return Err(YntraError::NotFoundError("Client not found".to_string())),
        };

        if client_ws != workspace_id {
            return Err(YntraError::AuthError("Access denied: client belongs to a different workspace".to_string()));
        }

        let is_authorized = if author_role == "platform_admin" || author_role == "admin" {
            true
        } else if let Some(tid) = &client_team {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &author_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            true
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let author = store.users.iter().find(|u| u.id == author_id)
            .ok_or_else(|| YntraError::AuthError("Author not found".to_string()))?;
        if author.workspace_id.as_deref() != Some(&workspace_id) {
            return Err(YntraError::AuthError("Access denied: author belongs to a different workspace".to_string()));
        }

        let client = store.clients.iter().find(|c| c.id == client_id)
            .ok_or_else(|| YntraError::NotFoundError("Client not found".to_string()))?;
        if client.workspace_id != workspace_id {
            return Err(YntraError::AuthError("Access denied: client belongs to a different workspace".to_string()));
        }

        let is_authorized = author.role == "platform_admin" || author.role == "admin" || client.team_id.is_none();
        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = JournalEntry {
        id: id.clone(),
        client_id,
        author_id: Some(author_id),
        content,
        created_at,
        workspace_id: workspace_id.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let enc_content = crate::infra::crypto::encrypt_field(&item.content, &workspace_id);

        conn.execute(
            "INSERT INTO client_journals (id, client_id, author_id, content, created_at, workspace_id, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
            crate::params![
                &item.id,
                &item.client_id,
                &item.author_id,
                &enc_content,
                &item.created_at,
                &item.workspace_id,
                &item.updated_at
            ],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.journals.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn add_medication(
    workspace_id: String,
    client_id: String,
    actor_id: String,
    name: String,
    dosage: String,
    frequency: String,
    instructions: String,
) -> Result<MedicationItem, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let actor_row: Option<(String, String)> = conn.query_row(
            "SELECT role, workspace_id FROM users WHERE id = ?1",
            crate::params![&actor_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (actor_role, actor_ws) = match actor_row {
            Some((role, ws)) => (role, ws),
            None => return Err(YntraError::AuthError("Actor not found".to_string())),
        };

        if actor_ws != workspace_id {
            return Err(YntraError::AuthError("Access denied: actor belongs to a different workspace".to_string()));
        }

        let client_row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT workspace_id, team_id FROM clients WHERE id = ?1",
            crate::params![&client_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (client_ws, client_team) = match client_row {
            Some((ws, team)) => (ws, team),
            None => return Err(YntraError::NotFoundError("Client not found".to_string())),
        };

        if client_ws != workspace_id {
            return Err(YntraError::AuthError("Access denied: client belongs to a different workspace".to_string()));
        }

        let is_authorized = if actor_role == "platform_admin" || actor_role == "admin" {
            true
        } else if let Some(tid) = &client_team {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            true
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let actor = store.users.iter().find(|u| u.id == actor_id)
            .ok_or_else(|| YntraError::AuthError("Actor not found".to_string()))?;
        if actor.workspace_id.as_deref() != Some(&workspace_id) {
            return Err(YntraError::AuthError("Access denied: actor belongs to a different workspace".to_string()));
        }

        let client = store.clients.iter().find(|c| c.id == client_id)
            .ok_or_else(|| YntraError::NotFoundError("Client not found".to_string()))?;
        if client.workspace_id != workspace_id {
            return Err(YntraError::AuthError("Access denied: client belongs to a different workspace".to_string()));
        }

        let is_authorized = actor.role == "platform_admin" || actor.role == "admin" || client.team_id.is_none();
        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = MedicationItem {
        id: id.clone(),
        client_id,
        name,
        dosage: Some(dosage),
        frequency: Some(frequency),
        instructions: Some(instructions),
        created_at,
        workspace_id: workspace_id.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let enc_dosage = crate::infra::crypto::encrypt_opt_field(item.dosage.clone(), Some(workspace_id.clone()));
        let enc_freq = crate::infra::crypto::encrypt_opt_field(item.frequency.clone(), Some(workspace_id.clone()));
        let enc_instr = crate::infra::crypto::encrypt_opt_field(item.instructions.clone(), Some(workspace_id.clone()));

        conn.execute(
            "INSERT INTO client_medications (id, client_id, name, dosage, frequency, instructions, created_at, workspace_id, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
            crate::params![
                &item.id,
                &item.client_id,
                &item.name,
                &enc_dosage,
                &enc_freq,
                &enc_instr,
                &item.created_at,
                &item.workspace_id,
                &item.updated_at
            ],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.medications.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn add_client_via_directory(
    workspace_id: String,
    team_id: Option<String>,
    first_name: String,
    last_name: String,
    personal_number: String,
    care_level: String,
) -> Result<ClientProfile, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = ClientProfile {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        team_id,
        first_name,
        last_name,
        personal_number: Some(personal_number),
        care_level: Some(care_level),
        message_settings: "{}".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let encrypted_pnum = crate::infra::crypto::encrypt_opt_field(item.personal_number.clone(), Some(workspace_id.clone()));

        conn.execute(
            "INSERT INTO clients (id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}', ?8, ?9, 'pending')",
            crate::params![
                &item.id,
                &item.workspace_id,
                &item.team_id,
                &item.first_name,
                &item.last_name,
                &encrypted_pnum,
                &item.care_level,
                &item.created_at,
                &item.updated_at
            ],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.clients.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn update_client_profile(
    client_id: String,
    first_name: String,
    last_name: String,
    personal_number: Option<String>,
    care_level: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let ws_id: String = {
            let mut stmt = conn.prepare("SELECT workspace_id FROM clients WHERE id = ?1").await?;
            let mut rows = stmt.query(crate::params![&client_id]).await?;
            if let Some(row) = rows.next().await? {
                row.get(0)?
            } else {
                return Err(YntraError::NotFoundError("Client not found".to_string()));
            }
        };
        let encrypted_pnum = crate::infra::crypto::encrypt_opt_field(personal_number.clone(), Some(ws_id));

        conn.execute(
            "UPDATE clients SET first_name = ?1, last_name = ?2, personal_number = ?3, care_level = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
            crate::params![first_name, last_name, encrypted_pnum, care_level, now_ms, client_id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(c) = store.clients.iter_mut().find(|c| c.id == client_id) {
            c.first_name = first_name;
            c.last_name = last_name;
            c.personal_number = personal_number;
            c.care_level = care_level;
            c.updated_at = now_ms;
            c.sync_status = "pending".to_string();
        }
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn delete_client(client_id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        // 1. Delete associated medication items
        conn.execute("DELETE FROM client_medications WHERE client_id = ?1", crate::params![&client_id]).await?;

        // 2. Delete associated journal entries
        conn.execute("DELETE FROM client_journals WHERE client_id = ?1", crate::params![&client_id]).await?;

        // 3. Delete client profile record
        conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![&client_id]).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();

        // Remove medications associated with client
        store.medications.retain(|m| m.client_id != client_id);

        // Remove journals associated with client
        store.journals.retain(|j| j.client_id != client_id);

        // Remove client profile
        store.clients.retain(|c| c.id != client_id);

        notify_observers();
        Ok(())
    }
}
