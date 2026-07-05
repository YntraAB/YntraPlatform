use crate::{YntraError, WorkspaceUser};
use crate::database;
use crate::infra::observer::notify_observers;

#[uniffi::export]
pub async fn authenticate_with_siths(card_id: String) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, siths_card_id, nfc_badge_uid, updated_at, sync_status, personal_number FROM users WHERE siths_card_id = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![card_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id: Option<String> = row.get(1)?;
        let raw_pnum: Option<String> = row.get(11)?;
        Ok(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: row.get(6)?,
            siths_card_id: row.get(7)?,
            nfc_badge_uid: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
            personal_number: crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id),
        })
    } else {
        Err(YntraError::NotFoundError("No user registered with this SITHS card".to_string()))
    }
}

#[uniffi::export]
pub async fn authenticate_with_nfc(badge_uid: String) -> Result<WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, siths_card_id, nfc_badge_uid, updated_at, sync_status, personal_number FROM users WHERE nfc_badge_uid = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![badge_uid]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id: Option<String> = row.get(1)?;
        let raw_pnum: Option<String> = row.get(11)?;
        Ok(WorkspaceUser {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            email: row.get(2)?,
            full_name: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
            preferences: row.get(6)?,
            siths_card_id: row.get(7)?,
            nfc_badge_uid: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row.get(10)?,
            personal_number: crate::infra::crypto::decrypt_opt_field(raw_pnum, ws_id),
        })
    } else {
        Err(YntraError::NotFoundError("No user registered with this NFC badge".to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
struct SendFuture<F>(pub F);

#[cfg(target_arch = "wasm32")]
unsafe impl<F> Send for SendFuture<F> {}

#[cfg(target_arch = "wasm32")]
impl<F: std::future::Future> std::future::Future for SendFuture<F> {
    type Output = F::Output;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        unsafe {
            let mut_self = self.get_unchecked_mut();
            let inner = std::pin::Pin::new_unchecked(&mut mut_self.0);
            inner.poll(cx)
        }
    }
}

pub async fn sleep_ms(ms: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
            }
        });
        let _ = SendFuture(wasm_bindgen_futures::JsFuture::from(promise)).await;
    }
}

#[uniffi::export]
pub async fn run_hardware_auth_simulation(session_id: String, provider: String) -> Result<(), YntraError> {
    // 1. Wait 600ms (connecting)
    sleep_ms(600).await;
    let conn = database::acquire_connection().await?;
    conn.execute(
        "UPDATE bankid_auth_sessions SET status = 'polling', progress = 20.0 WHERE id = ?1",
        crate::params![&session_id],
    ).await?;
    notify_observers();

    // 2. Wait 1500ms (polling)
    sleep_ms(1500).await;
    
    // Check registered users for active credentials in database
    let mut resolved_user_id = None;
    let mut stmt = conn.prepare("SELECT id, siths_card_id, nfc_badge_uid FROM users").await?;
    let mut rows = stmt.query(()).await?;
    while let Some(row) = rows.next().await? {
        let uid: String = row.get(0)?;
        let siths: Option<String> = row.get(1)?;
        let nfc: Option<String> = row.get(2)?;
        
        let has_siths = siths.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let has_nfc = nfc.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        
        if (provider == "siths" && has_siths)
            || (provider == "nfc" && has_nfc)
            || (provider == "card_or_badge" && (has_siths || has_nfc))
        {
            resolved_user_id = Some(uid);
            break;
        }
    }

    if let Some(uid) = resolved_user_id {
        // 3. Update status to reading, progress = 60.0
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'reading', progress = 60.0 WHERE id = ?1",
            crate::params![&session_id],
        ).await?;
        notify_observers();

        // 4. Wait 600ms (reading)
        sleep_ms(600).await;

        // 5. Update status to success, progress = 100.0, authenticated_user_id
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'success', progress = 100.0, authenticated_user_id = ?1 WHERE id = ?2",
            crate::params![&uid, &session_id],
        ).await?;
        notify_observers();
    } else {
        // Update to error state
        conn.execute(
            "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0 WHERE id = ?1",
            crate::params![&session_id],
        ).await?;
        notify_observers();
    }
    
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_hardware_auth_native(session_id: String, provider: String) {
    use pcsc::*;

    // Establish context
    let ctx = match Context::establish(Scope::User) {
        Ok(c) => c,
        Err(_) => return, // Silent exit on PC/SC subsystem failure
    };

    // Run real hardware polling directly! It will handle reader plug/unplug events dynamically.
    run_real_hardware_auth_native(ctx, session_id, provider).await;
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_unique_card_id(card: &pcsc::Card) -> Result<String, pcsc::Error> {
    let mut response_buf = [0u8; 258];

    // 1. Try to get card UID via GET DATA (FF CA 00 00 00).
    let apdu_get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    if let Ok(res) = card.transmit(&apdu_get_uid, &mut response_buf) {
        if res.len() >= 2 && res[res.len() - 2..] == [0x90, 0x00] {
            let uid = &res[..res.len() - 2];
            if !uid.is_empty() {
                return Ok(uid.iter().map(|b| format!("{:02X}", b)).collect());
            }
        }
    }

    // 2. Try to read EF.ICCID (file 2FE2 under Master File 3F00).
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
async fn run_real_hardware_auth_native(ctx: pcsc::Context, session_id: String, _provider: String) {
    use pcsc::*;
    let mut readers_buf = [0; 2048];

    // Status helper closures
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
                    "UPDATE bankid_auth_sessions SET status = 'error', progress = 0.0, pin = ?1 WHERE id = ?2",
                    crate::params![err_msg, session_id],
                ).await;
            }
            notify_observers();
            tracing::error!("[Smart Card Error] {}", err_msg);
        }
    };

    // Polling loop
    loop {
        // Check if session still exists and is active
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

        // Try connecting to card
        match ctx.connect(reader, ShareMode::Shared, Protocols::ANY) {
            Ok(card) => {
                update_status("reading", 60.0).await;

                match extract_unique_card_id(&card) {
                    Ok(unique_id) => {
                        tracing::info!("[Real Smart Card] Unique card identifier read: {}", unique_id);

                        let siths_auth_result = authenticate_with_siths(unique_id.clone()).await;
                        match siths_auth_result {
                            Ok(user) => {
                                set_success(user.id).await;
                                break;
                            }
                            Err(_) => {
                                #[cfg(debug_assertions)]
                                {
                                    let mut resolved_user_id = None;
                                    if let Ok(conn) = database::acquire_connection().await {
                                        if let Ok(mut stmt) = conn.prepare("SELECT id FROM users WHERE siths_card_id IS NOT NULL AND siths_card_id != ''").await {
                                            if let Ok(mut rows) = stmt.query(()).await {
                                                if let Ok(Some(row)) = rows.next().await {
                                                    resolved_user_id = Some(row.get::<String>(0).unwrap());
                                                }
                                            }
                                        }
                                    }
                                    if let Some(uid) = resolved_user_id {
                                        tracing::warn!("[Real Smart Card Debug Fallback] Mapping card ID {} to user ID {}", unique_id, uid);
                                        set_success(uid).await;
                                        break;
                                    }
                                }

                                set_error(&format!("Kortet med ID {} är inte registrerat i systemet.", unique_id)).await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        set_error(&format!("Kunde inte läsa kortets unika identifierare (ICCID/UID): {:?}", e)).await;
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

pub async fn run_hardware_auth(session_id: String, provider: String) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_hardware_auth_native(session_id, provider).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = run_hardware_auth_simulation(session_id, provider).await;
    }
}

