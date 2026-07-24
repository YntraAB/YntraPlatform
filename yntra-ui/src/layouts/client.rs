use crate::components;
use crate::locales::t;
use crate::state::AppState;
use crate::views;
use dioxus::prelude::*;

#[component]
pub fn ClientLayout() -> Element {
    let state = use_context::<AppState>();

    let active_user = state
        .users
        .read()
        .as_ref()
        .and_then(|u_list| {
            u_list
                .iter()
                .find(|u| u.id == *state.active_user_id.read())
                .cloned()
        })
        .unwrap_or_else(|| yntra_core::WorkspaceUser {
            id: String::new(),
            workspace_id: None,
            email: String::new(),
            full_name: Some("Guest User".to_string()),
            phone: None,
            role: "guest".to_string(),
            preferences: "{}".to_string(),
            siths_card_id: None,
            nfc_badge_uid: None,
            updated_at: 0,
            sync_status: "synced".to_string(),
            personal_number: None,
            public_key: None,
        });

    let theme_mode = {
        let prefs_str = state.account_preferences.read();
        let mut theme = "dark".to_string();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&prefs_str)
            && let Some(t) = val.get("theme").and_then(|v| v.as_str())
        {
            theme = t.to_string();
        }
        theme
    };

    let unread_messages_count = state
        .messages
        .read()
        .as_ref()
        .map(|m_list| {
            m_list
                .iter()
                .filter(|m| !m.is_read && m.receiver_id == Some(active_user.id.clone()))
                .count()
        })
        .unwrap_or(0);

    let mut active_section = state.active_section;
    let mut messaging_view_tab = state.messaging_view_tab;
    let mut active_message_id = state.active_message_id;
    let mut selected_directory_team = state.selected_directory_team;
    let mut directory_level = state.directory_level;
    let mut logged_in = state.logged_in;
    let mut header_profile_open = state.header_profile_open;
    let auth_region = state.auth_region;

    let workspace = state.workspace.read().clone().unwrap_or_else(|| yntra_core::Workspace {
        id: "workspace-1".to_string(),
        name: "Yntra Operations Ltd".to_string(),
        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
        settings: "{}".to_string(),
        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
        logo_url: None,
        block_settings: "{}".to_string(),
        updated_at: 0,
        sync_status: "synced".to_string(),
    });
    let db_trigger = state.db_trigger;
    let trigger_jobs = state.trigger_jobs;

    let is_client = true;

    let show_add_team_modal = state.show_add_team_modal;
    let show_invite_member_modal = state.show_invite_member_modal;
    let show_client_manager_modal = state.show_client_manager_modal;
    let new_team_name = state.new_team_name;
    let new_member_name = state.new_member_name;
    let new_member_email = state.new_member_email;
    let new_member_role = state.new_member_role;
    let new_client_first_name = state.new_client_first_name;
    let new_client_last_name = state.new_client_last_name;
    let new_client_personal_number = state.new_client_personal_number;
    let new_client_care_level = state.new_client_care_level;

    let selected_directory_workspace = state.selected_directory_workspace;
    let compose_recipient_id = state.compose_recipient_id;
    let compose_subject = state.compose_subject;
    let compose_body = state.compose_body;
    let compose_status = state.compose_status;

    rsx! {
        div { class: "relative flex h-screen overflow-hidden bg-background {theme_mode}",
            // Left Sidebar
            aside { class: "z-20 flex w-64 flex-col border-r border-border bg-sidebar shadow-sm transition-all duration-300",
                // Sidebar Header
                div { class: "flex h-14 items-center border-b border-border bg-sidebar px-4",
                    span { class: "bg-gradient-to-r from-primary to-primary/60 bg-clip-text text-lg font-bold tracking-tight text-foreground text-transparent",
                        "{t(\"client-portal-title\", &auth_region.read())}"
                    }
                }
                // Sidebar Links
                div { class: "flex flex-1 flex-col gap-1 overflow-y-auto px-3 py-4",
                    button {
                        onclick: move |_| active_section.set("client_portal".to_string()),
                        class: if *active_section.read() == "client_portal" {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer bg-primary font-medium text-primary-foreground shadow-md"
                        } else {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer text-muted-foreground hover:bg-secondary hover:text-foreground"
                        },
                        "{t(\"client-portal-sections-start\", &auth_region.read())}"
                    }
                    button {
                        onclick: move |_| {
                            active_section.set("messaging".to_string());
                            messaging_view_tab.set("inbox".to_string());
                            active_message_id.set(None);
                        },
                        class: if *active_section.read() == "messaging" {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer bg-primary font-medium text-primary-foreground shadow-md"
                        } else {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer text-muted-foreground hover:bg-secondary hover:text-foreground"
                        },
                        "{t(\"client-portal-messages\", &auth_region.read())}"
                    }
                    button {
                        onclick: move |_| {
                            active_section.set("directory".to_string());
                            selected_directory_team.set(None);
                            directory_level.set("teams".to_string());
                        },
                        class: if *active_section.read() == "directory" {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer bg-primary font-medium text-primary-foreground shadow-md"
                        } else {
                            "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-all duration-200 border-0 bg-transparent text-left cursor-pointer text-muted-foreground hover:bg-secondary hover:text-foreground"
                        },
                        "{t(\"sidebar-teams\", &auth_region.read())}"
                    }
                }
                // Logout button at bottom of sidebar
                div { class: "mt-auto border-t border-border p-4",
                    button {
                        onclick: move |_| {
                            logged_in.set(false);
                            active_section.set("dashboard".to_string());
                        },
                        class: "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium text-red-500 transition-colors hover:bg-red-500/10 hover:text-red-400 border-0 bg-transparent text-left cursor-pointer",
                        components::LucideIcon { name: "logout", class: "h-4 w-4" }
                        "{t(\"common-logout\", &auth_region.read())}"
                    }
                }
            }

            // Right Content Area
            div { class: "flex min-w-0 flex-1 flex-col bg-background/50",
                // Header Bar
                header { class: "sticky top-0 z-10 flex h-14 items-center justify-between border-b border-border bg-sidebar px-4 backdrop-blur-sm",
                    div { class: "flex items-center gap-2 text-sm font-medium capitalize text-foreground animate-in fade-in slide-in-from-left-2",
                        {
                            let current_sec = active_section.read().clone();
                            match current_sec.as_str() {
                                "client_portal" => t("client-portal-sections-start", &auth_region.read()),
                                "messaging" => t("client-portal-messages", &auth_region.read()),
                                "directory" => t("sidebar-teams", &auth_region.read()),
                                _ => current_sec,
                            }
                        }
                    }

                    // Header Profile trigger (right aligned)
                    div { class: "relative flex items-center gap-3",
                        div {
                            onclick: move |_| {
                                let open = *header_profile_open.read();
                                header_profile_open.set(!open);
                            },
                            class: "flex cursor-pointer items-center gap-3 rounded-xl py-1 pl-3 transition-colors hover:bg-secondary",
                            div { class: "hidden text-right md:block",
                                div { class: "text-sm font-medium leading-tight text-foreground",
                                    "{active_user.full_name.clone().unwrap_or_default()}"
                                }
                                div { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground",
                                    "{t(\"client-portal-account\", &auth_region.read())}"
                                }
                            }
                            div { class: "flex h-9 w-9 items-center justify-center rounded-full border border-primary/30 bg-primary/20 font-bold text-primary shadow-inner text-sm",
                                "{active_user.full_name.clone().unwrap_or_default().chars().next().unwrap_or('?').to_uppercase()}"
                            }
                            components::LucideIcon { name: "chevron-down", class: "h-4 w-4 text-muted-foreground" }
                        }
                        if *header_profile_open.read() {
                            div { class: "absolute right-0 top-14 z-50 w-48 rounded-xl border border-border bg-sidebar py-2 shadow-2xl animate-in fade-in slide-in-from-top-2",
                                button {
                                    class: "flex w-full items-center gap-3 px-4 py-2 text-sm text-red-500 transition-colors hover:bg-secondary hover:text-red-400 border-0 bg-transparent cursor-pointer",
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

                // Main View Viewport
                main { class: "h-full w-full flex-1 overflow-y-auto p-4",
                    components::ErrorBoundary {
                        {
                            let current_sec = active_section.read().clone();
                            match current_sec.as_str() {
                                "client_portal" => {
                                    rsx! {
                                        views::ClientPortalView {
                                            active_user: active_user.clone(),
                                            db_trigger: db_trigger,
                                            trigger_jobs: trigger_jobs,
                                            workspace: workspace.clone(),
                                        }
                                    }
                                }
                                "messaging" => {
                                    rsx! {
                                        views::MessagingView {
                                            active_user: active_user.clone(),
                                            unread_messages_count,
                                            messaging_view_tab: messaging_view_tab,
                                            active_message_id: active_message_id,
                                            compose_recipient_id: compose_recipient_id,
                                            compose_subject: compose_subject,
                                            compose_body: compose_body,
                                            compose_status: compose_status,
                                            db_trigger: db_trigger,
                                            is_client,
                                        }
                                    }
                                }
                                "directory" => {
                                    rsx! {
                                        views::DirectoryView {
                                            active_user: active_user.clone(),
                                            workspace: workspace.clone(),
                                            directory_level: directory_level,
                                            selected_directory_workspace: selected_directory_workspace,
                                            selected_directory_team: selected_directory_team,
                                            show_add_team_modal: show_add_team_modal,
                                            show_invite_member_modal: show_invite_member_modal,
                                            show_client_manager_modal: show_client_manager_modal,
                                            new_team_name: new_team_name,
                                            new_member_name: new_member_name,
                                            new_member_email: new_member_email,
                                            new_member_role: new_member_role,
                                            new_client_first_name: new_client_first_name,
                                            new_client_last_name: new_client_last_name,
                                            new_client_personal_number: new_client_personal_number,
                                            new_client_care_level: new_client_care_level,
                                            db_trigger: db_trigger,
                                        }
                                    }
                                }
                                _ => {
                                    active_section.set("client_portal".to_string());
                                    rsx! {
                                        div { "Redirecting..." }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
