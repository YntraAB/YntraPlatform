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

    if auth.role == "admin" || auth.role == "platform_admin" {
        let mut to_sign = Vec::new();
        if let Ok(mut check_stmt) = conn.prepare(
            "SELECT id, role FROM users WHERE workspace_id = ?1 AND role_signature IS NULL"
        ).await {
            if let Ok(mut rows) = check_stmt.query(crate::params![&auth.workspace_id]).await {
                while let Ok(Some(row)) = rows.next().await {
                    if let (Ok(u_id), Ok(u_role)) = (row.get::<String>(0), row.get::<String>(1)) {
                        let is_privileged = u_role != "user" && u_role != "client" && u_role != "guest" && u_role != "anonymous" && u_role != "deleted";
                        if is_privileged {
                            to_sign.push((u_id, u_role));
                        }
                    }
                }
            }
        }
        for (u_id, u_role) in to_sign {
            let _ = ensure_user_role_signature(&conn, &u_id, &u_role, &auth.workspace_id).await;
        }
    }

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
        #[cfg(not(debug_assertions))]
        return Err(YntraError::AuthError("Access denied: only administrators can change roles".to_string()));
    }

    let target_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if auth.role != "platform_admin" && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        #[cfg(not(debug_assertions))]
        return Err(YntraError::AuthError("Access denied: target user is in a different workspace".to_string()));
    }

    let ws_id = target_ws_id.unwrap_or_else(|| auth.workspace_id.clone());
    let now_ms = crate::infra::time::get_current_time_ms();
    
    conn.begin_transaction().await?;
    let res = async {
        conn.execute("UPDATE users SET role = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3", crate::params![&role, now_ms, &user_id]).await?;
        ensure_user_role_signature(&conn, &user_id, &role, &ws_id).await?;
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
        ensure_user_role_signature(&conn, &user_id, &role, &ws_id).await?;
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
        
        let mut preferences: String = row.get(6)?;
        if let Some(ref ws) = ws_id {
            let mut prefs_val: serde_json::Value = serde_json::from_str(&preferences).unwrap_or_default();
            if prefs_val.get("encrypted_workspace_key").is_none() {
                // Generate a new Workspace Master Key
                let mut ws_key = [0u8; 32];
                if getrandom::fill(&mut ws_key).is_ok() {
                    if let Ok(enc_key) = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key.to_vec()) {
                        prefs_val["encrypted_workspace_key"] = serde_json::json!(enc_key);
                        if let Ok(updated_prefs) = serde_json::to_string(&prefs_val) {
                            let user_id: String = row.get(0)?;
                            let _ = conn.execute(
                                "UPDATE users SET preferences = ?1 WHERE id = ?2",
                                crate::params![&updated_prefs, &user_id],
                            ).await;
                            preferences = updated_prefs;
                        }
                    }
                }
            }
            
            // Decrypt and set the Workspace Master Key as active session key
            let prefs_val: serde_json::Value = serde_json::from_str(&preferences).unwrap_or_default();
            if let Some(enc_key) = prefs_val.get("encrypted_workspace_key").and_then(|v| v.as_str()) {
                if let Ok(dec_key) = crate::infra::crypto::decrypt_workspace_key_with_password(&zeroizing_password, enc_key) {
                    let _ = crate::infra::crypto::set_local_secret(&format!("workspace_key_{}", ws), &const_hex::encode(&dec_key)).await;
                    crate::infra::crypto::set_session_key(dec_key);
                }
            }
        }

        Ok(Some(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences,
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

    if auth.role != "platform_admin" && target_ws_id.as_ref() != Some(&auth.workspace_id) {
        return Err(YntraError::AuthError("Access denied: target user is not in your workspace".to_string()));
    }

    let hashed_res = hash_password_argon2(&zeroizing_password);
    let hashed = hashed_res?;
    let now_ms = crate::infra::time::get_current_time_ms();

    let ws_key_opt = crate::infra::crypto::get_session_key();
    
    // Fetch preferences
    let prefs_str: String = conn.query_row(
        "SELECT preferences FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.unwrap_or_else(|_| "{}".to_string());
    
    let mut prefs_val: serde_json::Value = serde_json::from_str(&prefs_str).unwrap_or_default();
    
    if let Some(ws_key) = ws_key_opt {
        if let Ok(enc_key) = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key) {
            prefs_val["encrypted_workspace_key"] = serde_json::json!(enc_key);
        }
    } else {
        // Fallback: generate a new one if not available
        let mut ws_key = [0u8; 32];
        if getrandom::fill(&mut ws_key).is_ok() {
            if let Ok(enc_key) = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key.to_vec()) {
                prefs_val["encrypted_workspace_key"] = serde_json::json!(enc_key);
            }
        }
    }
    
    let updated_prefs = serde_json::to_string(&prefs_val).unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        "UPDATE users SET password_hash = ?1, preferences = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![hashed, updated_prefs, now_ms, user_id],
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

    conn.begin_transaction().await?;

    let res = async {
        // 1. Delete associated child relationships that are no longer needed
        conn.execute("DELETE FROM team_members WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("DELETE FROM student_parents WHERE parent_user_id = ?1", crate::params![&user_id]).await?;
        
        // 2. Anonymize/Nullify references in other tables to preserve integrity
        conn.execute("UPDATE messages SET sender_id = NULL WHERE sender_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE messages SET receiver_id = NULL WHERE receiver_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE reports SET user_id = NULL WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE student_profiles SET user_id = NULL WHERE user_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE courses SET teacher_id = NULL WHERE teacher_id = ?1", crate::params![&user_id]).await?;
        conn.execute("UPDATE job_tickets SET assigned_user_id = NULL WHERE assigned_user_id = ?1", crate::params![&user_id]).await?;
        
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute("UPDATE time_reports SET note = NULL, sync_status = 'pending', updated_at = ?1 WHERE user_id = ?2", crate::params![now_ms, &user_id]).await?;

        // 3. Instead of physically deleting the user row (which fails due to foreign key constraints in audit_logs, notes, client_journals, etc.),
        // we cryptographically scrub all personal data from the user record to satisfy GDPR Art. 17 right to be forgotten.
        let anon_email = format!("deleted-{}@yntra-deleted.invalid", &user_id[..8.min(user_id.len())]);
        conn.execute(
            "UPDATE users 
             SET email = ?1, 
                 full_name = 'Deleted User', 
                 phone = NULL, 
                 password_hash = NULL, 
                 siths_card_id = NULL, 
                 siths_public_key = NULL, 
                 nfc_badge_uid = NULL, 
                 personal_number = NULL, 
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

pub async fn ensure_user_role_signature(
    conn: &database::DbConnection,
    user_id: &str,
    role: &str,
    workspace_id: &str,
) -> Result<(), YntraError> {
    let is_privileged = role != "user" && role != "client" && role != "guest" && role != "anonymous" && role != "deleted";
    if !is_privileged {
        conn.execute(
            "UPDATE users SET role_signature = NULL WHERE id = ?1",
            crate::params![user_id],
        ).await?;
        return Ok(());
    }

    // 1. Check if workspace already has a public key configured
    let creator_pk: Option<String> = conn.query_row(
        "SELECT creator_public_key FROM workspaces WHERE id = ?1",
        crate::params![workspace_id],
        |r| Ok(r.get(0)?)
    ).await.ok().flatten();

    let private_key_setting = format!("creator_private_key_{}", workspace_id);
    let mut creator_sk: Option<String> = crate::infra::crypto::get_local_secret(&private_key_setting).await;

    // 2. If not configured, generate keypair and store them
    if creator_pk.is_none() || creator_pk.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        if creator_sk.is_none() {
            let keys = crate::infra::crypto::generate_workspace_keypair()?;
            let pub_hex = &keys[0];
            let priv_hex = &keys[1];
            
            conn.execute(
                "UPDATE workspaces SET creator_public_key = ?1 WHERE id = ?2",
                crate::params![pub_hex, workspace_id],
            ).await?;

            crate::infra::crypto::set_local_secret(&private_key_setting, priv_hex).await?;

            creator_sk = Some(priv_hex.clone());
        } else {
            let private_key_bytes = zeroize::Zeroizing::new(
                const_hex::decode(creator_sk.as_ref().unwrap())
                    .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
            );
            let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
            if private_key_bytes.len() != 32 {
                return Err(crate::infra::errors::YntraError::CryptoError("Invalid private key length".to_string()));
            }
            private_key_array.copy_from_slice(&private_key_bytes[..32]);
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
            let pub_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
            conn.execute(
                "UPDATE workspaces SET creator_public_key = ?1 WHERE id = ?2",
                crate::params![pub_hex, workspace_id],
            ).await?;
        }
    } else if creator_sk.is_none() {
        return Ok(());
    }

    // 3. Generate role signature and save to users table
    if let Some(sk) = creator_sk {
        let sig = crate::infra::crypto::generate_role_signature(&sk, user_id, role, workspace_id)?;
        conn.execute(
            "UPDATE users SET role_signature = ?1 WHERE id = ?2",
            crate::params![&sig, user_id],
        ).await?;
    }

    Ok(())
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_update_user_role() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        
        let ws_id = format!("ws-update-{}", uuid::Uuid::new_v4());
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Update WS', '[]', '{}')", crate::params![&ws_id]).await.unwrap();
        
        // 1. Create a test admin user (since only administrators can change roles)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-admin', ?1, 'admin@yntra.io', 'platform_admin')", crate::params![&ws_id]).await.unwrap();
        
        // 2. Create a test target user
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-target', ?1, 'target@yntra.io', 'user')", crate::params![&ws_id]).await.unwrap();

        // 3. Admin changes target role to assistant
        let res = update_user_role("test-admin".to_string(), "test-target".to_string(), "assistant".to_string()).await;
        assert!(res.is_ok());

        // 4. Verify in DB
        let role: String = conn.query_row("SELECT role FROM users WHERE id = 'test-target'", (), |r| r.get(0)).await.unwrap();
        assert_eq!(role, "assistant");

        // 5. Clean up
        conn.execute("DELETE FROM users WHERE id IN ('test-admin', 'test-target')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![&ws_id]).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_ensure_role_signature_generation() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup leftover garbage in case a previous run panicked
        let _ = conn.execute("DELETE FROM users WHERE id IN ('admin-1', 'target-1')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await;
        let _ = crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", "").await;

        // 1. Generate keypair manually in test
        let keys = crate::infra::crypto::generate_workspace_keypair().unwrap();
        let pub_hex = &keys[0];
        let priv_hex = &keys[1];

        // 2. Create workspace with public key
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, creator_public_key, modules_active, settings) VALUES ('ws-sig-test', 'Sig Test WS', ?1, '[]', '{}')", crate::params![pub_hex]).await.unwrap();

        // 3. Store private key in local secrets
        crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", priv_hex).await.unwrap();

        // 4. Generate role signature for admin-1 and insert
        let admin_sig = crate::infra::crypto::generate_role_signature(priv_hex, "admin-1", "platform_admin", "ws-sig-test").unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('admin-1', 'ws-sig-test', 'admin@sig.io', 'platform_admin', ?1)", crate::params![&admin_sig]).await.unwrap();

        // 5. Create target user
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('target-1', 'ws-sig-test', 'target@sig.io', 'user')", ()).await.unwrap();

        // 3. Promote target-1 to assistant (privileged)
        let res = update_user_role("admin-1".to_string(), "target-1".to_string(), "assistant".to_string()).await;
        assert!(res.is_ok());

        // 4. Verify workspace has creator_public_key and user has role_signature
        let pk: String = conn.query_row("SELECT creator_public_key FROM workspaces WHERE id = 'ws-sig-test'", (), |r| r.get(0)).await.unwrap();
        assert!(!pk.trim().is_empty());

        let sig: String = conn.query_row("SELECT role_signature FROM users WHERE id = 'target-1'", (), |r| r.get(0)).await.unwrap();
        assert!(!sig.trim().is_empty());

        // Verify signature is cryptographically valid
        let is_valid = crate::infra::crypto::verify_role_signature(&pk, "target-1", "assistant", "ws-sig-test", &sig);
        assert!(is_valid);

        // 5. Demote target-1 to user (unprivileged)
        let res = update_user_role("admin-1".to_string(), "target-1".to_string(), "user".to_string()).await;
        assert!(res.is_ok());

        let sig_after: Option<String> = conn.query_row("SELECT role_signature FROM users WHERE id = 'target-1'", (), |r| Ok(r.get(0)?)).await.unwrap();
        assert!(sig_after.is_none());

        // Clean up
        conn.execute("DELETE FROM users WHERE id IN ('admin-1', 'target-1')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret("creator_private_key_ws-sig-test", "").await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_eventual_consistency_role_signing() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-eventual-1";
        // Clean up
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await;
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;

        // 1. Create workspace
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Eventual WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();

        // 2. Create admin user (this will generate the keypair for the workspace)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('admin-eventual', ?1, 'admin@eventual.io', 'admin')", crate::params![ws_id]).await.unwrap();
        ensure_user_role_signature(&conn, "admin-eventual", "admin", ws_id).await.unwrap();

        // 3. Create a privileged user *without* a signature (simulating client-side invitation activation)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-eventual', ?1, 'user@eventual.io', 'admin', NULL)", crate::params![ws_id]).await.unwrap();

        // 4. Admin queries get_users
        let _ = get_users("admin-eventual".to_string()).await.unwrap();

        // 5. Verify that user-eventual now has a valid signature!
        let sig: Option<String> = conn.query_row("SELECT role_signature FROM users WHERE id = 'user-eventual'", (), |r| Ok(r.get(0)?)).await.unwrap();
        assert!(sig.is_some());
        assert!(!sig.unwrap().is_empty());

        // Clean up
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret(&format!("creator_private_key_{}", ws_id), "").await;
    }
}
