use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use yntra_core::{
    get_clients, get_events, get_messages, get_notes, get_reports, get_teams,
    get_time_reports, get_users, get_workspace, get_workspaces, register_observer,
    get_user_by_email, init_wasm_db, init_tracing, start_background_sync,
    Workspace, WorkspaceUser, Team, TeamEvent, MessageItem, DailyNote, TimeReport,
    ClientProfile, ReportItem,
};
use crate::locales::get_system_locale;
use crate::utils::{DioxusDbObserver, get_supabase_user_email};

fn extract_access_token(hash: &str) -> Option<String> {
    let hash_clean = hash.trim_start_matches('#').trim_start_matches('?');
    for part in hash_clean.split('&') {
        let mut kv = part.splitn(2, '=');
        if let (Some("access_token"), Some(v)) = (kv.next(), kv.next()) {
            return Some(v.to_string());
        }
    }
    None
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub db_trigger: Signal<u32>,
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

    // Desktop OAuth flow triggers
    pub on_desktop_oauth: Callback<String>,
}

pub fn use_init_app_state() -> AppState {
    // Core state signals
    let mut db_trigger = use_signal(|| 0);
    let mut trigger_todos = use_signal(|| 0);
    let mut trigger_users = use_signal(|| 0);
    let mut trigger_teams = use_signal(|| 0);
    let mut trigger_events = use_signal(|| 0);
    let mut trigger_messages = use_signal(|| 0);
    let mut trigger_notes = use_signal(|| 0);
    let mut trigger_time = use_signal(|| 0);
    let mut trigger_clients = use_signal(|| 0);
    let mut trigger_reports = use_signal(|| 0);
    let mut trigger_workspaces = use_signal(|| 0);
    let active_user_id = use_signal(|| "user-1".to_string());
    let active_section = use_signal(|| "dashboard".to_string());
    let needs_setup = use_signal(|| false);
    let two_factor_user = use_signal(|| Option::<WorkspaceUser>::None);

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
            let _ = init_wasm_db().await;
            start_background_sync(30);
            let mut eval = dioxus::document::eval(
                r#"
                let logged_in = localStorage.getItem("yntra_logged_in") === "true";
                let uid = localStorage.getItem("yntra_active_user_id") || "";
                dioxus.send(JSON.stringify({ logged_in, uid }));
                "#
            );
            if let Ok(serde_json::Value::Object(obj)) = eval.recv::<serde_json::Value>().await {
                let has_logged_in = obj.get("logged_in").and_then(|v| v.as_bool()).unwrap_or(false);
                let uid = obj.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if has_logged_in && !uid.is_empty() {
                    if let Some(user) = get_users(uid.clone()).await.ok().and_then(|all_users| {
                        all_users.into_iter().find(|u| u.id == uid)
                    }) {
                        active_uid.set(uid);
                        if user.role == "client" {
                            active_sec.set("client_portal".to_string());
                        } else {
                            active_sec.set("dashboard".to_string());
                        }
                        let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
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
            "#.to_string()
        };
        let _ = dioxus::document::eval(&js);
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
    let filter_categories = use_signal(|| vec![
        "schedule".to_string(),
        "bookings".to_string(),
        "personal".to_string(),
        "assistance".to_string(),
        "medical".to_string(),
    ]);

    // Inbox messaging signals
    let messaging_view_tab = use_signal(|| "inbox".to_string()); // inbox / sent / compose
    let active_message_id = use_signal(|| Option::<String>::None);
    let compose_recipient_id = use_signal(|| Option::<String>::None);
    let compose_subject = use_signal(String::new);
    let compose_body = use_signal(String::new);
    let compose_status = use_signal(|| "idle".to_string()); // idle / sending / success

    // Dynamic database query outputs (resources)
    let workspace = use_resource(move || {
        let _trig = trigger_workspaces.read();
        async move {
            get_workspace().await.unwrap_or_else(|_| Workspace {
                id: "workspace-1".to_string(),
                name: "Yntra Operations Ltd".to_string(),
                modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
                settings: "{}".to_string(),
                brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
                logo_url: None,
                block_settings: "{}".to_string(),
            })
        }
    });
    let users = use_resource(move || {
        let _trig = trigger_users.read();
        let uid = active_user_id.read().clone();
        async move {
            get_users(uid).await.unwrap_or_default()
        }
    });
    let teams = use_resource(move || {
        let _trig = trigger_teams.read();
        let uid = active_user_id.read().clone();
        async move {
            get_teams(uid).await.unwrap_or_default()
        }
    });
    let events = use_resource(move || {
        let _trig = trigger_events.read();
        let uid = active_user_id.read().clone();
        async move {
            get_events(uid, None).await.unwrap_or_default()
        }
    });
    let messages = use_resource(move || {
        let _trig = trigger_messages.read();
        let uid = active_user_id.read().clone();
        async move {
            get_messages(uid.clone(), uid).await.unwrap_or_default()
        }
    });
    let notes = use_resource(move || {
        let _trig = trigger_notes.read();
        let uid = active_user_id.read().clone();
        async move {
            get_notes(uid, None).await.unwrap_or_default()
        }
    });
    let time_reports = use_resource(move || {
        let _trig = trigger_time.read();
        let uid = active_user_id.read().clone();
        async move {
            get_time_reports(uid, None).await.unwrap_or_default()
        }
    });
    let clients = use_resource(move || {
        let _trig = trigger_clients.read();
        let uid = active_user_id.read().clone();
        async move {
            get_clients(uid).await.unwrap_or_default()
        }
    });
    let reports = use_resource(move || {
        let _trig = trigger_reports.read();
        let uid = active_user_id.read().clone();
        async move {
            get_reports(uid).await.unwrap_or_default()
        }
    });
    let workspaces = use_resource(move || {
        let _trig = trigger_workspaces.read();
        let uid = active_user_id.read().clone();
        async move {
            get_workspaces(uid).await.unwrap_or_default()
        }
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
                while let Some(table) = rx.recv().await {
                    match table.as_str() {
                        "todos" => {
                            let v = *trigger_todos.read();
                            trigger_todos.set(v + 1);
                        }
                        "users" | "team_members" => {
                            let v = *trigger_users.read();
                            trigger_users.set(v + 1);
                        }
                        "teams" => {
                            let v = *trigger_teams.read();
                            trigger_teams.set(v + 1);
                        }
                        "events" => {
                            let v = *trigger_events.read();
                            trigger_events.set(v + 1);
                        }
                        "messages" => {
                            let v = *trigger_messages.read();
                            trigger_messages.set(v + 1);
                        }
                        "notes" => {
                            let v = *trigger_notes.read();
                            trigger_notes.set(v + 1);
                        }
                        "time_reports" => {
                            let v = *trigger_time.read();
                            trigger_time.set(v + 1);
                        }
                        "clients" | "client_medications" | "client_journals" => {
                            let v = *trigger_clients.read();
                            trigger_clients.set(v + 1);
                        }
                        "reports" => {
                            let v = *trigger_reports.read();
                            trigger_reports.set(v + 1);
                        }
                        "workspaces" => {
                            let v = *trigger_workspaces.read();
                            trigger_workspaces.set(v + 1);
                        }
                        _ => {
                            let v_todos = *trigger_todos.read(); trigger_todos.set(v_todos + 1);
                            let v_users = *trigger_users.read(); trigger_users.set(v_users + 1);
                            let v_teams = *trigger_teams.read(); trigger_teams.set(v_teams + 1);
                            let v_events = *trigger_events.read(); trigger_events.set(v_events + 1);
                            let v_messages = *trigger_messages.read(); trigger_messages.set(v_messages + 1);
                            let v_notes = *trigger_notes.read(); trigger_notes.set(v_notes + 1);
                            let v_time = *trigger_time.read(); trigger_time.set(v_time + 1);
                            let v_clients = *trigger_clients.read(); trigger_clients.set(v_clients + 1);
                            let v_reports = *trigger_reports.read(); trigger_reports.set(v_reports + 1);
                            let v_workspaces = *trigger_workspaces.read(); trigger_workspaces.set(v_workspaces + 1);
                        }
                    }
                    let val = *db_trigger.read();
                    db_trigger.set(val + 1);
                }
            });
        }

        let tx = channel.0.clone();
        let observer = Box::new(DioxusDbObserver { tx });
        register_observer(observer);
    });

    // Listen for desktop OAuth loopback redirects
    use_effect(move || {
        let mut rx_opt = oauth_channel.1.lock().unwrap();
        if let Some(mut rx) = rx_opt.take() {
            let mut active_uid = active_user_id;
            let mut active_sec = active_section;
            let mut setup_needed = needs_setup;
            let mut is_logged_in = logged_in;
            let mut log_error = login_error;
            let mut two_factor_user = two_factor_user;

            spawn(async move {
                while let Some(hash) = rx.recv().await {
                    log::info!("[Desktop OAuth] Receiver captured hash callback!");
                        if let Some(token) = extract_access_token(&hash) {
                            log::info!("[Desktop OAuth] Extracted token, calling get_supabase_user_email...");
                            match get_supabase_user_email(&token).await {
                                Ok(email) => {
                                    log::info!("[Desktop OAuth] Supabase returned email: {}", email);
                                    if let Ok(Some(user)) = get_user_by_email(email.clone()).await {
                                        log::info!("[Desktop OAuth] Found user in database: id={}, role={}", user.id, user.role);
                                            let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
                                            let mfa_enabled = prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                                            if mfa_enabled {
                                                log::info!("[Desktop OAuth] 2FA is enabled for user, showing 2FA modal");
                                                two_factor_user.set(Some(user.clone()));
                                            } else {
                                                log::info!("[Desktop OAuth] Logging in user...");
                                                active_uid.set(user.id.clone());
                                                if user.role == "client" {
                                                    active_sec.set("client_portal".to_string());
                                                } else {
                                                    active_sec.set("dashboard".to_string());
                                                }
                                                let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                                setup_needed.set(is_new_invite);
                                                is_logged_in.set(true);
                                            }
                                        } else {
                                            let err_msg = format!("User '{}' authenticated by Supabase is not registered in this Yntra workspace.", email);
                                            log::error!("[Desktop OAuth] Error: {}", err_msg);
                                            log_error.set(Some(err_msg));
                                        }
                                    }
                                Err(e) => {
                                    let err_msg = format!("Supabase authentication failed: {}", e);
                                    log::error!("[Desktop OAuth] Error: {}", err_msg);
                                    log_error.set(Some(err_msg));
                                }
                            }
                        }
                }
            });
        }
    });

    // Check for Supabase redirect callback (hash fragment #access_token=...)
    use_effect(move || {
        let mut active_uid = active_user_id;
        let mut active_sec = active_section;
        let mut setup_needed = needs_setup;
        let mut is_logged_in = logged_in;
        let mut log_error = login_error;
        let mut two_factor_user = two_factor_user;

        spawn(async move {
            let mut eval = dioxus::document::eval("dioxus.send(window.location.hash);");
            if let Ok(serde_json::Value::String(hash)) = eval.recv::<serde_json::Value>().await {
                if let Some(token) = extract_access_token(&hash) {
                        match get_supabase_user_email(&token).await {
                            Ok(email) => {
                                if let Ok(Some(user)) = get_user_by_email(email.clone()).await {
                                        let prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
                                        let mfa_enabled = prefs.get("two_factor_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                                        if mfa_enabled {
                                            two_factor_user.set(Some(user.clone()));
                                        } else {
                                            active_uid.set(user.id.clone());
                                            if user.role == "client" {
                                                active_sec.set("client_portal".to_string());
                                            } else {
                                                active_sec.set("dashboard".to_string());
                                            }
                                            let is_new_invite = user.phone.is_none() || user.phone.as_ref().map(|p| p.is_empty()).unwrap_or(true);
                                            setup_needed.set(is_new_invite);
                                            is_logged_in.set(true);
                                        }
                                        
                                        // Clean up url hash
                                        let _ = dioxus::document::eval("window.location.hash = '';");
                                    } else {
                                        log_error.set(Some(format!("User '{}' authenticated by Supabase is not registered in this Yntra workspace.", email)));
                                    }
                                }
                            Err(e) => {
                                log_error.set(Some(format!("Supabase authentication failed: {}", e)));
                            }
                        }
                }
            }
        });
    });

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
        });

        // Only set if they are empty/default (first load)
        let ws_name = ws.name.clone();
        if settings_name.read().is_empty() && *settings_name.read() != ws_name {
            settings_name.set(ws_name);
        }
        // Always sync brand color if not changed
        if *settings_brand_color.read() == "hsl(217.2, 91.2%, 59.8%)" && ws.brand_color != "hsl(217.2, 91.2%, 59.8%)" {
            settings_brand_color.set(ws.brand_color.clone());
        }
        let ws_logo = ws.logo_url.clone().unwrap_or_default();
        if settings_logo_url.read().is_empty() && *settings_logo_url.read() != ws_logo {
            settings_logo_url.set(ws_logo);
        }

        // Sync active user details
        let current_uid = active_user_id.read().clone();
        let users_list = users.read().clone().unwrap_or_default();
        if let Some(u) = users_list.iter().find(|u| u.id == current_uid)
            && current_uid != *last_synced_user_id.read() {
                account_name.set(u.full_name.clone().unwrap_or_default());
                account_phone.set(u.phone.clone().unwrap_or_default());
                account_preferences.set(u.preferences.clone());
                last_synced_user_id.set(current_uid);
            }
    });

    let users_list = users.read().clone().unwrap_or_default();
    let active_user_id_for_effect = active_user_id;
    use_effect(move || {
        let uid = active_user_id_for_effect.read().clone();
        if let Some(user) = users_list.iter().find(|u| u.id == uid) {
            let user_prefs: serde_json::Value = serde_json::from_str(&user.preferences).unwrap_or_default();
            let mut lang_opt = user_prefs.get("language").and_then(|l| l.as_str()).map(|s| s.to_string());
            
            if lang_opt.is_none() {
                if let Some(ws) = workspace.read().clone() {
                    let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                    lang_opt = settings_val.get("language").and_then(|l| l.as_str()).map(|s| s.to_string());
                }
            }
            
            if let Some(lang) = lang_opt {
                let norm_lang = match lang.to_lowercase().as_str() {
                    "sv" | "se" => "SE",
                    "no" | "nb" | "nn" => "NO",
                    "da" | "dk" => "DK",
                    "fi" => "FI",
                    _ => "US",
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
        on_desktop_oauth,
    }
}
