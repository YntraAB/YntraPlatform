use crate::{BankIdAuthSession, YntraError};
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
use crate::database;
#[cfg(not(target_arch = "wasm32"))]
use crate::infra::observer::notify_observers;

#[cfg(target_arch = "wasm32")]
use crate::infra::wasm_store;
#[cfg(target_arch = "wasm32")]
use crate::infra::observer::notify_observers;
#[cfg(target_arch = "wasm32")]
use js_sys;
#[cfg(target_arch = "wasm32")]
use web_sys;

#[cfg(not(target_arch = "wasm32"))]
use super::hardware::run_hardware_auth_native;

#[cfg(all(debug_assertions, target_arch = "wasm32"))]
use super::hardware::run_hardware_auth_simulation_wasm;

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
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO bankid_auth_sessions (id, target_role, provider, status, pin, qr_data, progress, authenticated_user_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            crate::params![
                session.id,
                session.target_role,
                session.provider,
                session.status,
                session.pin,
                session.qr_data,
                session.progress,
                session.authenticated_user_id,
                session.created_at
            ],
        ).await?;

        notify_observers();

        if provider == "card_or_badge" || provider == "siths" || provider == "nfc" {
            let session_id_clone = session.id.clone();
            let provider_clone = provider.clone();
            tokio::spawn(async move {
                run_hardware_auth_native(session_id_clone, provider_clone).await;
            });
        }

        if provider == "se_bankid" || provider == "dk_mitid" {
            let session_id_clone = session.id.clone();
            tokio::spawn(async move {
                for elapsed in 1..=30 {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    if let Ok(conn) = database::native::acquire_connection().await {
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

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.bankid_sessions.push(session.clone());
        notify_observers();

        #[cfg(debug_assertions)]
        if provider == "card_or_badge" || provider == "siths" || provider == "nfc" {
            let session_id_clone = session.id.clone();
            let provider_clone = provider.clone();
            wasm_bindgen_futures::spawn_local(async move {
                run_hardware_auth_simulation_wasm(session_id_clone, provider_clone).await;
            });
        }

        #[cfg(debug_assertions)]
        if provider == "se_bankid" || provider == "dk_mitid" {
            let session_id_clone = session.id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                for elapsed in 1..=30 {
                    delay_wasm(1000).await;
                    let mut session_found = false;
                    let mut status_qr_scan = false;
                    {
                        let mut store = wasm_store::get_store().lock().unwrap();
                        if let Some(s) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id_clone) {
                            session_found = true;
                            if s.status == "qr_scan" {
                                status_qr_scan = true;
                                s.qr_data = format!("bankid.status.qrs.format.{}.{}", elapsed, session_id_clone);
                            }
                        }
                    }
                    if session_found && status_qr_scan {
                        notify_observers();
                    } else {
                        break;
                    }
                }
            });
        }

        Ok(session)
    }
}

