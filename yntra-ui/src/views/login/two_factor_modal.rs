use crate::components;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;

#[derive(Props, Clone)]
pub struct TwoFactorModalProps {
    pub two_factor_user: Signal<Option<WorkspaceUser>>,
    pub two_factor_code: Signal<Vec<String>>,
    pub two_factor_error: Signal<Option<String>>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub logged_in: Signal<bool>,
}

impl PartialEq for TwoFactorModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TwoFactorModal(props: TwoFactorModalProps) -> Element {
    let mut two_factor_user = props.two_factor_user;
    let mut two_factor_code = props.two_factor_code;
    let mut two_factor_error = props.two_factor_error;
    let mut active_user_id = props.active_user_id;
    let mut active_section = props.active_section;
    let mut needs_setup = props.needs_setup;
    let mut logged_in = props.logged_in;

    rsx! {
        components::Dialog {
            open: two_factor_user.read().is_some(),
            title: "Tvåfaktorsautentisering".to_string(),
            onclose: move |_| {
                two_factor_user.set(None);
            },
            div { class: "flex flex-col items-center text-center gap-5 w-full text-sm",
                style: "max-width: 320px;",
                h3 { class: "m-0 font-extrabold text-foreground", "Ange verifieringskod" }
                p { class: "text-muted-foreground m-0 text-xs",
                    style: "line-height: 1.4;",
                    "Skriv in den 6-siffriga koden från din autentiseringsapp."
                }

                // Verification OTP code slots
                div { class: "flex flex-col gap-2 items-center",
                    div { class: "flex gap-2",
                        {
                            (0..6)
                                .map(|i| {
                                    let current_otp = two_factor_code.read()[i].clone();
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
                                                let mut codes = two_factor_code.read().clone();
                                                codes[i] = val.chars().next().map(|c| c.to_string()).unwrap_or_default();
                                                two_factor_code.set(codes);
                                            },
                                        }
                                    }
                                })
                        }
                    }
                }

                if let Some(err) = two_factor_error.read().as_ref() {
                    p { class: "text-xs text-center m-0",
                        style: "color:var(--danger);", "{err}" }
                }

                // Actions
                div { class: "grid gap-3 mt-2 w-full",
                    style: "grid-template-columns:1fr 1fr;",
                    button {
                        class: "yntra-btn secondary",
                        onclick: move |_| two_factor_user.set(None),
                        "Avbryt"
                    }
                    button {
                        class: "yntra-btn",
                        disabled: two_factor_code.read().iter().any(|c| c.is_empty()),
                        onclick: move |_| {
                            let user_opt = two_factor_user.read().clone();
                            if let Some(user) = user_opt {
                                let code_str = two_factor_code.read().join("");
                                
                                let totp_secret = {
                                    let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
                                    prefs.get("totp_secret")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                };

                                if !totp_secret.is_empty() && yntra_core::verify_user_totp(totp_secret, code_str) {
                                    active_user_id.set(user.id.clone());
                                    if user.role == "client" {
                                        active_section.set("client_portal".to_string());
                                    } else {
                                        active_section.set("dashboard".to_string());
                                    }
                                    let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                    needs_setup.set(is_new_invite);
                                    logged_in.set(true);
                                    two_factor_user.set(None);
                                    two_factor_error.set(None);
                                } else {
                                    two_factor_error.set(Some("Felaktig kod. Försök igen.".to_string()));
                                    two_factor_code.set(vec!["".to_string(); 6]);
                                }
                            }
                        },
                        "Verifiera"
                    }
                }
            }
        }
    }
}
