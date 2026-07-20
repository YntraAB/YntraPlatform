pub mod oauth;
pub mod resources;

use crate::locales::get_system_locale;
use crate::utils::DioxusDbObserver;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use yntra_core::{
    ClientProfile, DailyNote, MessageItem, ReportItem, Team, TeamEvent, TimeReport, TodoItem,
    Workspace, WorkspaceUser, clear_observers, clear_session_key, get_users, init_tracing,
    init_wasm_db, is_session_key_set, load_local_workspace_key, register_observer,
    start_background_sync, load_workspace_zero_copy_stores,
};

#[derive(Clone, Copy)]
pub struct AppState {
    pub db_trigger: Signal<u32>,
    pub trigger_jobs: Signal<u32>,
    pub trigger_todos: Signal<u32>,
    pub trigger_clients: Signal<u32>,
    pub trigger_school: Signal<u32>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub two_factor_user: Signal<Option<WorkspaceUser>>,
    pub logged_in: Signal<bool>,
    pub login_tab: Signal<String>,
    pub show_bankid_modal: Signal<bool>,
    pub scanning_state: Signal<String>,
    pub login_email: Signal<String>,
    pub login_password: Signal<String>,
    pub login_error: Signal<Option<String>>,
    pub auth_region: Signal<String>,
    pub active_user_role: Signal<String>,

    // View state sub-signals
    pub selected_note_team_id: Signal<String>,
    pub selected_note_id: Signal<Option<String>>,
    pub is_note_composing: Signal<bool>,
    pub selected_client_id: Signal<String>,

    // Calendar signals
    pub calendar_year: Signal<i32>,
    pub calendar_month: Signal<u32>,
    pub selected_calendar_date: Signal<String>,

    // Form inputs
    pub note_subject: Signal<String>,
    pub note_content: Signal<String>,
    pub event_title: Signal<String>,
    pub event_team: Signal<String>,
    pub event_assignee: Signal<String>,
    pub event_recipient: Signal<String>,
    pub event_start: Signal<String>,
    pub event_end: Signal<String>,

    pub time_date: Signal<String>,
    pub time_start: Signal<String>,
    pub time_end: Signal<String>,
    pub time_hours: Signal<String>,
    pub time_note: Signal<String>,
    pub time_view_tab: Signal<String>,
    pub time_filter_status: Signal<String>,
    pub selected_time_reports: Signal<Vec<String>>,
    pub time_search_query: Signal<String>,

    pub journal_content: Signal<String>,
    pub med_name: Signal<String>,
    pub med_dosage: Signal<String>,
    pub med_frequency: Signal<String>,
    pub med_instructions: Signal<String>,

    // Reporting signals
    pub report_tab: Signal<String>,
    pub report_type: Signal<String>,
    pub report_subject: Signal<String>,
    pub report_description: Signal<String>,
    pub report_date: Signal<String>,
    pub report_is_anonymous: Signal<bool>,
    pub report_status_filter: Signal<String>,
    pub report_type_filter: Signal<String>,
    pub selected_report_id: Signal<Option<String>>,
    pub show_report_details_modal: Signal<bool>,

    // Directory signals
    pub directory_level: Signal<String>,
    pub selected_directory_workspace: Signal<String>,
    pub selected_directory_team: Signal<Option<String>>,
    pub show_add_team_modal: Signal<bool>,
    pub show_invite_member_modal: Signal<bool>,
    pub show_client_manager_modal: Signal<bool>,
    pub new_team_name: Signal<String>,
    pub new_member_email: Signal<String>,
    pub new_member_name: Signal<String>,
    pub new_member_role: Signal<String>,
    pub new_client_first_name: Signal<String>,
    pub new_client_last_name: Signal<String>,
    pub new_client_personal_number: Signal<String>,
    pub new_client_care_level: Signal<String>,

