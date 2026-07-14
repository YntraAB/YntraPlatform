use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

mod account;
mod blocks;
mod finance_settings;
mod general;
mod notifications;
mod scheduler;
mod two_factor;

pub use account::AccountSettings;
pub use blocks::BlockSettings;
pub use finance_settings::FinanceSettings;
pub use general::GeneralSettings;
pub use notifications::NotificationsSettings;
pub use scheduler::SchedulerSettings;
pub use two_factor::TwoFactorSettings;

#[derive(Props, Clone)]
pub struct SettingsViewProps {
    pub active_user: WorkspaceUser,
    pub settings_save_status: Signal<String>,
    pub account_save_status: Signal<String>,
    pub settings_tab: Signal<String>,
    pub settings_name: Signal<String>,
    pub settings_brand_color: Signal<String>,
    pub settings_logo_url: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub messaging_enabled: bool,
    pub scheduling_enabled: bool,
    pub notes_enabled: bool,
    pub time_enabled: bool,
    pub directory_enabled: bool,
    pub reporting_enabled: bool,
    pub account_name: Signal<String>,
    pub account_phone: Signal<String>,
    pub account_preferences: Signal<String>,
    pub workspace: Workspace,
    pub locale: String,
}

impl PartialEq for SettingsViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SettingsView(props: SettingsViewProps) -> Element {
    let mut settings_tab = props.settings_tab;
    let active_user = props.active_user.clone();
    let is_admin = active_user.role == "admin" || active_user.role == "platform_admin";

    // Default tab check based on role if current tab is invalid or not general
    let current_tab = settings_tab.read().clone();
    let display_tab = if current_tab == "general" && !is_admin {
        "account".to_string()
    } else {
        current_tab
    };

    let save_status = props.settings_save_status.read().clone();
    let account_status = props.account_save_status.read().clone();
    let is_error = save_status.starts_with("error:") || account_status.starts_with("error:");
    let error_msg = if save_status.starts_with("error:") {
        Some(save_status.trim_start_matches("error:").to_string())
    } else if account_status.starts_with("error:") {
        Some(account_status.trim_start_matches("error:").to_string())
    } else {
        None
    };

    let is_saving = save_status == "saving" || account_status == "saving";

    let mut tabs_list = vec![];
    if is_admin {
        tabs_list.push(components::tabs::TabItem {
            value: "general".to_string(),
            label: crate::locales::t("settings-tabs-general", &props.locale),
            icon: Some("globe".to_string()),
        });
    }
    tabs_list.push(components::tabs::TabItem {
        value: "account".to_string(),
        label: crate::locales::t("settings-tabs-account", &props.locale),
        icon: Some("user".to_string()),
    });
    if is_admin {
        tabs_list.push(components::tabs::TabItem {
            value: "scheduler".to_string(),
            label: crate::locales::t("settings-tabs-scheduler", &props.locale),
            icon: Some("calendar".to_string()),
        });
        tabs_list.push(components::tabs::TabItem {
            value: "blocks".to_string(),
            label: crate::locales::t("settings-tabs-blocks", &props.locale),
            icon: Some("layout-grid".to_string()),
        });
    }
    tabs_list.push(components::tabs::TabItem {
        value: "notifications".to_string(),
        label: crate::locales::t("settings-tabs-notifications", &props.locale),
        icon: Some("mail".to_string()),
    });

