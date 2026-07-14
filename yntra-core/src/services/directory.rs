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
        return Err(YntraError::AuthError(
            "Access denied: only administrators can invite users".to_string(),
        ));
    }

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: cannot invite user to another workspace".to_string(),
        ));
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
        public_key: None,
    };

    conn.begin_transaction().await?;
    let res = async {
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

        crate::services::users::ensure_user_role_signature(&conn, &item.id, &item.role, &workspace_id).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
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
pub async fn activate_invitation_code(code: String) -> Result<WorkspaceUser, YntraError> {
    let trimmed = code.trim();
    let parts: Vec<&str> = trimmed.split(':').collect();

    let (lookup_code, pk_opt) = if parts.len() == 2 {
        let lookup = parts[0].trim().to_uppercase();
        let pk = parts[1].trim().to_lowercase();
        // Validate pk is 64 hex chars (32 bytes)
        let is_valid_hex = pk.len() == 64 && const_hex::decode(&pk).is_ok();
        if is_valid_hex {
            (lookup, Some(pk))
        } else {
            (trimmed.to_uppercase(), None)
        }
    } else {
        (trimmed.to_uppercase(), None)
    };

    let conn = database::acquire_connection().await?;

    // 1. Fetch the invitation
    let mut stmt = conn.prepare(
        "SELECT code, workspace_id, email, full_name, role, activated, metadata, encrypted_workspace_key FROM invitations WHERE UPPER(code) = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![lookup_code.clone()]).await?;
    if let Some(row) = rows.next().await? {
        let invitation_code: String = row.get(0)?;
        let workspace_id: String = row.get(1)?;
        let email: String = row.get(2)?;
        let full_name: String = row.get(3)?;
        let role: String = row.get(4)?;
        let activated: i64 = row.get(5)?;
        let metadata_str: Option<String> = row.get(6)?;
        let enc_workspace_key: Option<String> = row.get(7)?;

        let mut siths_card_id = None;
        let mut nfc_badge_uid = None;
        let mut public_key = None;
        if let Some(ref m_str) = metadata_str {
            if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                siths_card_id = meta_val
                    .get("siths_card_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                nfc_badge_uid = meta_val
                    .get("nfc_badge_uid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                public_key = meta_val
                    .get("public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        meta_val
                            .get("siths_public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
            }
        }

        if activated != 0 {
            return Err(YntraError::InvitationError(
                "Invitation code already activated".to_string(),
            ));
        }

        let now_ms = crate::infra::time::get_current_time_ms();
        conn.begin_transaction().await?;

        let res = async {
            // Decrypt workspace key using invitation code and cache it locally
            if let Some(ref enc_key) = enc_workspace_key {
                #[cfg(not(target_arch = "wasm32"))]
                let dec_res = {
                    let lookup_code_clone = lookup_code.clone();
                    let enc_key_clone = enc_key.clone();
                    tokio::task::spawn_blocking(move || {
                        crate::infra::crypto::decrypt_workspace_key_with_password(&lookup_code_clone, &enc_key_clone)
                    }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())))
                };
                #[cfg(target_arch = "wasm32")]
                let dec_res = crate::infra::crypto::decrypt_workspace_key_with_password(&lookup_code, enc_key);

                let pk = dec_res?;
                let key_setting = format!("workspace_key_{}", workspace_id);
                crate::infra::crypto::set_local_secret(&key_setting, &const_hex::encode(&pk)).await?;

                crate::infra::crypto::set_session_key(pk);
            }

            // Write the out-of-band verified public key to the keyring if provided
            if let Some(ref pk) = pk_opt {
                let key_setting = format!("workspace_public_key_{}", workspace_id);
                crate::infra::crypto::set_local_secret(&key_setting, pk).await?;

                // Cache it in-memory
                let mut cache = crate::infra::crypto::get_auth_key_cache().write().unwrap_or_else(|e| e.into_inner());
                cache.insert(workspace_id.clone(), pk.clone());
            }

            // 2. Mark activated = 1
            conn.execute(
                "UPDATE invitations SET activated = 1 WHERE code = ?1",
                crate::params![invitation_code],
            ).await?;

            // 3. Create the user in `users` table
            let user_id = uuid::Uuid::new_v4().to_string();
            let mut metadata_map = serde_json::Map::new();
            if let Some(ref s_id) = siths_card_id {
                metadata_map.insert("siths_card_id".to_string(), serde_json::Value::String(s_id.clone()));
            }
            if let Some(ref n_uid) = nfc_badge_uid {
                metadata_map.insert("nfc_badge_uid".to_string(), serde_json::Value::String(n_uid.clone()));
            }
            if let Some(ref pk) = public_key {
                metadata_map.insert("public_key".to_string(), serde_json::Value::String(pk.clone()));
            }
            let metadata_json = serde_json::to_string(&metadata_map).unwrap_or_else(|_| "{}".to_string());

            conn.execute(
                "INSERT INTO users (id, workspace_id, email, full_name, role, metadata, preferences, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', ?7, 'pending')",
                crate::params![
                    &user_id,
                    &workspace_id,
                    &email,
                    &full_name,
                    &role,
                    &metadata_json,
                    &now_ms
                ],
            ).await?;

            crate::services::users::ensure_user_role_signature(&conn, &user_id, &role, &workspace_id).await?;
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
                    public_key,
                })
            }
            Err(e) => {
                let _ = conn.rollback().await;
                Err(e)
            }
        }
    } else {
        Err(YntraError::InvitationError(
            "Invalid invitation code".to_string(),
        ))
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
        )
        .await;
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.email, "invited@dir.io");

        // Check user inserted
        let inserted: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE email = 'invited@dir.io'",
                (),
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
        assert_eq!(inserted, 1);

        // 3. Blocked invite by non-admin
        let res_normal = invite_user_via_directory(
            "u-dir-normal".to_string(),
            "ws-dir-1".to_string(),
            "bad@dir.io".to_string(),
            "Bad User".to_string(),
            "user".to_string(),
        )
        .await;
        assert!(res_normal.is_err());
        assert!(matches!(
            res_normal.err().unwrap(),
            YntraError::AuthError(_)
        ));

        // 4. Blocked invite by admin to another workspace
        let res_other_ws = invite_user_via_directory(
            "u-dir-admin".to_string(),
            "ws-dir-2".to_string(),
            "bad2@dir.io".to_string(),
            "Bad User 2".to_string(),
            "user".to_string(),
        )
        .await;
        assert!(res_other_ws.is_err());
        assert!(matches!(
            res_other_ws.err().unwrap(),
            YntraError::AuthError(_)
        ));

        // Cleanup
        conn.execute(
            "DELETE FROM users WHERE email IN ('invited@dir.io', 'admin@dir.io', 'normal@dir.io')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id IN ('ws-dir-1', 'ws-dir-2')",
            (),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_activate_invitation_code_scenarios() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dir-inv', 'Inv WS', '[]', '{}')", ()).await.unwrap();

        // 1. Insert a mock invitation with encrypted workspace key
        let test_key = vec![0u8; 32];
        let enc_test_key =
            crate::infra::crypto::encrypt_workspace_key_with_password("CODE123", test_key).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO invitations (code, workspace_id, email, full_name, role, activated, updated_at, encrypted_workspace_key) VALUES ('CODE123', 'ws-dir-inv', 'guest@dir.io', 'Guest User', 'user', 0, 0, ?1)",
            crate::params![enc_test_key],
        ).await.unwrap();

        // 2. Activate valid code (case-insensitive checks)
        let res = activate_invitation_code("  code123  ".to_string()).await;
        assert!(res.is_ok());
        let user = res.unwrap();
        assert_eq!(user.email, "guest@dir.io");

        // Verify marked activated = 1 and user created in DB
        let activated: i64 = conn
            .query_row(
                "SELECT activated FROM invitations WHERE code = 'CODE123'",
                (),
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(activated, 1);

        let user_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE email = 'guest@dir.io'",
                (),
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
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

        // 5. Test composite invitation code activation with creator public key
        // Cleanup key cache before run to ensure clean state
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_ws-dir-inv", "").await;

        let test_key_2 = vec![0u8; 32];
        let enc_test_key_2 =
            crate::infra::crypto::encrypt_workspace_key_with_password("CODE456", test_key_2)
                .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO invitations (code, workspace_id, email, full_name, role, activated, updated_at, encrypted_workspace_key) VALUES ('CODE456', 'ws-dir-inv', 'guest2@dir.io', 'Guest User 2', 'user', 0, 0, ?1)",
            crate::params![enc_test_key_2],
        ).await.unwrap();

        let mock_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let composite_code = format!("CODE456:{}", mock_pk);
        let res_composite = activate_invitation_code(composite_code).await;
        assert!(res_composite.is_ok());

        // Verify public key is stored in the local keyring
        let saved_pk = crate::infra::crypto::get_local_secret("workspace_public_key_ws-dir-inv")
            .await
            .unwrap();
        assert_eq!(saved_pk.as_deref(), Some(mock_pk));

        // Cleanup
        conn.execute(
            "DELETE FROM users WHERE email IN ('guest@dir.io', 'guest2@dir.io')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM invitations WHERE code IN ('CODE123', 'CODE456')",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-dir-inv'", ())
            .await
            .unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_ws-dir-inv", "").await;
    }
}