    // Settings signals
    pub settings_tab: Signal<String>,
    pub settings_name: Signal<String>,
    pub settings_brand_color: Signal<String>,
    pub settings_logo_url: Signal<String>,
    pub settings_save_status: Signal<String>,

    // Account signals
    pub account_name: Signal<String>,
    pub account_phone: Signal<String>,
    pub account_preferences: Signal<String>,
    pub account_save_status: Signal<String>,
    pub last_synced_user_id: Signal<String>,

    // Leave request signals
    pub scheduling_sidebar_tab: Signal<String>,
    pub leave_type: Signal<String>,
    pub leave_start: Signal<String>,
    pub leave_end: Signal<String>,
    pub leave_reason: Signal<String>,
    pub leave_save_status: Signal<String>,

    // UI layout / search state signals
    pub globalsearch_open: Signal<bool>,
    pub header_profile_open: Signal<bool>,
    pub time_group_expanded: Signal<bool>,
    pub filter_categories: Signal<Vec<String>>,

    // Messaging signals
    pub messaging_view_tab: Signal<String>,
    pub active_message_id: Signal<Option<String>>,
    pub compose_recipient_id: Signal<Option<String>>,
    pub compose_subject: Signal<String>,
    pub compose_body: Signal<String>,
    pub compose_status: Signal<String>,

    // Resolved database query signals
    pub workspace: Resource<Workspace>,
    pub users: Resource<Vec<WorkspaceUser>>,
    pub teams: Resource<Vec<Team>>,
    pub events: Resource<Vec<TeamEvent>>,
    pub messages: Resource<Vec<MessageItem>>,
    pub notes: Resource<Vec<DailyNote>>,
    pub time_reports: Resource<Vec<TimeReport>>,
    pub clients: Resource<Vec<ClientProfile>>,
    pub reports: Resource<Vec<ReportItem>>,
    pub workspaces: Resource<Vec<Workspace>>,
    pub todos: Resource<Vec<TodoItem>>,
    pub workspace_id: Memo<String>,

    // Desktop OAuth flow triggers
    pub on_desktop_oauth: Callback<String>,
    pub background_error: Signal<Option<yntra_core::YntraError>>,
    pub db_initialized: Signal<bool>,
}

impl AppState {
    pub fn get_passkey_seed(&self) -> String {
        let uid = self.active_user_id.read();
        format!("passkey_seed_{}", uid)
    }
}

