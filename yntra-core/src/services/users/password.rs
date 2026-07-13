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

pub(crate) fn hash_password_argon2(password: &str) -> Result<String, YntraError> {
    use argon2::{Algorithm, Version, Params};
    let salt = SaltString::generate(&mut OsRng);
    
    let params = Params::new(19456, 2, 1, Some(32))
        .map_err(|e| YntraError::CryptoError(format!("Argon2 params invalid: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    argon2.hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| YntraError::CryptoError(format!("Argon2 hashing failed: {}", e)))
}

pub(crate) fn verify_password_argon2(password: &str, stored_hash: &str) -> bool {
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
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, password_hash, metadata, updated_at, sync_status FROM users WHERE LOWER(email) = ?1",
    ).await?;

    let mut rows = stmt.query(crate::params![email_lower]).await?;
    if let Some(row) = rows.next().await? {
        let stored_hash: Option<String> = row.get(7)?;
        let auth_ok = match stored_hash {
            Some(h) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let pwd = zeroizing_password.to_string();
                    let h_cloned = h.clone();
                    tokio::task::spawn_blocking(move || {
                        let zeroing = zeroize::Zeroizing::new(pwd);
                        verify_password_argon2(&zeroing, &h_cloned)
                    }).await.unwrap_or(false)
                }
                #[cfg(target_arch = "wasm32")]
                {
                    verify_password_argon2(&zeroizing_password, &h)
                }
            }
            None => false,
        };
        if !auth_ok {
            return Ok(None);
        }
        let ws_id: Option<String> = row.get(1)?;
        let metadata_str: Option<String> = row.get(8)?;
        
        let mut siths_card_id = None;
        let mut nfc_badge_uid = None;
        let mut raw_pnum = None;

        if let Some(ref m_str) = metadata_str {
            if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                siths_card_id = meta_val.get("siths_card_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                nfc_badge_uid = meta_val.get("nfc_badge_uid").and_then(|v| v.as_str()).map(|s| s.to_string());
                raw_pnum = meta_val.get("personal_number").and_then(|v| v.as_str()).map(|s| s.to_string());
            }
        }
        
        let mut preferences: String = row.get(6)?;
        if let Some(ref ws) = ws_id {
            let mut prefs_val: serde_json::Value = serde_json::from_str(&preferences).unwrap_or_default();
            if prefs_val.get("encrypted_workspace_key").is_none() {
                // Generate a new Workspace Master Key
                let mut ws_key = [0u8; 32];
                if getrandom::fill(&mut ws_key).is_ok() {
                    #[cfg(not(target_arch = "wasm32"))]
                    let pwd = zeroizing_password.to_string();
                    #[cfg(not(target_arch = "wasm32"))]
                    let pwd_clone = pwd.clone();
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    let enc_key_res = tokio::task::spawn_blocking(move || {
                        let zeroing = zeroize::Zeroizing::new(pwd_clone);
                        crate::infra::crypto::encrypt_workspace_key_with_password(&zeroing, ws_key.to_vec())
                    }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())));
                    #[cfg(target_arch = "wasm32")]
                    let enc_key_res = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key.to_vec());

                    if let Ok(enc_key) = enc_key_res {
                        prefs_val.as_object_mut().unwrap().insert("encrypted_workspace_key".to_string(), serde_json::Value::String(enc_key));
                        if let Ok(updated_prefs) = serde_json::to_string(&prefs_val) {
                            preferences = updated_prefs;
                            let now_ms = crate::infra::time::get_current_time_ms();
                            let _ = conn.execute(
                                "UPDATE users SET preferences = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
                                crate::params![&preferences, now_ms, row.get::<String>(0)?]
                            ).await;
                        }
                    }
                }
            }
            
            // Decrypt and set the Workspace Master Key as active session key
            if let Some(enc_key) = prefs_val.get("encrypted_workspace_key").and_then(|v| v.as_str()) {
                #[cfg(not(target_arch = "wasm32"))]
                let pwd = zeroizing_password.to_string();
                #[cfg(not(target_arch = "wasm32"))]
                let enc_key_str = enc_key.to_string();
                #[cfg(not(target_arch = "wasm32"))]
                let dec_res = tokio::task::spawn_blocking(move || {
                    let zeroing = zeroize::Zeroizing::new(pwd);
                    crate::infra::crypto::decrypt_workspace_key_with_password(&zeroing, &enc_key_str)
                }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())));
                #[cfg(target_arch = "wasm32")]
                let dec_res = crate::infra::crypto::decrypt_workspace_key_with_password(&zeroizing_password, enc_key);

                if let Ok(dec_key) = dec_res {
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
            siths_card_id,
            nfc_badge_uid,
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
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

    #[cfg(not(target_arch = "wasm32"))]
    let pwd = zeroizing_password.to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let hashed_res = tokio::task::spawn_blocking(move || {
        let zeroing = zeroize::Zeroizing::new(pwd);
        hash_password_argon2(&zeroing)
    }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())));
    #[cfg(target_arch = "wasm32")]
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
        #[cfg(not(target_arch = "wasm32"))]
        let pwd = zeroizing_password.to_string();
        #[cfg(not(target_arch = "wasm32"))]
        let ws_key_clone = ws_key.to_vec();
        #[cfg(not(target_arch = "wasm32"))]
        let enc_res = tokio::task::spawn_blocking(move || {
            let zeroing = zeroize::Zeroizing::new(pwd);
            crate::infra::crypto::encrypt_workspace_key_with_password(&zeroing, ws_key_clone)
        }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())));
        #[cfg(target_arch = "wasm32")]
        let enc_res = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key.to_vec());

        if let Ok(enc_key) = enc_res {
            prefs_val["encrypted_workspace_key"] = serde_json::json!(enc_key);
        }
    } else {
        // Fallback: generate a new one if not available
        let mut ws_key = [0u8; 32];
        if getrandom::fill(&mut ws_key).is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            let pwd = zeroizing_password.to_string();
            #[cfg(not(target_arch = "wasm32"))]
            let ws_key_vec = ws_key.to_vec();
            #[cfg(not(target_arch = "wasm32"))]
            let enc_res = tokio::task::spawn_blocking(move || {
                let zeroing = zeroize::Zeroizing::new(pwd);
                crate::infra::crypto::encrypt_workspace_key_with_password(&zeroing, ws_key_vec)
            }).await.unwrap_or_else(|e| Err(YntraError::CryptoError(e.to_string())));
            #[cfg(target_arch = "wasm32")]
            let enc_res = crate::infra::crypto::encrypt_workspace_key_with_password(&zeroizing_password, ws_key.to_vec());

            if let Ok(enc_key) = enc_res {
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
