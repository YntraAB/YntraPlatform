use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::Workspace;
use yntra_core::WorkspaceUser;

pub mod bankid_modal;
pub mod hardware_modal;
pub mod invite_modal;
pub mod styles;

pub mod two_factor_modal;

use bankid_modal::BankIdModal;
use invite_modal::InviteModal;
use styles::get_keyframes_css;

use two_factor_modal::TwoFactorModal;

#[derive(Props, Clone)]
pub struct LoginViewProps {
    pub scanning_state: Signal<String>,
    pub show_bankid_modal: Signal<bool>,
    pub login_error: Signal<Option<String>>,
    pub login_tab: Signal<String>,
    pub auth_region: Signal<String>,
    pub logged_in: Signal<bool>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub login_email: Signal<String>,
    pub login_password: Signal<String>,
    pub workspace: Workspace,
    pub users: Vec<WorkspaceUser>,
    pub needs_setup: Signal<bool>,
    pub db_trigger: Signal<u32>,
    pub two_factor_user: Signal<Option<WorkspaceUser>>,
    pub on_desktop_oauth: EventHandler<String>,
}

impl PartialEq for LoginViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LoginView(props: LoginViewProps) -> Element {
    let is_scanning = *props.scanning_state.read() == "scanning";
    let _show_modal = *props.show_bankid_modal.read();

    let region = props.auth_region.read().clone();

    let dropdown_label = match props.auth_region.read().as_str() {
        "sv" => "Svenska",
        "no" => "Norsk",
        "da" => "Dansk",
        _ => "English",
    };

    let state = use_context::<crate::state::AppState>();
    let active_user_id = props.active_user_id;
    let active_section = props.active_section;
    let logged_in = props.logged_in;
    let show_bankid_modal = props.show_bankid_modal;
    let mut auth_region = props.auth_region;

    let needs_setup = props.needs_setup;
    let db_trigger = props.db_trigger;
    let users = props.users.clone();
    let users_for_effect = users.clone();
    let users_for_dev = users.clone();

    // BankID Flow State Signals
    let bankid_flow_state = use_signal(|| "idle".to_string()); // "idle" | "qr_scan" | "pending_pin" | "verifying" | "success"
    let bankid_progress = use_signal(|| 0.0f32);
    let bankid_qr_data = use_signal(String::new);
    let bankid_pin = use_signal(String::new);
    #[allow(unused_mut)]
    let mut active_session_id = use_signal(|| Option::<String>::None);
    #[allow(unused_mut)]
    let mut active_session_token = use_signal(|| Option::<String>::None);
    let mut provider_val = use_signal(|| "se_bankid".to_string());

    // Norway form states
    let norway_mobile = use_signal(String::new);
    let norway_birthdate = use_signal(String::new);

    // 2FA Auth states
    let two_factor_user = props.two_factor_user;
    let two_factor_code = use_signal(|| vec!["".to_string(); 6]);
    let two_factor_error = use_signal(|| Option::<String>::None);
    let mut dropdown_open = use_signal(|| false);

    let show_invite_modal = use_signal(|| false);
    let invite_code_input = use_signal(String::new);
    let invite_error = use_signal(|| Option::<String>::None);

    let show_hardware_modal = use_signal(|| false);
    #[allow(unused_variables, unused_mut)]
    let mut hardware_auth_type = use_signal(|| "siths".to_string()); // "siths" | "nfc"
    #[allow(unused_variables, unused_mut)]
    let mut hardware_reader_status = use_signal(|| "connecting".to_string()); // "connecting" | "polling" | "reading" | "error" | "success"
    #[allow(unused_variables, unused_mut)]
    let mut hardware_error_msg = use_signal(|| Option::<String>::None);
    let toast = dioxus_primitives::toast::use_toast();
    let pin_prompted_sessions = use_signal(std::collections::HashSet::<String>::new);
    let last_error_shown = use_signal(|| Option::<String>::None);

