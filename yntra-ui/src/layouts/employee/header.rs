use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use super::breadcrumbs::BreadcrumbItem;

#[derive(Props, Clone)]
pub struct LayoutHeaderProps {
    pub active_user: WorkspaceUser,
    pub breadcrumbs: Vec<BreadcrumbItem>,
    pub auth_region: Signal<String>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub header_profile_open: Signal<bool>,
    pub logged_in: Signal<bool>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for LayoutHeaderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LayoutHeader(props: LayoutHeaderProps) -> Element {
    let active_user = props.active_user.clone();
    let breadcrumbs = props.breadcrumbs.clone();
    let current_role = active_user.role.clone();

    let active_user_id = props.active_user_id;
    let mut active_section = props.active_section;
    let mut header_profile_open = props.header_profile_open;
    let mut logged_in = props.logged_in;
    let mut db_trigger = props.db_trigger;
    let auth_region = props.auth_region;

    let is_client = current_role == "client";
    let mut header_role_open = use_signal(|| false);

    rsx! {
        header { class: "flex h-14 items-center justify-between border-b border-border bg-sidebar px-6",
            div { class: "flex items-center gap-1.5 text-sm text-muted-foreground duration-200 animate-in fade-in slide-in-from-left-2",
                for (idx, item) in breadcrumbs.iter().enumerate() {
                    {
                        let is_last = idx == breadcrumbs.len() - 1;
                        let item_clone = item.clone();
                        rsx! {
                            if idx > 0 {
                                span { class: "text-muted-foreground/30 font-medium", "/" }
                            }
                            if is_last {
                                span { class: "text-foreground font-semibold capitalize", "{item.label}" }
                            } else {
                                span {
                                    class: "cursor-pointer hover:text-foreground transition-colors capitalize",
                                    onclick: move |_| item_clone.onclick.call(()),
                                    "{item.label}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex items-center gap-4",
                components::Dropdown {
                    label: match current_role.as_str() {
                        "platform_admin" => "Platform Admin".to_string(),
                        "admin" => "Admin View".to_string(),
                        "assistant" => "Assistant View".to_string(),
                        "user" => "User View".to_string(),
                        "client" => "Client Portal".to_string(),
                        "technician" => "Technician View".to_string(),
                        _ => current_role.clone(),
                    },
                    open: *header_role_open.read(),
                    ontoggle: move |_| {
                        let current = *header_role_open.read();
                        header_role_open.set(!current);
                    },
                    components::DropdownItem {
                        label: "Platform Admin".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "platform_admin".to_string()).await.is_ok() {
                                        if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                            active_section.set("dashboard".to_string());
                                        }
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Admin View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "admin".to_string()).await.is_ok() {
                                        if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                            active_section.set("dashboard".to_string());
                                        }
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Assistant View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "assistant".to_string()).await.is_ok() {
                                        if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                            active_section.set("dashboard".to_string());
                                        }
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "User View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "user".to_string()).await.is_ok() {
                                        if *active_section.read() == "client_portal" || *active_section.read() == "jobs" {
                                            active_section.set("dashboard".to_string());
                                        }
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Client Portal".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "client".to_string()).await.is_ok() {
                                        active_section.set("client_portal".to_string());
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                    components::DropdownItem {
                        label: "Technician View".to_string(),
                        onclick: {
                            let user_id = active_user_id.read().clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    if yntra_core::update_user_role(user_id.clone(), user_id.clone(), "technician".to_string()).await.is_ok() {
                                        active_section.set("jobs".to_string());
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                    header_role_open.set(false);
                                });
                            }
                        }
                    }
                }
                
                // Header User Profile trigger
                div { class: "relative flex items-center gap-3",
                    div {
                        onclick: move |_| {
                            let open = *header_profile_open.read();
                            header_profile_open.set(!open);
                        },
                        class: "flex cursor-pointer items-center gap-3 rounded-md py-1 pl-3 transition-colors hover:bg-secondary",
                        div { class: "hidden text-right md:block",
                            div { class: "text-sm font-medium leading-tight text-foreground",
                                "{active_user.full_name.clone().unwrap_or_default()}"
                            }
                            div { class: "text-[10px] font-bold uppercase tracking-wider text-primary",
                                "{active_user.role}"
                            }
                        }
                        div { class: "flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-primary to-primary/60 shadow-inner text-sm font-bold text-foreground",
                            "{active_user.full_name.clone().unwrap_or_default().chars().next().unwrap_or('?').to_uppercase()}"
                        }
                        components::LucideIcon { name: "chevron-down", class: "h-4 w-4 text-muted-foreground" }
                    }
                    
                    // Floating profile menu dropdown
                    if *header_profile_open.read() {
                        div { class: "absolute right-0 top-12 z-50 w-48 rounded-xl border border-border bg-sidebar py-2 shadow-2xl",
                            if !is_client {
                                button {
                                    class: "flex w-full items-center gap-3 px-4 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground border-0 bg-transparent cursor-pointer",
                                    onclick: move |_| {
                                        active_section.set("settings".to_string());
                                        header_profile_open.set(false);
                                    },
                                    components::LucideIcon { name: "settings", class: "h-4 w-4" }
                                    "{t(\"common-settings\", &auth_region.read())}"
                                }
                            }
                            if !is_client {
                                div { class: "my-1 border-t border-border" }
                            }
                            button {
                                class: "flex w-full items-center gap-3 px-4 py-2 text-sm text-red-400 transition-colors hover:bg-red-500/10 hover:text-red-300 border-0 bg-transparent cursor-pointer",
                                onclick: move |_| {
                                    logged_in.set(false);
                                    active_section.set("dashboard".to_string());
                                    header_profile_open.set(false);
                                },
                                components::LucideIcon { name: "logout", class: "h-4 w-4" }
                                "{t(\"common-logout\", &auth_region.read())}"
                            }
                        }
                    }
                }
            }
        }
    }
}
