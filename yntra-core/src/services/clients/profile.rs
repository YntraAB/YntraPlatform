use super::pnum::personal_numbers_match;
use crate::database;
use crate::observer::notify_observers;
use crate::{ClientProfile, JournalEntry, MedicationItem, YntraError};

#[uniffi::export]
pub async fn get_clients(requester_user_id: String) -> Result<Vec<ClientProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let personal_number: Option<String> = conn
        .query_row(
            "SELECT metadata ->> 'personal_number' FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| r.get(0),
        )
        .await
        .ok()
        .flatten();

    let decrypted_user_pnum = if auth.role == "client" {
        if let Some(ref pn) = personal_number {
            crate::infra::crypto::decrypt_field(pn, &auth.workspace_id).ok()
        } else {
            None
        }
    } else {
        None
    };

    let (query, params) = if auth.role == "platform_admin" {
        (
            "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients".to_string(),
            vec![],
        )
    } else if auth.role == "admin" {
        (
            "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients WHERE workspace_id = ?1".to_string(),
            vec![auth.workspace_id.clone()],
        )
    } else if auth.role == "client" {
        (
            "SELECT id, workspace_id, team_id, first_name, last_name, personal_number, care_level, message_settings, created_at, updated_at, sync_status FROM clients WHERE workspace_id = ?1".to_string(),
            vec![auth.workspace_id.clone()],
        )
    } else {
        (
            "SELECT c.id, c.workspace_id, c.team_id, c.first_name, c.last_name, c.personal_number, c.care_level, c.message_settings, c.created_at, c.updated_at, c.sync_status
             FROM clients c
             JOIN team_members tm ON c.team_id = tm.team_id
             WHERE tm.user_id = ?1 AND c.workspace_id = ?2".to_string(),
            vec![requester_user_id.clone(), auth.workspace_id.clone()],
        )
    };

    let mut cached_ciphers: std::collections::HashMap<
        String,
        crate::infra::crypto::WorkspaceCipher,
    > = std::collections::HashMap::new();

    let mut stmt = conn.prepare(&query).await?;
    let list = stmt
        .query_map(crate::rusqlite::params_from_iter(params), |row| {
            let ws_id: String = row.get(1)?;
            let raw_pnum: Option<String> = row.get(5)?;

            if !cached_ciphers.contains_key(&ws_id) {
                if let Ok(c) = crate::infra::crypto::WorkspaceCipher::new(&ws_id) {
                    cached_ciphers.insert(ws_id.clone(), c);
                }
            }

            let decrypted_pnum = cached_ciphers
                .get(&ws_id)
                .and_then(|c| c.decrypt_opt(raw_pnum));

            Ok(ClientProfile {
                id: row.get(0)?,
                workspace_id: ws_id.clone(),
                team_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                personal_number: decrypted_pnum,
                care_level: row.get(6)?,
                message_settings: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                sync_status: row.get(10)?,
            })
        })
        .await?;

    let list = if auth.role == "client" {
        if let Some(target_pnum) = decrypted_user_pnum {
            list.into_iter()
                .filter(|c| {
                    if let Some(ref c_pnum) = c.personal_number {
                        personal_numbers_match(c_pnum, &target_pnum)
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            vec![]
        }
    } else {
        list
    };

    Ok(list)
}

#[uniffi::export]
pub async fn add_client_via_directory(
    requester_user_id: String,
    workspace_id: String,
    team_id: Option<String>,
    first_name: String,
    last_name: String,
    personal_number: String,
    care_level: String,
) -> Result<ClientProfile, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

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

    let encrypted_pnum =
        crate::infra::crypto::encrypt_opt_field(item.personal_number.clone(), &workspace_id)?;

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

    Ok(item)
}

#[uniffi::export]
pub async fn update_client_profile(
    requester_user_id: String,
    client_id: String,
    first_name: String,
    last_name: String,
    personal_number: Option<String>,
    care_level: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }
    let encrypted_pnum = crate::infra::crypto::encrypt_opt_field(personal_number, &ws_id)?;

    conn.execute(
        "UPDATE clients SET first_name = ?1, last_name = ?2, personal_number = ?3, care_level = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
        crate::params![first_name, last_name, encrypted_pnum, care_level, now_ms, client_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_client(requester_user_id: String, client_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.begin_transaction().await?;

    let res = async {
        // 1. Delete associated medication items
        conn.execute(
            "DELETE FROM client_medications WHERE client_id = ?1",
            crate::params![&client_id],
        )
        .await?;

        // 2. Delete associated journal entries
        conn.execute(
            "DELETE FROM client_journals WHERE client_id = ?1",
            crate::params![&client_id],
        )
        .await?;

        // 3. Delete client profile record
        conn.execute(
            "DELETE FROM clients WHERE id = ?1",
            crate::params![&client_id],
        )
        .await?;
        Ok(())
    }
    .await;

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
pub async fn get_clients_rkyv(requester_user_id: String) -> Result<Vec<u8>, YntraError> {
    let clients = get_clients(requester_user_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&clients)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[uniffi::export]
pub async fn check_care_permission(
    requester_user_id: String,
    permission_name: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    Ok(has_care_permission(&auth, &permission_name))
}

fn has_care_permission(auth: &crate::AuthContext, permission_name: &str) -> bool {
    if auth.role == "platform_admin"
        || auth.role == "admin"
        || auth.role == "role-care-manager"
        || auth.role == "role-hvb-director"
        || auth.role == "role-lss-coordinator"
    {
        return true;
    }
    if let Some(ref settings_str) = auth.workspace_settings {
        if let Ok(settings_val) = serde_json::from_str::<serde_json::Value>(settings_str) {
            if let Some(roles_arr) = settings_val.get("roles").and_then(|r| r.as_array()) {
                for r in roles_arr {
                    let r_id = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let r_name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let is_match = r_id == auth.role
                        || r_id.ends_with(&format!("-{}", auth.role))
                        || r_id.strip_prefix("role-").map(|s| s == auth.role).unwrap_or(false)
                        || r_id.strip_prefix("role-care-").map(|s| s == auth.role).unwrap_or(false)
                        || r_id.strip_prefix("role-hvb-").map(|s| s == auth.role).unwrap_or(false)
                        || r_id.strip_prefix("role-lss-").map(|s| s == auth.role).unwrap_or(false)
                        || r_name.to_lowercase() == auth.role.to_lowercase();
                    if is_match {
                        if let Some(permissions) = r.get("permissions") {
                            if let Some(val) = permissions.get(permission_name).and_then(|v| v.as_bool()) {
                                return val;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn verify_care_permission(auth: &crate::AuthContext, permission_name: &str) -> Result<(), YntraError> {
    if has_care_permission(auth, permission_name) {
        Ok(())
    } else {
        Err(YntraError::AuthError(format!(
            "Access denied: role '{}' does not have permission '{}'",
            auth.role, permission_name
        )))
    }
}

#[uniffi::export]
pub async fn get_journals(
    client_id: String,
    actor_id: String,
) -> Result<Vec<JournalEntry>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    verify_care_permission(&auth, "can_view_journals")?;

    let client_ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if auth.role != "platform_admin" && client_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, client_id, workspace_id, content, author_id, created_at, updated_at, sync_status \
                  FROM client_journals WHERE client_id = ?1")
        .await?;
    let mut rows = stmt.query(crate::params![&client_id]).await?;
    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        let created_at: Option<String> = row.get(5)?;
        list.push(JournalEntry {
            id: row.get(0)?,
            client_id: row.get(1)?,
            workspace_id: row.get(2)?,
            content: row.get(3)?,
            author_id: row.get(4)?,
            created_at: created_at.unwrap_or_default(),
            updated_at: row.get(6)?,
            sync_status: row.get(7)?,
        });
    }
    Ok(list)
}

#[uniffi::export]
pub async fn add_journal_entry(
    actor_id: String,
    client_id: String,
    content: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    verify_care_permission(&auth, "can_write_journals")?;

    let client_ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if auth.role != "platform_admin" && client_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "INSERT INTO client_journals (id, client_id, workspace_id, content, author_id, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
        crate::params![
            &id,
            &client_id,
            &client_ws_id,
            &content,
            Some(&auth.user_id),
            &created_at,
            now_ms
        ],
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_medications(
    client_id: String,
    actor_id: String,
) -> Result<Vec<MedicationItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    verify_care_permission(&auth, "can_view_medications")?;

    let client_ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if auth.role != "platform_admin" && client_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, client_id, workspace_id, name, dosage, frequency, instructions, updated_at, sync_status \
                  FROM client_medications WHERE client_id = ?1")
        .await?;
    let mut rows = stmt.query(crate::params![&client_id]).await?;
    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        list.push(MedicationItem {
            id: row.get(0)?,
            client_id: row.get(1)?,
            workspace_id: row.get(2)?,
            name: row.get(3)?,
            dosage: row.get(4)?,
            frequency: row.get(5)?,
            instructions: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        });
    }
    Ok(list)
}

#[uniffi::export]
pub async fn add_medication(
    actor_id: String,
    client_id: String,
    name: String,
    dosage: String,
    frequency: String,
    instructions: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &actor_id).await?;
    verify_care_permission(&auth, "can_manage_medications")?;

    let client_ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM clients WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&client_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("Client not found".to_string()));
        }
    };
    if auth.role != "platform_admin" && client_ws_id != auth.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "INSERT INTO client_medications (id, client_id, workspace_id, name, dosage, frequency, instructions, updated_at, sync_status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &id,
            &client_id,
            &client_ws_id,
            &name,
            if dosage.is_empty() { None } else { Some(dosage) },
            if frequency.is_empty() { None } else { Some(frequency) },
            if instructions.is_empty() { None } else { Some(instructions) },
            now_ms
        ],
    ).await?;
    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_clients_probabilistic_encryption_match() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key".to_string().into_bytes(), "workspace-1".to_string());

        let conn = database::acquire_connection().await.unwrap();
        let user_id = "test-client-user-999";
        let email = "client-user@yntra.se";
        let ws_id = "workspace-1";
        let personal_number = "19900101-1234";

        // Insert the client user with encrypted personal number
        let enc_user_pnum =
            crate::infra::crypto::encrypt_opt_field(Some(personal_number.to_string()), ws_id)
                .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role, metadata) VALUES (?1, ?2, ?3, NULL, 'client', json_object('personal_number', ?4))",
            crate::params![user_id, ws_id, email, enc_user_pnum],
        ).await.unwrap();

        // Insert the client profile with encrypted personal number (random nonce generates different ciphertext)
        let client_id = "client-profile-999";
        let enc_client_pnum =
            crate::infra::crypto::encrypt_opt_field(Some(personal_number.to_string()), ws_id)
                .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, personal_number, care_level, created_at, updated_at) VALUES (?1, ?2, 'Alice', 'Smith', ?3, 'Normal', '2026-07-05', 0)",
            crate::params![client_id, ws_id, enc_client_pnum],
        ).await.unwrap();

        // Retrieve the client profile as the logged-in client user
        let clients_list = get_clients(user_id.to_string()).await.unwrap();
        assert_eq!(clients_list.len(), 1);
        assert_eq!(clients_list[0].id, client_id);
        assert_eq!(clients_list[0].first_name, "Alice");
        assert_eq!(
            clients_list[0].personal_number,
            Some(personal_number.to_string())
        );

        // Retrieve the client profile as the logged-in client user with rkyv
        let bytes = get_clients_rkyv(user_id.to_string()).await.unwrap();
        let rkyv_clients: Vec<ClientProfile> =
            rkyv::from_bytes::<Vec<ClientProfile>, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(rkyv_clients.len(), 1);
        assert_eq!(rkyv_clients[0].id, client_id);

        // Clean up
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id])
            .await
            .unwrap();
        conn.execute(
            "DELETE FROM clients WHERE id = ?1",
            crate::params![client_id],
        )
        .await
        .unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_delete_client_works_without_errors() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key-delete".to_string().into_bytes(), "workspace-delete-test".to_string());

        let conn = database::acquire_connection().await.unwrap();
        let requester_user_id = "test-admin-user-delete";
        let ws_id = "workspace-delete-test";

        // Create workspace and admin user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Delete Workspace', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'admin-delete@yntra.se', 'admin')",
            crate::params![requester_user_id, ws_id],
        ).await.unwrap();

        // Create a client
        let client_id = "client-to-delete-123";
        conn.execute(
            "INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, created_at, updated_at) VALUES (?1, ?2, 'Bob', 'Jones', '2026-07-05', 0)",
            crate::params![client_id, ws_id],
        ).await.unwrap();

        // Insert dummy records for client medications and journals to verify foreign keys and deletions
        conn.execute(
            "INSERT OR REPLACE INTO client_medications (id, client_id, workspace_id, name, updated_at) VALUES ('med-1', ?1, ?2, 'Aspirin', 0)",
            crate::params![client_id, ws_id],
        ).await.unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO client_journals (id, client_id, workspace_id, content, updated_at) VALUES ('journal-1', ?1, ?2, 'Patient felt good', 0)",
            crate::params![client_id, ws_id],
        ).await.unwrap();

        // Perform the deletion
        let delete_res = delete_client(requester_user_id.to_string(), client_id.to_string()).await;
        assert!(
            delete_res.is_ok(),
            "delete_client failed: {:?}",
            delete_res.err()
        );

        // Verify the client, medications, and journals are indeed gone
        let client_exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM clients WHERE id = ?1",
                crate::params![client_id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(client_exists, 0);

        let meds_exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM client_medications WHERE client_id = ?1",
                crate::params![client_id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(meds_exists, 0);

        let journals_exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM client_journals WHERE client_id = ?1",
                crate::params![client_id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(journals_exists, 0);

        // Cleanup workspace and admin
        conn.execute(
            "DELETE FROM users WHERE id = ?1",
            crate::params![requester_user_id],
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id = ?1",
            crate::params![ws_id],
        )
        .await
        .unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_care_permissions_enforcement() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key-care".to_string().into_bytes(), "workspace-care-test".to_string());

        let conn = database::acquire_connection().await.unwrap();
        let ws_id = "workspace-care-test";

        // Insert workspace with template config having role-care-caregiver (can_view_journals=true, can_write_journals=true, can_manage_medications=false)
        let settings = r#"{
            "template": "care",
            "roles": [
                {
                    "id": "role-care-caregiver",
                    "name": "Caregiver",
                    "permissions": {
                        "can_view_journals": true,
                        "can_write_journals": true,
                        "can_view_medications": true,
                        "can_manage_medications": false
                    }
                }
            ]
        }"#;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Care Workspace', '[]', ?2)", crate::params![ws_id, settings]).await.unwrap();

        // Create caregiver user
        let caregiver_id = "user-caregiver-1";
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'caregiver@yntra.se', 'role-care-caregiver')",
            crate::params![caregiver_id, ws_id],
        ).await.unwrap();

        // Create a client
        let client_id = "client-care-test-1";
        conn.execute(
            "INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, created_at, updated_at) VALUES (?1, ?2, 'Robert', 'Downey', '2026-07-05', 0)",
            crate::params![client_id, ws_id],
        ).await.unwrap();

        // 1. Caregiver tries to add a journal entry -> should succeed
        let journal_res = add_journal_entry(caregiver_id.to_string(), client_id.to_string(), "Client slept well".to_string()).await;
        assert!(journal_res.is_ok());

        // Verify journal entry exists
        let journals = get_journals(client_id.to_string(), caregiver_id.to_string()).await.unwrap();
        assert_eq!(journals.len(), 1);
        assert_eq!(journals[0].content, "Client slept well");
        assert_eq!(journals[0].author_id, Some(caregiver_id.to_string()));

        // 2. Caregiver tries to add a medication -> should fail (can_manage_medications = false)
        let med_res = add_medication(
            caregiver_id.to_string(),
            client_id.to_string(),
            "Aspirin".to_string(),
            "100mg".to_string(),
            "Daily".to_string(),
            "Take with water".to_string(),
        ).await;
        assert!(med_res.is_err());
        if let Err(YntraError::AuthError(msg)) = med_res {
            assert!(msg.contains("does not have permission 'can_manage_medications'"));
        } else {
            panic!("Expected AuthError, got {:?}", med_res);
        }

        // Clean up
        conn.execute("DELETE FROM client_journals WHERE client_id = ?1", crate::params![client_id]).await.unwrap();
        conn.execute("DELETE FROM clients WHERE id = ?1", crate::params![client_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![caregiver_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }
}
