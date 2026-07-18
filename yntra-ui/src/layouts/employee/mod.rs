use crate::components;
use crate::state::AppState;
use crate::views;
use dioxus::prelude::*;

pub mod breadcrumbs;
pub mod header;
pub mod sidebar;

use header::LayoutHeader;
use sidebar::LayoutSidebar;

#[component]
pub fn EmployeeLayout() -> Element {
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

    // Check module activation from JSON
    let modules_active_val: serde_json::Value =
        serde_json::from_str(&workspace.modules_active).unwrap_or_default();
    let messaging_enabled = modules_active_val
        .get("messaging")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let scheduling_enabled = modules_active_val
        .get("scheduling")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let notes_enabled = modules_active_val
        .get("notes")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let time_enabled = modules_active_val
        .get("time")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let directory_enabled = modules_active_val
        .get("directory")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let reporting_enabled = modules_active_val
        .get("reporting")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let jobs_enabled = modules_active_val
        .get("jobs")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let todos_enabled = modules_active_val
        .get("todos")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let current_role = active_user.role.clone();
    let is_client = current_role == "client";

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

    let active_section = state.active_section;
    let globalsearch_open = state.globalsearch_open;
    let selected_note_team_id = state.selected_note_team_id;
    let time_group_expanded = state.time_group_expanded;
    let filter_categories = state.filter_categories;
    let auth_region = state.auth_region;
    let header_profile_open = state.header_profile_open;
    let logged_in = state.logged_in;

    let selected_note_id = state.selected_note_id;
    let is_note_composing = state.is_note_composing;
    let messaging_view_tab = state.messaging_view_tab;
    let active_message_id = state.active_message_id;
    let selected_directory_team = state.selected_directory_team;
    let directory_level = state.directory_level;
    let calendar_year = state.calendar_year;
    let calendar_month = state.calendar_month;
    let selected_calendar_date = state.selected_calendar_date;
    let scheduling_sidebar_tab = state.scheduling_sidebar_tab;

    let event_title = state.event_title;
    let event_team = state.event_team;
    let event_assignee = state.event_assignee;
    let event_recipient = state.event_recipient;
    let event_start = state.event_start;
    let event_end = state.event_end;
    let leave_type = state.leave_type;
    let leave_start = state.leave_start;
    let leave_end = state.leave_end;
    let leave_reason = state.leave_reason;
    let leave_save_status = state.leave_save_status;
    let time_date = state.time_date;
    let time_start = state.time_start;
    let time_end = state.time_end;
    let time_hours = state.time_hours;
    let time_note = state.time_note;
    let time_view_tab = state.time_view_tab;
    let time_filter_status = state.time_filter_status;
    let selected_time_reports = state.selected_time_reports;
    let time_search_query = state.time_search_query;
    let settings_save_status = state.settings_save_status;
    let account_save_status = state.account_save_status;
    let settings_tab = state.settings_tab;
    let settings_name = state.settings_name;
    let settings_brand_color = state.settings_brand_color;
    let settings_logo_url = state.settings_logo_url;
    let account_name = state.account_name;
    let account_phone = state.account_phone;
    let account_preferences = state.account_preferences;
    let selected_directory_workspace = state.selected_directory_workspace;
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

    let compose_recipient_id = state.compose_recipient_id;
    let compose_subject = state.compose_subject;
    let compose_body = state.compose_body;
    let compose_status = state.compose_status;
    let active_user_id = state.active_user_id;

    rsx! {
        div { class: "flex h-screen overflow-hidden bg-background {theme_mode}",

            // 1. Sidebar Navigation
            LayoutSidebar {
                workspace: workspace.clone(),
                active_user_role: current_role.clone(),
                unread_messages_count,
                active_section,
                globalsearch_open,
                selected_note_team_id,
                time_group_expanded,
                filter_categories,
                auth_region,
                messaging_enabled,
                scheduling_enabled,
                notes_enabled,
                time_enabled,
                directory_enabled,
                reporting_enabled,
                jobs_enabled,
                todos_enabled,
            }

            // 2. Main Content Frame
            div { class: "flex min-w-0 flex-1 flex-col",

                // Top Header
                LayoutHeader {
                    active_user: active_user.clone(),
                    auth_region,
                    active_user_id,
                    active_section,
                    header_profile_open,
                    logged_in,
                    db_trigger,
                    workspace: workspace.clone(),
                }

                // Scrollable main content viewport
                main { class: "min-h-0 flex-1 overflow-y-auto",
                    components::ErrorBoundary {
                        {
                            let sec = active_section.read().clone();
                            let is_route_allowed = |sec_name: &str, role_str: &str| -> bool {
                                let r = role_str.to_lowercase();
                                if r == "student" || r == "role-school-student" {
                                    sec_name == "academics" || sec_name == "report_cards" || sec_name == "library" || sec_name == "dashboard" || sec_name == "settings"
                                } else if r == "parent" || r == "role-school-parent" {
                                    sec_name == "academics" || sec_name == "finance" || sec_name == "health_clinic" || sec_name == "report_cards" || sec_name == "library" || sec_name == "messaging" || sec_name == "directory" || sec_name == "dashboard" || sec_name == "settings"
                                } else {
                                    true
                                }
                            };

                            if !is_route_allowed(sec.as_str(), &current_role) {
                                rsx! {
                                    div { class: "p-8 text-center space-y-4 max-w-md mx-auto mt-20 border border-dashed border-red-500/20 rounded-2xl bg-red-500/5 animate-in fade-in duration-200",
                                        components::LucideIcon { name: "shield-alert", class: "h-12 w-12 text-red-500 mx-auto" }
                                        h3 { class: "text-lg font-bold text-foreground m-0", "Access Denied" }
                                        p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "Your user role does not have permission to access this section." }
                                    }
                                }
                            } else {
                                match sec.as_str() {
                                    "dashboard" => {
                                        rsx! {
                                            views::DashboardView {
                                                active_user: active_user.clone(),
                                                unread_messages_count,
                                                db_trigger: db_trigger,
                                                active_section: active_section,
                                                auth_region: auth_region,
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
                                    "scheduling" => {
                                        rsx! {
                                            views::SchedulingView {
                                                active_user: active_user.clone(),
                                                calendar_year: calendar_year,
                                                calendar_month: calendar_month,
                                                selected_calendar_date: selected_calendar_date,
                                                scheduling_sidebar_tab: scheduling_sidebar_tab,
                                                event_title: event_title,
                                                event_team: event_team,
                                                event_assignee: event_assignee,
                                                event_recipient: event_recipient,
                                                event_start: event_start,
                                                event_end: event_end,
                                                leave_type: leave_type,
                                                leave_start: leave_start,
                                                leave_end: leave_end,
                                                leave_reason: leave_reason,
                                                leave_save_status: leave_save_status,
                                                db_trigger: db_trigger,
                                                filter_categories: filter_categories,
                                                locale: auth_region.read().clone(),
                                            }
                                        }
                                    }

                                    "time" => {
                                        rsx! {
                                            views::TimeView {
                                                active_user: active_user.clone(),
                                                time_view_tab: time_view_tab,
                                                time_filter_status: time_filter_status,
                                                time_search_query: time_search_query,
                                                selected_time_reports: selected_time_reports,
                                                time_date: time_date,
                                                time_start: time_start,
                                                time_end: time_end,
                                                time_hours: time_hours,
                                                time_note: time_note,
                                                db_trigger: db_trigger,
                                            }
                                        }
                                    }

                                    "jobs" => {
                                        rsx! {
                                            views::JobsView {
                                                active_user_id: active_user_id,
                                                auth_region: auth_region,
                                                db_trigger: trigger_jobs,
                                            }
                                        }
                                    }


                                    "settings" => {
                                        rsx! {
                                            views::SettingsView {
                                                active_user: active_user.clone(),
                                                settings_save_status: settings_save_status,
                                                account_save_status: account_save_status,
                                                settings_tab: settings_tab,
                                                settings_name: settings_name,
                                                settings_brand_color: settings_brand_color,
                                                settings_logo_url: settings_logo_url,
                                                db_trigger: db_trigger,
                                                messaging_enabled,
                                                scheduling_enabled,
                                                notes_enabled,
                                                time_enabled,
                                                directory_enabled,
                                                reporting_enabled,
                                                account_name: account_name,
                                                account_phone: account_phone,
                                                account_preferences: account_preferences,
                                                workspace: workspace.clone(),
                                                locale: auth_region.read().clone(),
                                            }
                                        }
                                    }
                                    "todos" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "todos");
                                        if use_custom {
                                            rsx! {
                                                views::TodosView {
                                                    active_user_id: active_user_id.clone(),
                                                    auth_region: auth_region,
                                                    db_trigger: db_trigger,
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "notes" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "notes");
                                        if use_custom {
                                            rsx! {
                                                views::NotesView {
                                                    active_user: active_user.clone(),
                                                    selected_note_team_id: selected_note_team_id,
                                                    note_subject: state.note_subject,
                                                    note_content: state.note_content,
                                                    active_note_id: selected_note_id,
                                                    is_composing: is_note_composing,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "reporting" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "reporting");
                                        if use_custom {
                                            rsx! {
                                                views::ReportingView {
                                                    active_user: active_user.clone(),
                                                    report_tab: state.report_tab,
                                                    report_status_filter: state.report_status_filter,
                                                    report_type_filter: state.report_type_filter,
                                                    report_type: state.report_type,
                                                    report_date: state.report_date,
                                                    report_subject: state.report_subject,
                                                    report_description: state.report_description,
                                                    report_is_anonymous: state.report_is_anonymous,
                                                    selected_report_id: state.selected_report_id,
                                                    show_report_details_modal: state.show_report_details_modal,
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

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

                                    "academics" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "academics");
                                        if use_custom {
                                            rsx! {
                                                views::AcademicsView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "attendance" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "attendance");
                                        if use_custom {
                                            rsx! {
                                                views::AttendanceView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "finance" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "finance");
                                        if use_custom {
                                            rsx! {
                                                views::FinanceView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "library" => {
                                        let use_custom = get_block_use_custom_ui(&workspace, "library");
                                        if use_custom {
                                            rsx! {
                                                views::LibraryView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                views::DynamicBlockView {
                                                    active_user_id: active_user_id.read().clone(),
                                                    workspace_id: workspace.id.clone(),
                                                    block_id: active_section.read().clone(),
                                                    db_trigger: db_trigger,
                                                    locale: auth_region.read().clone(),
                                                }
                                            }
                                        }
                                    }

                                    "health_clinic" => {
                                        rsx! {
                                            views::HealthClinicView {
                                                active_user_id: active_user_id.read().clone(),
                                                workspace_id: workspace.id.clone(),
                                                block_id: active_section.read().clone(),
                                                db_trigger: db_trigger,
                                                locale: auth_region.read().clone(),
                                            }
                                        }
                                    }

                                    "report_cards" => {
                                        rsx! {
                                            views::ReportCardsView {
                                                active_user_id: active_user_id.read().clone(),
                                                workspace_id: workspace.id.clone(),
                                                block_id: active_section.read().clone(),
                                                db_trigger: db_trigger,
                                                locale: auth_region.read().clone(),
                                            }
                                        }
                                    }

                                    _ => {
                                        rsx! {
                                            views::DynamicBlockView {
                                                active_user_id: active_user_id.read().clone(),
                                                workspace_id: workspace.id.clone(),
                                                block_id: active_section.read().clone(),
                                                db_trigger: db_trigger,
                                                locale: auth_region.read().clone(),
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
    }
}

fn get_block_use_custom_ui(workspace: &yntra_core::Workspace, block_id: &str) -> bool {
    let block_settings_val: serde_json::Value =
        serde_json::from_str(&workspace.block_settings).unwrap_or_default();
    block_settings_val
        .get(block_id)
        .and_then(|b| b.get("use_custom_ui"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}
