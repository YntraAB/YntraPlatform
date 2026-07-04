use crate::components;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;

#[derive(Props, Clone)]
pub struct SsoModalProps {
    pub show_sso_modal: Signal<bool>,
    pub sso_provider: Signal<String>,
    pub sso_email_input: Signal<String>,
    pub sso_error: Signal<Option<String>>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub logged_in: Signal<bool>,
    pub two_factor_user: Signal<Option<WorkspaceUser>>,
    pub users: Vec<WorkspaceUser>,
}

impl PartialEq for SsoModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SsoModal(props: SsoModalProps) -> Element {
    let mut show_sso_modal = props.show_sso_modal;
    let sso_provider = props.sso_provider;
    let mut sso_email_input = props.sso_email_input;
    let mut sso_error = props.sso_error;
    let mut active_user_id = props.active_user_id;
    let mut active_section = props.active_section;
    let mut needs_setup = props.needs_setup;
    let mut logged_in = props.logged_in;
    let mut two_factor_user = props.two_factor_user;
    let users = props.users.clone();

    let trigger_sso_login = move |_| {
        let email = sso_email_input.read().trim().to_lowercase();
        if email.is_empty() {
            sso_error.set(Some("Vänligen ange din e-postadress.".to_string()));
            return;
        }

        if let Some(user) = users.iter().find(|u| u.email.to_lowercase() == email) {
            let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
            let mfa_enabled = prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            if mfa_enabled {
                two_factor_user.set(Some(user.clone()));
            } else {
                active_user_id.set(user.id.clone());
                if user.role == "client" {
                    active_section.set("client_portal".to_string());
                } else {
                    active_section.set("dashboard".to_string());
                }
                let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                needs_setup.set(is_new_invite);
                logged_in.set(true);
            }
            show_sso_modal.set(false);
            sso_email_input.set(String::new());
            sso_error.set(None);
        } else {
            sso_error.set(Some(format!("Användare med e-post '{}' hittades inte i Yntra-katalogen.", email)));
        }
    };

    rsx! {
        components::Dialog {
            open: *show_sso_modal.read(),
            title: format!("SSO-inloggning ({})", sso_provider.read()),
            onclose: move |_| {
                show_sso_modal.set(false);
            },
            div { class: "flex flex-col items-center text-center gap-5 w-full text-sm",
                style: "max-width: 320px;",
                h3 { class: "m-0 font-extrabold text-foreground",
                    "SSO-legitimering"
                }
                p { class: "text-muted-foreground m-0 text-xs",
                    style: "line-height: 1.4;",
                    "Ange din registrerade e-postadress för att simulera inloggning via {sso_provider.read()}."
                }

                input {
                    r#type: "email",
                    class: "dxc-input w-full text-center font-semibold",
                    style: "box-sizing: border-box;",
                    placeholder: "t.ex. marie.andersson@yntra.se",
                    value: "{sso_email_input}",
                    oninput: move |e| {
                        sso_email_input.set(e.value());
                    },
                }

                if let Some(err) = sso_error.read().as_ref() {
                    p { class: "text-xs text-center m-0",
                        style: "color: var(--danger);", "{err}" }
                }

                // Actions
                div { class: "grid gap-3 mt-2 w-full",
                    style: "grid-template-columns: 1fr 1fr;",
                    button {
                        class: "yntra-btn secondary",
                        onclick: move |_| show_sso_modal.set(false),
                        "Avbryt"
                    }
                    button {
                        class: "yntra-btn",
                        onclick: trigger_sso_login,
                        "Logga in"
                    }
                }
            }
        }
    }
}
