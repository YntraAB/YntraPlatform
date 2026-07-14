use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::update_user_profile;

#[derive(Props, Clone)]
pub struct SetupViewProps {
    pub active_user_id: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub logged_in: Signal<bool>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub users: Vec<WorkspaceUser>,
}

impl PartialEq for SetupViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SetupView(props: SetupViewProps) -> Element {
    let active_uid = props.active_user_id.read().clone();
    let users = props.users.clone();
    let current_user = users.iter().find(|u| u.id == active_uid).cloned();

    let region = props.auth_region.read().clone();

    // Prefill the full name if we already have it from the directory invitation
    let initial_name = current_user
        .as_ref()
        .and_then(|u| u.full_name.clone())
        .unwrap_or_default();

    let mut full_name = use_signal(|| initial_name);
    let mut phone = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let _active_user_id = props.active_user_id;
    let mut needs_setup = props.needs_setup;
    let mut logged_in = props.logged_in;
    let db_trigger = props.db_trigger;

    // Define localized strings retrieved from FTL locales
    let title_label = t("auth-setup-pin-title", &region);
    let description_label = t("auth-setup-pin-description", &region);
    let pin_label = t("auth-setup-pin-label", &region);
    let confirm_pin_label = t("auth-setup-pin-confirm-label", &region);
    let mismatch_error_label = t("auth-setup-pin-mismatch-error", &region);
    let short_error_label = t("auth-setup-pin-short-error", &region);
    let name_required_label = t("auth-setup-name-required-error", &region);

    let save_region = region.clone();
    let handle_save = move |_| {
        let name_val = full_name.read().trim().to_string();
        let phone_val = phone.read().trim().to_string();
        let pass_val = password.read().clone();
        let confirm_val = confirm_password.read().clone();

        if name_val.is_empty() {
            error_msg.set(Some(name_required_label.to_string()));
            return;
        }

        if pass_val.len() < 4 {
            error_msg.set(Some(short_error_label.to_string()));
            return;
        }

        if pass_val != confirm_val {
            error_msg.set(Some(mismatch_error_label.to_string()));
            return;
        }

        // Call the database profile update function
        let prefs = current_user
            .as_ref()
            .map(|u| u.preferences.clone())
            .unwrap_or_else(|| "{}".to_string());
        let phone_opt = if phone_val.is_empty() {
            None
        } else {
            Some(phone_val)
        };

        let mut error = error_msg;
        let mut setup = needs_setup;
        let mut trigger = db_trigger;
        let save_region_clone = save_region.clone();
        let uid = active_uid.clone();

        spawn(async move {
            if let Err(e) = yntra_core::set_user_password(uid.clone(), uid.clone(), pass_val).await
            {
                error.set(Some(format!(
                    "{}: {}",
                    t("auth-setup-error-prefix", &save_region_clone),
                    e
                )));
                return;
            }

            match update_user_profile(uid.clone(), uid, Some(name_val), phone_opt, prefs).await {
                Ok(_) => {
                    let current_trig = *trigger.read();
                    trigger.set(current_trig + 1);
                    error.set(None);
                    setup.set(false);
                }
                Err(e) => {
                    error.set(Some(format!(
                        "{}: {}",
                        t("auth-setup-error-prefix", &save_region_clone),
                        e
                    )));
                }
            }
        });
    };

    let handle_logout = move |_| {
        needs_setup.set(false);
        logged_in.set(false);
    };

    rsx! {
        div { class: "login-container dark",
            div { class: "login-grid-bg", }
            components::Card { class: "login-card", style: "width: 440px;",
                div { class: "login-header",
                    div { class: "flex justify-center w-full items-center mb-2 text-primary",
                        components::LucideIcon { name: "lock", size: "36" }
                    }
                }
                h2 { class: "text-2xl font-extrabold text-center",
                    style: "margin: 0.25rem 0 0 0;",
                    "{title_label}"
                }
                p { class: "m-0 text-muted-foreground text-sm text-center",
                    style: "line-height: 1.4;",
                    "{description_label}"
                }

                div { class: "flex flex-col gap-3.5 mt-4",
                    div {
                        label {
                            class: "text-xs font-bold text-muted-foreground mb-1 block",
                            r#for: "setup-full-name",
                            "{t(\"auth-setup-full-name\", &region)}"
                        }
                        components::Input {
                            id: "setup-full-name".to_string(),
                            placeholder: t("auth-setup-full-name", &region).to_string(),
                            value: full_name.read().clone(),
                            oninput: move |e: FormEvent| full_name.set(e.value()),
                        }
                    }

                    div {
                        label {
                            class: "text-xs font-bold text-muted-foreground mb-1 block",
                            r#for: "setup-phone",
                            "{t(\"auth-setup-phone-optional\", &region)}"
                        }
                        components::Input {
                            id: "setup-phone".to_string(),
                            placeholder: "+46 70 123 45 67".to_string(),
                            value: phone.read().clone(),
                            oninput: move |e: FormEvent| phone.set(e.value()),
                        }
                    }

                    div {
                        label {
                            class: "text-xs font-bold text-muted-foreground mb-1 block",
                            r#for: "setup-password",
                            "{pin_label}"
                        }
                        components::Input {
                            id: "setup-password".to_string(),
                            r#type: "password".to_string(),
                            placeholder: "••••".to_string(),
                            value: password.read().clone(),
                            oninput: move |e: FormEvent| password.set(e.value()),
                        }
                    }

                    div {
                        label {
                            class: "text-xs font-bold text-muted-foreground mb-1 block",
                            r#for: "setup-confirm-password",
                            "{confirm_pin_label}"
                        }
                        components::Input {
                            id: "setup-confirm-password".to_string(),
                            r#type: "password".to_string(),
                            placeholder: "••••".to_string(),
                            value: confirm_password.read().clone(),
                            oninput: move |e: FormEvent| confirm_password.set(e.value()),
                        }
                    }

                    if let Some(err) = error_msg.read().clone() {
                        div { class: "text-xs font-semibold text-center mt-1",
                            style: "color: var(--danger);",
                            "{err}"
                        }
                    }

                    div { class: "flex flex-col gap-2 mt-2",
                        components::Button {
                            variant: components::ButtonVariant::Primary,
                            onclick: handle_save,
                            "{t(\"auth-setup-save-continue\", &region)}"
                        }
                        components::Button {
                            variant: components::ButtonVariant::Secondary,
                            onclick: handle_logout,
                            "{t(\"auth-setup-cancel-logout\", &region)}"
                        }
                    }
                }
            }
        }
    }
}
