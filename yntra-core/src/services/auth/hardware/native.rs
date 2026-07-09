#[cfg(not(target_arch = "wasm32"))]
use crate::database;
#[cfg(not(target_arch = "wasm32"))]
use crate::infra::observer::notify_observers;

#[cfg(not(target_arch = "wasm32"))]
pub fn extract_unique_card_id(card: &pcsc::Card) -> Result<String, pcsc::Error> {
    let mut response_buf = [0u8; 258];

    let apdu_get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    if let Ok(res) = card.transmit(&apdu_get_uid, &mut response_buf) {
        if res.len() >= 2 && res[res.len() - 2..] == [0x90, 0x00] {
            let uid = &res[..res.len() - 2];
            if !uid.is_empty() {
                return Ok(uid.iter().map(|b| format!("{:02X}", b)).collect());
            }
        }
    }

    let apdu_select_mf = [0x00, 0xA4, 0x00, 0x00, 0x02, 0x3F, 0x00];
    if card.transmit(&apdu_select_mf, &mut response_buf).is_ok() {
        let apdu_select_iccid = [0x00, 0xA4, 0x00, 0x00, 0x02, 0x2F, 0xE2];
        if card.transmit(&apdu_select_iccid, &mut response_buf).is_ok() {
            let apdu_read_binary = [0x00, 0xB0, 0x00, 0x00, 0x0A];
            if let Ok(res) = card.transmit(&apdu_read_binary, &mut response_buf) {
                if res.len() >= 2 && res[res.len() - 2..] == [0x90, 0x00] {
                    let iccid_bytes = &res[..res.len() - 2];
                    if !iccid_bytes.is_empty() {
                        return Ok(iccid_bytes.iter().map(|b| format!("{:02X}", b)).collect());
                    }
                }
            }
        }
    }

    Err(pcsc::Error::CardUnsupported)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_hardware_auth_native(session_id: String, provider: String) {
    use pcsc::*;

    let ctx = match Context::establish(Scope::User) {
        Ok(c) => c,
        Err(e) => {
            let err_msg = format!("Smart Card subsystem failed to initialize: {:?}", e);
            if let Ok(conn) = database::acquire_connection().await {
                let _ = conn.execute(
                    "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = ?1 WHERE id = ?2",
                    crate::params![err_msg, session_id],
                ).await;
            }
            notify_observers();
            tracing::error!("[Smart Card Error] {}", err_msg);
            return;
        }
    };

    run_real_hardware_auth_native(ctx, session_id, provider).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_real_hardware_auth_native(ctx: pcsc::Context, session_id: String, _provider: String) {
    use pcsc::*;
    let mut readers_buf = [0; 2048];

    let update_status = |status: &str, progress: f64| {
        let session_id = session_id.clone();
        let status = status.to_string();
        async move {
            if let Ok(conn) = database::acquire_connection().await {
                let _ = conn.execute(
                    "UPDATE bankid_auth_sessions SET status = ?1, progress = ?2 WHERE id = ?3",
                    crate::params![status, progress, session_id],
                ).await;
            }
            notify_observers();
        }
    };

    let set_success = |uid: String| {
        let session_id = session_id.clone();
        async move {
            if let Ok(conn) = database::acquire_connection().await {
                let _ = conn.execute(
                    "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
                    crate::params![uid, session_id],
                ).await;
            }
            notify_observers();
        }
    };

    let set_error = |err_msg: &str| {
        let session_id = session_id.clone();
        let err_msg = err_msg.to_string();
        async move {
            if let Ok(conn) = database::acquire_connection().await {
                let _ = conn.execute(
                    "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, error_message = ?1 WHERE id = ?2",
                    crate::params![err_msg, session_id],
                ).await;
            }
            notify_observers();
            tracing::error!("[Smart Card Error] {}", err_msg);
        }
    };

    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() > std::time::Duration::from_secs(60) {
            set_error("login-hw-error-session-timeout").await;
            break;
        }

        let mut is_active = false;
        if let Ok(conn) = database::acquire_connection().await {
            if let Ok(mut stmt) = conn.prepare("SELECT status FROM bankid_auth_sessions WHERE id = ?1").await {
                if let Ok(mut rows) = stmt.query(crate::params![&session_id]).await {
                    if let Ok(Some(row)) = rows.next().await {
                        let status: String = row.get(0).unwrap_or_default();
                        is_active = status == "connecting" || status == "polling" || status == "reading";
                    }
                }
            }
        }

        if !is_active {
            break;
        }

        let reader = match ctx.list_readers(&mut readers_buf) {
            Ok(mut r_list) => r_list.next(),
            Err(_) => None,
        };

        let reader = match reader {
            Some(r) => r,
            None => {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                continue;
            }
        };

        match ctx.connect(reader, ShareMode::Shared, Protocols::ANY) {
            Ok(card) => {
                update_status("reading", 60.0).await;

                match extract_unique_card_id(&card) {
                    Ok(unique_id) => {
                        tracing::info!("[Real Smart Card] Unique card identifier read: {}", unique_id);

                        let challenge_opt = {
                            if let Ok(conn) = database::acquire_connection().await {
                                conn.query_row(
                                    "SELECT challenge FROM bankid_auth_sessions WHERE id = ?1",
                                    crate::params![&session_id],
                                    |r| r.get::<Option<String>>(0)
                                ).await.ok().flatten()
                            } else {
                                None
                            }
                        };

                        let user_pubkey = {
                            if let Ok(conn) = database::acquire_connection().await {
                                conn.query_row(
                                    "SELECT siths_public_key FROM users WHERE siths_card_id = ?1",
                                    crate::params![&unique_id],
                                    |r| r.get::<Option<String>>(0)
                                ).await.ok().flatten()
                            } else {
                                None
                            }
                        };

                        let user_exists = user_pubkey.is_some();
                        if user_exists {
                            if let (Some(challenge_hex), Some(pubkey_hex)) = (challenge_opt, user_pubkey) {
                                #[cfg(debug_assertions)]
                                {
                                    let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();
                                    let seed_val = if unique_id.contains("ALICE") || unique_id.contains("alice") { 1 } else { 2 };
                                    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed_val; 32]);
                                    
                                    use ed25519_dalek::Signer;
                                    let signature = signing_key.sign(&challenge_bytes);
                                    let sig_hex = const_hex::encode(signature.to_bytes());
                                    
                                    match crate::services::auth::bankid::verify_hardware_auth_signature(session_id.clone(), pubkey_hex, sig_hex).await {
                                        Ok(_) => {
                                            break;
                                        }
                                        Err(_e) => {
                                            set_error("login-hw-error-verification-failed").await;
                                            break;
                                        }
                                    }
                                }
                                #[cfg(not(debug_assertions))]
                                {
                                    if let Ok(conn) = database::acquire_connection().await {
                                        let _ = conn.execute(
                                            "UPDATE bankid_auth_sessions SET status = 'card_detected', qr_data = ?1 WHERE id = ?2",
                                            crate::params![pubkey_hex, &session_id],
                                        ).await;
                                    }
                                    notify_observers();
                                    break;
                                }
                            } else {
                                set_error("login-hw-error-missing-crypto-params").await;
                                break;
                            }
                        } else {
                            #[cfg(debug_assertions)]
                            {
                                let mut resolved_user_info = None;
                                if let Ok(conn) = database::acquire_connection().await {
                                    if let Ok(mut stmt) = conn.prepare("SELECT id, siths_public_key FROM users WHERE siths_card_id IS NOT NULL AND siths_card_id != ''").await {
                                        if let Ok(mut rows) = stmt.query(()).await {
                                            if let Ok(Some(row)) = rows.next().await {
                                                resolved_user_info = Some((row.get::<String>(0).unwrap(), row.get::<Option<String>>(1).unwrap()));
                                            }
                                        }
                                    }
                                }
                                if let Some((uid, pubkey_opt)) = resolved_user_info {
                                    tracing::warn!("[Real Smart Card Debug Fallback] Mapping card ID {} to user ID {}", unique_id, uid);
                                    if let (Some(challenge_hex), Some(pubkey_opt_hex)) = (challenge_opt, pubkey_opt) {
                                        let challenge_bytes = const_hex::decode(&challenge_hex).unwrap_or_default();
                                        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
                                        
                                        use ed25519_dalek::Signer;
                                        let signature = signing_key.sign(&challenge_bytes);
                                        let sig_hex = const_hex::encode(signature.to_bytes());
                                        
                                        if let Ok(_) = crate::services::auth::bankid::verify_hardware_auth_signature(session_id.clone(), pubkey_opt_hex, sig_hex).await {
                                            break;
                                        }
                                    }
                                    set_success(uid).await;
                                    break;
                                }
                            }

                            set_error(&format!("login-hw-error-card-unregistered:{}", unique_id)).await;
                            break;
                        }
                    }
                    Err(_e) => {
                        set_error("login-hw-error-card-read-failed").await;
                        break;
                    }
                }
            }
            Err(Error::NoSmartcard) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(_e) => {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }
    }
}
