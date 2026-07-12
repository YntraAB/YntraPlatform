pub mod password;
pub mod signatures;

pub use password::*;
pub use signatures::*;

use crate::database;
use crate::observer::notify_observers;
use crate::{WorkspaceUser, YntraError};

#[uniffi::export]
pub async fn get_user_by_email(email: String) -> Result<Option<WorkspaceUser>, YntraError> {
    let email_lower = email.trim().to_lowercase();
    let conn = database::acquire_connection().await?;
    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, updated_at, sync_status FROM users WHERE LOWER(email) = ?1",
    ).await?;

    let mut rows = stmt.query(crate::params![email_lower]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id: Option<String> = row.get(1)?;
        Ok(Some(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: row.get(6)?,
            siths_card_id: None,
            nfc_badge_uid: None,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
            personal_number: None,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn get_users(requester_user_id: String) -> Result<Vec<WorkspaceUser>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let cipher = crate::infra::crypto::WorkspaceCipher::new(&auth.workspace_id)?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, metadata, updated_at, sync_status FROM users WHERE workspace_id = ?1",
    ).await?;

    let list = stmt.query_map(crate::params![&auth.workspace_id], |row| {
        let id: String = row.get(0)?;
        let ws_id: Option<String> = row.get(1)?;
        let metadata_str: Option<String> = row.get(7)?;
        
        let is_self = id == requester_user_id;
        
        let mut siths_card_id = None;
        let mut nfc_badge_uid = None;
        let mut personal_number = None;

        if auth.is_admin || is_self {
            if let Some(ref m_str) = metadata_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                    siths_card_id = meta_val.get("siths_card_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nfc_badge_uid = meta_val.get("nfc_badge_uid").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let raw_pnum = meta_val.get("personal_number").and_then(|v| v.as_str()).map(|s| s.to_string());
                    if raw_pnum.is_some() {
                        personal_number = cipher.decrypt_opt(raw_pnum);
                    }
                }
            }
        }

        Ok(WorkspaceUser {
            id,
            workspace_id: ws_id,
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: row.get(6)?,
            siths_card_id,
            nfc_badge_uid,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
            personal_number,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn update_user_role(requester_user_id: String, user_id: String, role: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "admin" && auth.role != "platform_admin" {
        let is_dev_self_change = cfg!(debug_assertions) && requester_user_id == user_id;
        if !is_dev_self_change {
            return Err(YntraError::AuthError("Access denied: only administrators can change roles".to_string()));
        }
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if auth.role != "platform_admin" && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let ws_id = target_ws_id.unwrap_or_else(|| auth.workspace_id.clone());
    let now_ms = crate::infra::time::get_current_time_ms();
    
    conn.begin_transaction().await?;
    let res = async {
        conn.execute("UPDATE users SET role = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3", crate::params![&role, now_ms, &user_id]).await?;
        signatures::ensure_user_role_signature(&conn, &user_id, &role, &ws_id).await?;
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
pub async fn update_user_profile(
    requester_user_id: String,
    user_id: String,
    full_name: Option<String>,
    phone: Option<String>,
    preferences: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let is_self = requester_user_id == user_id;

    if !is_self && !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: you can only update your own profile or require administrator privileges".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if target_ws_id.is_some() && target_ws_id != Some(auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE users SET full_name = ?1, phone = ?2, preferences = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![full_name, phone, preferences, now_ms, user_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_user_via_directory(
    requester_user_id: String,
    user_id: String,
    full_name: Option<String>,
    phone: Option<String>,
    role: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: only administrators can edit users via directory".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if target_ws_id.is_some() && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let ws_id = target_ws_id.unwrap_or_else(|| auth.workspace_id.clone());
    let now_ms = crate::infra::time::get_current_time_ms();
    
    conn.begin_transaction().await?;
    let res = async {
        conn.execute(
            "UPDATE users SET full_name = ?1, phone = ?2, role = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
            crate::params![full_name, phone, role, now_ms, user_id],
        ).await?;
        signatures::ensure_user_role_signature(&conn, &user_id, &role, &ws_id).await?;
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
pub async fn delete_user(requester_user_id: String, user_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "admin" && auth.role != "platform_admin" {
        return Err(YntraError::AuthError("Access denied: only administrators can delete users".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if auth.role != "platform_admin" && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    conn.begin_transaction().await?;

    let res = async {
        conn.execute("DELETE FROM team_members WHERE user_id = ?1", crate::params![&user_id]).await?;
        
        conn.execute("UPDATE messages SET sender_id = NULL WHERE sender_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE messages SET receiver_id = NULL WHERE receiver_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE reports SET user_id = NULL WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE job_tickets SET assigned_user_id = NULL WHERE assigned_user_id = ?1", crate::params![&user_id]).await?;
        
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute("UPDATE time_reports SET note = NULL, sync_status = 'pending', updated_at = ?1 WHERE user_id = ?2", crate::params![now_ms, &user_id]).await?;

        let anon_email = format!("deleted-{}@yntra-deleted.invalid", &user_id[..8.min(user_id.len())]);
        conn.execute(
            "UPDATE users 
             SET email = ?1, 
                 full_name = 'Deleted User', 
                 phone = NULL, 
                 password_hash = NULL, 
                 metadata = '{}', 
                 role = 'deleted', 
                 role_signature = NULL, 
                 updated_at = ?2, 
                 sync_status = 'pending' 
             WHERE id = ?3",
            crate::params![anon_email, now_ms, user_id],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2_password_hashing() {
        let password = "super_secure_password_123";
        let hashed = password::hash_password_argon2(password).unwrap();
        assert!(hashed.starts_with("$argon2id$"));
        assert!(password::verify_password_argon2(password, &hashed));
        assert!(!password::verify_password_argon2("wrong_password", &hashed));
    }


    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_null_password_hash_auth_bypass_fixed() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        let user_id = "test-bypass-user-123";
        let email = "bypass@yntra.io";
        
        conn.execute(
            "INSERT OR REPLACE INTO users (id, email, password_hash, role) VALUES (?1, ?2, NULL, 'user')",
            crate::params![user_id, email],
        ).await.unwrap();

        let res = verify_email_password(email.to_string(), "any_password".to_string()).await.unwrap();
        assert!(res.is_none());

        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id]).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_user_by_email_excludes_sensitive_fields() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        let user_id = "test-sensitive-user-123";
        let email = "sensitive@yntra.io";
        
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role, metadata) VALUES (?1, 'workspace-1', ?2, NULL, 'user', '{\"personal_number\":\"19850101-9999\",\"siths_card_id\":\"card-123\",\"nfc_badge_uid\":\"badge-456\"}')",
            crate::params![user_id, email],
        ).await.unwrap();

        let res = get_user_by_email(email.to_string()).await.unwrap().unwrap();
        assert_eq!(res.id, user_id);
        assert_eq!(res.email, email);
        assert!(res.personal_number.is_none());
        assert!(res.siths_card_id.is_none());
        assert!(res.nfc_badge_uid.is_none());

        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id]).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_users_self_decryption() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key".to_string().into_bytes());

        let conn = database::acquire_connection().await.unwrap();
        let user1_id = "test-self-user-1";
        let user2_id = "test-self-user-2";
        let personal_number = "19900101-1234";

        let enc_pnum = crate::infra::crypto::encrypt_opt_field(Some(personal_number.to_string()), "workspace-1").unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES (?1, 'workspace-1', 'user1@yntra.io', 'assistant', json_object('personal_number', ?2))",
            crate::params![user1_id, enc_pnum.clone()],
        ).await.unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES (?1, 'workspace-1', 'user2@yntra.io', 'assistant', json_object('personal_number', ?2))",
            crate::params![user2_id, enc_pnum.clone()],
        ).await.unwrap();

        let list = get_users(user1_id.to_string()).await.unwrap();
        
        let self_user = list.iter().find(|u| u.id == user1_id).unwrap();
        assert_eq!(self_user.personal_number, Some(personal_number.to_string()));

        let other_user = list.iter().find(|u| u.id == user2_id).unwrap();
        assert!(other_user.personal_number.is_none());

        conn.execute("DELETE FROM users WHERE id IN (?1, ?2)", crate::params![user1_id, user2_id]).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_update_user_role() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        
        let ws_id = format!("ws-update-{}", uuid::Uuid::new_v4());
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Update WS', '[]', '{}')", crate::params![&ws_id]).await.unwrap();
        
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-admin', ?1, 'admin@yntra.io', 'platform_admin')", crate::params![&ws_id]).await.unwrap();
        
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-target', ?1, 'target@yntra.io', 'user')", crate::params![&ws_id]).await.unwrap();

        let res = update_user_role("test-admin".to_string(), "test-target".to_string(), "assistant".to_string()).await;
        assert!(res.is_ok());

        let role: String = conn.query_row("SELECT role FROM users WHERE id = 'test-target'", (), |r| r.get(0)).await.unwrap();
        assert_eq!(role, "assistant");

        conn.execute("DELETE FROM users WHERE id IN ('test-admin', 'test-target')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![&ws_id]).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_ensure_role_signature_generation() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let _ = conn.execute("DELETE FROM users WHERE id IN ('admin-1', 'target-1')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await;
        let _ = crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", "").await;
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_ws-sig-test", "").await;

        let keys = crate::infra::crypto::generate_workspace_keypair().unwrap();
        let pub_hex = &keys[0];
        let priv_hex = &keys[1];

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, creator_public_key, modules_active, settings) VALUES ('ws-sig-test', 'Sig Test WS', ?1, '[]', '{}')", crate::params![pub_hex]).await.unwrap();

        crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", priv_hex).await.unwrap();

        let admin_sig = crate::infra::crypto::generate_role_signature(priv_hex, "admin-1", "platform_admin", "ws-sig-test").unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('admin-1', 'ws-sig-test', 'admin@sig.io', 'platform_admin', ?1)", crate::params![&admin_sig]).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('target-1', 'ws-sig-test', 'target@sig.io', 'user')", ()).await.unwrap();

        let res = update_user_role("admin-1".to_string(), "target-1".to_string(), "assistant".to_string()).await;
        assert!(res.is_ok());

        let pk: String = conn.query_row("SELECT creator_public_key FROM workspaces WHERE id = 'ws-sig-test'", (), |r| r.get(0)).await.unwrap();
        assert!(!pk.trim().is_empty());

        let sig: String = conn.query_row("SELECT role_signature FROM users WHERE id = 'target-1'", (), |r| r.get(0)).await.unwrap();
        assert!(!sig.trim().is_empty());

        let is_valid = crate::infra::crypto::verify_role_signature(&pk, "target-1", "assistant", "ws-sig-test", &sig);
        assert!(is_valid);

        let res = update_user_role("admin-1".to_string(), "target-1".to_string(), "user".to_string()).await;
        assert!(res.is_ok());

        let sig_after: Option<String> = conn.query_row("SELECT role_signature FROM users WHERE id = 'target-1'", (), |r| Ok(r.get(0)?)).await.unwrap();
        assert!(sig_after.is_some());
        let sig_after_val = sig_after.unwrap();
        assert!(!sig_after_val.trim().is_empty());
        assert!(crate::infra::crypto::verify_role_signature(&pk, "target-1", "user", "ws-sig-test", &sig_after_val));

        let res = update_user_role("admin-1".to_string(), "target-1".to_string(), "deleted".to_string()).await;
        assert!(res.is_ok());

        let sig_deleted: Option<String> = conn.query_row("SELECT role_signature FROM users WHERE id = 'target-1'", (), |r| Ok(r.get(0)?)).await.unwrap();
        assert!(sig_deleted.is_none());

        conn.execute("DELETE FROM users WHERE id IN ('admin-1', 'target-1')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", "").await;
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_ws-sig-test", "").await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_eventual_consistency_role_signing() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-eventual-1";
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await;
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "").await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Eventual WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('admin-eventual', ?1, 'admin@eventual.io', 'admin')", crate::params![ws_id]).await.unwrap();
        signatures::ensure_user_role_signature(&conn, "admin-eventual", "admin", ws_id).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-eventual', ?1, 'user@eventual.io', 'admin', NULL)", crate::params![ws_id]).await.unwrap();

        reconcile_role_signatures("admin-eventual".to_string()).await.unwrap();

        let sig: Option<String> = conn.query_row("SELECT role_signature FROM users WHERE id = 'user-eventual'", (), |r| Ok(r.get(0)?)).await.unwrap();
        assert!(sig.is_some());
        assert!(!sig.unwrap().is_empty());

        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "").await;
    }
}
