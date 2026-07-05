use crate::database;
use crate::observer::notify_observers;
use crate::{WorkspaceUser, YntraError};

#[uniffi::export]
pub async fn invite_user_via_directory(
    requester_user_id: String,
    workspace_id: String,
    email: String,
    name: String,
    role: String,
) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;
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

    Ok(item)
}

#[uniffi::export]
pub async fn activate_invitation_code(code: String) -> Result<WorkspaceUser, YntraError> {
    let code_upper = code.trim().to_uppercase();

    let conn = database::acquire_connection().await?;

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
        conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;

        let res = async {
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
            Ok(user_id)
        }.await;

        match res {
            Ok(user_id) => {
                conn.execute("COMMIT", ()).await?;
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
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    } else {
        Err(YntraError::InvitationError("Invalid invitation code".to_string()))
    }
}
