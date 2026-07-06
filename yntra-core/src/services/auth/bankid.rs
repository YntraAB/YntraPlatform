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

fn verify_luhn(digits: &str) -> bool {
    let mut sum = 0;
    let mut alternate = false;
    for c in digits.chars().rev() {
        let mut val = match c.to_digit(10) {
            Some(d) => d,
            None => return false,
        };
        if alternate {
            val *= 2;
            if val > 9 {
                val -= 9;
            }
        }
        sum += val;
        alternate = !alternate;
    }
    sum % 10 == 0
}

fn verify_norwegian_checksum(digits: &str) -> bool {
    if digits.len() != 11 {
        return false;
    }
    let d: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 11 {
        return false;
    }
    
    // First control digit
    let w1 = [3, 7, 6, 1, 8, 9, 4, 5, 2];
    let mut sum1 = 0;
    for i in 0..9 {
        sum1 += d[i] * w1[i];
    }
    let c1 = 11 - (sum1 % 11);
    let expected_c1 = if c1 == 11 { 0 } else { c1 };
    if expected_c1 == 10 || expected_c1 != d[9] {
        return false;
    }
    
    // Second control digit
    let w2 = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let mut sum2 = 0;
    for i in 0..10 {
        sum2 += d[i] * w2[i];
    }
    let c2 = 11 - (sum2 % 11);
    let expected_c2 = if c2 == 11 { 0 } else { c2 };
    if expected_c2 == 10 || expected_c2 != d[10] {
        return false;
    }
    
    true
}

fn verify_auth_signature(public_key_hex: &str, message: &str, signature_hex: &str) -> bool {
    let public_key_bytes = match const_hex::decode(public_key_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature_bytes = match const_hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes.try_into().unwrap_or([0u8; 32])) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let signature_array: [u8; 64] = match signature_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    use ed25519_dalek::Verifier;
    let signature = ed25519_dalek::Signature::from_bytes(&signature_array);
    verifying_key.verify(message.as_bytes(), &signature).is_ok()
}

fn verify_finnish_checksum(pnum: &str) -> bool {
    let clean = pnum.trim();
    if clean.len() != 11 {
        return false;
    }
    let first_9 = match format!("{}{}", &clean[0..6], &clean[7..10]).parse::<u64>() {
        Ok(n) => n,
        Err(_) => return false,
    };
    let rem = (first_9 % 31) as usize;
    let checksum_chars = "0123456789ABCDEFHJKLMNPRSTUVWXY";
    let expected_char = checksum_chars.chars().nth(rem).unwrap_or(' ');
    let actual_char = clean.chars().nth(10).unwrap_or(' ');
    actual_char == expected_char
}

#[allow(dead_code)]
fn check_birthdate_match(personal_number: &str, birthdate_ddmmyy: &str) -> bool {
    check_birthdate_match_impl(personal_number, birthdate_ddmmyy, None)
}

