use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, get_oauth_login_status, get_users, initiate_oauth_login};

fn extract_access_token(hash: &str) -> Option<String> {
    let hash_clean = hash.trim_start_matches('#').trim_start_matches('?');
    for part in hash_clean.split('&') {
        let mut kv = part.splitn(2, '=');
        if let (Some("access_token"), Some(v)) = (kv.next(), kv.next()) {
            return Some(v.to_string());
        }
    }
    None
}

pub fn init_oauth_handlers(
    active_user_id: Signal<String>,
    active_section: Signal<String>,
    needs_setup: Signal<bool>,
    logged_in: Signal<bool>,
    login_error: Signal<Option<String>>,
    two_factor_user: Signal<Option<WorkspaceUser>>,
    active_user_role: Signal<String>,
    oauth_channel: &(
        tokio::sync::mpsc::UnboundedSender<String>,
        std::sync::Arc<std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<String>>>>,
    ),
) {
    // Listen for desktop OAuth loopback redirects
    let oauth_rx_channel = oauth_channel.clone();
    use_effect(move || {
        let mut rx_opt = oauth_rx_channel.1.lock().unwrap();
        if let Some(mut rx) = rx_opt.take() {
            let mut active_uid = active_user_id;
            let mut active_sec = active_section;
            let mut setup_needed = needs_setup;
            let mut is_logged_in = logged_in;
            let mut log_error = login_error;
            let mut tf_user = two_factor_user;
            let mut active_role = active_user_role;

            spawn(async move {
                while let Some(hash) = rx.recv().await {
                    log::info!("[Desktop OAuth] Receiver captured hash callback!");
                    if let Some(token) = extract_access_token(&hash) {
                        log::info!(
                            "[Desktop OAuth] Extracted token, initiating OAuth login in DB..."
                        );
                        match initiate_oauth_login("supabase".to_string(), token).await {
                            Ok(session_id) => {
                                log::info!(
                                    "[Desktop OAuth] Created auth session ID: {}",
                                    session_id
                                );
                                loop {
                                    match get_oauth_login_status(session_id.clone()).await {
                                        Ok(Some(session)) => {
                                            match session.status.as_str() {
                                                "success" => {
                                                    let uid =
                                                        session.authenticated_user_id.unwrap();
                                                    log::info!(
                                                        "[Desktop OAuth] Auth success, loading user ID: {}",
                                                        uid
                                                    );
                                                    if let Ok(all_users) =
                                                        get_users(uid.clone()).await
                                                    {
                                                        if let Some(user) = all_users
                                                            .into_iter()
                                                            .find(|u| u.id == uid)
                                                        {
                                                            let prefs: serde_json::Value =
                                                                serde_json::from_str(
                                                                    &user.preferences,
                                                                )
                                                                .unwrap_or_default();
                                                            let mfa_enabled = prefs
                                                                .get("two_factor_enabled")
                                                                .and_then(|v| v.as_bool())
                                                                .unwrap_or(false);
                                                            if mfa_enabled {
                                                                tf_user.set(Some(user.clone()));
                                                            } else {
                                                                active_uid.set(user.id.clone());
                                                                active_role.set(user.role.clone());
                                                                if user.role == "client" {
                                                                    active_sec.set(
                                                                        "client_portal".to_string(),
                                                                    );
                                                                } else {
                                                                    active_sec.set(
                                                                        "dashboard".to_string(),
                                                                    );
                                                                }
                                                                let is_new_invite =
                                                                    user.phone.is_none()
                                                                        || user
                                                                            .phone
                                                                            .as_ref()
                                                                            .map(|p| p.is_empty())
                                                                            .unwrap_or(true);
                                                                setup_needed.set(is_new_invite);
                                                                is_logged_in.set(true);
                                                            }
                                                        }
                                                    }
                                                    break;
                                                }
                                                "error" => {
                                                    let err_msg =
                                                        session.error_message.unwrap_or_else(
                                                            || "Unknown error".to_string(),
                                                        );
                                                    log::error!(
                                                        "[Desktop OAuth] Auth error: {}",
                                                        err_msg
                                                    );
                                                    log_error.set(Some(err_msg));
                                                    break;
                                                }
                                                _ => {
                                                    // Pending, wait a moment
                                                    crate::utils::sleep_ms(100).await;
                                                }
                                            }
                                        }
                                        _ => {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Failed to initiate login session: {}", e);
                                log::error!("[Desktop OAuth] Error: {}", err_msg);
                                log_error.set(Some(err_msg));
                            }
                        }
                    }
                }
            });
        }
    });

    // Check for Supabase redirect callback (hash fragment #access_token=...)
    let mut active_role_sig = active_user_role;
    use_effect(move || {
        let mut active_uid = active_user_id;
        let mut active_sec = active_section;
        let mut setup_needed = needs_setup;
        let mut is_logged_in = logged_in;
        let mut log_error = login_error;
        let mut tf_user = two_factor_user;

        spawn(async move {
            let mut eval = dioxus::document::eval("dioxus.send(window.location.hash);");
            if let Ok(serde_json::Value::String(hash)) = eval.recv::<serde_json::Value>().await {
                if let Some(token) = extract_access_token(&hash) {
                    log::info!("[Web OAuth] Extracted token, initiating OAuth login in DB...");
                    match initiate_oauth_login("supabase".to_string(), token).await {
                        Ok(session_id) => {
                            loop {
                                match get_oauth_login_status(session_id.clone()).await {
                                    Ok(Some(session)) => {
                                        match session.status.as_str() {
                                            "success" => {
                                                let uid = session.authenticated_user_id.unwrap();
                                                if let Ok(all_users) = get_users(uid.clone()).await
                                                {
                                                    if let Some(user) =
                                                        all_users.into_iter().find(|u| u.id == uid)
                                                    {
                                                        let prefs: serde_json::Value =
                                                            serde_json::from_str(&user.preferences)
                                                                .unwrap_or_default();
                                                        let mfa_enabled = prefs
                                                            .get("two_factor_enabled")
                                                            .and_then(|v| v.as_bool())
                                                            .unwrap_or(false);
                                                        if mfa_enabled {
                                                            tf_user.set(Some(user.clone()));
                                                        } else {
                                                            active_uid.set(user.id.clone());
                                                            active_role_sig.set(user.role.clone());
                                                            if user.role == "client" {
                                                                active_sec.set(
                                                                    "client_portal".to_string(),
                                                                );
                                                            } else {
                                                                active_sec
                                                                    .set("dashboard".to_string());
                                                            }
                                                            let is_new_invite =
                                                                user.phone.is_none()
                                                                    || user
                                                                        .phone
                                                                        .as_ref()
                                                                        .map(|p| p.is_empty())
                                                                        .unwrap_or(true);
                                                            setup_needed.set(is_new_invite);
                                                            is_logged_in.set(true);
                                                        }
                                                    }
                                                }
                                                // Clean up url hash
                                                let _ = dioxus::document::eval(
                                                    "window.location.hash = '';",
                                                );
                                                break;
                                            }
                                            "error" => {
                                                log_error.set(Some(
                                                    session.error_message.unwrap_or_else(|| {
                                                        "Unknown error".to_string()
                                                    }),
                                                ));
                                                break;
                                            }
                                            _ => {
                                                crate::utils::sleep_ms(100).await;
                                            }
                                        }
                                    }
                                    _ => {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log_error.set(Some(format!("Failed to initiate login session: {}", e)));
                        }
                    }
                }
            }
        });
    });
}
