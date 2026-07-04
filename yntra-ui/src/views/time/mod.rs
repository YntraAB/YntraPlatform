use dioxus::prelude::*;
use yntra_core::TimeReport;
use yntra_core::Workspace;
use yntra_core::WorkspaceUser;

pub mod kpi;
pub mod org_list;
pub mod member_list;
pub mod assistant_teams;
pub mod shift_list;
pub mod report_modal;
pub mod utils;

use kpi::KpiSummary;
use org_list::OrganizationList;
use member_list::MemberList;
use assistant_teams::AssistantTeamsList;
use shift_list::ShiftList;
use report_modal::TimeReportModal;

#[derive(Props, Clone)]
pub struct TimeViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub time_reports: Vec<TimeReport>,
    pub time_view_tab: Signal<String>,
    pub time_filter_status: Signal<String>,
    pub time_search_query: Signal<String>,
    pub selected_time_reports: Signal<Vec<String>>,
    pub time_date: Signal<String>,
    pub time_start: Signal<String>,
    pub time_end: Signal<String>,
    pub time_hours: Signal<String>,
    pub time_note: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub workspaces: Vec<Workspace>,
    pub teams: Vec<yntra_core::Team>,
}

impl PartialEq for TimeViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TimeView(props: TimeViewProps) -> Element {
    let active_user = props.active_user.clone();
    let user_prefs: serde_json::Value = serde_json::from_str(&active_user.preferences).unwrap_or_default();
    let region = user_prefs.get("language").and_then(|l| l.as_str()).unwrap_or("US").to_string();
    let users = props.users.clone();
    let time_reports = props.time_reports.clone();
    let workspaces = props.workspaces.clone();
    let teams = props.teams.clone();

    let _time_view_tab = props.time_view_tab;
    let time_filter_status = props.time_filter_status;
    let time_search_query = props.time_search_query;
    let selected_time_reports = props.selected_time_reports;
    let time_date = props.time_date;
    let time_start = props.time_start;
    let time_end = props.time_end;
    let time_hours = props.time_hours;
    let time_note = props.time_note;
    let db_trigger = props.db_trigger;

    let is_manager = active_user.role == "platform_admin" || active_user.role == "admin";

    // Navigation levels: platform_overview, team_overview, assistant_teams, shift_list
    let current_level = use_signal(|| {
        if active_user.role == "platform_admin" {
            "platform_overview".to_string()
        } else if active_user.role == "admin" {
            "team_overview".to_string()
        } else {
            "assistant_teams".to_string()
        }
    });
    let selected_workspace_id = use_signal(|| Option::<String>::None);
    let selected_user_id = use_signal(|| Option::<String>::None);
    let selected_team_id = use_signal(|| Option::<String>::None);
    let list_mode = use_signal(|| "current".to_string());
    let current_page = use_signal(|| 1);
    let show_report_modal = use_signal(|| false);

    let status_filter = time_filter_status.read().clone();
    let search_q = time_search_query.read().trim().to_lowercase();

    // Filter time reports for current view/search
    let filtered_reports: Vec<TimeReport> = time_reports
        .iter()
        .filter(|r| {
            let user_allowed = if is_manager {
                true
            } else {
                r.user_id == active_user.id
            };
            
            let user_ok = if let Some(uid) = selected_user_id.read().as_ref() {
                r.user_id == *uid
            } else {
                true
            };

            let team_ok = if let Some(tid) = selected_team_id.read().as_ref() {
                r.team_id.as_ref() == Some(tid)
            } else {
                true
            };

            let emp_name = users
                .iter()
                .find(|u| u.id == r.user_id)
                .and_then(|u| u.full_name.clone())
                .unwrap_or_else(|| "Unknown".to_string())
                .to_lowercase();
            let note_str = r.note.clone().unwrap_or_default().to_lowercase();
            let search_ok = search_q.is_empty() || emp_name.contains(&search_q) || note_str.contains(&search_q);

            let status_ok = if status_filter == "all" {
                true
            } else {
                r.status == status_filter
            };

            user_allowed && user_ok && team_ok && search_ok && status_ok
        })
        .cloned()
        .collect();

