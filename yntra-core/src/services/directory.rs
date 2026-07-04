#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{WorkspaceUser, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn invite_user_via_directory(
    requester_user_id: String,
    workspace_id: String,
    email: String,
    name: String,
    role: String,
) -> Result<WorkspaceUser, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let requester_row: Option<(String, Option<String>)> = conn.query_row(
            "SELECT role, workspace_id FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.ok();

        let (req_role, req_ws_id) = match requester_row {
            Some((role, Some(ws_id))) => (role, ws_id),
            _ => return Err(YntraError::AuthError("Requester user not found".to_string())),
        };

        if req_role != "admin" && req_role != "platform_admin" {
            return Err(YntraError::AuthError("Access denied: only administrators can invite users".to_string()));
        }

        if req_ws_id != workspace_id {
            return Err(YntraError::AuthError("Access denied: cannot invite user to another workspace".to_string()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id);
        let req_role = requester.map(|u| u.role.as_str()).unwrap_or("user");
        let req_ws = requester.and_then(|u| u.workspace_id.clone());

        if req_role != "admin" && req_role != "platform_admin" {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }

        if req_ws != Some(workspace_id.clone()) {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = WorkspaceUser {
        id: id.clone(),
        workspace_id: Some(workspace_id.clone()),
        email,
        full_name: Some(name),
        phone: None,
        role,
        preferences: "{}".to_string(),
        siths_card_id: None,
        nfc_badge_uid: None,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
        personal_number: None,
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, full_name, role, preferences, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6, 'pending')",
            crate::params![
                &item.id,
                &item.workspace_id,
                &item.email,
                &item.full_name,
                &item.role,
                &item.updated_at
            ],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.users.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn activate_invitation_code(code: String) -> Result<WorkspaceUser, YntraError> {
    let code_upper = code.trim().to_uppercase();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        // 1. Fetch the invitation
        let mut stmt = conn.prepare(
            "SELECT code, workspace_id, email, full_name, role, activated, siths_card_id, nfc_badge_uid FROM invitations WHERE UPPER(code) = ?1"
        ).await?;

        let mut rows = stmt.query(crate::params![code_upper.clone()]).await?;
        if let Some(row) = rows.next().await? {
            let invitation_code: String = row.get(0)?;
            let workspace_id: String = row.get(1)?;
            let email: String = row.get(2)?;
            let full_name: String = row.get(3)?;
            let role: String = row.get(4)?;
            let activated: i64 = row.get(5)?;
            let siths_card_id: Option<String> = row.get(6)?;
            let nfc_badge_uid: Option<String> = row.get(7)?;

            if activated != 0 {
                return Err(YntraError::InvitationError("Invitation code already activated".to_string()));
            }

            let now_ms = crate::infra::time::get_current_time_ms();
            // 2. Mark activated = 1
            conn.execute(
                "UPDATE invitations SET activated = 1 WHERE code = ?1",
                crate::params![invitation_code],
            ).await?;

            // 3. Create the user in `users` table
            let user_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO users (id, workspace_id, email, full_name, role, siths_card_id, nfc_badge_uid, preferences, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}', ?8, 'pending')",
                crate::params![
                    &user_id,
                    &workspace_id,
                    &email,
                    &full_name,
                    &role,
                    &siths_card_id,
                    &nfc_badge_uid,
                    &now_ms
                ],
            ).await?;

            notify_observers();

            Ok(WorkspaceUser {
                id: user_id,
                workspace_id: Some(workspace_id),
                email,
                full_name: Some(full_name),
                phone: None,
                role,
                preferences: "{}".to_string(),
                siths_card_id,
                nfc_badge_uid,
                updated_at: now_ms,
                sync_status: "pending".to_string(),
                personal_number: None,
            })
        } else {
            Err(YntraError::InvitationError("Invalid invitation code".to_string()))
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let (email, full_name, role) = match code_upper.as_str() {
            "YNTRA-CARE-2026" => ("marie.andersson@yntra.se", "Marie Andersson", "assistant"),
            "WELCOME-OFFLINE-FIRST" => ("dev.user@yntra.se", "Dev User", "platform_admin"),
            _ => return Err(YntraError::InvitationError("Invalid invitation code".to_string())),
        };

        // Check if already in the users list
        let mut store = wasm_store::get_store().lock().unwrap();
        if store.users.iter().any(|u| u.email == email) {
            return Err(YntraError::InvitationError("Invitation code already activated".to_string()));
        }

        let user_id = uuid::Uuid::new_v4().to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        let new_user = WorkspaceUser {
            id: user_id,
            workspace_id: Some("workspace-1".to_string()),
            email: email.to_string(),
            full_name: Some(full_name.to_string()),
            phone: None,
            role: role.to_string(),
            preferences: "{}".to_string(),
            siths_card_id: match code_upper.as_str() {
                "YNTRA-CARE-2026" => Some("SITHS-BOB-456".to_string()),
                "WELCOME-OFFLINE-FIRST" => Some("SITHS-ALICE-123".to_string()),
                _ => None,
            },
            nfc_badge_uid: match code_upper.as_str() {
                "YNTRA-CARE-2026" => Some("NFC-BOB-888".to_string()),
                "WELCOME-OFFLINE-FIRST" => Some("NFC-ALICE-999".to_string()),
                _ => None,
            },
            updated_at: now_ms,
            sync_status: "pending".to_string(),
            personal_number: match code_upper.as_str() {
                "YNTRA-CARE-2026" => Some("198506121234".to_string()),
                "WELCOME-OFFLINE-FIRST" => Some("199001015678".to_string()),
                _ => None,
            },
        };

        store.users.push(new_user.clone());
        notify_observers();

        Ok(new_user)
    }
}