#[uniffi::export]
pub async fn get_bankid_auth_session(session_id: String) -> Result<Option<BankIdAuthSession>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        let mut stmt = conn.prepare(
            "SELECT id, target_role, provider, status, pin, qr_data, progress, authenticated_user_id, created_at FROM bankid_auth_sessions WHERE id = ?1"
        ).await?;

        let mut rows = stmt.query(crate::params![session_id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(BankIdAuthSession {
                id: row.get(0)?,
                target_role: row.get(1)?,
                provider: row.get(2)?,
                status: row.get(3)?,
                pin: row.get(4)?,
                qr_data: row.get(5)?,
                progress: row.get(6)?,
                authenticated_user_id: row.get(7)?,
                created_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let session = store.bankid_sessions.iter().find(|s| s.id == session_id).cloned();
        Ok(session)
    }
}

#[allow(dead_code)]
fn check_birthdate_match(personal_number: &str, birthdate_ddmmyy: &str) -> bool {
    if birthdate_ddmmyy.len() != 6 {
        return false;
    }
    let digits: String = personal_number.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 11 {
        // DDMMYYXXXXX (Norwegian)
        digits.starts_with(birthdate_ddmmyy)
    } else if digits.len() == 12 {
        // YYYYMMDDXXXX
        if digits.len() < 8 { return false; }
        let yy = &digits[2..4];
        let mm = &digits[4..6];
        let dd = &digits[6..8];
        let expected_ddmmyy = format!("{}{}{}", dd, mm, yy);
        expected_ddmmyy == birthdate_ddmmyy
    } else if digits.len() == 10 {
        // YYMMDDXXXX
        if digits.len() < 6 { return false; }
        let yy = &digits[0..2];
        let mm = &digits[2..4];
        let dd = &digits[4..6];
        let expected_ddmmyy = format!("{}{}{}", dd, mm, yy);
        expected_ddmmyy == birthdate_ddmmyy
    } else {
        digits.contains(birthdate_ddmmyy)
    }
}

#[uniffi::export]
pub async fn submit_bankid_pin(session_id: String, pin: String) -> Result<(), YntraError> {
    #[cfg(debug_assertions)]
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // 1. First set status to verifying and progress = 0.0
            {
                let conn = database::native::acquire_connection().await?;
                conn.execute(
                    "UPDATE bankid_auth_sessions SET pin = ?1, status = 'verifying', progress = 0.0 WHERE id = ?2",
                    crate::params![pin, session_id],
                ).await?;
            }
            notify_observers();

            // 2. Spawn a background thread that ticks progress up to 100% and then completes the login!
            let session_id_clone = session_id.clone();
            let pin_clone = pin.clone();
            tokio::spawn(async move {
                let mut resolved_id = None;
                if let Ok(conn) = database::native::acquire_connection().await {
                    if pin_clone.contains('|') {
                        let parts: Vec<&str> = pin_clone.split('|').collect();
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
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        if let Ok(conn) = database::native::acquire_connection().await {
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
                    if let Ok(conn) = database::native::acquire_connection().await {
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

        #[cfg(target_arch = "wasm32")]
        {
            let mut store = wasm_store::get_store().lock().unwrap();
            let target_role = store.bankid_sessions.iter().find(|s| s.id == session_id).map(|s| s.target_role.clone());
            if let Some(role_str) = target_role {
                let mut db_user_id = None;
                if pin.contains('|') {
                    let parts: Vec<&str> = pin.split('|').collect();
                    if parts.len() == 2 {
                        let phone_input = parts[0];
                        db_user_id = store.users.iter()
                            .find(|u| u.phone.as_deref() == Some(phone_input) || u.phone.as_ref().map(|p| p.ends_with(phone_input)).unwrap_or(false))
                            .map(|u| u.id.clone());
                    }
                }
                if db_user_id.is_none() {
                    db_user_id = store.users.iter().find(|u| u.role == role_str).map(|u| u.id.clone());
                }
                
                if let Some(resolved_id) = db_user_id {
                    if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                        session.pin = pin;
                        session.status = "success".to_string();
                        session.progress = 100.0;
                        session.authenticated_user_id = Some(resolved_id);
                    }
                } else {
                    if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                        session.pin = pin;
                        session.status = "error".to_string();
                        session.progress = 0.0;
                        session.authenticated_user_id = None;
                    }
                }
            }
            notify_observers();
            Ok(())
        }
    }

    #[cfg(not(debug_assertions))]
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Transition status to verifying
            {
                let conn = database::native::acquire_connection().await?;
                conn.execute(
                    "UPDATE bankid_auth_sessions SET pin = ?1, status = 'verifying', progress = 0.0 WHERE id = ?2",
                    crate::params![&pin, &session_id],
                ).await?;
            }
            notify_observers();

            // Retrieve provider to decide validation
            let provider = {
                let conn = database::native::acquire_connection().await?;
                let mut stmt = conn.prepare("SELECT provider FROM bankid_auth_sessions WHERE id = ?1").await?;
                let mut rows = stmt.query(crate::params![&session_id]).await?;
                if let Some(row) = rows.next().await? {
                    row.get::<String>(0)?
                } else {
                    return Err(YntraError::NotFoundError("Session not found".to_string()));
                }
            };

            if provider == "no_bankid" {
                if !pin.contains('|') {
                    let conn = database::native::acquire_connection().await?;
                    let _ = conn.execute("UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0 WHERE id = ?1", crate::params![&session_id]).await;
                    notify_observers();
                    return Err(YntraError::ValidationError("Invalid Norwegian BankID payload".to_string()));
                }
                let parts: Vec<&str> = pin.split('|').collect();
                let phone_input = parts[0];
                let birthdate_input = parts[1];

                let user_info = {
                    let conn = database::native::acquire_connection().await?;
                    let mut stmt = conn.prepare("SELECT id, personal_number, workspace_id FROM users WHERE phone = ?1 OR phone LIKE '%' || ?1 LIMIT 1").await?;
                    let mut rows = stmt.query(crate::params![phone_input]).await?;
                    if let Some(row) = rows.next().await? {
                        let id: String = row.get(0)?;
                        let raw_pnum: Option<String> = row.get(1)?;
                        let ws_id: Option<String> = row.get(2)?;
                        let decrypted_pnum = crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id);
                        Some((id, decrypted_pnum))
                    } else {
                        None
                    }
                };

                if let Some((uid, Some(pnum))) = user_info {
                    if check_birthdate_match(&pnum, birthdate_input) {
                        let conn = database::native::acquire_connection().await?;
                        conn.execute(
                            "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
                            crate::params![uid, &session_id],
                        ).await?;
                        notify_observers();
                        return Ok(());
                    }
                }

                // Error fallback
                let conn = database::native::acquire_connection().await?;
                let _ = conn.execute("UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0 WHERE id = ?1", crate::params![&session_id]).await;
                notify_observers();
                Err(YntraError::AuthError("Norwegian BankID verification failed: invalid credentials".to_string()))
            } else {
                // For other providers in production: we keep in verifying status and wait for host validation
                Ok(())
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut store = wasm_store::get_store().lock().unwrap();
            let session_provider = store.bankid_sessions.iter().find(|s| s.id == session_id).map(|s| s.provider.clone());
            if let Some(provider) = session_provider {
                if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                    session.pin = pin.clone();
                    session.status = "verifying".to_string();
                    session.progress = 0.0;
                }
                
                if provider == "no_bankid" {
                    if !pin.contains('|') {
                        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                            session.status = "error".to_string();
                        }
                        notify_observers();
                        return Err(YntraError::ValidationError("Invalid Norwegian BankID payload".to_string()));
                    }
                    let parts: Vec<&str> = pin.split('|').collect();
                    let phone_input = parts[0];
                    let birthdate_input = parts[1];

                    let matched_user = store.users.iter().find(|u| {
                        let phone_matches = u.phone.as_deref() == Some(phone_input) || u.phone.as_ref().map(|p| p.ends_with(phone_input)).unwrap_or(false);
                        phone_matches && u.personal_number.as_ref().map(|pnum| check_birthdate_match(pnum, birthdate_input)).unwrap_or(false)
                    }).cloned();

                    if let Some(user) = matched_user {
                        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                            session.status = "success".to_string();
                            session.progress = 100.0;
                            session.authenticated_user_id = Some(user.id.clone());
                        }
                        notify_observers();
                        Ok(())
                    } else {
                        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
                            session.status = "error".to_string();
                        }
                        notify_observers();
                        Err(YntraError::AuthError("Norwegian BankID verification failed: invalid credentials".to_string()))
                    }
                } else {
                    notify_observers();
                    Ok(())
                }
            } else {
                Err(YntraError::NotFoundError("Session not found".to_string()))
            }
        }
    }
}

