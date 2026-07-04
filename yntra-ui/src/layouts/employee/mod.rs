use dioxus::prelude::*;
use crate::state::AppState;
use crate::views;
use crate::components;

pub mod breadcrumbs;
pub mod sidebar;
pub mod header;

use sidebar::LayoutSidebar;
use header::LayoutHeader;
use breadcrumbs::get_breadcrumbs;

#[component]
pub fn EmployeeLayout() -> Element {
    let state = use_context::<AppState>();
    
    let active_user = state.users.read().as_ref().and_then(|u_list| u_list.iter().find(|u| u.id == *state.active_user_id.read()).cloned()).unwrap_or_else(|| yntra_core::WorkspaceUser {
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
    });
    
    let theme_mode = {
        let prefs_str = state.account_preferences.read();
        let mut theme = "dark".to_string();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&prefs_str)
            && let Some(t) = val.get("theme").and_then(|v| v.as_str()) {
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
    });
    let users = state.users.read().clone().unwrap_or_default();
    let teams = state.teams.read().clone().unwrap_or_default();
    let events = state.events.read().clone().unwrap_or_default();
    let messages = state.messages.read().clone().unwrap_or_default();
    let notes = state.notes.read().clone().unwrap_or_default();
    let time_reports = state.time_reports.read().clone().unwrap_or_default();
    let clients = state.clients.read().clone().unwrap_or_default();
    let reports = state.reports.read().clone().unwrap_or_default();
    let workspaces = state.workspaces.read().clone().unwrap_or_default();
    let db_trigger = state.db_trigger;

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
    let journals_enabled = modules_active_val
        .get("journals")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let medications_enabled = modules_active_val
        .get("medications")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
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
    let academics_enabled = modules_active_val
        .get("academics")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let attendance_enabled = modules_active_val
        .get("attendance")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let finance_enabled = modules_active_val
        .get("finance")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let library_enabled = modules_active_val
        .get("library")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);


    let current_role = active_user.role.clone();
    let is_client = current_role == "client";

    let unread_messages_count = messages
        .iter()
        .filter(|m| !m.is_read && m.receiver_id == Some(active_user.id.clone()))
        .count();

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
    let note_subject = state.note_subject;
    let note_content = state.note_content;
    let time_date = state.time_date;
    let time_start = state.time_start;
    let time_end = state.time_end;
    let time_hours = state.time_hours;
    let time_note = state.time_note;
    let time_view_tab = state.time_view_tab;
    let time_filter_status = state.time_filter_status;
    let selected_time_reports = state.selected_time_reports;
    let time_search_query = state.time_search_query;
    let selected_client_id = state.selected_client_id;
    let med_name = state.med_name;
    let med_dosage = state.med_dosage;
    let med_frequency = state.med_frequency;
    let med_instructions = state.med_instructions;
    let journal_content = state.journal_content;
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
    let report_tab = state.report_tab;
    let report_status_filter = state.report_status_filter;
    let report_type_filter = state.report_type_filter;
    let report_type = state.report_type;
    let report_date = state.report_date;
    let report_subject = state.report_subject;
    let report_description = state.report_description;
    let report_is_anonymous = state.report_is_anonymous;
    let selected_report_id = state.selected_report_id;
    let show_report_details_modal = state.show_report_details_modal;
    let compose_recipient_id = state.compose_recipient_id;
    let compose_subject = state.compose_subject;
    let compose_body = state.compose_body;
    let compose_status = state.compose_status;
    let active_user_id = state.active_user_id;

    let locale = auth_region.read().clone();

    let breadcrumbs = get_breadcrumbs(
        active_section,
        selected_note_team_id,
        selected_note_id,
        is_note_composing,
        active_message_id,
        messaging_view_tab,
        selected_directory_team,
        directory_level,
        selected_directory_workspace,
        selected_calendar_date,
        &current_role,
        &locale,
        &teams,
        &notes,
        &messages,
        &workspaces,
    );

    rsx! {
        div { class: "flex h-screen overflow-hidden bg-background {theme_mode}",
            
            // 1. Sidebar Navigation
            LayoutSidebar {
                workspace: workspace.clone(),
                teams: teams.clone(),
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
                journals_enabled,
                medications_enabled,
                directory_enabled,
                reporting_enabled,
                jobs_enabled,
                todos_enabled,
                academics_enabled,
                attendance_enabled,
                finance_enabled,
                library_enabled,
            }

            // 2. Main Content Frame
            div { class: "flex min-w-0 flex-1 flex-col",
                
                // Top Header
                LayoutHeader {
                    active_user: active_user.clone(),
                    breadcrumbs,
                    auth_region,
                    active_user_id,
                    active_section,
                    header_profile_open,
                    logged_in,
                    db_trigger,
                }

                // Scrollable main content viewport
                main { class: "min-h-0 flex-1 overflow-y-auto",
                    components::ErrorBoundary {
                        {
                            match active_section.read().as_str() {
                                "dashboard" => {
                                    rsx! {
                                        views::DashboardView {
                                            active_user: active_user.clone(),
                                            events: events.clone(),
                                            unread_messages_count,
                                            time_reports: time_reports.clone(),
                                            notes: notes.clone(),
                                            db_trigger: db_trigger,
                                            teams: teams.clone(),
                                            clients: clients.clone(),
                                            active_section: active_section,
                                            auth_region: auth_region,
                                        }
                                    }
                                }
                                "messaging" => {
                                    rsx! {
                                        views::MessagingView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            messages: messages.clone(),
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
                                            users: users.clone(),
                                            teams: teams.clone(),
                                            events: events.clone(),
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
                                "notes" => {
                                    rsx! {
                                        views::NotesView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            teams: teams.clone(),
                                            notes: notes.clone(),
                                            selected_note_team_id: selected_note_team_id,
                                            note_subject: note_subject,
                                            note_content: note_content,
                                            active_note_id: selected_note_id,
                                            is_composing: is_note_composing,
                                            locale: auth_region.read().clone(),
                                        }
                                    }
                                }
                                "time" => {
                                    rsx! {
                                        views::TimeView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            time_reports: time_reports.clone(),
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
                                            workspaces: workspaces.clone(),
                                            teams: teams.clone(),
                                        }
                                    }
                                }
                                "assistance" | "journals" | "medications" => {
                                    let init_tab = match active_section.read().as_str() {
                                        "journals" => "journal",
                                        "medications" => "medication",
                                        _ => "journal"
                                    }.to_string();
                                    rsx! {
                                        views::AssistanceView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            clients: clients.clone(),
                                            selected_client_id: selected_client_id,
                                            med_name: med_name,
                                            med_dosage: med_dosage,
                                            med_frequency: med_frequency,
                                            med_instructions: med_instructions,
                                            journal_content: journal_content,
                                            db_trigger: db_trigger,
                                            is_client,
                                            locale: auth_region.read().clone(),
                                            journals_enabled,
                                            medications_enabled,
                                            initial_tab: Some(init_tab),
                                        }
                                    }
                                }
                                "jobs" => {
                                    rsx! {
                                        views::JobsView {
                                            active_user_id: active_user_id,
                                            auth_region: auth_region,
                                            db_trigger: db_trigger,
                                        }
                                    }
                                }
                                "todos" => {
                                    rsx! {
                                        views::TodosView {
                                            active_user_id: active_user_id,
                                            auth_region: auth_region,
                                            db_trigger: db_trigger,
                                        }
                                    }
                                }
                                "academics" | "attendance" | "finance" | "library" => {
                                    let init_tab = match active_section.read().as_str() {
                                        "academics" => "courses",
                                        "attendance" => "attendance",
                                        "finance" => "billing",
                                        "library" => "library",
                                        _ => "dashboard"
                                    }.to_string();
                                    rsx! {
                                        views::SchoolView {
                                            active_user: active_user.clone(),
                                            db_trigger: db_trigger,
                                            locale: auth_region.read().clone(),
                                            initial_tab: Some(init_tab),
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
                                            journals_enabled,
                                            medications_enabled,
                                            directory_enabled,
                                            reporting_enabled,
                                            academics_enabled,
                                            attendance_enabled,
                                            finance_enabled,
                                            library_enabled,
                                            account_name: account_name,
                                            account_phone: account_phone,
                                            account_preferences: account_preferences,
                                            workspace: workspace.clone(),
                                            locale: auth_region.read().clone(),
                                        }
                                    }
                                }
                                "client_portal" => {
                                    rsx! {
                                        views::ClientPortalView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            clients: clients.clone(),
                                            events: events.clone(),
                                            db_trigger: db_trigger,
                                            workspace: workspace.clone(),
                                        }
                                    }
                                }
                                "directory" => {
                                    rsx! {
                                        views::DirectoryView {
                                            active_user: active_user.clone(),
                                            workspace: workspace.clone(),
                                            users: users.clone(),
                                            teams: teams.clone(),
                                            clients: clients.clone(),
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
                                "reporting" => {
                                    rsx! {
                                        views::ReportingView {
                                            active_user: active_user.clone(),
                                            users: users.clone(),
                                            reports: reports.clone(),
                                            report_tab: report_tab,
                                            report_status_filter: report_status_filter,
                                            report_type_filter: report_type_filter,
                                            report_type: report_type,
                                            report_date: report_date,
                                            report_subject: report_subject,
                                            report_description: report_description,
                                            report_is_anonymous: report_is_anonymous,
                                            selected_report_id: selected_report_id,
                                            show_report_details_modal: show_report_details_modal,
                                        }
                                    }
                                }
                                _ => rsx! {
                                    div { "Unknown Section" }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
