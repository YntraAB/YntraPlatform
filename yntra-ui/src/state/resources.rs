use dioxus::prelude::*;
use yntra_core::{
    ClientProfile, DailyNote, InAppNotification, MessageItem, ReportItem, Team, TeamEvent,
    TimeReport, TodoItem, UserPresence, Workspace, WorkspaceUser, get_clients, get_events,
    get_messages, get_notes, get_reports, get_teams, get_time_reports, get_todos,
    get_user_notifications, get_users, get_workspace, get_workspace_presences, get_workspaces,
};

pub fn init_resources(
    db_initialized: Signal<bool>,
    active_user_id: Signal<String>,
    logged_in: Signal<bool>,
    background_error: Signal<Option<yntra_core::YntraError>>,
    trigger_workspaces: Signal<u32>,
    trigger_users: Signal<u32>,
    trigger_teams: Signal<u32>,
    trigger_events: Signal<u32>,
    trigger_messages: Signal<u32>,
    trigger_notes: Signal<u32>,
    trigger_time: Signal<u32>,
    trigger_clients: Signal<u32>,
    trigger_reports: Signal<u32>,
    trigger_todos: Signal<u32>,
    trigger_presences: Signal<u32>,
    trigger_notifications: Signal<u32>,
) -> (
    Resource<Workspace>,
    Resource<Vec<WorkspaceUser>>,
    Resource<Vec<Team>>,
    Resource<Vec<TeamEvent>>,
    Resource<Vec<MessageItem>>,
    Resource<Vec<DailyNote>>,
    Resource<Vec<TimeReport>>,
    Resource<Vec<ClientProfile>>,
    Resource<Vec<ReportItem>>,
    Resource<Vec<Workspace>>,
    Resource<Vec<TodoItem>>,
    Resource<Vec<UserPresence>>,
    Resource<Vec<InAppNotification>>,
) {
    let workspace = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let uid = active_user_id.read().clone();
        let _trig = trigger_workspaces.read();
        let mut bg_err = background_error;
        async move {
            if !initialized || !is_login {
                return Workspace {
                    id: "workspace-1".to_string(),
                    name: "Yntra Operations Ltd".to_string(),
                    modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
                    settings: "{}".to_string(),
                    brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
                    logo_url: None,
                    block_settings: "{}".to_string(),
                    updated_at: 0,
                    sync_status: "synced".to_string(),
                };
            }
            match get_workspace(uid).await {
                Ok(w) => w,
                Err(e) => {
                    bg_err.set(Some(e));
                    Workspace {
                        id: "workspace-1".to_string(),
                        name: "Yntra Operations Ltd".to_string(),
                        modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
                        settings: "{}".to_string(),
                        brand_color: "hsl(217.2, 91.2%, 59.8%)".to_string(),
                        logo_url: None,
                        block_settings: "{}".to_string(),
                        updated_at: 0,
                        sync_status: "synced".to_string(),
                    }
                }
            }
        }
    });

    let modules_active = use_memo(move || {
        if let Some(ws) = workspace.read().as_ref() {
            let val: serde_json::Value =
                serde_json::from_str(&ws.modules_active).unwrap_or_default();
            val
        } else {
            serde_json::Value::Null
        }
    });

    let scheduling_enabled = use_memo(move || {
        modules_active
            .read()
            .get("scheduling")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let messaging_enabled = use_memo(move || {
        modules_active
            .read()
            .get("messaging")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let notes_enabled = use_memo(move || {
        modules_active
            .read()
            .get("notes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let time_enabled = use_memo(move || {
        modules_active
            .read()
            .get("time")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let assistance_enabled = use_memo(move || {
        let val = modules_active.read();
        val.get("assistance")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
            || val
                .get("journals")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || val
                .get("medications")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
    });

    let reporting_enabled = use_memo(move || {
        modules_active
            .read()
            .get("reporting")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let users = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_users.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            match get_users(uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let teams = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_teams.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            match get_teams(uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let events = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_events.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *scheduling_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            match get_events(uid, None).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let messages = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_messages.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *messaging_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            match get_messages(uid.clone(), uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let notes = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_notes.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *notes_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            match get_notes(uid, None).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let time_reports = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_time.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *time_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            match get_time_reports(uid, None).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let clients = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_clients.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *assistance_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            match get_clients(uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let reports = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_reports.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let enabled = *reporting_enabled.read();
        async move {
            if !initialized || !is_login || !enabled {
                return Vec::new();
            }
            let anon_ids = crate::utils::browser::get_anon_report_ids();
            match get_reports(uid, anon_ids).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let workspaces = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_workspaces.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            match get_workspaces(uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let todos = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_todos.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_id = workspace
            .read()
            .as_ref()
            .map(|w| w.id.clone())
            .unwrap_or_else(|| "workspace-1".to_string());
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            match get_todos(uid, ws_id).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
        }
    });

    let presences = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_presences.read();
        let uid = active_user_id.read().clone();
        let ws_id = workspace
            .read()
            .as_ref()
            .map(|w| w.id.clone())
            .unwrap_or_else(|| "workspace-1".to_string());
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            get_workspace_presences(uid, ws_id)
                .await
                .unwrap_or_default()
        }
    });

    let notifications = use_resource(move || {
        let initialized = *db_initialized.read();
        let is_login = *logged_in.read();
        let _trig = trigger_notifications.read();
        let uid = active_user_id.read().clone();
        async move {
            if !initialized || !is_login {
                return Vec::new();
            }
            get_user_notifications(uid.clone(), uid)
                .await
                .unwrap_or_default()
        }
    });

    (
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
        presences,
        notifications,
    )
}
