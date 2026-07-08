use crate::database;
use crate::observer::notify_observers;
use crate::{JournalEntry, MedicationItem, YntraError};

#[uniffi::export]
pub async fn get_medications(client_id: String, actor_id: String) -> Result<Vec<MedicationItem>, YntraError> {
    let conn = database::acquire_connection().await?;

    crate::services::audit::log_action_with_conn(&conn, actor_id.clone(), Some(client_id.clone()), "read_medications".to_string()).await?;

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
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    let is_authorized = {
        if auth.role == "platform_admin" {
            true
        } else if auth.role == "admin" {
            auth.workspace_id == client.0
        } else if let Some(tid) = &client.1 {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            false
        }
    };

    if !is_authorized {
        return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
    }

    let ws_id = client.0;
    let cipher = crate::infra::crypto::WorkspaceCipher::new(&ws_id)?;

    // 3. Query medications and decrypt sensitive fields
    let mut stmt = conn.prepare("SELECT id, client_id, name, dosage, frequency, instructions, created_at, workspace_id, updated_at, sync_status FROM client_medications WHERE client_id = ?1").await?;

    let list = stmt.query_map(crate::params![client_id], |row| {
        let raw_dosage: Option<String> = row.get(3)?;
        let raw_freq: Option<String> = row.get(4)?;
        let raw_instr: Option<String> = row.get(5)?;
        Ok(MedicationItem {
            id: row.get(0)?,
            client_id: row.get(1)?,
            name: row.get(2)?,
            dosage: cipher.decrypt_opt(raw_dosage),
            frequency: cipher.decrypt_opt(raw_freq),
            instructions: cipher.decrypt_opt(raw_instr),
            created_at: row.get(6)?,
            workspace_id: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_journals(client_id: String, actor_id: String) -> Result<Vec<JournalEntry>, YntraError> {
    let conn = database::acquire_connection().await?;

    crate::services::audit::log_action_with_conn(&conn, actor_id.clone(), Some(client_id.clone()), "read_journals".to_string()).await?;

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
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    let is_authorized = {
        if auth.role == "platform_admin" {
            true
        } else if auth.role == "admin" {
            auth.workspace_id == client.0
        } else if let Some(tid) = &client.1 {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            false
        }
    };

    if !is_authorized {
        return Err(YntraError::AuthError("You are not authorized to view this client's health records".to_string()));
    }

    let ws_id = client.0;
    let cipher = crate::infra::crypto::WorkspaceCipher::new(&ws_id)?;

    // 3. Query journals and decrypt sensitive content field
    let mut stmt = conn.prepare("SELECT id, client_id, author_id, content, created_at, workspace_id, updated_at, sync_status FROM client_journals WHERE client_id = ?1 ORDER BY created_at DESC").await?;

    let list = stmt.query_map(crate::params![client_id], move |row| {
        let raw_content: String = row.get(3)?;
        Ok(JournalEntry {
            id: row.get(0)?,
            client_id: row.get(1)?,
            author_id: row.get(2)?,
            content: cipher.decrypt(&raw_content).unwrap_or(raw_content),
            created_at: row.get(4)?,
            workspace_id: row.get(5)?,
            updated_at: row.get(6)?,
            sync_status: row.get(7)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_journal_entry(
    workspace_id: String,
    client_id: String,
    author_id: String,
    content: String,
) -> Result<JournalEntry, YntraError> {
    let conn = database::acquire_connection().await?;
    conn.begin_transaction().await?;

    let res = async {
        let auth = crate::AuthContext::authorize(&conn, &author_id).await?;

        if auth.workspace_id != workspace_id {
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

        let is_authorized = if auth.role == "platform_admin" || auth.role == "admin" {
            true
        } else if let Some(tid) = &client_team {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &author_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            false
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }

        crate::services::audit::log_action_with_conn(&conn, author_id.clone(), Some(client_id.clone()), "add_journal_entry".to_string()).await?;

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

        let enc_content = crate::infra::crypto::encrypt_field(&item.content, &workspace_id)?;

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

        Ok(item)
    }.await;

    match res {
        Ok(item) => {
            conn.commit().await?;
            notify_observers();
            Ok(item)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
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
    let conn = database::acquire_connection().await?;
    conn.begin_transaction().await?;

    let res = async {
        let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;

        if auth.workspace_id != workspace_id {
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

        let is_authorized = if auth.role == "platform_admin" || auth.role == "admin" {
            true
        } else if let Some(tid) = &client_team {
            let mut member_stmt = conn.prepare("SELECT 1 FROM team_members WHERE team_id = ?1 AND user_id = ?2").await?;
            let mut member_rows = member_stmt.query(crate::params![tid, &actor_id]).await?;
            member_rows.next().await?.is_some()
        } else {
            false
        };

        if !is_authorized {
            return Err(YntraError::AuthError("You are not authorized to write to this client's records".to_string()));
        }

        crate::services::audit::log_action_with_conn(&conn, actor_id.clone(), Some(client_id.clone()), "add_medication".to_string()).await?;

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

        let cipher = crate::infra::crypto::WorkspaceCipher::new(&workspace_id)?;
        let enc_dosage = cipher.encrypt_opt(item.dosage.clone())?;
        let enc_freq = cipher.encrypt_opt(item.frequency.clone())?;
        let enc_instr = cipher.encrypt_opt(item.instructions.clone())?;

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

        Ok(item)
    }.await;

    match res {
        Ok(item) => {
            conn.commit().await?;
            notify_observers();
            Ok(item)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compliance_logging_for_client_writes() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        // Setup test workspace and user
        let user_id = "test-author-user-777";
        let client_id = "test-client-777";
        let ws_id = "workspace-1";

        {
            let conn = database::acquire_connection().await.unwrap();
            // Pre-clean in case of previous test crashes
            let _ = conn.execute("DELETE FROM client_journals WHERE author_id = ?1", crate::params![user_id]).await;
            let _ = conn.execute("DELETE FROM client_medications WHERE client_id = ?1", crate::params![client_id]).await;
            let _ = conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id]).await;
            let _ = conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![client_id]).await;
            let _ = conn.execute("DELETE FROM audit_logs WHERE actor_id = ?1", crate::params![user_id]).await;

            conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'author@yntra.io', 'admin')", crate::params![user_id, ws_id]).await.unwrap();
            conn.execute("INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, care_level, created_at, updated_at) VALUES (?1, ?2, 'Bob', 'Jones', 'Normal', '2026-07-05', 0)", crate::params![client_id, ws_id]).await.unwrap();
        }

        crate::infra::crypto::set_session_key("test-session-key".to_string().into_bytes());

        // 1. Add Journal Entry
        let _journal = add_journal_entry(ws_id.to_string(), client_id.to_string(), user_id.to_string(), "Patient condition stable".to_string()).await.unwrap();

        // 2. Add Medication
        let _medication = add_medication(ws_id.to_string(), client_id.to_string(), user_id.to_string(), "Aspirin".to_string(), "500mg".to_string(), "Daily".to_string(), "Take after meal".to_string()).await.unwrap();

        // 3. Verify that audit logs contain entries for both writes
        let logs = crate::get_audit_logs(user_id.to_string()).await.unwrap();
        
        let has_journal_log = logs.iter().any(|l| l.actor_id == user_id && l.target_client_id.as_deref() == Some(client_id) && l.action_type == "add_journal_entry");
        let has_medication_log = logs.iter().any(|l| l.actor_id == user_id && l.target_client_id.as_deref() == Some(client_id) && l.action_type == "add_medication");

        assert!(has_journal_log);
        assert!(has_medication_log);

        // Clean up
        {
            let conn = database::acquire_connection().await.unwrap();
            conn.execute("DELETE FROM client_journals WHERE author_id = ?1", crate::params![user_id]).await.unwrap();
            conn.execute("DELETE FROM client_medications WHERE client_id = ?1", crate::params![client_id]).await.unwrap();
            conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id]).await.unwrap();
            conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![client_id]).await.unwrap();
            conn.execute("DELETE FROM audit_logs WHERE actor_id = ?1", crate::params![user_id]).await.unwrap();
        }
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_unassigned_client_access_control() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let assistant_id = "test-assistant-123";
        let client_id = "test-unassigned-client-123";
        let ws_id = "workspace-1";

        // Setup assistant and unassigned client profile
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'assistant@yntra.io', 'assistant')", crate::params![assistant_id, ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO clients (id, workspace_id, team_id, first_name, last_name, care_level, created_at, updated_at) VALUES (?1, ?2, NULL, 'John', 'Doe', 'Normal', '2026-07-05', 0)", crate::params![client_id, ws_id]).await.unwrap();

        // Get medications for unassigned client as assistant - must fail
        let res_meds = get_medications(client_id.to_string(), assistant_id.to_string()).await;
        assert!(res_meds.is_err());
        assert!(matches!(res_meds.unwrap_err(), YntraError::AuthError(_)));

        // Get journals for unassigned client as assistant - must fail
        let res_journals = get_journals(client_id.to_string(), assistant_id.to_string()).await;
        assert!(res_journals.is_err());
        assert!(matches!(res_journals.unwrap_err(), YntraError::AuthError(_)));

        // Clean up
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![assistant_id]).await.unwrap();
        conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![client_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_cross_tenant_admin_access_blocked() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let admin_id = "test-other-admin-123";
        let client_id = "test-tenant-client-123";

        // Setup admin in workspace-client-1 and client in workspace-client-2
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-client-1', 'WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-client-2', 'WS 2', '[]', '{}')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, 'workspace-client-1', 'admin@other.io', 'admin')", crate::params![admin_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO clients (id, workspace_id, team_id, first_name, last_name, care_level, created_at, updated_at) VALUES (?1, 'workspace-client-2', NULL, 'Jane', 'Doe', 'Normal', '2026-07-05', 0)", crate::params![client_id]).await.unwrap();

        // Admin of workspace-client-1 tries to read workspace-client-2 client medications - must fail
        let res_meds = get_medications(client_id.to_string(), admin_id.to_string()).await;
        assert!(res_meds.is_err());
        assert!(matches!(res_meds.unwrap_err(), YntraError::AuthError(_)));

        // Admin of workspace-client-1 tries to read workspace-client-2 client journals - must fail
        let res_journals = get_journals(client_id.to_string(), admin_id.to_string()).await;
        assert!(res_journals.is_err());
        assert!(matches!(res_journals.unwrap_err(), YntraError::AuthError(_)));

        // Clean up
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![admin_id]).await.unwrap();
        conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![client_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('workspace-client-1', 'workspace-client-2')", ()).await.unwrap();
    }
}
