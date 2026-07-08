use dioxus::prelude::*;
use yntra_core::{get_user_by_email, WorkspaceUser};
use crate::utils::get_supabase_user_email;

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
    oauth_channel: &(tokio::sync::mpsc::UnboundedSender<String>, std::sync::Arc<std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<String>>>>),
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
                        log::info!("[Desktop OAuth] Extracted token, calling get_supabase_user_email...");
                        match get_supabase_user_email(&token).await {
                            Ok(email) => {
                                log::info!("[Desktop OAuth] Supabase returned email: {}", email);
                                if let Ok(Some(user)) = get_user_by_email(email.clone()).await {
                                    log::info!("[Desktop OAuth] Found user in database: id={}, role={}", user.id, user.role);
                                    let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
                                    let mfa_enabled = prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                                    if mfa_enabled {
                                        log::info!("[Desktop OAuth] 2FA is enabled for user, showing 2FA modal");
                                        tf_user.set(Some(user.clone()));
                                    } else {
                                        log::info!("[Desktop OAuth] Logging in user...");
                                        active_uid.set(user.id.clone());
                                        active_role.set(user.role.clone());
                                        if user.role == "client" {
                                            active_sec.set("client_portal".to_string());
                                        } else {
                                            active_sec.set("dashboard".to_string());
                                        }
                                        let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                        setup_needed.set(is_new_invite);
                                        is_logged_in.set(true);
                                    }
                                } else {
                                    let err_msg = format!("User '{}' authenticated by Supabase is not registered in this Yntra workspace.", email);
                                    log::error!("[Desktop OAuth] Error: {}", err_msg);
                                    log_error.set(Some(err_msg));
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Supabase authentication failed: {}", e);
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
                    match get_supabase_user_email(&token).await {
                        Ok(email) => {
                            if let Ok(Some(user)) = get_user_by_email(email.clone()).await {
                                let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
                                let mfa_enabled = prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                                if mfa_enabled {
                                    tf_user.set(Some(user.clone()));
                                } else {
                                    active_uid.set(user.id.clone());
                                    active_role_sig.set(user.role.clone());
                                    if user.role == "client" {
                                        active_sec.set("client_portal".to_string());
                                    } else {
                                        active_sec.set("dashboard".to_string());
                                    }
                                    let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                    setup_needed.set(is_new_invite);
                                    is_logged_in.set(true);
                                }
                                
                                // Clean up url hash
                                let _ = dioxus::document::eval("window.location.hash = '';");
                            } else {
                                log_error.set(Some(format!("User '{}' authenticated by Supabase is not registered in this Yntra workspace.", email)));
                            }
                        }
                        Err(e) => {
                            log_error.set(Some(format!("Supabase authentication failed: {}", e)));
                        }
                    }
                }
            }
        });
    });
}