    // 1. Web NFC (NDEFReader) Passive Background Listener (Runs only on Web/WASM target if browser supports Web NFC)
    use_effect(move || {
        let mut active_user_id = props.active_user_id;
        let mut active_section = props.active_section;
        let mut needs_setup = props.needs_setup;
        let mut logged_in = props.logged_in;
        let mut two_factor_user = props.two_factor_user;

        let mut ev = dioxus::document::eval(
            r#"
            if (typeof NDEFReader !== 'undefined') {
                console.log("[Web NFC] Browser supports NDEFReader. Starting passive scanning...");
                const ndef = new NDEFReader();
                ndef.scan().then(() => {
                    console.log("[Web NFC] Scan started passively. Ready to scan badges.");
                    ndef.onreading = (event) => {
                        console.log("[Web NFC] Scanned NFC Serial:", event.serialNumber);
                        dioxus.send(event.serialNumber);
                    };
                }).catch(err => {
                    console.warn("[Web NFC] Scanner initialization failed:", err);
                });
            }
        "#,
        );

        spawn(async move {
            if let Ok(serde_json::Value::String(badge_uid)) = ev.recv().await {
                let mut auth_res = yntra_core::authenticate_with_nfc(badge_uid.clone(), None).await;
                if let Err(yntra_core::YntraError::AuthError(ref msg)) = auth_res {
                    if msg.contains("PIN") {
                        let mut eval_prompt = dioxus::document::eval(
                            r#"
                            try {
                                let pin = prompt("Vänligen ange din NFC-PIN / Please enter your NFC PIN:");
                                dioxus.send(pin || "");
                            } catch(e) {
                                dioxus.send("");
                            }
                        "#,
                        );
                        if let Ok(serde_json::Value::String(provided_pin)) =
                            eval_prompt.recv().await
                        {
                            if !provided_pin.is_empty() {
                                auth_res = yntra_core::authenticate_with_nfc(
                                    badge_uid,
                                    Some(provided_pin),
                                )
                                .await;
                            }
                        }
                    }
                }
                if let Ok(user) = auth_res {
                    let prefs: serde_json::Value =
                        serde_json::from_str(&user.preferences).unwrap_or_default();
                    let mfa_enabled = prefs
                        .get("two_factor_enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if mfa_enabled {
                        two_factor_user.set(Some(user));
                    } else {
                        active_user_id.set(user.id.clone());
                        if user.role == "client" {
                            active_section.set("client_portal".to_string());
                        } else {
                            active_section.set("dashboard".to_string());
                        }
                        let is_new_invite = user.phone.is_none()
                            || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                        needs_setup.set(is_new_invite);
                        logged_in.set(true);
                    }
                }
            }
        });
    });

