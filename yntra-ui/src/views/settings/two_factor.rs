use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::update_user_profile;

#[derive(Props, Clone)]
pub struct TwoFactorSettingsProps {
    pub active_user: WorkspaceUser,
    pub account_preferences: Signal<String>,
    pub account_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for TwoFactorSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TwoFactorSettings(props: TwoFactorSettingsProps) -> Element {
    let active_user = props.active_user.clone();
    let mut account_preferences = props.account_preferences;
    let mut account_save_status = props.account_save_status;
    let mut db_trigger = props.db_trigger;
    let locale = props.locale.clone();

    let mut is_enabled = use_signal(|| {
        let prefs: serde_json::Value = serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false)
    });
    let mut show_enroll_modal = use_signal(|| false);
    let mut otp_code = use_signal(|| vec!["".to_string(); 6]);
    let mut show_secret = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

    let is_enabled_val = *is_enabled.read();
    let show_enroll_val = *show_enroll_modal.read();
    let show_secret_val = *show_secret.read();

    let mut enrollment_secret = use_signal(String::new);
    let totp_secret = use_memo(move || {
        if *show_enroll_modal.read() {
            (*enrollment_secret.read()).clone()
        } else {
            let prefs: serde_json::Value = serde_json::from_str(&account_preferences.read()).unwrap_or_default();
            prefs.get("totp_secret").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
    });

    let icon_name = if is_enabled_val {
        "alert-circle"
    } else {
        "alert-triangle"
    };
    let wrapper_style = if is_enabled_val {
        "border-radius:12px; padding:0.5rem; transition:all 0.3s; background:rgba(16,185,129,0.1); color:hsl(160.1, 84.1%, 39.4%);"
    } else {
        "border-radius:12px; padding:0.5rem; transition:all 0.3s; background:rgba(245,158,11,0.1); color:hsl(37.7, 92.1%, 50.2%);"
    };
    let title_style = if is_enabled_val {
        "margin:0; font-size:0.95rem; font-weight:700; color:hsl(160.1, 84.1%, 39.4%);"
    } else {
        "margin:0; font-size:0.95rem; font-weight:700; color:hsl(37.7, 92.1%, 50.2%);"
    };
    let button_class = if is_enabled_val {
        "yntra-btn secondary"
    } else {
        "yntra-btn"
    };
    let button_style = if is_enabled_val {
        "color:hsl(0, 84.2%, 60.2%); border-color:rgba(239,68,68,0.2);"
    } else {
        ""
    };
    let button_text = if is_enabled_val {
        t("settings-account-mfa-disable-button", &locale)
    } else {
        t("settings-account-mfa-enable-button", &locale)
    };

    let active_user_for_disable = active_user.clone();
    let active_user_for_enable = active_user.clone();
    rsx! {
        div { class: "bg-white/[0.02] border border-border p-8 rounded-xl flex flex-col gap-6",
            // Title Header
            div { class: "flex justify-between items-center",
                div { class: "flex items-center gap-3",
                    div { style: "{wrapper_style}",
                        components::LucideIcon { name: icon_name, class: "h-5 w-5", }
                    }
                    div {
                        p { style: "{title_style}", "{t(\"settings-account-mfa-title\", &locale)}" }
                        p { class: "text-xs text-muted-foreground/60",
                    style: "margin:0.15rem 0 0 0;",
                            if is_enabled_val {
                                "{t(\"settings-account-mfa-enabled\", &locale)}"
                            } else {
                                "{t(\"settings-account-mfa-disabled\", &locale)}"
                            }
                        }
                    }
                }

                button {
                    class: "{button_class}",
                    style: "{button_style}",
                    onclick: move |_| {
                        if is_enabled_val {
                            is_enabled.set(false);
                            account_save_status.set("saving".to_string());
                            let mut prefs: serde_json::Value = serde_json::from_str(&account_preferences.read()).unwrap_or_default();
                            prefs["two_factor_enabled"] = serde_json::json!(false);
                            prefs["totp_secret"] = serde_json::Value::Null;
                            let prefs_str = serde_json::to_string(&prefs).unwrap_or_default();
                            account_preferences.set(prefs_str.clone());

                            let uid = active_user_for_disable.id.clone();
                            let name_val = active_user_for_disable.full_name.clone();
                            let phone_val = active_user_for_disable.phone.clone();
                            let prefs_val = prefs_str.clone();
                            spawn(async move {
                                let _ = update_user_profile(uid.clone(), uid, name_val, phone_val, prefs_val).await;
                            });
                            let current_trig = *db_trigger.read();
                            db_trigger.set(current_trig + 1);
                            account_save_status.set("saved".to_string());
                        } else {
                            let new_sec = yntra_core::generate_totp_secret();
                            enrollment_secret.set(new_sec);
                            show_enroll_modal.set(true);
                            otp_code.set(vec!["".to_string(); 6]);
                            error_message.set(None);
                        }
                    },
                    "{button_text}"
                }
            }

            // Enrollment Overlay Dialog
            if show_enroll_val {
                div { class: "flex justify-center items-center",
                    style: "position:fixed; top:0; left:0; right:0; bottom:0; background:rgba(0,0,0,0.8); backdrop-filter:blur(4px); z-index:1000; animation:fadeIn 0.2s;",
                    div { class: "bg-background border border-border rounded-2xl p-8 w-full flex flex-col gap-6",
                    style: "max-width:440px; box-shadow:0 10px 25px rgba(0,0,0,0.5);",
                        div { class: "flex justify-between items-center",
                            h3 { class: "m-0 font-extrabold text-primary",
                    style: "font-size:1.2rem;",
                                "{t(\"settings-account-mfa-setup-title\", &locale)}"
                            }
                            button {
                                class: "bg-transparent border-0 text-muted-foreground/60 cursor-pointer text-xl",
                                onclick: move |_| {
                                    show_enroll_modal.set(false);
                                    enrollment_secret.set(String::new());
                                },
                                "×"
                            }
                        }

                        p { class: "m-0 text-xs text-muted-foreground",
                    style: "line-height:1.4;",
                            "{t(\"settings-account-mfa-setup-desc\", &locale)}"
                        }

                        // QR code container
                        div { class: "flex flex-col items-center gap-3",
                            div { class: "p-4 rounded-xl flex justify-center items-center",
                                style: "background:white;",
                                div {
                                    style: "width: 160px; height: 160px;",
                                    {crate::utils::qr::render_qr_svg(&format!("otpauth://totp/YntraPlatform:{}?secret={}&issuer=YntraPlatform", active_user.email, totp_secret))}
                                }
                            }
                            button {
                                class: "bg-transparent border-0 text-[11px] text-muted-foreground/60 cursor-pointer",
                    style: "text-decoration:underline;",
                                onclick: move |_| show_secret.set(!show_secret_val),
                                if show_secret_val {
                                    "{t(\"settings-account-mfa-hide-secret\", &locale)}"
                                } else {
                                    "{t(\"settings-account-mfa-show-secret\", &locale)}"
                                }
                            }
                            if show_secret_val {
                                code { class: "border border-border text-xs text-primary",
                    style: "background:rgba(255,255,255,0.03); padding:0.4rem 0.8rem; border-radius:6px; font-family:monospace;",
                                    "{totp_secret}"
                                }
                            }
                        }

                        // Verification OTP code slots
                        div { class: "flex flex-col gap-2 items-center",
                            label { class: "text-[11px] font-bold text-muted-foreground/60 uppercase",
                    style: "tracking-widest:1px;",
                                "{t(\"settings-account-mfa-verify-title\", &locale)}"
                            }
                            div { class: "flex gap-2",
                                {
                                    (0..6)
                                        .map(|i| {
                                            let current_otp = otp_code.read()[i].clone();
                                            rsx! {
                                                input {
                                                    key: "{i}",
                                                    r#type: "text",
                                                    maxlength: "1",
                                                    class: "border border-border rounded-lg text-center font-extrabold bg-white/[0.02]",
                    style: "width:40px; height:45px; font-size:1.2rem; color:var(--text-main); font-family:monospace;",
                                                    value: "{current_otp}",
                                                    oninput: move |e| {
                                                        let val = e.value();
                                                        let mut codes = otp_code.read().clone();
                                                        codes[i] = val.chars().next().map(|c| c.to_string()).unwrap_or_default();
                                                        otp_code.set(codes);
                                                    },
                                                }
                                            }
                                        })
                                }
                            }
                        }

                        if let Some(err) = error_message.read().as_ref() {
                            p { class: "text-xs text-center m-0",
                    style: "color:var(--danger);", "{err}" }
                        }

                        // Actions
                        div { class: "grid gap-3 mt-2",
                    style: "grid-template-columns:1fr 1fr;",
                            button {
                                class: "yntra-btn secondary",
                                onclick: move |_| {
                                    show_enroll_modal.set(false);
                                    enrollment_secret.set(String::new());
                                },
                                "{t(\"common-cancel\", &locale)}"
                            }
                            button {
                                class: "yntra-btn",
                                disabled: otp_code.read().iter().any(|c| c.is_empty()),
                                onclick: move |_| {
                                    let code_str = otp_code.read().join("");
                                    let sec = enrollment_secret.read().clone();
                                    if yntra_core::verify_user_totp(sec.clone(), code_str) {
                                        let ws_id = active_user_for_enable.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                        match yntra_core::encrypt_field(&sec, &ws_id) {
                                            Ok(enc_sec) => {
                                                is_enabled.set(true);
                                                show_enroll_modal.set(false);
                                                error_message.set(None);

                                                account_save_status.set("saving".to_string());
                                                let mut prefs: serde_json::Value = serde_json::from_str(&account_preferences.read()).unwrap_or_default();
                                                prefs["two_factor_enabled"] = serde_json::json!(true);
                                                prefs["totp_secret"] = serde_json::json!(enc_sec);
                                                let prefs_str = serde_json::to_string(&prefs).unwrap_or_default();
                                                account_preferences.set(prefs_str.clone());

                                                let uid = active_user_for_enable.id.clone();
                                                let name_val = active_user_for_enable.full_name.clone();
                                                let phone_val = active_user_for_enable.phone.clone();
                                                let prefs_val = prefs_str.clone();
                                                spawn(async move {
                                                    let _ = update_user_profile(uid.clone(), uid, name_val, phone_val, prefs_val).await;
                                                });
                                                let current_trig = *db_trigger.read();
                                                db_trigger.set(current_trig + 1);
                                                account_save_status.set("saved".to_string());
                                                enrollment_secret.set(String::new());
                                            }
                                            Err(_) => {
                                                error_message.set(Some("Krypteringsfel: kunde inte kryptera MFA-hemligheten. Kontrollera sessionsstatus.".to_string()));
                                                otp_code.set(vec!["".to_string(); 6]);
                                            }
                                        }
                                    } else {
                                        error_message.set(Some(t("settings-account-mfa-error-verify", &locale)));
                                        otp_code.set(vec!["".to_string(); 6]);
                                    }
                                },
                                "{t(\"settings-account-mfa-verify-button\", &locale)}"
                            }
                        }
                    }
                }
            }
        }
    }
}
