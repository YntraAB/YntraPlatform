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
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: only administrators can invite users".to_string()));
    }

    if auth.workspace_id != workspace_id {
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
        conn.begin_transaction().await?;

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
                conn.commit().await?;
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
                let _ = conn.rollback().await;
                Err(e)
            }
        }
    } else {
        Err(YntraError::InvitationError("Invalid invitation code".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_invite_user_via_directory_permissions() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspaces and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dir-1', 'Dir WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dir-2', 'Dir WS 2', '[]', '{}')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dir-admin', 'ws-dir-1', 'admin@dir.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dir-normal', 'ws-dir-1', 'normal@dir.io', 'user')", ()).await.unwrap();

        // 2. Successful invite by admin in same workspace
        let user = invite_user_via_directory(
            "u-dir-admin".to_string(),
            "ws-dir-1".to_string(),
            "invited@dir.io".to_string(),
            "Invited User".to_string(),
            "user".to_string(),
        ).await;
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.email, "invited@dir.io");

        // Check user inserted
        let inserted: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE email = 'invited@dir.io'",
            (),
            |r| r.get(0),
        ).await.unwrap_or(0);
        assert_eq!(inserted, 1);

        // 3. Blocked invite by non-admin
        let res_normal = invite_user_via_directory(
            "u-dir-normal".to_string(),
            "ws-dir-1".to_string(),
            "bad@dir.io".to_string(),
            "Bad User".to_string(),
            "user".to_string(),
        ).await;
        assert!(res_normal.is_err());
        assert!(matches!(res_normal.err().unwrap(), YntraError::AuthError(_)));

        // 4. Blocked invite by admin to another workspace
        let res_other_ws = invite_user_via_directory(
            "u-dir-admin".to_string(),
            "ws-dir-2".to_string(),
            "bad2@dir.io".to_string(),
            "Bad User 2".to_string(),
            "user".to_string(),
        ).await;
        assert!(res_other_ws.is_err());
        assert!(matches!(res_other_ws.err().unwrap(), YntraError::AuthError(_)));

        // Cleanup
        conn.execute("DELETE FROM users WHERE email IN ('invited@dir.io', 'admin@dir.io', 'normal@dir.io')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-dir-1', 'ws-dir-2')", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_activate_invitation_code_scenarios() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dir-inv', 'Inv WS', '[]', '{}')", ()).await.unwrap();

        // 1. Insert a mock invitation
        conn.execute(
            "INSERT OR REPLACE INTO invitations (code, workspace_id, email, full_name, role, activated, updated_at) VALUES ('CODE123', 'ws-dir-inv', 'guest@dir.io', 'Guest User', 'user', 0, 0)",
            (),
        ).await.unwrap();

        // 2. Activate valid code (case-insensitive checks)
        let res = activate_invitation_code("  code123  ".to_string()).await;
        assert!(res.is_ok());
        let user = res.unwrap();
        assert_eq!(user.email, "guest@dir.io");

        // Verify marked activated = 1 and user created in DB
        let activated: i64 = conn.query_row(
            "SELECT activated FROM invitations WHERE code = 'CODE123'",
            (),
            |r| r.get(0)
        ).await.unwrap();
        assert_eq!(activated, 1);

        let user_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE email = 'guest@dir.io'",
            (),
            |r| r.get(0)
        ).await.unwrap_or(0);
        assert_eq!(user_exists, 1);

        // 3. Try activating again (should fail)
        let res_retry = activate_invitation_code("CODE123".to_string()).await;
        assert!(res_retry.is_err());
        if let Err(YntraError::InvitationError(msg)) = res_retry {
            assert!(msg.contains("Invitation code already activated"));
        } else {
            panic!("Expected InvitationError");
        }

        // 4. Try activating invalid code
        let res_invalid = activate_invitation_code("INVALID_CODE".to_string()).await;
        assert!(res_invalid.is_err());
        if let Err(YntraError::InvitationError(msg)) = res_invalid {
            assert!(msg.contains("Invalid invitation code"));
        } else {
            panic!("Expected InvitationError");
        }

        // Cleanup
        conn.execute("DELETE FROM users WHERE email = 'guest@dir.io'", ()).await.unwrap();
        conn.execute("DELETE FROM invitations WHERE code = 'CODE123'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-dir-inv'", ()).await.unwrap();
    }
}
