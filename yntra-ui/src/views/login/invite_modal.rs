use crate::components;
use crate::locales::t;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct InviteModalProps {
    pub show_invite_modal: Signal<bool>,
    pub invite_code_input: Signal<String>,
    pub invite_error: Signal<Option<String>>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub logged_in: Signal<bool>,
    pub region: String,
}

impl PartialEq for InviteModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn InviteModal(props: InviteModalProps) -> Element {
    let mut show_invite_modal = props.show_invite_modal;
    let mut invite_code_input = props.invite_code_input;
    let mut invite_error = props.invite_error;
    let active_user_id = props.active_user_id;
    let active_section = props.active_section;
    let needs_setup = props.needs_setup;
    let logged_in = props.logged_in;
    let region = props.region.clone();
    let region_c1 = region.clone();
    let region_c2 = region.clone();
    let region_c3 = region.clone();
    let region_c4 = region.clone();

    let trigger_activate_invite = move |_| {
        let code = invite_code_input.read().clone();
        if code.trim().is_empty() {
            invite_error.set(Some(t("login-invite-error-empty", &region_c1)));
            return;
        }

        let mut a_uid = active_user_id;
        let mut a_sec = active_section;
        let mut n_setup = needs_setup;
        let mut log_in = logged_in;
        let mut s_invite = show_invite_modal;
        let mut code_input = invite_code_input;
        let mut error = invite_error;
        spawn(async move {
            match yntra_core::activate_invitation_code(code).await {
                Ok(user) => {
                    a_uid.set(user.id.clone());
                    if user.role == "client" {
                        a_sec.set("client_portal".to_string());
                    } else {
                        a_sec.set("dashboard".to_string());
                    }
                    let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                    n_setup.set(is_new_invite);
                    log_in.set(true);
                    s_invite.set(false);
                    code_input.set(String::new());
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        components::Dialog {
            open: *show_invite_modal.read(),
            title: t("login-invite-title", &region_c2),
            onclose: move |_| {
                show_invite_modal.set(false);
            },
            div { class: "flex flex-col items-center text-center gap-5 w-full text-sm",
                style: "max-width: 320px;",
                h3 { class: "m-0 font-extrabold text-foreground",
                    "{t(\"login-invite-title\", &region_c3)}"
                }
                p { class: "text-muted-foreground m-0 text-xs",
                    style: "line-height: 1.4;",
                    "{t(\"login-invite-description\", &region_c4)}"
                }

                input {
                    r#type: "text",
                    class: "dxc-input w-full text-center font-semibold tracking-wide",
                    style: "box-sizing: border-box;",
                    placeholder: "{t(\"login-invite-placeholder\", &props.region)}",
                    value: "{invite_code_input}",
                    oninput: move |e| {
                        invite_code_input.set(e.value());
                    },
                }

                if let Some(err) = invite_error.read().as_ref() {
                    p { class: "text-xs text-center m-0",
                        style: "color: var(--danger);", "{err}" }
                }

                // Actions
                div { class: "grid gap-3 mt-2 w-full",
                    style: "grid-template-columns: 1fr 1fr;",
                    button {
                        class: "yntra-btn secondary",
                        onclick: move |_| show_invite_modal.set(false),
                        "{t(\"login-invite-btn-cancel\", &props.region)}"
                    }
                    button {
                        class: "yntra-btn",
                        onclick: trigger_activate_invite,
                        "{t(\"login-invite-btn-submit\", &props.region)}"
                    }
                }
            }
        }
    }
}