pub fn use_init_app_state() -> AppState {
    // Core state signals
    let mut db_trigger = use_signal(|| 0);
    let mut trigger_jobs = use_signal(|| 0);
    let mut trigger_todos = use_signal(|| 0);
    let mut trigger_users = use_signal(|| 0);
    let mut trigger_teams = use_signal(|| 0);
    let mut trigger_events = use_signal(|| 0);
    let mut trigger_messages = use_signal(|| 0);
    let mut trigger_notes = use_signal(|| 0);
    let mut trigger_time = use_signal(|| 0);
    let mut trigger_clients = use_signal(|| 0);
    let mut trigger_school = use_signal(|| 0);
    let mut trigger_reports = use_signal(|| 0);
    let mut trigger_workspaces = use_signal(|| 0);
    let active_user_id = use_signal(|| "user-1".to_string());
    let active_section = use_signal(|| "dashboard".to_string());
    let needs_setup = use_signal(|| false);
    let two_factor_user = use_signal(|| Option::<WorkspaceUser>::None);
    let background_error = use_signal(|| Option::<yntra_core::YntraError>::None);
    let mut active_user_role = use_signal(|| "guest".to_string());
    let mut db_initialized = use_signal(|| false);

    // Login & Auth State Signals
    let logged_in = use_signal(|| false);
    let login_tab = use_signal(|| "employee".to_string());
    let show_bankid_modal = use_signal(|| false);
    let scanning_state = use_signal(|| "idle".to_string());
    let mut login_email = use_signal(String::new);
    let mut login_password = use_signal(String::new);
    let login_error = use_signal(|| Option::<String>::None);
    let mut auth_region = use_signal(get_system_locale);

    // Restore session on startup
    let mut active_uid = active_user_id;
    let mut is_logged_in = logged_in;
    let mut active_sec = active_section;
    let mut setup_needed = needs_setup;
    use_effect(move || {
        spawn(async move {
            let _ = init_tracing();
            match init_wasm_db().await {
                Ok(_) => log::info!("Database initialized successfully."),
                Err(e) => log::error!("Database initialization failed: {:?}", e),
            }
            db_initialized.set(true);
            let _ = load_workspace_zero_copy_stores("workspace-1".to_string()).await;
            start_background_sync(30);
            let bypass_script = if cfg!(debug_assertions) {
                r#"localStorage.setItem("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES", "true");"#
            } else {
                ""
            };
            let script = format!(
                r#"
                {}
                let logged_in = localStorage.getItem("yntra_logged_in") === "true";
                let uid = localStorage.getItem("yntra_active_user_id") || "";
                dioxus.send(JSON.stringify({{ logged_in, uid }}));
                "#,
                bypass_script
            );
            let mut eval = dioxus::document::eval(&script);
            if let Ok(serde_json::Value::Object(obj)) = eval.recv::<serde_json::Value>().await {
                let has_logged_in = obj
                    .get("logged_in")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let uid = obj
                    .get("uid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if has_logged_in && !uid.is_empty() {
                    if let Some(user) = get_users(uid.clone())
                        .await
                        .ok()
                        .and_then(|all_users| all_users.into_iter().find(|u| u.id == uid))
                    {
                        active_uid.set(uid);
                        active_user_role.set(user.role.clone());
                        if user.role == "client" {
                            active_sec.set("client_portal".to_string());
                        } else {
                            active_sec.set("dashboard".to_string());
                        }
                        let is_new_invite = user.phone.is_none()
                            || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                        setup_needed.set(is_new_invite);
                        is_logged_in.set(true);
                    }
                }
            }
        });
    });

    // Persist session changes
    use_effect(move || {
        let is_login = *logged_in.read();
        let uid = active_user_id.read().clone();

        if is_login {
            login_password.set(String::new());
            login_email.set(String::new());
        }

        let js = if is_login {
            format!(
                r#"
                localStorage.setItem("yntra_logged_in", "true");
                localStorage.setItem("yntra_active_user_id", "{}");
                "#,
                uid
            )
        } else {
            r#"
            localStorage.removeItem("yntra_logged_in");
            localStorage.removeItem("yntra_active_user_id");
            "#
            .to_string()
        };
        let _ = dioxus::document::eval(&js);
    });

    // Automatically set/clear session key when active_user_id changes
    let active_uid_for_session = active_user_id;
    let mut active_role_sig = active_user_role;
    use_effect(move || {
        let db_ready = *db_initialized.read();
        if db_ready {
            let uid = active_uid_for_session.read().clone();
            if !uid.is_empty() {
                spawn(async move {
                    if let Ok(all_users) = get_users(uid.clone()).await {
                        if let Some(user) = all_users.into_iter().find(|u| u.id == uid) {
                            let ws_id = user
                                .workspace_id
                                .clone()
                                .unwrap_or_else(|| "workspace-1".to_string());
                            if !is_session_key_set() {
                                let _ = load_local_workspace_key(ws_id.clone(), uid.clone()).await;
                            }
                            let _ = load_workspace_zero_copy_stores(ws_id).await;
                            active_role_sig.set(user.role.clone());
                        }
                    }
                });
            } else {
                clear_session_key();
                active_role_sig.set("guest".to_string());
            }
        }
    });

    // View state sub-signals
    let selected_note_team_id = use_signal(String::new);
    let selected_note_id = use_signal(|| Option::<String>::None);
    let is_note_composing = use_signal(|| false);
    let selected_client_id = use_signal(|| "client-1".to_string());

    // Calendar signals
    let calendar_year = use_signal(|| 2026i32);
    let calendar_month = use_signal(|| 6u32); // 1-indexed, e.g. June = 6
    let selected_calendar_date = use_signal(|| "2026-06-30".to_string());

    // Form inputs
    let note_subject = use_signal(String::new);
    let note_content = use_signal(String::new);
    let event_title = use_signal(String::new);
    let event_team = use_signal(|| "team-1".to_string());
    let event_assignee = use_signal(|| "user-2".to_string());
    let event_recipient = use_signal(|| "user-4".to_string());
    let event_start = use_signal(|| "09:00".to_string());
    let event_end = use_signal(|| "17:00".to_string());

    let time_date = use_signal(|| "2026-07-01".to_string());
    let time_start = use_signal(|| "08:00".to_string());
    let time_end = use_signal(|| "16:00".to_string());
    let time_hours = use_signal(|| "8.0".to_string());
    let time_note = use_signal(String::new);
    let time_view_tab = use_signal(|| "report".to_string()); // report / history
    let time_filter_status = use_signal(|| "all".to_string()); // all / pending_attest / approved / rejected
    let selected_time_reports = use_signal(Vec::<String>::new);
    let time_search_query = use_signal(String::new);

    let journal_content = use_signal(String::new);
    let med_name = use_signal(String::new);
    let med_dosage = use_signal(String::new);
    let med_frequency = use_signal(String::new);
    let med_instructions = use_signal(String::new);

    // Reporting signals
    let report_tab = use_signal(|| "send".to_string());
    let report_type = use_signal(|| "complaint".to_string());
    let report_subject = use_signal(String::new);
    let report_description = use_signal(String::new);
    let report_date = use_signal(|| "2026-07-01".to_string());
    let report_is_anonymous = use_signal(|| false);
    let report_status_filter = use_signal(|| "all".to_string());
    let report_type_filter = use_signal(|| "all".to_string());
    let selected_report_id = use_signal(|| Option::<String>::None);
    let show_report_details_modal = use_signal(|| false);

    // Directory signals
    let directory_level = use_signal(|| "teams".to_string());
    let selected_directory_workspace = use_signal(|| "workspace-1".to_string());
    let selected_directory_team = use_signal(|| Option::<String>::None);
    let show_add_team_modal = use_signal(|| false);
    let show_invite_member_modal = use_signal(|| false);
    let show_client_manager_modal = use_signal(|| false);
    let new_team_name = use_signal(String::new);
    let new_member_email = use_signal(String::new);
    let new_member_name = use_signal(String::new);
    let new_member_role = use_signal(|| "assistant".to_string());
    let new_client_first_name = use_signal(String::new);
    let new_client_last_name = use_signal(String::new);
    let new_client_personal_number = use_signal(String::new);
    let new_client_care_level = use_signal(|| "High Care".to_string());

    // Settings signals
    let settings_tab = use_signal(|| "general".to_string());
    let mut settings_name = use_signal(String::new);
    let mut settings_brand_color = use_signal(|| "hsl(217.2, 91.2%, 59.8%)".to_string());
    let mut settings_logo_url = use_signal(String::new);
    let settings_save_status = use_signal(|| "idle".to_string()); // idle / saving / saved

    // Account signals
    let mut account_name = use_signal(String::new);
    let mut account_phone = use_signal(String::new);
    let mut account_preferences = use_signal(String::new);
    let account_save_status = use_signal(|| "idle".to_string()); // idle / saving / saved
    let mut last_synced_user_id = use_signal(String::new);

    // Leave request signals
    let scheduling_sidebar_tab = use_signal(|| "shift".to_string());
    let leave_type = use_signal(|| "vacation".to_string());
    let leave_start = use_signal(|| "2026-07-01".to_string());
    let leave_end = use_signal(|| "2026-07-08".to_string());
    let leave_reason = use_signal(String::new);
    let leave_save_status = use_signal(|| "idle".to_string()); // idle / success / error
    let globalsearch_open = use_signal(|| false);
    let header_profile_open = use_signal(|| false);
    let time_group_expanded = use_signal(|| true);
    let filter_categories = use_signal(|| {
        vec![
            "schedule".to_string(),
            "bookings".to_string(),
            "personal".to_string(),
            "assistance".to_string(),
            "medical".to_string(),
        ]
    });

    // Inbox messaging signals
    let messaging_view_tab = use_signal(|| "inbox".to_string()); // inbox / sent / compose
    let active_message_id = use_signal(|| Option::<String>::None);
    let compose_recipient_id = use_signal(|| Option::<String>::None);
    let compose_subject = use_signal(String::new);
    let compose_body = use_signal(String::new);
    let compose_status = use_signal(|| "idle".to_string()); // idle / sending / success

    // Dynamic database query outputs (resources)
    let (
        workspace,
        users,
        teams,
        events,
        mut messages,
        mut notes,
        time_reports,
        clients,
        reports,
        workspaces,
        mut todos,
    ) = resources::init_resources(
        db_initialized,
        active_user_id,
        logged_in,
        background_error,
        trigger_workspaces,
        trigger_users,
        trigger_teams,
        trigger_events,
        trigger_messages,
        trigger_notes,
        trigger_time,
        trigger_clients,
        trigger_reports,
        trigger_todos,
    );

    let workspace_id = use_memo(move || {
        workspace
            .read()
            .as_ref()
            .map(|w| w.id.clone())
            .unwrap_or_else(|| "workspace-1".to_string())
    });

    // Multi-thread channel mapping database notifications to the Dioxus UI thread
    let channel = use_hook(move || {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        (tx, Arc::new(Mutex::new(Some(rx))))
    });

    // Multi-thread channel mapping OAuth redirects to the Dioxus UI thread
    let oauth_channel = use_hook(move || {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        (tx, Arc::new(Mutex::new(Some(rx))))
    });

    let on_desktop_oauth = Callback::new({
        let tx = oauth_channel.0.clone();
        move |provider: String| {
            crate::utils::loopback::start_loopback_listener(tx.clone());
            let auth_url = format!(
                "https://ileffmdueouhbooesjnv.supabase.co/auth/v1/authorize?provider={}&redirect_to=http://localhost:5173/",
                provider
            );
            crate::utils::browser::open_in_system_browser(&auth_url);
        }
    });

    // Run the receiver task and register the observer
    use_effect(move || {
        let mut rx_opt = channel.1.lock().unwrap();
        if let Some(mut rx) = rx_opt.take() {
            spawn(async move {
                let mut pending_tables = std::collections::HashSet::new();
                loop {
                    tokio::select! {
                        val = rx.recv() => {
                            if let Some(table) = val {
                                pending_tables.insert(table);

                                // Coalescing window: sleep first to let subsequent table changes pool in the channel
                                crate::utils::sleep_ms(50).await;

                                // Now drain all accumulated changes from the channel in a single batch
                                while let Ok(table) = rx.try_recv() {
                                    pending_tables.insert(table);
                                }

                                let mut update_todos = false;
                                let mut update_users = false;
                                let mut update_teams = false;
                                let mut update_events = false;
                                let mut update_messages = false;
                                let mut update_notes = false;
                                let mut update_time = false;
                                let mut update_clients = false;
                                let mut update_school = false;
                                let mut update_reports = false;
                                let mut update_workspaces = false;
                                let mut update_jobs = false;
                                let mut update_db = false;

                                let mut todo_record_updates = Vec::new();
                                let mut message_record_updates = Vec::new();
                                let mut note_record_updates = Vec::new();

                                for table in pending_tables.drain() {
                                    if let Some(pos) = table.find(':') {
                                        let table_name = &table[..pos];
                                        let record_id = &table[pos+1..];
                                        match table_name {
                                            "todos" => todo_record_updates.push(record_id.to_string()),
                                            "messages" => message_record_updates.push(record_id.to_string()),
                                            "notes" => note_record_updates.push(record_id.to_string()),
                                            _ => {
                                                match table_name {
                                                    "todos" => update_todos = true,
                                                    "users" | "team_members" => update_users = true,
                                                    "teams" => update_teams = true,
                                                    "events" => update_events = true,
                                                    "messages" => update_messages = true,
                                                    "notes" | "note_updates" => update_notes = true,
                                                    "time_reports" => update_time = true,
                                                    "clients" | "client_medications" | "client_journals" => update_clients = true,
                                                    "student_profiles"
                                                    | "courses"
                                                    | "assignments"
                                                    | "submissions"
                                                    | "attendance_records"
                                                    | "term_grades"
                                                    | "report_cards"
                                                    | "library_books"
                                                    | "library_lending_logs"
                                                    | "school_invoices"
                                                    | "school_payments"
                                                    | "student_parents"
                                                    | "health_records"
                                                    | "health_incidents"
                                                    | "school_conflicts"
                                                    | "local_blobs" => update_school = true,
                                                    "reports" => update_reports = true,
                                                    "workspaces" => update_workspaces = true,
                                                    "job_tickets" | "move_inventory" | "move_quotes" => update_jobs = true,
                                                    _ => {}
                                                }
                                            }
                                        }
                                    } else {
                                        match table.as_str() {
                                            "todos" => update_todos = true,
                                            "users" | "team_members" => update_users = true,
                                            "teams" => update_teams = true,
                                            "events" => update_events = true,
                                            "messages" => update_messages = true,
                                            "notes" | "note_updates" => update_notes = true,
                                            "time_reports" => update_time = true,
                                            "clients" | "client_medications" | "client_journals" => update_clients = true,
                                            "student_profiles"
                                            | "courses"
                                            | "assignments"
                                            | "submissions"
                                            | "attendance_records"
                                            | "term_grades"
                                            | "report_cards"
                                            | "library_books"
                                            | "library_lending_logs"
                                            | "school_invoices"
                                            | "school_payments"
                                            | "student_parents"
                                            | "health_records"
                                            | "health_incidents"
                                            | "school_conflicts"
                                            | "local_blobs" => update_school = true,
                                            "reports" => update_reports = true,
                                            "workspaces" => update_workspaces = true,
                                            "job_tickets" | "move_inventory" | "move_quotes" => update_jobs = true,
                                            "audit_logs" => {},
                                            "bankid_auth_sessions" => {},
                                            _ => {
                                                update_todos = true;
                                                update_users = true;
                                                update_teams = true;
                                                update_events = true;
                                                update_messages = true;
                                                update_notes = true;
                                                update_time = true;
                                                update_clients = true;
                                                update_school = true;
                                                update_reports = true;
                                                update_workspaces = true;
                                                update_jobs = true;
                                            }
                                        }
                                    }
                                    if table != "audit_logs" {
                                        update_db = true;
                                    }
                                }

                                if todo_record_updates.len() > 3 {
                                    update_todos = true;
                                } else {
                                    for record_id in todo_record_updates {
                                        let ws_id = workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());
                                        let uid = active_user_id.read().clone();
                                        spawn({
                                            let record_id = record_id.clone();
                                            async move {
                                                if let Ok(Some(item)) = yntra_core::get_todo_by_id(uid, ws_id, record_id).await {
                                                    if let Some(list) = todos.write().as_mut() {
                                                        if let Some(pos) = list.iter().position(|x| x.id == item.id) {
                                                            list[pos] = item;
                                                        } else {
                                                            list.push(item);
                                                            list.sort_by(|a, b| a.id.cmp(&b.id));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }

                                if message_record_updates.len() > 3 {
                                    update_messages = true;
                                } else {
                                    for record_id in message_record_updates {
                                        let ws_id = workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());
                                        let uid = active_user_id.read().clone();
                                        spawn({
                                            let record_id = record_id.clone();
                                            async move {
                                                if let Ok(Some(item)) = yntra_core::get_message_by_id(uid, ws_id, record_id).await {
                                                    if let Some(list) = messages.write().as_mut() {
                                                        if let Some(pos) = list.iter().position(|x| x.id == item.id) {
                                                            list[pos] = item;
                                                        } else {
                                                            list.push(item);
                                                            list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }

                                if note_record_updates.len() > 3 {
                                    update_notes = true;
                                } else {
                                    for record_id in note_record_updates {
                                        let ws_id = workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());
                                        let uid = active_user_id.read().clone();
                                        spawn({
                                            let record_id = record_id.clone();
                                            async move {
                                                if let Ok(Some(item)) = yntra_core::get_note_by_id(uid, ws_id, record_id).await {
                                                    if let Some(list) = notes.write().as_mut() {
                                                        if let Some(pos) = list.iter().position(|x| x.id == item.id) {
                                                            list[pos] = item;
                                                        } else {
                                                            list.push(item);
                                                            list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }

                                 if update_todos { let next = *trigger_todos.read() + 1; trigger_todos.set(next); }
                                 if update_users { let next = *trigger_users.read() + 1; trigger_users.set(next); }
                                 if update_teams { let next = *trigger_teams.read() + 1; trigger_teams.set(next); }
                                 if update_events { let next = *trigger_events.read() + 1; trigger_events.set(next); }
                                 if update_messages { let next = *trigger_messages.read() + 1; trigger_messages.set(next); }
                                 if update_notes { let next = *trigger_notes.read() + 1; trigger_notes.set(next); }
                                 if update_time { let next = *trigger_time.read() + 1; trigger_time.set(next); }
                                 if update_clients { let next = *trigger_clients.read() + 1; trigger_clients.set(next); }
                                 if update_school { let next = *trigger_school.read() + 1; trigger_school.set(next); }
                                 if update_reports { let next = *trigger_reports.read() + 1; trigger_reports.set(next); }
                                 if update_workspaces { let next = *trigger_workspaces.read() + 1; trigger_workspaces.set(next); }
                                 if update_jobs { let next = *trigger_jobs.read() + 1; trigger_jobs.set(next); }
                                 if update_db { let next = *db_trigger.read() + 1; db_trigger.set(next); }
                            } else {
                                break;
                            }
                        }
                    }
                }
            });
        }

        let tx = channel.0.clone();
        clear_observers();
        let observer = Box::new(DioxusDbObserver { tx });
        register_observer(observer);
    });

    // Listen for desktop OAuth loopback redirects & Supabase hash redirects
    oauth::init_oauth_handlers(
        active_user_id,
        active_section,
        needs_setup,
        logged_in,
        login_error,
        two_factor_user,
        active_user_role,
        &oauth_channel,
    );

    // Sync settings signals with loaded workspace and active user
    use_effect(move || {
        let ws = workspace.read().clone().unwrap_or_else(|| Workspace {
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

        let ws_name = ws.name.clone();
        if settings_name.read().is_empty() && *settings_name.read() != ws_name {
            settings_name.set(ws_name);
        }
        if *settings_brand_color.read() == "hsl(217.2, 91.2%, 59.8%)"
            && ws.brand_color != "hsl(217.2, 91.2%, 59.8%)"
        {
            settings_brand_color.set(ws.brand_color.clone());
        }
        let ws_logo = ws.logo_url.clone().unwrap_or_default();
        if settings_logo_url.read().is_empty() && *settings_logo_url.read() != ws_logo {
            settings_logo_url.set(ws_logo);
        }

        let current_uid = active_user_id.read().clone();
        let users_list = users.read().clone().unwrap_or_default();
        if let Some(u) = users_list.iter().find(|u| u.id == current_uid)
            && current_uid != *last_synced_user_id.read()
        {
            account_name.set(u.full_name.clone().unwrap_or_default());
            account_phone.set(u.phone.clone().unwrap_or_default());
            account_preferences.set(u.preferences.clone());
            last_synced_user_id.set(current_uid);
        }
    });

    let active_user_id_for_effect = active_user_id;
    use_effect(move || {
        let uid = active_user_id_for_effect.read().clone();
        let users_list = users.read().clone().unwrap_or_default();
        if let Some(user) = users_list.iter().find(|u| u.id == uid) {
            let user_prefs: serde_json::Value =
                serde_json::from_str(&user.preferences).unwrap_or_default();
            let mut lang_opt = user_prefs
                .get("language")
                .and_then(|l| l.as_str())
                .map(|s| s.to_string());

            if lang_opt.is_none() {
                if let Some(ws) = workspace.read().clone() {
                    let settings_val: serde_json::Value =
                        serde_json::from_str(&ws.settings).unwrap_or_default();
                    lang_opt = settings_val
                        .get("language")
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());
                }
            }

            if let Some(lang) = lang_opt {
                let norm_lang = match lang.to_lowercase().as_str() {
                    "sv" | "se" => "sv",
                    "no" | "nb" | "nn" => "no",
                    "da" | "dk" => "da",
                    "fi" => "fi",
                    _ => "en",
                };
                if *auth_region.read() != norm_lang {
                    auth_region.set(norm_lang.to_string());
                }
            }
        }
    });

    AppState {
        db_trigger,
        active_user_id,
        active_section,
        needs_setup,
        two_factor_user,
        logged_in,
        login_tab,
        show_bankid_modal,
        scanning_state,
        login_email,
        login_password,
        login_error,
        auth_region,

        selected_note_team_id,
        selected_note_id,
        is_note_composing,
        selected_client_id,

        calendar_year,
        calendar_month,
        selected_calendar_date,

        note_subject,
        note_content,
        event_title,
        event_team,
        event_assignee,
        event_recipient,
        event_start,
        event_end,

        time_date,
        time_start,
        time_end,
        time_hours,
        time_note,
        time_view_tab,
        time_filter_status,
        selected_time_reports,
        time_search_query,

        journal_content,
        med_name,
        med_dosage,
        med_frequency,
        med_instructions,

        report_tab,
        report_type,
        report_subject,
        report_description,
        report_date,
        report_is_anonymous,
        report_status_filter,
        report_type_filter,
        selected_report_id,
        show_report_details_modal,

        directory_level,
        selected_directory_workspace,
        selected_directory_team,
        show_add_team_modal,
        show_invite_member_modal,
        show_client_manager_modal,
        new_team_name,
        new_member_email,
        new_member_name,
        new_member_role,
        new_client_first_name,
        new_client_last_name,
        new_client_personal_number,
        new_client_care_level,

        settings_tab,
        settings_name,
        settings_brand_color,
        settings_logo_url,
        settings_save_status,

        account_name,
        account_phone,
        account_preferences,
        account_save_status,
        last_synced_user_id,

        scheduling_sidebar_tab,
        leave_type,
        leave_start,
        leave_end,
        leave_reason,
        leave_save_status,

        active_user_role,

        globalsearch_open,
        header_profile_open,
        time_group_expanded,
        filter_categories,

        messaging_view_tab,
        active_message_id,
        compose_recipient_id,
        compose_subject,
        compose_body,
        compose_status,

        trigger_jobs,
        trigger_todos,
        trigger_clients,
        trigger_school,

        workspace,
        users,
        teams,
        events,
        messages,
        notes,
        time_reports,
        clients,
        reports,
        workspaces,
        todos,
        workspace_id,
        on_desktop_oauth,
        background_error,
        db_initialized,
    }
}