#[uniffi::export]
pub async fn complete_auth_session(session_id: String, user_id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        // Verify user exists
        if !store.users.iter().any(|u| u.id == user_id) {
            return Err(YntraError::NotFoundError("User does not exist".to_string()));
        }
        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
            session.status = "success".to_string();
            session.progress = 100.0;
            session.authenticated_user_id = Some(user_id);
            notify_observers();
            Ok(())
        } else {
            Err(YntraError::NotFoundError("Session not found".to_string()))
        }
    }
}

#[uniffi::export]
pub async fn fail_auth_session(session_id: String, error_msg: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, pin = ?1 WHERE id = ?2",
            crate::params![error_msg, session_id],
        ).await?;
        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
            session.status = "error".to_string();
            session.progress = 0.0;
            session.pin = error_msg;
            notify_observers();
            Ok(())
        } else {
            Err(YntraError::NotFoundError("Session not found".to_string()))
        }
    }
}

#[uniffi::export]
pub async fn update_auth_session_status(session_id: String, status: String, progress: f64) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE bankid_auth_sessions SET status = ?1, progress = ?2 WHERE id = ?3",
            crate::params![status, progress, session_id],
        ).await?;
        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(session) = store.bankid_sessions.iter_mut().find(|s| s.id == session_id) {
            session.status = status;
            session.progress = progress;
            notify_observers();
            Ok(())
        } else {
            Err(YntraError::NotFoundError("Session not found".to_string()))
        }
    }
}

#[cfg(all(debug_assertions, target_arch = "wasm32"))]
async fn delay_wasm(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