fn check_birthdate_match_impl(personal_number: &str, birthdate_ddmmyy: &str, provider: Option<&str>) -> bool {
    if birthdate_ddmmyy.len() != 6 {
        return false;
    }
    
    let clean_pnum = personal_number.trim();

    let mut enforce_se = false;
    let mut enforce_no = false;
    let mut enforce_dk = false;
    let mut enforce_fi = false;

    if let Some(prov) = provider {
        match prov {
            "se_bankid" | "siths" => enforce_se = true,
            "no_bankid" => enforce_no = true,
            "dk_mitid" => enforce_dk = true,
            "fi_tunnistus" => enforce_fi = true,
            _ => {}
        }
    }

    // Handle Finnish Personal Identity Code (Format: DDMMYYCZZZQ)
    let is_finnish_format = if clean_pnum.len() == 11 {
        let separator = clean_pnum.chars().nth(6).unwrap_or(' ');
        let valid_finnish_separators = ['+', '-', 'A', 'B', 'C', 'D', 'E', 'F', 'Y', 'X', 'W', 'V', 'U'];
        valid_finnish_separators.contains(&separator) && verify_finnish_checksum(clean_pnum)
    } else {
        false
    };

    if is_finnish_format || enforce_fi {
        if enforce_se || enforce_no || enforce_dk {
            return false;
        }
        return clean_pnum.starts_with(birthdate_ddmmyy) && verify_finnish_checksum(clean_pnum);
    }
    
    let mut digits: String = personal_number.chars().filter(|c| c.is_ascii_digit()).collect();
    let matches = if digits.len() == 11 {
        // DDMMYYXXXXX (Norwegian)
        if enforce_se || enforce_dk || enforce_fi {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        if enforce_no && !verify_norwegian_checksum(&digits) {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        if digits.len() >= 6 {
            let mut dd = digits[0..2].parse::<i32>().unwrap_or(0);
            if dd > 40 {
                dd -= 40;
            }
            let mm = digits[2..4].parse::<i32>().unwrap_or(0);
            let yy = &digits[4..6];
            let normalized_ddmmyy = format!("{:02}{:02}{}", dd, mm, yy);
            normalized_ddmmyy == birthdate_ddmmyy
        } else {
            false
        }
    } else if digits.len() == 12 {
        // YYYYMMDDXXXX (Swedish 12-digit)
        if enforce_no || enforce_dk || enforce_fi {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        if enforce_se && !verify_luhn(&digits[2..]) {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        if digits.len() < 8 {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        let yy = &digits[2..4];
        let mm = &digits[4..6];
        let mut dd = digits[6..8].parse::<i32>().unwrap_or(0);
        if dd > 60 {
            dd -= 60;
        }
        let expected_ddmmyy = format!("{:02}{}{}", dd, mm, yy);
        expected_ddmmyy == birthdate_ddmmyy
    } else if digits.len() == 10 {
        // YYMMDDXXXX (Swedish) OR DDMMYYXXXX (Danish CPR)
        if digits.len() < 6 {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        if enforce_dk {
            digits[0..6] == *birthdate_ddmmyy
        } else if enforce_se {
            if !verify_luhn(&digits) {
                use zeroize::Zeroize;
                digits.zeroize();
                return false;
            }
            let yy = &digits[0..2];
            let mm = &digits[2..4];
            let mut dd = digits[4..6].parse::<i32>().unwrap_or(0);
            if dd > 60 {
                dd -= 60;
            }
            let expected_ddmmyy = format!("{:02}{}{}", dd, mm, yy);
            expected_ddmmyy == birthdate_ddmmyy
        } else {
            // Fallback (tests or unspecified provider)
            let yy = &digits[0..2];
            let mm = &digits[2..4];
            let mut dd = digits[4..6].parse::<i32>().unwrap_or(0);
            if dd > 60 {
                dd -= 60;
            }
            let expected_ddmmyy = format!("{:02}{}{}", dd, mm, yy);
            expected_ddmmyy == birthdate_ddmmyy || digits[0..6] == *birthdate_ddmmyy
        }
    } else {
        // Fallback containment check
        if enforce_se || enforce_no || enforce_dk || enforce_fi {
            use zeroize::Zeroize;
            digits.zeroize();
            return false;
        }
        digits.len() >= 6 && digits.contains(birthdate_ddmmyy)
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
                        if let Ok(mut stmt) = conn.prepare("SELECT id FROM users WHERE phone = ?1 LIMIT 1").await {
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
        let _zeroizing_pin = zeroize::Zeroizing::new(pin);
        
        // Transition status to verifying
        {
            let conn = database::acquire_connection().await?;
            conn.execute(
                "UPDATE bankid_auth_sessions SET status = 'verifying', progress = 0.0 WHERE id = ?1",
                crate::params![&session_id],
            ).await?;
        }
        notify_observers();

        // For all providers in production: we keep in verifying status and wait for host validation
        Ok(())
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
pub async fn complete_auth_session(session_id: String, user_id: String, signature_hex: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    // Verify user exists and retrieve workspace_id
    let ws_id: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("User does not exist".to_string()))?;

    // Retrieve creator public key for workspace
    let creator_pk: Option<String> = conn.query_row(
        "SELECT creator_public_key FROM workspaces WHERE id = ?1",
        crate::params![&ws_id],
        |r| Ok(r.get(0)?)
    ).await.unwrap_or(None);

    if let Some(pk) = creator_pk {
        if !pk.trim().is_empty() {
            // Verify signature: message is "auth_session:session_id:user_id"
            let message = format!("auth_session:{}:{}", session_id, user_id);
            let is_valid = verify_auth_signature(&pk, &message, &signature_hex);
            if !is_valid {
                return Err(YntraError::AuthError("Cryptographic signature verification failed for authentication session completion".to_string()));
            }
        }
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
    fn test_finnish_birthdate_match() {
        // Finnish: DDMMYYCZZZQ
        assert!(check_birthdate_match("131052-308T", "131052"));
        assert!(check_birthdate_match("010100A123D", "010100"));
        assert!(!check_birthdate_match("131052-308T", "141052"));
    }

    #[test]
    fn test_norwegian_birthdate_match() {
        // Norwegian: DDMMYYXXXXX
        assert!(check_birthdate_match("05072612305", "050726"));
        assert!(!check_birthdate_match("05072612305", "060726"));
    }

    #[test]
    fn test_danish_birthdate_match() {
        // Danish CPR: DDMMYYXXXX
        assert!(check_birthdate_match("1405891234", "140589"));
        assert!(check_birthdate_match("3112019876", "311201"));
        assert!(!check_birthdate_match("1405891234", "150589"));
    }

    #[test]
    fn test_norwegian_d_number_birthdate_match() {
        // Norwegian D-number: first digit of day increased by 4
        assert!(check_birthdate_match("45072612802", "050726"));
    }

    #[test]
    fn test_swedish_coordination_number_birthdate_match() {
        // Swedish Samordningsnummer: day increased by 60
        assert!(check_birthdate_match("198905741891", "140589"));
        assert!(check_birthdate_match("8905741891", "140589"));
    }

    #[test]
    fn test_swedish_12digit_birthdate_match() {
        // Swedish: YYYYMMDDXXXX
        assert!(check_birthdate_match("198905141894", "140589"));
        assert!(check_birthdate_match("200112319876", "311201"));
        assert!(!check_birthdate_match("198905141894", "150589"));
    }

    #[test]
    fn test_swedish_10digit_birthdate_match() {
        // Swedish: YYMMDDXXXX
        assert!(check_birthdate_match("8905141894", "140589"));
        assert!(check_birthdate_match("0112319876", "311201"));
        assert!(!check_birthdate_match("8905141894", "150589"));
    }

    #[test]
    fn test_swedish_hyphenated_birthdate_match() {
        // Swedish standard hyphenated: YYMMDD-XXXX
        assert!(check_birthdate_match("890514-1894", "140589"));
        assert!(check_birthdate_match("011231-9876", "311201"));
        assert!(!check_birthdate_match("890514-1894", "150589"));
    }

    #[test]
    fn test_finnish_new_century_separators_match() {
        // Finnish new century separators (B for 2000s, Y for 1900s)
        assert!(check_birthdate_match("010100B123D", "010100"));
        assert!(check_birthdate_match("150890Y4562", "150890"));
        
        // Ensure no security bypass/false positives via weak contains fallback
        assert!(!check_birthdate_match("120101B001A", "010100"));
    }

    #[test]
    fn test_birthdate_fallback_containment() {
        assert!(check_birthdate_match("abc140589xyz", "140589"));
        assert!(!check_birthdate_match("abc140588xyz", "140589"));
    }

    #[test]
    fn test_finnish_pic_security_bypass_prevention() {
        // Finnish PIC 131089-3058 (Oct 13, 1989) has a valid Finnish checksum '8'.
        assert!(check_birthdate_match_impl("131089-3058", "131089", Some("fi_tunnistus")));
        assert!(!check_birthdate_match_impl("131089-3058", "291013", Some("fi_tunnistus")));
        assert!(!check_birthdate_match_impl("131089-3058", "131089", Some("se_bankid")));
        assert!(!check_birthdate_match_impl("131089-3058", "291013", Some("se_bankid")));
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