    rsx! {
        div { class: "mx-auto w-full max-w-5xl space-y-8 p-4 md:p-8",
            // Header Section
            div { class: "flex justify-between items-center gap-6",
                    style: "border-b:1px solid var(--border-color); padding-bottom:1.5rem;",
                div { class: "flex items-center gap-4",
                    div { class: "flex items-center justify-center rounded-2xl border border-border",
                    style: "width:64px; height:64px; overflow:hidden; background:rgba(255,255,255,0.03); box-shadow:inset 0 2px 4px rgba(0,0,0,0.2);",
                        if !props.settings_logo_url.read().is_empty() {
                            img {
                                src: "{props.settings_logo_url}",
                                alt: "{props.settings_name}",
                                class: "w-full h-full p-2",
                                style: "object-fit:contain;",
                            }
                        } else {
                            components::LucideIcon {
                                name: "layout-grid",
                                class: "h-8 w-8 text-primary",
                            }
                        }
                    }
                    div {
                        h1 {
                            class: "text-3xl font-bold tracking-tight text-foreground m-0",

                            if is_admin {
                                "{props.settings_name}"
                            } else {
                                "{crate::locales::t(\"settings-header-personal-profile\", &props.locale)}"
                            }
                        }
                        p { class: "text-sm text-muted-foreground/60",
                            style: "margin:0.25rem 0 0 0;",
                            if is_admin {
                                "{crate::locales::t(\"settings-header-admin-desc\", &props.locale)}"
                            } else {
                                "{crate::locales::t(\"settings-header-personal-desc\", &props.locale)}"
                            }
                        }
                    }
                }

                // Saved/Saving/Error State Badge
                div { class: "flex items-center gap-2 rounded-full border border-border bg-white/[0.02] text-xs font-bold uppercase",
                    style: "px:1rem; py:0.5rem; tracking-widest:1px;",
                    if is_error {
                        components::LucideIcon {
                            name: "alert-triangle",
                            class: "h-3.5 w-3.5 text-destructive",
                        }
                        span { class: "text-destructive",
                            style: "margin-left:0.25rem; margin-right:0.5rem;",
                            "{error_msg.as_deref().unwrap_or_default()}"
                        }
                    } else if is_saving {
                        components::LucideIcon {
                            name: "refresh-cw",
                            class: "h-3.5 w-3.5 text-primary animate-spin",
                        }
                        span { class: "text-primary",
                            style: "margin-left:0.25rem; margin-right:0.5rem;",
                            "{crate::locales::t(\"common-saving\", &props.locale)}"
                        }
                    } else {
                        components::LucideIcon {
                            name: "check-circle",
                            class: "h-3.5 w-3.5 text-emerald-500",
                        }
                        span { style: "color:hsl(160.1, 84.1%, 39.4%); margin-left:0.25rem; margin-right:0.5rem;",
                            "{crate::locales::t(\"common-saved-label\", &props.locale)}"
                        }
                    }
                }
            }

            // Custom Tabs Navigation
            div {
                class: "flex items-center gap-1.5 p-1 rounded-xl bg-muted/40 border border-border/40 backdrop-blur-sm max-w-max",
                for tab in tabs_list.iter() {
                    {
                        let is_active = tab.value == display_tab;
                        let tab_val = tab.value.clone();
                        let label = tab.label.clone();
                        let icon_name = tab.icon.clone();

                        rsx! {
                            button {
                                key: "{tab_val}",
                                class: format!(
                                    "flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-lg transition-all duration-300 border-0 cursor-pointer {}",
                                    if is_active {
                                        "bg-primary text-primary-foreground shadow-md shadow-primary/10"
                                    } else {
                                        "bg-transparent text-muted-foreground hover:bg-muted/80 hover:text-foreground"
                                    }
                                ),
                                onclick: move |_| settings_tab.set(tab_val.clone()),
                                if let Some(icon) = icon_name {
                                    components::LucideIcon { name: icon, class: "h-3.5 w-3.5" }
                                }
                                span { "{label}" }
                            }
                        }
                    }
                }
            }

            // Active Tab Content
            div { style: "padding-bottom:3rem;",
                match display_tab.as_str() {
                    "general" => rsx! {
                        GeneralSettings {
                            settings_name: props.settings_name,
                            settings_brand_color: props.settings_brand_color,
                            settings_logo_url: props.settings_logo_url,
                            settings_save_status: props.settings_save_status,
                            db_trigger: props.db_trigger,
                            workspace: props.workspace.clone(),
                            locale: props.locale.clone(),
                        }
                    },
                    "account" => rsx! {
                        AccountSettings {
                            active_user: props.active_user,
                            account_name: props.account_name,
                            account_phone: props.account_phone,
                            account_preferences: props.account_preferences,
                            account_save_status: props.account_save_status,
                            db_trigger: props.db_trigger,
                            locale: props.locale.clone(),
                        }
                    },
                    "scheduler" => rsx! {
                        SchedulerSettings {
                            settings_save_status: props.settings_save_status,
                            db_trigger: props.db_trigger,
                            workspace: props.workspace.clone(),
                            locale: props.locale.clone(),
                        }
                    },
                    "blocks" => rsx! {
                        BlockSettings {
                            settings_save_status: props.settings_save_status,
                            db_trigger: props.db_trigger,
                            messaging_enabled: props.messaging_enabled,
                            scheduling_enabled: props.scheduling_enabled,
                            notes_enabled: props.notes_enabled,
                            time_enabled: props.time_enabled,
                            directory_enabled: props.directory_enabled,
                            reporting_enabled: props.reporting_enabled,
                            active_user: props.active_user.clone(),
                            account_preferences: props.account_preferences,
                            workspace: props.workspace.clone(),
                            locale: props.locale.clone(),
                        }
                    },
                    "notifications" => rsx! {
                        NotificationsSettings {
                            settings_save_status: props.settings_save_status,
                            active_user: props.active_user.clone(),
                            account_preferences: props.account_preferences,
                            db_trigger: props.db_trigger,
                            locale: props.locale.clone(),
                        }
                    },
                    _ => rsx! {
                        div { "Tab not implemented" }
                    },
                }
            }
        }
    }
}