    // 2. Desktop/Native Smart Card & NFC Reader Passive Background Listener
    use_effect(move || {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let db_ready = *state.db_initialized.read();
            if db_ready {
                spawn(async move {
                    let is_active = active_session_id.read().is_some();
                    if !is_active {
                        hardware_auth_type.set("card_or_badge".to_string());
                        hardware_reader_status.set("connecting".to_string());
                        hardware_error_msg.set(None);
                        let mut active_tok = active_session_token;
                        if let Ok(sess) = yntra_core::initiate_bankid_auth(
                            "assistant".to_string(),
                            "card_or_badge".to_string(),
                        )
                        .await
                        {
                            active_session_id.set(Some(sess.id.clone()));
                            active_tok.set(Some(sess.token.clone()));
                        }
                    }
                });
            }
        }
    });

    let trigger_bankid_employee = move |_| {
        let provider_name = match auth_region.read().as_str() {
            "sv" => "se_bankid",
            "no" => "no_bankid",
            "da" => "dk_mitid",
            _ => "us_global",
        }
        .to_string();

        provider_val.set(provider_name.clone());

        let mut show_bankid = show_bankid_modal;
        let mut active_sess = active_session_id;
        let mut active_tok = active_session_token;
        let mut flow_state = bankid_flow_state;
        let mut progress = bankid_progress;
        let mut qr_data = bankid_qr_data;
        let mut pin = bankid_pin;
        let mut mobile = norway_mobile;
        let mut bdate = norway_birthdate;

        spawn(async move {
            if let Ok(sess) =
                yntra_core::initiate_bankid_auth("assistant".to_string(), provider_name).await
            {
                show_bankid.set(true);
                active_sess.set(Some(sess.id.clone()));
                active_tok.set(Some(sess.token.clone()));
                flow_state.set(sess.status.clone());
                progress.set(sess.progress as f32);
                qr_data.set(sess.qr_data.clone());
                pin.set(String::new());
                mobile.set(String::new());
                bdate.set(String::new());
            }
        });
    };

    use_effect(move || {
        let _trig = db_trigger.read();
        let sid_opt = active_session_id.read().clone();
        let tok_opt = active_session_token.read().clone();
        if let (Some(sid), Some(tok)) = (sid_opt, tok_opt) {
            let mut active_user_id = active_user_id;
            let mut active_section = active_section;
            let mut needs_setup = needs_setup;
            let mut logged_in = logged_in;
            let mut show_bankid_modal = show_bankid_modal;
            let mut show_hardware_modal = show_hardware_modal;
            let mut bankid_flow_state = bankid_flow_state;
            let mut bankid_progress = bankid_progress;
            let mut bankid_qr_data = bankid_qr_data;
            let mut hardware_reader_status = hardware_reader_status;
            let mut active_session_id = active_session_id;
            let mut active_session_token = active_session_token;
            let mut two_factor_user = two_factor_user;
            let users = users_for_effect.clone();
            let toast = toast;
            let mut pin_prompted_sessions = pin_prompted_sessions;
            let mut last_error_shown = last_error_shown;
            let region = props.auth_region.read().clone();

            spawn(async move {
                let tok_for_polling = tok.clone();
                if let Ok(Some(s)) = yntra_core::get_bankid_auth_session(sid, tok.clone()).await {
                    if s.provider == "siths" || s.provider == "nfc" || s.provider == "card_or_badge"
                    {
                        hardware_reader_status.set(s.status.clone());
                        if s.status == "card_detected" {
                            let sid_str = s.id.clone();
                            let already_prompted = pin_prompted_sessions.read().contains(&sid_str);
                            if !already_prompted {
                                pin_prompted_sessions.write().insert(sid_str.clone());
                                let mut eval_prompt = dioxus::document::eval(
                                    r#"
                                    try {
                                        let pin = prompt("Vänligen ange din kort-PIN / Please enter your card PIN:");
                                        dioxus.send(pin || "");
                                    } catch(e) {
                                        dioxus.send("");
                                    }
                                "#,
                                );
                                let session_id = sid_str;
                                let tok_for_hw = tok_for_polling.clone();
                                spawn(async move {
                                    if let Ok(serde_json::Value::String(pin)) =
                                        eval_prompt.recv().await
                                    {
                                        if !pin.is_empty() {
                                            let _ =
                                                yntra_core::complete_hardware_auth(session_id, tok_for_hw, pin)
                                                    .await;
                                        }
                                    }
                                });
                            }
                        } else if s.status == "error" {
                            let err_msg = s.error_message.clone().unwrap_or_default();
                            let final_msg = if err_msg.is_empty() {
                                t("login-hw-error-card-unregistered", &region)
                            } else if err_msg.starts_with("login-hw-error-") {
                                if let Some(colon_pos) = err_msg.find(':') {
                                    let key = &err_msg[..colon_pos];
                                    let arg = &err_msg[colon_pos + 1..];
                                    crate::locales::t_with_args(key, &region, &[("id", arg)])
                                } else {
                                    t(&err_msg, &region)
                                }
                            } else {
                                err_msg
                            };
                            let already_shown =
                                last_error_shown.read().as_ref() == Some(&final_msg);
                            if !already_shown {
                                last_error_shown.set(Some(final_msg.clone()));
                                toast.error(
                                    t("login-hw-title-siths", &region),
                                    dioxus_primitives::toast::ToastOptions::new()
                                        .description(final_msg),
                                );
                            }
                            active_session_id.set(None);
                            active_session_token.set(None);
                        } else if s.status == "no_service" {
                            active_session_id.set(None);
                            active_session_token.set(None);
                        } else if s.status == "success" {
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                            if let Some(uid) = s.authenticated_user_id
                                && let Some(user) = users.iter().find(|u| u.id == uid)
                            {
                                let prefs: serde_json::Value =
                                    serde_json::from_str(&user.preferences).unwrap_or_default();
                                let mfa_enabled = prefs
                                    .get("two_factor_enabled")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if mfa_enabled {
                                    two_factor_user.set(Some(user.clone()));
                                } else {
                                    active_user_id.set(user.id.clone());
                                    if user.role == "client" {
                                        active_section.set("client_portal".to_string());
                                    } else {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let is_new_invite = user.phone.is_none()
                                        || user
                                            .phone
                                            .as_ref()
                                            .map(|p| p.is_empty())
                                            .unwrap_or(true);
                                    needs_setup.set(is_new_invite);
                                    logged_in.set(true);
                                }
                            }
                            show_hardware_modal.set(false);
                            active_session_id.set(None);
                            active_session_token.set(None);
                        }
                    } else {
                        bankid_flow_state.set(s.status.clone());
                        bankid_progress.set(s.progress as f32);
                        bankid_qr_data.set(s.qr_data.clone());

                        if s.status == "success" {
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                            if let Some(uid) = s.authenticated_user_id
                                && let Some(user) = users.iter().find(|u| u.id == uid)
                            {
                                let prefs: serde_json::Value =
                                    serde_json::from_str(&user.preferences).unwrap_or_default();
                                let mfa_enabled = prefs
                                    .get("two_factor_enabled")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if mfa_enabled {
                                    two_factor_user.set(Some(user.clone()));
                                } else {
                                    active_user_id.set(user.id.clone());
                                    if user.role == "client" {
                                        active_section.set("client_portal".to_string());
                                    } else {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let is_new_invite = user.phone.is_none()
                                        || user
                                            .phone
                                            .as_ref()
                                            .map(|p| p.is_empty())
                                            .unwrap_or(true);
                                    needs_setup.set(is_new_invite);
                                    logged_in.set(true);
                                }
                            }
                            show_bankid_modal.set(false);
                            bankid_flow_state.set("idle".to_string());
                            active_session_id.set(None);
                            active_session_token.set(None);
                        }
                    }
                }
            });
        }
    });

    let trigger_google = move |_: Event<MouseData>| {
        #[cfg(target_arch = "wasm32")]
        {
            let js = r#"
                let origin = window.location.origin || "http://localhost:8080";
                window.location.href = "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/authorize?provider=google&redirect_to=" + encodeURIComponent(origin + "/");
            "#;
            let _ = dioxus::document::eval(js);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            props.on_desktop_oauth.call("google".to_string());
        }
    };

    let _trigger_github = move |_: Event<MouseData>| {
        #[cfg(target_arch = "wasm32")]
        {
            let js = r#"
                let origin = window.location.origin || "http://localhost:8080";
                window.location.href = "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/authorize?provider=github&redirect_to=" + encodeURIComponent(origin + "/");
            "#;
            let _ = dioxus::document::eval(js);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            props.on_desktop_oauth.call("github".to_string());
        }
    };

    let trigger_facebook = move |_: Event<MouseData>| {
        #[cfg(target_arch = "wasm32")]
        {
            let js = r#"
                let origin = window.location.origin || "http://localhost:8080";
                window.location.href = "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/authorize?provider=facebook&redirect_to=" + encodeURIComponent(origin + "/");
            "#;
            let _ = dioxus::document::eval(js);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            props.on_desktop_oauth.call("facebook".to_string());
        }
    };

    let trigger_apple = move |_: Event<MouseData>| {
        #[cfg(target_arch = "wasm32")]
        {
            let js = r#"
                let origin = window.location.origin || "http://localhost:8080";
                window.location.href = "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/authorize?provider=apple&redirect_to=" + encodeURIComponent(origin + "/");
            "#;
            let _ = dioxus::document::eval(js);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            props.on_desktop_oauth.call("apple".to_string());
        }
    };

    let trigger_hardware_auth = move |_: Event<MouseData>| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            active_session_id.set(None);
            active_session_token.set(None);
            hardware_auth_type.set("card_or_badge".to_string());
            hardware_reader_status.set("connecting".to_string());
            hardware_error_msg.set(None);
            let mut active_sess = active_session_id;
            let mut active_tok = active_session_token;
            let toast_clone = toast.clone();
            let region_clone = props.auth_region.read().clone();
            spawn(async move {
                if let Ok(sess) = yntra_core::initiate_bankid_auth(
                    "assistant".to_string(),
                    "card_or_badge".to_string(),
                )
                .await
                {
                    active_sess.set(Some(sess.id.clone()));
                    active_tok.set(Some(sess.token.clone()));
                    toast_clone.info(
                        t("login-hw-title-siths", &region_clone),
                        dioxus_primitives::toast::ToastOptions::new()
                            .description(t("login-hw-polling-siths", &region_clone)),
                    );
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let region_clone = props.auth_region.read().clone();
            toast.info(
                t("login-hw-title-nfc", &region_clone),
                dioxus_primitives::toast::ToastOptions::new()
                    .description(t("login-hw-polling-nfc", &region_clone)),
            );
        }
    };

    let handle_dev_login = move |_| {
        let users_list = users_for_dev.clone();
        let mut active_uid = active_user_id;
        let mut active_sec = active_section;
        let mut n_setup = needs_setup;
        let mut log_in = logged_in;
        let toast_err = toast.clone();
        spawn(async move {
            let targeted_user = users_list
                .iter()
                .find(|u| {
                    u.email == "dev.user@yntra.se"
                        || u.email == "admin@yntra.se"
                        || u.role == "platform_admin"
                        || u.role == "admin"
                })
                .or_else(|| users_list.first());

            if let Some(user) = targeted_user {
                active_uid.set(user.id.clone());
                if user.role == "client" {
                    active_sec.set("client_portal".to_string());
                } else {
                    active_sec.set("dashboard".to_string());
                }
                let is_new_invite = user.phone.is_none()
                    || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                let is_dev_or_admin =
                    user.email == "dev.user@yntra.se" || user.email == "admin@yntra.se";
                n_setup.set(is_new_invite && !is_dev_or_admin);
                log_in.set(true);
            } else {
                // Auto-activate dev invitation on first bypass click
                match yntra_core::activate_invitation_code("WELCOME-OFFLINE-FIRST".to_string()).await {
                    Ok(user) => {
                        active_uid.set(user.id.clone());
                        active_sec.set("dashboard".to_string());
                        n_setup.set(false); // WELCOME-OFFLINE-FIRST is the dev invitation code, so bypass setup
                        log_in.set(true);
                    }
                    Err(e1) => {
                        log::error!("activate_invitation_code WELCOME-OFFLINE-FIRST failed: {:?}", e1);
                        match yntra_core::get_user_by_email("user-2".to_string(), "dev.user@yntra.se".to_string()).await {
                            Ok(Some(user)) => {
                                active_uid.set(user.id.clone());
                                active_sec.set("dashboard".to_string());
                                let is_new_invite = user.phone.is_none()
                                    || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                let is_dev_or_admin =
                                    user.email == "dev.user@yntra.se" || user.email == "admin@yntra.se";
                                n_setup.set(is_new_invite && !is_dev_or_admin);
                                log_in.set(true);
                            }
                            Ok(None) => {
                                log::error!("get_user_by_email dev.user@yntra.se returned None");
                                toast_err.error(
                                    "Dev Login Failed".to_string(),
                                    dioxus_primitives::toast::ToastOptions::new().description("User dev.user@yntra.se not found in database."),
                                );
                            }
                            Err(e2) => {
                                log::error!("get_user_by_email dev.user@yntra.se failed: {:?}", e2);
                                toast_err.error(
                                    "Dev Login Failed".to_string(),
                                    dioxus_primitives::toast::ToastOptions::new().description(format!("Verification bypass failed: {:?}", e2)),
                                );
                            }
                        }
                    }
                }
            }
        });
    };

    let is_dropdown_open = *dropdown_open.read();

    let keyframes_css = get_keyframes_css(&props.workspace.brand_color);

    rsx! {
        style { "{keyframes_css}" }
        div { class: "flex min-h-screen flex-col bg-background relative",
            // Language selector & Dev Login placed absolute top-right of the page
            div {
                class: "flex gap-3 items-center",
                style: "position: absolute; top: 1.5rem; right: 1.5rem; z-index: 150;",
                if cfg!(debug_assertions) {
                    button {
                        class: "text-xs text-white border-0 font-semibold px-2.5 py-1 rounded bg-indigo-600 hover:bg-indigo-700 transition-colors cursor-pointer mr-2",
                        onclick: handle_dev_login,
                        "Dev Login"
                    }
                }
                components::Dropdown {
                    label: dropdown_label.to_string(),
                    icon_name: Some("languages"),
                    align_right: true,
                    open: is_dropdown_open,
                    ontoggle: move |_| dropdown_open.set(!is_dropdown_open),
                    components::DropdownItem {
                        label: "Svenska".to_string(),
                        onclick: move |_| {
                            auth_region.set("sv".to_string());
                            dropdown_open.set(false);
                        }
                    }
                    components::DropdownItem {
                        label: "Norsk".to_string(),
                        onclick: move |_| {
                            auth_region.set("no".to_string());
                            dropdown_open.set(false);
                        }
                    }
                    components::DropdownItem {
                        label: "Dansk".to_string(),
                        onclick: move |_| {
                            auth_region.set("da".to_string());
                            dropdown_open.set(false);
                        }
                    }
                    components::DropdownItem {
                        label: "English".to_string(),
                        onclick: move |_| {
                            auth_region.set("en".to_string());
                            dropdown_open.set(false);
                        }
                    }
                }
            }

            if is_scanning {
                div { class: "scan-animation", }
            }

            // BankID modal dialog
            BankIdModal {
                show_bankid_modal,
                bankid_flow_state,
                bankid_progress,
                bankid_qr_data,
                bankid_pin,
                active_session_id,
                active_session_token,
                provider_val,
                norway_mobile,
                norway_birthdate,
                region: region.clone(),
            }

            // Hardware modal rendering removed (runs passively in the background)

            // Invite code modal dialog
            InviteModal {
                show_invite_modal,
                invite_code_input,
                invite_error,
                active_user_id,
                active_section,
                needs_setup,
                logged_in,
                region: region.clone(),
            }



            // Two factor modal dialog
            TwoFactorModal {
                two_factor_user,
                two_factor_code,
                two_factor_error,
                active_user_id,
                active_section,
                needs_setup,
                logged_in,
            }

            // Main login form matching LoginPage.tsx structure and aesthetics
            div { class: "flex flex-1 items-center justify-center px-4 py-12",
                div { class: "animate-fade-in w-full max-w-md",
                    div { class: "mb-10 text-center",
                        // Logo matching YntraLogo
                        div { class: "mb-6 flex items-center justify-center",
                            svg {
                                width: "48",
                                height: "48",
                                view_box: "0 0 48 48",
                                fill: "none",
                                class: "-rotate-12 transform",
                                path {
                                    d: "M28 4L12 24H22L18 44L36 20H24L28 4Z",
                                    fill: "url(#yntra-gradient)",
                                    stroke: "url(#yntra-gradient)",
                                    stroke_width: "2",
                                    stroke_linejoin: "round",
                                }
                                defs {
                                    linearGradient {
                                        id: "yntra-gradient",
                                        x1: "12",
                                        y1: "4",
                                        x2: "36",
                                        y2: "44",
                                        gradient_units: "userSpaceOnUse",
                                        stop { stop_color: "#8B5CF6" }
                                        stop { offset: "1", stop_color: "#A78BFA" }
                                    }
                                }
                            }
                            span { class: "ml-2 text-2xl font-bold text-foreground", "data-testid": "yntra-logo", "Yntra" }
                        }
                        h1 { class: "mb-2 text-3xl font-bold text-foreground m-0 tracking-tight", "{t(\"auth-login-title\", &region)}" }
                        p { class: "text-sm text-muted-foreground m-0 mt-1", "{t(\"auth-login-subtitle\", &region)}" }
                    }

                    div { class: "flex flex-col gap-3",
                        // 1. Swedish / Norwegian / Danish National eID (BankID / MitID)
                        if region == "sv" || region == "no" || region == "da" {
                            {
                                let bankid_label = match region.as_str() {
                                    "sv" => "Mobilt BankID",
                                    "no" => "BankID",
                                    "da" => "MitID",
                                    _ => "National eID",
                                };
                                let bankid_desc = match region.as_str() {
                                    "sv" => "Legitimera dig med BankID-appen",
                                    "no" => "Logg inn med BankID på mobil",
                                    "da" => "Log ind med MitID",
                                    _ => "Secure electronic identification",
                                };
                                rsx! {
                                    button {
                                        r#type: "button",
                                        onclick: trigger_bankid_employee,
                                        class: "flex w-full items-center justify-between rounded-xl border border-border bg-secondary/80 px-4 py-4 text-left transition-all duration-200 hover:border-primary/60 hover:bg-secondary cursor-pointer",
                                        div { class: "flex items-center gap-4",
                                            div { class: "flex h-10 w-10 items-center justify-center rounded-full bg-background text-foreground shadow-inner",
                                                components::LucideIcon { name: "smartphone", class: "h-5 w-5 text-indigo-500" }
                                            }
                                            div {
                                                div { class: "text-sm font-medium text-foreground", "{bankid_label}" }
                                                div { class: "text-xs text-muted-foreground", "{bankid_desc}" }
                                            }
                                        }
                                        components::LucideIcon { name: "arrow-right", class: "h-4 w-4 text-muted-foreground" }
                                    }
                                }
                            }

                            // 2. Hardware Smart Card / NFC Badge authentication
                            {
                                let hw_label = match region.as_str() {
                                    "sv" => "SITHS-kort / NFC-bricka",
                                    _ => "Smart Card / NFC Badge",
                                };
                                let hw_desc = match region.as_str() {
                                    "sv" => "Identifiera dig med kortläsare eller NFC",
                                    _ => "Authenticate using secure hardware reader",
                                };
                                rsx! {
                                    button {
                                        r#type: "button",
                                        onclick: trigger_hardware_auth,
                                        class: "flex w-full items-center justify-between rounded-xl border border-border bg-secondary/80 px-4 py-4 text-left transition-all duration-200 hover:border-primary/60 hover:bg-secondary cursor-pointer",
                                        div { class: "flex items-center gap-4",
                                            div { class: "flex h-10 w-10 items-center justify-center rounded-full bg-background text-foreground shadow-inner",
                                                components::LucideIcon { name: "credit-card", class: "h-5 w-5 text-indigo-500" }
                                            }
                                            div {
                                                div { class: "text-sm font-medium text-foreground", "{hw_label}" }
                                                div { class: "text-xs text-muted-foreground", "{hw_desc}" }
                                            }
                                        }
                                        components::LucideIcon { name: "arrow-right", class: "h-4 w-4 text-muted-foreground" }
                                    }
                                }
                            }
                        }

                        // 3. Google Social Login Button
                        button {
                            r#type: "button",
                            onclick: trigger_google,
                            class: "flex w-full items-center justify-between rounded-xl border border-border bg-secondary/80 px-4 py-4 text-left transition-all duration-200 hover:border-primary/60 hover:bg-secondary cursor-pointer",
                            div { class: "flex items-center gap-4",
                                div { class: "flex h-10 w-10 items-center justify-center rounded-full bg-background text-foreground shadow-inner",
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true", class: "h-5 w-5",
                                        path { fill: "#EA4335", d: "M12 10.2v3.9h5.5c-.2 1.2-.9 2.2-1.9 2.9l3.1 2.4c1.8-1.7 2.8-4.1 2.8-6.9 0-.7-.1-1.5-.2-2.2H12Z" }
                                        path { fill: "#34A853", d: "M12 21c2.5 0 4.6-.8 6.1-2.2L15 16.4c-.8.5-1.8.8-3 .8-2.3 0-4.3-1.6-5-3.8H3.8v2.5A9.2 9.2 0 0 0 12 21Z" }
                                        path { fill: "#FBBC05", d: "M7 13.4a5.5 5.5 0 0 1 0-3.4V7.5H3.8a9.2 9.2 0 0 0 0 8.4L7 13.4Z" }
                                        path { fill: "#4285F4", d: "M12 6.8c1.3 0 2.4.4 3.3 1.3l2.5-2.5A9 9 0 0 0 12 3 9.2 9.2 0 0 0 3.8 7.5L7 10c.7-2.2 2.7-3.2 5-3.2Z" }
                                    }
                                }
                                div {
                                    div { class: "text-sm font-medium text-foreground", "{t(\"auth-login-google-label\", &region)}" }
                                    div { class: "text-xs text-muted-foreground", "{t(\"auth-login-google-desc\", &region)}" }
                                }
                            }
                            components::LucideIcon { name: "arrow-right", class: "h-4 w-4 text-muted-foreground" }
                        }

                        // 4. Facebook Social Login Button
                        button {
                            r#type: "button",
                            onclick: trigger_facebook,
                            class: "flex w-full items-center justify-between rounded-xl border border-border bg-secondary/80 px-4 py-4 text-left transition-all duration-200 hover:border-primary/60 hover:bg-secondary cursor-pointer",
                            div { class: "flex items-center gap-4",
                                div { class: "flex h-10 w-10 items-center justify-center rounded-full bg-background text-foreground shadow-inner",
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true", class: "h-5 w-5",
                                        path { fill: "#1877F2", d: "M24 12a12 12 0 1 0-13.9 11.9v-8.4H7.1V12h3V9.4c0-3 1.8-4.7 4.5-4.7 1.3 0 2.7.2 2.7.2v3h-1.5c-1.5 0-1.9.9-1.9 1.8V12h3.3l-.5 3.5H14v8.4A12 12 0 0 0 24 12Z" }
                                    }
                                }
                                div {
                                    div { class: "text-sm font-medium text-foreground", "{t(\"auth-login-facebook-label\", &region)}" }
                                    div { class: "text-xs text-muted-foreground", "{t(\"auth-login-facebook-desc\", &region)}" }
                                }
                            }
                            components::LucideIcon { name: "arrow-right", class: "h-4 w-4 text-muted-foreground" }
                        }

                        // 5. Apple Social Login Button
                        button {
                            r#type: "button",
                            onclick: trigger_apple,
                            class: "flex w-full items-center justify-between rounded-xl border border-border bg-secondary/80 px-4 py-4 text-left transition-all duration-200 hover:border-primary/60 hover:bg-secondary cursor-pointer",
                            div { class: "flex items-center gap-4",
                                div { class: "flex h-10 w-10 items-center justify-center rounded-full bg-background text-foreground shadow-inner",
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true", class: "h-5 w-5 fill-current text-foreground",
                                        path { d: "M16.7 12.8c0-2.4 2-3.5 2.1-3.6-1.1-1.7-2.9-1.9-3.5-1.9-1.5-.2-2.9.9-3.6.9-.8 0-1.9-.9-3.1-.8-1.6 0-3.1.9-3.9 2.3-1.7 2.9-.4 7.2 1.2 9.5.8 1.1 1.7 2.3 2.9 2.2 1.2 0 1.6-.7 3-.7s1.8.7 3 .7c1.2 0 2.1-1.1 2.8-2.2.9-1.3 1.3-2.6 1.3-2.7-.1 0-2.2-.9-2.2-3.7Zm-2.4-7c.6-.8 1-1.9.9-3-1 .1-2.2.7-2.9 1.5-.6.7-1.1 1.8-1 2.9 1.1.1 2.3-.6 3-1.4Z" }
                                    }
                                }
                                div {
                                    div { class: "text-sm font-medium text-foreground", "{t(\"auth-login-apple-label\", &region)}" }
                                    div { class: "text-xs text-muted-foreground", "{t(\"auth-login-apple-desc\", &region)}" }
                                }
                            }
                            components::LucideIcon { name: "arrow-right", class: "h-4 w-4 text-muted-foreground" }
                        }

                    }

                    if let Some(err) = props.login_error.read().as_ref() {
                        div { class: "mt-5 rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300",
                            "{err}"
                        }
                    }

                    div { class: "mt-6 text-center text-sm text-muted-foreground",
                        "{t(\"auth-login-footer-info\", &region)}"
                    }
                }
            }
        }
    }
}
