use crate::database;
use crate::observer::notify_observers;
use crate::{WorkspaceUser, YntraError};
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use zeroize::Zeroize;

#[uniffi::export]
pub async fn get_user_by_email(email: String) -> Result<Option<WorkspaceUser>, YntraError> {
    let email_lower = email.trim().to_lowercase();
    let conn = database::acquire_connection().await?;
    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, siths_card_id, nfc_badge_uid, updated_at, sync_status, personal_number FROM users WHERE LOWER(email) = ?1",
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
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
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

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, siths_card_id, nfc_badge_uid, updated_at, sync_status, personal_number FROM users WHERE workspace_id = ?1",
    ).await?;

    let list = stmt.query_map(crate::params![&auth.workspace_id], |row| {
        let id: String = row.get(0)?;
        let ws_id: Option<String> = row.get(1)?;
        let raw_pnum: Option<String> = row.get(11)?;
        
        let is_self = id == requester_user_id;
        
        let personal_number = if auth.is_admin || is_self {
            crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id.as_deref().unwrap_or(""))
        } else {
            None
        };
        
        let siths_card_id = if auth.is_admin || is_self { row.get(7)? } else { None };
        let nfc_badge_uid = if auth.is_admin || is_self { row.get(8)? } else { None };

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
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
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
        return Err(YntraError::AuthError("Access denied: only administrators can change roles".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if auth.role != "platform_admin" && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute("UPDATE users SET role = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3", crate::params![role, now_ms, user_id]).await?;

    notify_observers();
    Ok(())
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

    if target_ws_id.is_some() && target_ws_id != Some(auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE users SET full_name = ?1, phone = ?2, role = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![full_name, phone, role, now_ms, user_id],
    ).await?;

    notify_observers();
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[allow(dead_code)]
fn hash_password_pbkdf2(password: &str, salt: &[u8]) -> String {
    let mut out_hash = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, 10_000, &mut out_hash);
    let result = format!("{}:{}", const_hex::encode(salt), const_hex::encode(&out_hash));
    out_hash.zeroize();
    result
}

fn verify_password_pbkdf2(password: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = stored_hash.split(':').collect();
    if parts.len() != 2 {
        return false;
    }

    let mut salt_bytes = match const_hex::decode(parts[0]) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let mut expected_hash_bytes = match const_hex::decode(parts[1]) {
        Ok(b) => b,
        Err(_) => {
            salt_bytes.zeroize();
            return false;
        }
    };

    let mut out_hash = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), &salt_bytes, 10_000, &mut out_hash);

    let res = constant_time_eq(&out_hash, &expected_hash_bytes);
    
    salt_bytes.zeroize();
    expected_hash_bytes.zeroize();
    out_hash.zeroize();
    
    res
}

