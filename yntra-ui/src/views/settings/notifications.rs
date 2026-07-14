use crate::components;
use crate::locales::t;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct NotificationsSettingsProps {
    pub settings_save_status: Signal<String>,
    pub active_user: yntra_core::WorkspaceUser,
    pub account_preferences: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for NotificationsSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn NotificationsSettings(props: NotificationsSettingsProps) -> Element {
    let mut settings_save_status = props.settings_save_status;
    let active_user = props.active_user.clone();
    let mut account_preferences = props.account_preferences;
    let mut db_trigger = props.db_trigger;

    // Load initial states from user preferences JSON in the local database
    let mut notif_on = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("push_notifications_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });
    let mut notif_type = use_signal(|| {
        let prefs: serde_json::Value =
            serde_json::from_str(&account_preferences.read()).unwrap_or_default();
        prefs
            .get("push_notifications_type")
            .and_then(|v| v.as_str())
            .unwrap_or("full_content")
            .to_string()
    });

    let update_notif_setting = {
        let active_user_id = active_user.id.clone();
        let active_user_name = active_user.full_name.clone();
        let active_user_phone = active_user.phone.clone();

        move |key: &str, value: serde_json::Value| {
            settings_save_status.set("saving".to_string());

            let mut prefs: serde_json::Value =
                serde_json::from_str(&account_preferences.read()).unwrap_or_default();
            prefs[key] = value;

            let prefs_str = serde_json::to_string(&prefs).unwrap_or_default();
            account_preferences.set(prefs_str.clone());

            let uid = active_user_id.clone();
            let name_val = active_user_name.clone();
            let phone_val = active_user_phone.clone();
            let prefs_val = prefs_str.clone();
            let requester_uid = uid.clone();
            spawn(async move {
                let _ = yntra_core::update_user_profile(
                    requester_uid.clone(),
                    requester_uid,
                    name_val,
                    phone_val,
                    prefs_val,
                )
                .await;
            });

            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            settings_save_status.set("saved".to_string());
        }
    };

    rsx! {
        components::Card { class: "overflow-hidden border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
            components::CardHeader { class: "pb-6",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center gap-3",
                        div { class: "rounded-2xl bg-primary/10 p-3 text-primary",
                            components::LucideIcon { name: "mail", class: "h-6 w-6" }
                        }
                        div {
                            components::CardTitle { class: "text-xl", "{t(\"settings-notifications-title\", &props.locale)}" }
                            components::CardDescription { class: "mt-1", "{t(\"settings-notifications-desc\", &props.locale)}" }
                        }
                    }
                    components::Switch {
                        checked: *notif_on.read(),
                        onchange: {
                            let update_notif_setting = update_notif_setting.clone();
                            move |val| {
                                let mut update_notif_setting = update_notif_setting.clone();
                                notif_on.set(val);
                                update_notif_setting("push_notifications_enabled", serde_json::json!(val));
                            }
                        }
                    }
                }
            }
            components::CardContent { class: "space-y-8",
                div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                    // Option 1: Full Content
                    button {
                        disabled: !*notif_on.read(),
                        onclick: {
                            let update_notif_setting = update_notif_setting.clone();
                            move |_| {
                                let mut update_notif_setting = update_notif_setting.clone();
                                notif_type.set("full_content".to_string());
                                update_notif_setting("push_notifications_type", serde_json::json!("full_content"));
                            }
                        },
                        class: format!(
                            "group relative flex items-start gap-4 rounded-2xl border-2 p-5 text-left transition-all bg-transparent cursor-pointer {}",
                            if !*notif_on.read() {
                                "cursor-not-allowed opacity-50 border-border/50 bg-background/50"
                            } else if *notif_type.read() == "full_content" {
                                "border-primary bg-primary/10"
                            } else {
                                "border-border/50 bg-background/50 hover:border-primary/30"
                            }
                        ),
                        div {
                            class: format!(
                                "mt-1 rounded-xl p-2.5 shadow-sm transition-colors {}",
                                if *notif_type.read() == "full_content" && *notif_on.read() {
                                    "bg-primary text-primary-foreground"
                                } else {
                                    "bg-muted text-muted-foreground"
                                }
                            ),
                            components::LucideIcon { name: "bell-ring", class: "h-5 w-5" }
                        }
                        div {
                            div {
                                class: format!(
                                    "font-bold {}",
                                    if *notif_type.read() == "full_content" && *notif_on.read() {
                                        "text-foreground"
                                    } else {
                                        "text-muted-foreground"
                                    }
                                ),
                                "{t(\"settings-notifications-full-content\", &props.locale)}"
                            }
                            div { class: "mt-1.5 text-xs leading-relaxed text-muted-foreground",
                                "{t(\"settings-notifications-full-content-desc\", &props.locale)}"
                            }
                        }
                        if *notif_type.read() == "full_content" && *notif_on.read() {
                            div { class: "absolute right-3 top-3",
                                components::LucideIcon { name: "check", class: "h-4 w-4 text-primary" }
                            }
                        }
                    }

                    // Option 2: Alert Only
                    button {
                        disabled: !*notif_on.read(),
                        onclick: {
                            let update_notif_setting = update_notif_setting.clone();
                            move |_| {
                                let mut update_notif_setting = update_notif_setting.clone();
                                notif_type.set("alert_only".to_string());
                                update_notif_setting("push_notifications_type", serde_json::json!("alert_only"));
                            }
                        },
                        class: format!(
                            "group relative flex items-start gap-4 rounded-2xl border-2 p-5 text-left transition-all bg-transparent cursor-pointer {}",
                            if !*notif_on.read() {
                                "cursor-not-allowed opacity-50 border-border/50 bg-background/50"
                            } else if *notif_type.read() == "alert_only" {
                                "border-primary bg-primary/10"
                            } else {
                                "border-border/50 bg-background/50 hover:border-primary/30"
                            }
                        ),
                        div {
                            class: format!(
                                "mt-1 rounded-xl p-2.5 shadow-sm transition-colors {}",
                                if *notif_type.read() == "alert_only" && *notif_on.read() {
                                    "bg-primary text-primary-foreground"
                                } else {
                                    "bg-muted text-muted-foreground"
                                }
                            ),
                            components::LucideIcon { name: "mail", class: "h-5 w-5" }
                        }
                        div {
                            div {
                                class: format!(
                                    "font-bold {}",
                                    if *notif_type.read() == "alert_only" && *notif_on.read() {
                                        "text-foreground"
                                    } else {
                                        "text-muted-foreground"
                                    }
                                ),
                                "{t(\"settings-notifications-alert-only\", &props.locale)}"
                            }
                            div { class: "mt-1.5 text-xs leading-relaxed text-muted-foreground",
                                "{t(\"settings-notifications-alert-only-desc\", &props.locale)}"
                            }
                        }
                        if *notif_type.read() == "alert_only" && *notif_on.read() {
                            div { class: "absolute right-3 top-3",
                                components::LucideIcon { name: "check", class: "h-4 w-4 text-primary" }
                            }
                        }
                    }
                }

                // Bottom no spam info card
                div { class: "flex items-center gap-3 rounded-xl border border-border/50 bg-muted/30 p-4",
                    div { class: "h-2 w-2 animate-pulse rounded-full bg-primary" }
                    p { class: "text-xs font-medium text-muted-foreground m-0",
                        "{t(\"settings-notifications-no-spam\", &props.locale)}"
                    }
                }
            }
        }
    }
}
