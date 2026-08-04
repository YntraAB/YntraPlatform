use crate::YntraError;
use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::time::sleep_ms;

#[uniffi::export]
pub async fn run_hardware_auth_simulation(
    session_id: String,
    provider: String,
) -> Result<(), YntraError> {
    #[cfg(not(debug_assertions))]
    {
        let _ = session_id;
        let _ = provider;
        return Err(YntraError::AuthError(
            "Hardware authentication simulation is disabled in release builds".to_string(),
        ));
    }

    #[cfg(debug_assertions)]
    {
        sleep_ms(600).await;
        {
            let conn = database::acquire_connection().await?;
            conn.execute(
                "UPDATE bankid_auth_sessions SET status = 'polling', progress = 20.0 WHERE id = ?1",
                crate::params![&session_id],
            )
            .await?;
        }
        notify_observers();

        sleep_ms(1500).await;

        let mut resolved_user_info = None;
        {
            let conn = database::acquire_connection().await?;
            let mut stmt = conn.prepare("SELECT id, metadata FROM users").await?;
            let mut rows = stmt.query(()).await?;
            while let Some(row) = rows.next().await? {
                let uid: String = row.get(0)?;
                let metadata_str: Option<String> = row.get(1)?;

                let mut siths = None;
                let mut nfc = None;
                let mut pubkey = None;

                if let Some(ref m_str) = metadata_str {
                    if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                        siths = meta_val
                            .get("siths_card_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        nfc = meta_val
                            .get("nfc_badge_uid")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        pubkey = meta_val
                            .get("siths_public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }

                let has_siths = siths.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
                let has_nfc = nfc.as_ref().map(|s| !s.is_empty()).unwrap_or(false);

                if (provider == "siths" && has_siths)
                    || (provider == "nfc" && has_nfc)
                    || (provider == "card_or_badge" && (has_siths || has_nfc))
                {
                    resolved_user_info = Some((uid, pubkey));
                    break;
                }
            }
        }

        if let Some((uid, pubkey_opt)) = resolved_user_info {
            {
                let conn = database::acquire_connection().await?;
                conn.execute(
                    "UPDATE bankid_auth_sessions SET status = 'reading', progress = 60.0 WHERE id = ?1",
                    crate::params![&session_id],
                ).await?;
            }
            notify_observers();

            sleep_ms(600).await;

            let challenge_opt = {
                let conn = database::acquire_connection().await?;
                conn.query_row(
                    "SELECT challenge FROM bankid_auth_sessions WHERE id = ?1",
                    crate::params![&session_id],
                    |r| r.get::<Option<String>>(0),
                )
                .await
                .ok()
                .flatten()
            };

            if let (Some(challenge_hex), Some(pubkey_hex)) = (challenge_opt, pubkey_opt) {
                let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();

                let seed_val = if uid == "user-1" { 1 } else { 2 };
                let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed_val; 32]);

                use ed25519_dalek::Signer;
                let signature = signing_key.sign(&challenge_bytes);
                let sig_hex = const_hex::encode(signature.to_bytes());

                crate::services::auth::bankid::verify_hardware_auth_signature(
                    session_id, pubkey_hex, sig_hex,
                )
                .await?;
            } else {
                {
                    let conn = database::acquire_connection().await?;
                    conn.execute(
                        "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = 'login-hw-error-missing-crypto-params' WHERE id = ?1",
                        crate::params![&session_id],
                    ).await?;
                }
                notify_observers();
                return Err(YntraError::AuthError(
                    "Cryptographic parameters missing during hardware simulation".to_string(),
                ));
            }
        } else {
            {
                let conn = database::acquire_connection().await?;
                conn.execute(
                    "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = 'login-hw-error-card-unregistered:simulation' WHERE id = ?1",
                    crate::params![&session_id],
                ).await?;
            }
            notify_observers();
        }

        Ok(())
    }
}

#[uniffi::export]
pub async fn complete_hardware_auth(
    session_id: String,
    token: String,
    pin: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    let session_row: Option<(String, String, Option<String>, Option<String>, String)> = conn.query_row(
        "SELECT status, provider, challenge, authenticated_user_id, token FROM bankid_auth_sessions WHERE id = ?1",
        crate::params![&session_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    ).await.ok();

    let (status, _provider, challenge_opt, _authenticated_uid, db_token) = match session_row {
        Some(row) => row,
        None => return Err(YntraError::NotFoundError("Session not found".to_string())),
    };

    if db_token != token {
        return Err(YntraError::AuthError(
            "Access denied: invalid session token".to_string(),
        ));
    }

    if status == "success" || status == "error" {
        return Err(YntraError::ValidationError(
            "Session already finalized".to_string(),
        ));
    }

    let challenge_hex = match challenge_opt {
        Some(c) => c,
        None => {
            return Err(YntraError::ValidationError(
                "No active challenge for this session".to_string(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        #[cfg(not(debug_assertions))]
        {
            let _ = pin;
            return Err(YntraError::AuthError(
                "Hardware authentication simulation is disabled in release builds".to_string(),
            ));
        }

        #[cfg(debug_assertions)]
        {
            if pin.is_empty() {
                return Err(YntraError::AuthError("PIN cannot be empty".to_string()));
            }

            let mut resolved_user_info = None;
            let mut stmt = conn
                .prepare("SELECT id, metadata ->> 'siths_public_key' FROM users")
                .await?;
            let mut rows = stmt.query(()).await?;
            while let Some(row) = rows.next().await? {
                let uid: String = row.get(0)?;
                let pubkey: Option<String> = row.get(1)?;
                if let Some(pk) = pubkey {
                    resolved_user_info = Some((uid, pk));
                    break;
                }
            }

            if let Some((uid, pubkey_hex)) = resolved_user_info {
                let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();
                let seed_val = if uid == "user-1" { 1 } else { 2 };
                let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed_val; 32]);

                use ed25519_dalek::Signer;
                let signature = signing_key.sign(&challenge_bytes);
                let sig_hex = const_hex::encode(signature.to_bytes());

                crate::services::auth::bankid::verify_hardware_auth_signature(
                    session_id, pubkey_hex, sig_hex,
                )
                .await?;
            } else {
                return Err(YntraError::NotFoundError(
                    "No user registered for smart card authentication".to_string(),
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let is_simulated = {
            let mut has_reader = false;
            if let Ok(ctx) = pcsc::Context::establish(pcsc::Scope::User) {
                let mut readers_buf = [0; 2048];
                if let Ok(mut r_list) = ctx.list_readers(&mut readers_buf) {
                    if r_list.next().is_some() {
                        has_reader = true;
                    }
                }
            }
            !has_reader
        };

        if is_simulated {
            #[cfg(not(debug_assertions))]
            {
                let _ = pin;
                let _ = challenge_hex;
                return Err(YntraError::AuthError(
                    "Hardware authentication simulation is disabled in release builds. No smart card reader detected.".to_string(),
                ));
            }

            #[cfg(debug_assertions)]
            {
                if pin.is_empty() {
                    return Err(YntraError::AuthError("PIN cannot be empty".to_string()));
                }

                let mut resolved_user_info = None;
                let mut stmt = conn
                    .prepare("SELECT id, metadata ->> 'siths_public_key' FROM users")
                    .await?;
                let mut rows = stmt.query(()).await?;
                while let Some(row) = rows.next().await? {
                    let uid: String = row.get(0)?;
                    let pubkey: Option<String> = row.get(1)?;
                    if let Some(pk) = pubkey {
                        resolved_user_info = Some((uid, pk));
                        break;
                    }
                }

                if let Some((uid, pubkey_hex)) = resolved_user_info {
                    let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();
                    let seed_val = if uid == "user-1" { 1 } else { 2 };
                    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed_val; 32]);

                    use ed25519_dalek::Signer;
                    let signature = signing_key.sign(&challenge_bytes);
                    let sig_hex = const_hex::encode(signature.to_bytes());

                    crate::services::auth::bankid::verify_hardware_auth_signature(
                        session_id, pubkey_hex, sig_hex,
                    )
                    .await?;
                } else {
                    return Err(YntraError::NotFoundError(
                        "No user registered for smart card authentication".to_string(),
                    ));
                }
            }
        } else {
            use pcsc::*;
            let ctx = Context::establish(Scope::User).map_err(|e| {
                YntraError::AuthError(format!("Smart Card subsystem error: {:?}", e))
            })?;
            let mut readers_buf = [0; 2048];
            let mut r_list = ctx.list_readers(&mut readers_buf).map_err(|e| {
                YntraError::AuthError(format!("Failed to list card readers: {:?}", e))
            })?;
            let reader = r_list.next().ok_or_else(|| {
                YntraError::AuthError("No smart card reader connected".to_string())
            })?;

            let card = ctx
                .connect(reader, ShareMode::Shared, Protocols::ANY)
                .map_err(|e| {
                    YntraError::AuthError(format!("Failed to connect to smart card: {:?}", e))
                })?;

            let mut pin_bytes = [0xFFu8; 8];
            let pin_len = std::cmp::min(pin.len(), 8);
            pin_bytes[..pin_len].copy_from_slice(&pin.as_bytes()[..pin_len]);
            let apdu_verify_pin = [
                0x00,
                0x20,
                0x00,
                0x80,
                0x08,
                pin_bytes[0],
                pin_bytes[1],
                pin_bytes[2],
                pin_bytes[3],
                pin_bytes[4],
                pin_bytes[5],
                pin_bytes[6],
                pin_bytes[7],
            ];

            let mut response_buf = [0u8; 258];
            let res = card
                .transmit(&apdu_verify_pin, &mut response_buf)
                .map_err(|e| {
                    YntraError::AuthError(format!("Failed to transmit verify PIN APDU: {:?}", e))
                })?;

            if res.len() < 2 || res[res.len() - 2..] != [0x90, 0x00] {
                return Err(YntraError::AuthError(
                    "Smart Card PIN verification failed".to_string(),
                ));
            }

            let unique_card_id = super::native::extract_unique_card_id(&card).map_err(|e| {
                YntraError::AuthError(format!("Failed to read card identity: {:?}", e))
            })?;

            let user_info: Option<(String, String)> = conn.query_row(
                "SELECT id, metadata ->> 'siths_public_key' FROM users WHERE metadata ->> 'siths_card_id' = ?1",
                crate::params![&unique_card_id],
                |r| Ok((r.get(0)?, r.get(1)?))
            ).await.ok();

            if let Some((_user_id, pubkey_hex)) = user_info {
                let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();

                // Construct PSO: Compute Digital Signature APDU command
                // CLA: 00, INS: 2A, P1: 9E, P2: 9A
                let mut apdu_sign = vec![0x00, 0x2A, 0x9E, 0x9A];
                apdu_sign.push(challenge_bytes.len() as u8);
                apdu_sign.extend_from_slice(&challenge_bytes);
                apdu_sign.push(0x00); // Le

                let mut signature_buf = [0u8; 258];
                let mut card_sign_ok = false;
                let mut sig_hex = String::new();

                if let Ok(res) = card.transmit(&apdu_sign, &mut signature_buf) {
                    if res.len() >= 2 && res[res.len() - 2..] == [0x90, 0x00] {
                        let signature_bytes = &res[..res.len() - 2];
                        sig_hex = const_hex::encode(signature_bytes);
                        card_sign_ok = true;
                    }
                }

                if card_sign_ok {
                    crate::services::auth::bankid::verify_hardware_auth_signature(
                        session_id, pubkey_hex, sig_hex,
                    )
                    .await?;
                } else {
                    return Err(YntraError::AuthError(
                        "On-device smart card cryptographic signing failed or is unsupported on the connected card.".to_string(),
                    ));
                }
            } else {
                return Err(YntraError::NotFoundError(
                    "No user is registered with this Smart Card".to_string(),
                ));
            }
        }
    }

    Ok(())
}