    // KPI panel stats
    let (approved_hrs, pending_hrs, pending_cnt, rejected_hrs) = {
        let relevant: Vec<&TimeReport> = time_reports
            .iter()
            .filter(|r| is_manager || r.user_id == active_user.id)
            .collect();
        let app: f64 = relevant
            .iter()
            .filter(|r| r.status == "approved")
            .map(|r| r.hours)
            .sum();
        let pen_hrs: f64 = relevant
            .iter()
            .filter(|r| r.status == "pending_attest")
            .map(|r| r.hours)
            .sum();
        let pen_cnt = relevant
            .iter()
            .filter(|r| r.status == "pending_attest")
            .count();
        let rej: f64 = relevant
            .iter()
            .filter(|r| r.status == "rejected")
            .map(|r| r.hours)
            .sum();
        (app, pen_hrs, pen_cnt, rej)
    };

    let resolution_rate_str = {
        let relevant: Vec<&TimeReport> = time_reports
            .iter()
            .filter(|r| is_manager || r.user_id == active_user.id)
            .collect();
        let total = relevant.len();
        let resolved = relevant.iter().filter(|r| r.status == "approved" || r.status == "rejected").count();
        (resolved * 100)
            .checked_div(total)
            .map(|v| format!("{}%", v))
            .unwrap_or_else(|| "100%".to_string())
    };

    // Filter users list based on selected workspace (for team_overview)
    let filtered_users: Vec<WorkspaceUser> = users
        .iter()
        .filter(|u| {
            if let Some(ws_id) = selected_workspace_id.read().as_ref() {
                u.workspace_id.as_ref() == Some(ws_id)
            } else {
                true
            }
        })
        .cloned()
        .collect();

    // Assistant / Caregiver Assigned Teams
    let assistant_teams_list: Vec<yntra_core::Team> = teams
        .iter()
        .filter(|t| {
            if active_user.role == "platform_admin" {
                true
            } else if let Some(ws_id) = active_user.workspace_id.as_ref() {
                t.workspace_id == *ws_id
            } else {
                false
            }
        })
        .cloned()
        .collect();

    let default_team_id = assistant_teams_list.first().map(|t| t.id.clone()).unwrap_or_default();
    let selected_report_team_id = use_signal(|| default_team_id.clone());

    rsx! {
        div { class: "flex flex-col h-full w-full bg-background box-border overflow-hidden",
            // KPI summary widgets wrapped in a padded section
            div { class: "px-8 pt-6 pb-2 shrink-0",
                KpiSummary {
                    approved_hrs,
                    pending_hrs,
                    pending_cnt,
                    rejected_hrs,
                    resolution_rate: resolution_rate_str,
                }
            }

            // Render views based on current level state
            if *current_level.read() == "platform_overview" {
                OrganizationList {
                    workspaces,
                    time_reports: time_reports.clone(),
                    selected_workspace_id,
                    current_level,
                }
            } else if *current_level.read() == "team_overview" {
                MemberList {
                    active_user_role: active_user.role.clone(),
                    filtered_users,
                    time_reports: time_reports.clone(),
                    selected_workspace_id,
                    selected_user_id,
                    selected_team_id,
                    current_level,
                }
            } else if *current_level.read() == "assistant_teams" {
                AssistantTeamsList {
                    active_user_id: active_user.id.clone(),
                    assistant_teams_list: assistant_teams_list.clone(),
                    time_reports: time_reports.clone(),
                    selected_team_id,
                    selected_user_id,
                    current_level,
                }
            } else {
                ShiftList {
                    filtered_reports,
                    users,
                    teams,
                    is_manager,
                    selected_time_reports,
                    time_filter_status,
                    db_trigger,
                    selected_user_id,
                    selected_team_id,
                    current_level,
                    active_user_role: active_user.role.clone(),
                    list_mode,
                    current_page,
                    time_search_query,
                    show_report_modal,
                    locale: region.clone(),
                }
            }

            // Modal Dialog for manual Time Reporting (Rapportera tid)
            if *show_report_modal.read() {
                TimeReportModal {
                    assistant_teams_list: assistant_teams_list.clone(),
                    time_date,
                    time_start,
                    time_end,
                    time_hours,
                    time_note,
                    selected_report_team_id,
                    db_trigger,
                    active_user_id: active_user.id.clone(),
                    workspace_id: active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string()),
                    show_report_modal,
                    locale: region.clone(),
                }
            }
        }
    }
}