fn hash_password_argon2(password: &str) -> Result<String, YntraError> {
    use argon2::{Algorithm, Version, Params};
    let salt = SaltString::generate(&mut OsRng);
    
    // Configure Argon2id: 19MB memory (19456 KB), 2 iterations, 1 thread
    let params = Params::new(19456, 2, 1, Some(32))
        .map_err(|e| YntraError::CryptoError(format!("Argon2 params invalid: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    argon2.hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| YntraError::CryptoError(format!("Argon2 hashing failed: {}", e)))
}

fn verify_password_argon2(password: &str, stored_hash: &str) -> bool {
    if !stored_hash.starts_with('$') {
        return verify_password_pbkdf2(password, stored_hash);
    }
    
    if let Ok(parsed_hash) = PasswordHash::new(stored_hash) {
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    } else {
        false
    }
}

#[uniffi::export]
pub async fn verify_email_password(email: String, password: String) -> Result<Option<WorkspaceUser>, YntraError> {
    let email_lower = email.trim().to_lowercase();
    let zeroizing_password = zeroize::Zeroizing::new(password);

    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, password_hash, siths_card_id, nfc_badge_uid, updated_at, sync_status, personal_number FROM users WHERE LOWER(email) = ?1",
    ).await?;

    let mut rows = stmt.query(crate::params![email_lower]).await?;
    if let Some(row) = rows.next().await? {
        let stored_hash: Option<String> = row.get(7)?;
        let auth_ok = match stored_hash {
            Some(h) => verify_password_argon2(&zeroizing_password, &h),
            None => false,
        };
        if !auth_ok {
            return Ok(None);
        }
        let ws_id: Option<String> = row.get(1)?;
        let raw_pnum: Option<String> = row.get(12)?;
        Ok(Some(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: row.get(6)?,
            siths_card_id: row.get(8)?,
            nfc_badge_uid: row.get(9)?,
            updated_at: row.get(10)?,
            sync_status: row.get(11)?,
            personal_number: crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id.as_deref().unwrap_or("")),
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn set_user_password(requester_user_id: String, user_id: String, password: String) -> Result<(), YntraError> {
    let zeroizing_password = zeroize::Zeroizing::new(password);
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let is_self = requester_user_id == user_id;

    if !is_self && !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: you can only change your own password or require administrator privileges".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if target_ws_id.is_some() && target_ws_id != Some(auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let hashed_res = hash_password_argon2(&zeroizing_password);
    let hashed = hashed_res?;
    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![hashed, now_ms, user_id],
    ).await?;

    notify_observers();
    Ok(())
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

    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;

    let res = async {
        // 1. Delete associated child relationships
        conn.execute("DELETE FROM team_members WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("DELETE FROM time_reports WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("DELETE FROM student_parents WHERE parent_user_id = ?1", crate::params![&user_id]).await?;
        
        // 2. Anonymize/Nullify references in other tables to preserve integrity
        conn.execute("UPDATE messages SET sender_id = NULL WHERE sender_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE messages SET receiver_id = NULL WHERE receiver_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE reports SET user_id = NULL WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE student_profiles SET user_id = NULL WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE courses SET teacher_id = NULL WHERE teacher_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE job_tickets SET assigned_user_id = NULL WHERE assigned_user_id = ?1", crate::params![&user_id]).await?;

        // 3. Delete user profile record
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![&user_id]).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.execute("COMMIT", ()).await?;
            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
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
        let hashed = hash_password_argon2(password).unwrap();
        assert!(hashed.starts_with("$argon2id$"));
        assert!(verify_password_argon2(password, &hashed));
        assert!(!verify_password_argon2("wrong_password", &hashed));
    }

    #[test]
    fn test_pbkdf2_fallback() {
        let password = "legacy_password_abc";
        let salt = b"salt123";
        let hashed_pbkdf2 = hash_password_pbkdf2(password, salt);
        assert!(verify_password_argon2(password, &hashed_pbkdf2));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_null_password_hash_auth_bypass_fixed() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        // Prepare database connection and create test user with NULL password hash
        let conn = database::acquire_connection().await.unwrap();
        let user_id = "test-bypass-user-123";
        let email = "bypass@yntra.io";
        
        // Insert user with NULL password_hash
        conn.execute(
            "INSERT OR REPLACE INTO users (id, email, password_hash, role) VALUES (?1, ?2, NULL, 'user')",
            crate::params![user_id, email],
        ).await.unwrap();

        // Attempt verification with some password - it must return Ok(None) (auth rejected)
        let res = verify_email_password(email.to_string(), "any_password".to_string()).await.unwrap();
        assert!(res.is_none());

        // Clean up
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
            "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role, personal_number, siths_card_id, nfc_badge_uid) VALUES (?1, 'workspace-1', ?2, NULL, 'user', '19850101-9999', 'card-123', 'badge-456')",
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
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, personal_number) VALUES (?1, 'workspace-1', 'user1@yntra.io', 'assistant', ?2)",
            crate::params![user1_id, enc_pnum.clone()],
        ).await.unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, personal_number) VALUES (?1, 'workspace-1', 'user2@yntra.io', 'assistant', ?2)",
            crate::params![user2_id, enc_pnum.clone()],
        ).await.unwrap();

        // Querying as user1
        let list = get_users(user1_id.to_string()).await.unwrap();
        
        // Find user1 in the returned list
        let self_user = list.iter().find(|u| u.id == user1_id).unwrap();
        assert_eq!(self_user.personal_number, Some(personal_number.to_string()));

        // Find user2 in the returned list (should be None since requester is not admin and it's not user2 themselves)
        let other_user = list.iter().find(|u| u.id == user2_id).unwrap();
        assert!(other_user.personal_number.is_none());

        // Clean up
        conn.execute("DELETE FROM users WHERE id IN (?1, ?2)", crate::params![user1_id, user2_id]).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }
}
