use dioxus::prelude::*;
use yntra_core::{
    get_clients, get_events, get_messages, get_notes, get_reports, get_teams,
    get_time_reports, get_users, get_workspace, get_workspaces,
    Workspace, WorkspaceUser, Team, TeamEvent, MessageItem, DailyNote, TimeReport,
    ClientProfile, ReportItem,
};

pub fn init_resources(
    active_user_id: Signal<String>,
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
) {
    let workspace = use_resource(move || {
        let _trig = trigger_workspaces.read();
        let mut bg_err = background_error;
        async move {
            match get_workspace().await {
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

    let users = use_resource(move || {
        let _trig = trigger_users.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
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
        let _trig = trigger_teams.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
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
        let _trig = trigger_events.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("scheduling").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled {
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
        let _trig = trigger_messages.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("messaging").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled {
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
        let _trig = trigger_notes.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("notes").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled {
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
        let _trig = trigger_time.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("time").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled {
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
        let _trig = trigger_clients.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("assistance").and_then(|v| v.as_bool()).unwrap_or(true)
                || modules_active_val.get("journals").and_then(|v| v.as_bool()).unwrap_or(false)
                || modules_active_val.get("medications").and_then(|v| v.as_bool()).unwrap_or(false);
            if !enabled {
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
        let _trig = trigger_reports.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        let ws_val = workspace.read().clone();
        async move {
            let modules_active_val: serde_json::Value = ws_val
                .as_ref()
                .and_then(|w| serde_json::from_str(&w.modules_active).ok())
                .unwrap_or_default();
            let enabled = modules_active_val.get("reporting").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled {
                return Vec::new();
            }
            let mut eval = dioxus::document::eval(
                r#"
                try {
                    let ids = JSON.parse(localStorage.getItem("yntra_anon_report_ids") || "[]");
                    dioxus.send(ids);
                } catch(e) {
                    dioxus.send([]);
                }
                "#
            );
            let anon_ids = match eval.recv::<Vec<String>>().await {
                Ok(ids) => ids,
                Err(_) => Vec::new(),
            };
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
        let _trig = trigger_workspaces.read();
        let uid = active_user_id.read().clone();
        let mut bg_err = background_error;
        async move {
            match get_workspaces(uid).await {
                Ok(list) => list,
                Err(e) => {
                    bg_err.set(Some(e));
                    Vec::new()
                }
            }
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
    )
}
