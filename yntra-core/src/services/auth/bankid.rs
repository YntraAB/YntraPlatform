use crate::{BankIdAuthSession, YntraError};
use uuid::Uuid;
use crate::database;
use crate::infra::observer::notify_observers;
use super::hardware::sleep_ms;

fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::spawn(future);
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[allow(dead_code)]
fn check_birthdate_match(personal_number: &str, birthdate_ddmmyy: &str) -> bool {
    if birthdate_ddmmyy.len() != 6 {
        return false;
    }
    let mut digits: String = personal_number.chars().filter(|c| c.is_ascii_digit()).collect();
    let matches = if digits.len() == 11 {
        // DDMMYYXXXXX (Norwegian)
        digits.starts_with(birthdate_ddmmyy)
    } else if digits.len() == 12 {
        // YYYYMMDDXXXX
        if digits.len() < 8 {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        let yy = &digits[2..4];
        let mm = &digits[4..6];
        let dd = &digits[6..8];
        let expected_ddmmyy = format!("{}{}{}", dd, mm, yy);
        expected_ddmmyy == birthdate_ddmmyy
    } else if digits.len() == 10 {
        // YYMMDDXXXX
        if digits.len() < 6 {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        let yy = &digits[0..2];
        let mm = &digits[2..4];
        let dd = &digits[4..6];
        let expected_ddmmyy = format!("{}{}{}", dd, mm, yy);
        expected_ddmmyy == birthdate_ddmmyy
    } else {
        digits.contains(birthdate_ddmmyy)
    };
    use zeroize::Zeroize;
    digits.zeroize();
    matches
}

#[uniffi::export]
pub async fn initiate_bankid_auth(target_role: String, provider: String) -> Result<BankIdAuthSession, YntraError> {
    let session_id = Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    
    // Initial status depending on ID provider
    let status = match provider.as_str() {
        "se_bankid" => "qr_scan".to_string(),
        "dk_mitid" => "qr_scan".to_string(),
        "no_bankid" => "no_prompt".to_string(), // Norway: prompt for Birthdate & Phone
        "us_global" => "us_prompt".to_string(), // US: prompt for Smart Card PIN
        "card_or_badge" | "siths" | "nfc" => "connecting".to_string(),
        _ => "qr_scan".to_string(),
    };

    let mut challenge_bytes = [0u8; 32];
    let challenge_val = if getrandom::fill(&mut challenge_bytes).is_ok() {
        Some(const_hex::encode(&challenge_bytes))
    } else {
        None
    };

    let session = BankIdAuthSession {
        id: session_id.clone(),
        target_role: target_role.clone(),
        provider: provider.clone(),
        status,
        pin: "".to_string(),
        qr_data: format!("{}-{}-qr", provider, session_id),
        progress: 0.0,
        authenticated_user_id: None,
        created_at,
        challenge: challenge_val.clone(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO bankid_auth_sessions (id, target_role, provider, status, error_message, qr_data, progress, authenticated_user_id, created_at, challenge) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            session.id,
            session.target_role,
            session.provider,
            session.status,
            None::<String>,
            session.qr_data,
            session.progress,
            session.authenticated_user_id,
            session.created_at,
            session.challenge
        ],
    ).await?;

    notify_observers();

    if provider == "card_or_badge" || provider == "siths" || provider == "nfc" {
        let session_id_clone = session.id.clone();
        let provider_clone = provider.clone();
        spawn_task(async move {
            super::hardware::run_hardware_auth(session_id_clone, provider_clone).await;
        });
    }

    if provider == "se_bankid" || provider == "dk_mitid" {
        let session_id_clone = session.id.clone();
        spawn_task(async move {
            for elapsed in 1..=30 {
                sleep_ms(1000).await;
                if let Ok(conn) = database::acquire_connection().await {
                    let mut status: Option<String> = None;
                    if let Ok(mut stmt) = conn.prepare("SELECT status FROM bankid_auth_sessions WHERE id = ?1").await {
                        if let Ok(mut rows) = stmt.query(crate::params![&session_id_clone]).await {
                            if let Ok(Some(row)) = rows.next().await {
                                status = row.get::<String>(0).ok();
                            }
                        }
                    }
                    if let Some(ref s) = status {
                        if s == "qr_scan" {
                            let new_qr = format!("bankid.status.qrs.format.{}.{}", elapsed, session_id_clone);
                            let _ = conn.execute(
                                "UPDATE bankid_auth_sessions SET qr_data = ?1 WHERE id = ?2",
                                crate::params![new_qr, &session_id_clone],
                            ).await;
                            notify_observers();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
    }

    Ok(session)
}

#[uniffi::export]
pub async fn get_bankid_auth_session(session_id: String) -> Result<Option<BankIdAuthSession>, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, target_role, provider, status, error_message, qr_data, progress, authenticated_user_id, created_at, challenge FROM bankid_auth_sessions WHERE id = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![session_id]).await?;
    if let Some(row) = rows.next().await? {
        let err_msg: Option<String> = row.get(4)?;
        Ok(Some(BankIdAuthSession {
            id: row.get(0)?,
            target_role: row.get(1)?,
            provider: row.get(2)?,
            status: row.get(3)?,
            pin: err_msg.unwrap_or_default(),
            qr_data: row.get(5)?,
            progress: row.get(6)?,
            authenticated_user_id: row.get(7)?,
            created_at: row.get(8)?,
            challenge: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn submit_bankid_pin(session_id: String, pin: String) -> Result<(), YntraError> {
    #[cfg(debug_assertions)]
    {
        let zeroizing_pin = zeroize::Zeroizing::new(pin);
        
        // 1. First set status to verifying and progress = 0.0
        {
            let conn = database::acquire_connection().await?;
            conn.execute(
                "UPDATE bankid_auth_sessions SET status = 'verifying', progress = 0.0 WHERE id = ?1",
                crate::params![session_id],
            ).await?;
        }
        notify_observers();

        // 2. Spawn a background thread that ticks progress up to 100% and then completes the login!
        let session_id_clone = session_id.clone();
        let pin_clone = zeroizing_pin.clone();
        spawn_task(async move {
            let zeroizing_pin_clone = zeroize::Zeroizing::new(pin_clone);
            let mut resolved_id = None;
            if let Ok(conn) = database::acquire_connection().await {
                if zeroizing_pin_clone.contains('|') {
                    let parts: Vec<&str> = zeroizing_pin_clone.split('|').collect();
                    if parts.len() == 2 {
                        let phone_input = parts[0];
                        if let Ok(mut stmt) = conn.prepare("SELECT id FROM users WHERE phone = ?1 OR phone LIKE '%' || ?1 LIMIT 1").await {
                            if let Ok(mut rows) = stmt.query(crate::params![phone_input]).await {
                                if let Ok(Some(row)) = rows.next().await {
                                    resolved_id = row.get::<String>(0).ok();
                                }
                            }
                        }
                    }
                }

                if resolved_id.is_none() {
                    let mut target_role = None;
                    if let Ok(mut stmt) = conn.prepare("SELECT target_role FROM bankid_auth_sessions WHERE id = ?1").await {
                        if let Ok(mut rows) = stmt.query(crate::params![&session_id_clone]).await {
                            if let Ok(Some(row)) = rows.next().await {
                                target_role = row.get::<String>(0).ok();
                            }
                        }
                    }
                    
                    let role_str = target_role.as_deref().unwrap_or("assistant");
                    if let Ok(mut stmt) = conn.prepare("SELECT id FROM users WHERE role = ?1 LIMIT 1").await {
                        if let Ok(mut rows) = stmt.query(crate::params![role_str]).await {
                            if let Ok(Some(row)) = rows.next().await {
                                resolved_id = row.get::<String>(0).ok();
                            }
                        }
                    }
                }
            }

            if let Some(uid) = resolved_id {
                for progress_pct in [20.0, 40.0, 60.0, 80.0, 100.0] {
                    sleep_ms(200).await;
                    if let Ok(conn) = database::acquire_connection().await {
                        if progress_pct >= 100.0 {
                            let _ = conn.execute(
                                "UPDATE bankid_auth_sessions SET progress = ?1, status = 'success', authenticated_user_id = ?2 WHERE id = ?3",
                                crate::params![progress_pct, uid, session_id_clone],
                            ).await;
                        } else {
                            let _ = conn.execute(
                                "UPDATE bankid_auth_sessions SET progress = ?1 WHERE id = ?2",
                                crate::params![progress_pct, &session_id_clone],
                            ).await;
                        }
                    }
                    notify_observers();
                }
            } else {
                if let Ok(conn) = database::acquire_connection().await {
                    let _ = conn.execute(
                        "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0 WHERE id = ?1",
                        crate::params![&session_id_clone],
                    ).await;
                }
                notify_observers();
            }
        });

        Ok(())
    }

    #[cfg(not(debug_assertions))]
    {
        let zeroizing_pin = zeroize::Zeroizing::new(pin);
        
        // Transition status to verifying
        {
            let conn = database::acquire_connection().await?;
            conn.execute(
                "UPDATE bankid_auth_sessions SET status = 'verifying', progress = 0.0 WHERE id = ?1",
                crate::params![&session_id],
            ).await?;
        }
        notify_observers();

        // Retrieve provider to decide validation
        let provider = {
            let conn = database::acquire_connection().await?;
            let mut stmt = conn.prepare("SELECT provider FROM bankid_auth_sessions WHERE id = ?1").await?;
            let mut rows = stmt.query(crate::params![&session_id]).await?;
            if let Some(row) = rows.next().await? {
                row.get::<String>(0)?
            } else {
                return Err(YntraError::NotFoundError("Session not found".to_string()));
            }
        };

        if provider == "no_bankid" {
            if !zeroizing_pin.contains('|') {
                let conn = database::acquire_connection().await?;
                let _ = conn.execute("UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = '' WHERE id = ?1", crate::params![&session_id]).await;
                notify_observers();
                return Err(YntraError::ValidationError("Invalid Norwegian BankID payload".to_string()));
            }
            let parts: Vec<&str> = zeroizing_pin.split('|').collect();
            let phone_input = parts[0];
            let birthdate_input = parts[1];

            let user_info = {
                let conn = database::acquire_connection().await?;
                let mut stmt = conn.prepare("SELECT id, personal_number, workspace_id FROM users WHERE phone = ?1 OR phone LIKE '%' || ?1 LIMIT 1").await?;
                let mut rows = stmt.query(crate::params![phone_input]).await?;
                if let Some(row) = rows.next().await? {
                    let id: String = row.get(0)?;
                    let raw_pnum: Option<String> = row.get(1)?;
                    let ws_id: Option<String> = row.get(2)?;
                    let decrypted_pnum = crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id.as_deref().unwrap_or("workspace-1"));
                    Some((id, decrypted_pnum))
                } else {
                    None
                }
            };

            if let Some((uid, Some(mut pnum))) = user_info {
                let matches = check_birthdate_match(&pnum, birthdate_input);
                use zeroize::Zeroize;
                pnum.zeroize();
                if matches {
                    let conn = database::acquire_connection().await?;
                    conn.execute(
                        "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
                        crate::params![uid, &session_id],
                    ).await?;
                    notify_observers();
                    return Ok(());
                }
            }

            // Error fallback
            let conn = database::acquire_connection().await?;
            let _ = conn.execute("UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = 'Invalid credentials' WHERE id = ?1", crate::params![&session_id]).await;
            notify_observers();
            Err(YntraError::AuthError("Norwegian BankID verification failed: invalid credentials".to_string()))
        } else {
            // For other providers in production: we keep in verifying status and wait for host validation
            Ok(())
        }
    }
}

#[uniffi::export]
pub async fn verify_hardware_auth_signature(
    session_id: String,
    public_key_hex: String,
    signature_hex: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    
    // 1. Get the challenge from the session
    let session_row: Option<(String, Option<String>)> = conn.query_row(
        "SELECT status, challenge FROM bankid_auth_sessions WHERE id = ?1",
        crate::params![&session_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();
    
    let (status, challenge_opt) = match session_row {
        Some(row) => row,
        None => return Err(YntraError::NotFoundError("Session not found".to_string())),
    };
    
    if status == "success" || status == "error" {
        return Err(YntraError::ValidationError("Session already finalized".to_string()));
    }
    
    let challenge_hex = match challenge_opt {
        Some(c) => c,
        None => return Err(YntraError::ValidationError("No active cryptographic challenge for this session".to_string())),
    };
    
    let challenge_bytes = match const_hex::decode(&challenge_hex) {
        Ok(b) => b,
        Err(_) => return Err(YntraError::ValidationError("Invalid challenge format".to_string())),
    };
    
    // 2. Parse public key and signature
    let pub_key_bytes = match const_hex::decode(&public_key_hex) {
        Ok(b) => {
            if b.len() != 32 {
                return Err(YntraError::ValidationError("Invalid public key length (must be 32 bytes)".to_string()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        Err(_) => return Err(YntraError::ValidationError("Invalid public key hex".to_string())),
    };
    
    let sig_bytes = match const_hex::decode(&signature_hex) {
        Ok(b) => {
            if b.len() != 64 {
                return Err(YntraError::ValidationError("Invalid signature length (must be 64 bytes)".to_string()));
            }
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&b);
            arr
        }
        Err(_) => return Err(YntraError::ValidationError("Invalid signature hex".to_string())),
    };
    
    // 3. Verify signature using ed25519-dalek
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};
    let verifying_key = VerifyingKey::from_bytes(&pub_key_bytes)
        .map_err(|e| YntraError::CryptoError(format!("Invalid public key bytes: {}", e)))?;
    let signature = Signature::from_bytes(&sig_bytes);
    
    if verifying_key.verify(&challenge_bytes, &signature).is_err() {
        // Update session to error state
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = 'Signature verification failed' WHERE id = ?1",
            crate::params![&session_id],
        ).await?;
        notify_observers();
        return Err(YntraError::AuthError("Cryptographic signature verification failed".to_string()));
    }
    
    // 4. Lookup user by siths_public_key
    let mut stmt = conn.prepare("SELECT id FROM users WHERE siths_public_key = ?1 LIMIT 1").await?;
    let mut rows = stmt.query(crate::params![public_key_hex]).await?;
    if let Some(row) = rows.next().await? {
        let user_id: String = row.get(0)?;
        // Update session to success
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
            crate::params![user_id, session_id],
        ).await?;
        notify_observers();
        Ok(())
    } else {
        // Update session to error state
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = 'Smart Card not registered to any user' WHERE id = ?1",
            crate::params![&session_id],
        ).await?;
        notify_observers();
        Err(YntraError::NotFoundError("No user is registered with this Smart Card public key".to_string()))
    }
}

#[uniffi::export]
pub async fn complete_auth_session(session_id: String, user_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    // Verify user exists first to ensure integrity
    let mut stmt = conn.prepare("SELECT 1 FROM users WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&user_id]).await?;
    if rows.next().await?.is_none() {
        return Err(YntraError::NotFoundError("User does not exist".to_string()));
    }

    conn.execute(
        "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
        crate::params![user_id, session_id],
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn fail_auth_session(session_id: String, error_msg: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = ?1 WHERE id = ?2",
        crate::params![error_msg, session_id],
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_auth_session_status(session_id: String, status: String, progress: f64) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE bankid_auth_sessions SET status = ?1, progress = ?2 WHERE id = ?3",
        crate::params![status, progress, session_id],
    ).await?;
    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_norwegian_birthdate_match() {
        // Norwegian: DDMMYYXXXXX
        assert!(check_birthdate_match("05072612345", "050726"));
        assert!(!check_birthdate_match("05072612345", "060726"));
    }

    #[test]
    fn test_swedish_12digit_birthdate_match() {
        // Swedish: YYYYMMDDXXXX
        assert!(check_birthdate_match("198905141234", "140589"));
        assert!(check_birthdate_match("200112319876", "311201"));
        assert!(!check_birthdate_match("198905141234", "150589"));
    }

    #[test]
    fn test_swedish_10digit_birthdate_match() {
        // Swedish: YYMMDDXXXX
        assert!(check_birthdate_match("8905141234", "140589"));
        assert!(check_birthdate_match("0112319876", "311201"));
        assert!(!check_birthdate_match("8905141234", "150589"));
    }

    #[test]
    fn test_birthdate_fallback_containment() {
        assert!(check_birthdate_match("abc140589xyz", "140589"));
        assert!(!check_birthdate_match("abc140588xyz", "140589"));
    }

    #[test]
    fn test_hardware_challenge_response_signature() {
        let challenge_bytes = [42u8; 32];
        
        let seed = [1u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let public_key = signing_key.verifying_key();
        
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(&challenge_bytes);
        
        use ed25519_dalek::{VerifyingKey, Signature, Verifier};
        let vk = VerifyingKey::from_bytes(public_key.as_bytes()).unwrap();
        let sig = Signature::from_bytes(&signature.to_bytes());
        assert!(vk.verify(&challenge_bytes, &sig).is_ok());
    }
}

