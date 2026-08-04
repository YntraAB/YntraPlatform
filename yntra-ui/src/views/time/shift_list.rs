use super::utils::format_month_year;

use crate::components;

use crate::locales::t;

use dioxus::prelude::*;

use yntra_core::TimeReport;

use yntra_core::WorkspaceUser;

use yntra_core::{delete_time_report, update_time_report_status};

#[derive(Props, Clone)]

pub struct ShiftListProps {
    pub filtered_reports: Vec<TimeReport>,

    pub users: Vec<WorkspaceUser>,

    pub teams: Vec<yntra_core::Team>,

    pub is_manager: bool,

    pub selected_time_reports: Signal<Vec<String>>,

    pub time_filter_status: Signal<String>,

    pub db_trigger: Signal<u32>,

    pub selected_user_id: Signal<Option<String>>,

    pub selected_team_id: Signal<Option<String>>,

    pub current_level: Signal<String>,

    pub active_user_role: String,

    pub list_mode: Signal<String>,

    pub current_page: Signal<i32>,

    pub time_search_query: Signal<String>,

    pub show_report_modal: Signal<bool>,

    pub locale: String,
}

impl PartialEq for ShiftListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]

pub fn ShiftList(props: ShiftListProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let region = state.auth_region.read();

    let requester_id = state.active_user_id.read().clone();

    let mut context_menu_open = use_signal(|| false);

    let mut context_menu_pos = use_signal(|| (0, 0));

    let mut context_menu_report = use_signal(|| Option::<TimeReport>::None);
    let requester_id_approve = requester_id.clone();

    let requester_id_reject = requester_id.clone();

    let requester_id_delete = requester_id.clone();

    let filtered_reports = props.filtered_reports.clone();

    let users = props.users.clone();

    let teams = props.teams.clone();

    let is_manager = props.is_manager;

    let mut selected_time_reports = props.selected_time_reports;

    let mut time_filter_status = props.time_filter_status;

    let mut db_trigger = props.db_trigger;

    let mut selected_user_id = props.selected_user_id;

    let mut selected_team_id = props.selected_team_id;

    let mut current_level = props.current_level;

    let active_user_role = props.active_user_role.clone();

    let mut list_mode = props.list_mode;

    let mut current_page = props.current_page;

    let mut time_search_query = props.time_search_query;

    let mut show_report_modal = props.show_report_modal;

    let history_reports: Vec<TimeReport> = filtered_reports
        .iter()
        .filter(|r| r.status == "approved")
        .cloned()
        .collect();

    let items_per_page = 10;

    let total_pages = (std::cmp::max(1, filtered_reports.len().div_ceil(items_per_page))) as i32;

    let current_page_val = *current_page.read();

    let current_page_val = if current_page_val > total_pages {
        total_pages
    } else {
        current_page_val
    };

    let start_idx = ((current_page_val - 1) * items_per_page as i32) as usize;

    let end_idx = std::cmp::min(
        filtered_reports.len(),
        (current_page_val * items_per_page as i32) as usize,
    );

    let page_reports = if start_idx < filtered_reports.len() {
        filtered_reports[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };

    let all_page_ids: Vec<String> = page_reports.iter().map(|r| r.id.clone()).collect();

    let is_all_selected = !all_page_ids.is_empty()
        && all_page_ids
            .iter()
            .all(|id| selected_time_reports.read().contains(id));

    rsx! {



        div { class: "relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in",



            // View subheader tab selection



            div { class: "flex h-14 shrink-0 items-center justify-between border-b border-border/50 px-8 bg-sidebar",



                div { class: "flex h-full items-center gap-8",



                    button {



                        onclick: move |_| {



                            list_mode.set("current".to_string());



                            selected_time_reports.set(Vec::new());



                        },



                        class: if *list_mode.read() == "current" { "flex h-full items-center gap-2 border-b-2 border-primary text-sm font-semibold text-foreground transition-all duration-200 bg-transparent border-0 cursor-pointer p-0" } else { "flex h-full items-center gap-2 border-b-2 border-transparent text-sm font-semibold text-muted-foreground hover:text-foreground/70 transition-all duration-200 bg-transparent border-0 cursor-pointer p-0" },



                        "Aktuella pass"



                    }



                    button {



                        onclick: move |_| {



                            list_mode.set("history".to_string());



                            selected_time_reports.set(Vec::new());



                        },



                        class: if *list_mode.read() == "history" { "flex h-full items-center gap-2 border-b-2 border-primary text-sm font-semibold text-foreground transition-all duration-200 bg-transparent border-0 cursor-pointer p-0" } else { "flex h-full items-center gap-2 border-b-2 border-transparent text-sm font-semibold text-muted-foreground hover:text-foreground/70 transition-all duration-200 bg-transparent border-0 cursor-pointer p-0" },



                        "Tidigare månader"



                    }



                }



                button {



                    class: "yntra-btn secondary text-xs flex items-center gap-1.5 h-8 px-3 rounded-lg font-semibold border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer",



                    onclick: move |_| {



                        selected_user_id.set(None);



                        selected_team_id.set(None);



                        if active_user_role == "platform_admin" {



                            current_level.set("platform_overview".to_string());



                        } else if active_user_role == "admin" {



                            current_level.set("team_overview".to_string());



                        } else {



                            current_level.set("assistant_teams".to_string());



                        }



                        selected_time_reports.set(Vec::new());



                    },



                    components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }



                    "Tillbaka"



                }



            }







            if *list_mode.read() == "history" {



                div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",



                    if history_reports.is_empty() {



                        div { class: "text-center text-muted-foreground/60 py-16",



                            components::LucideIcon { name: "calendar", class: "h-12 w-12 mx-auto mb-3 opacity-20" }



                            p { class: "text-sm m-0", "Det finns ingen historik för godkända pass." }



                        }



                    } else {



                        for report in history_reports.iter() {



                            {



                                let month_label = format_month_year(&report.date);



                                rsx! {



                                    div {



                                        key: "{report.id}",



                                        class: "group flex items-center border-b border-border/30 px-8 py-4 transition-colors hover:bg-white/[0.015] list-item-hover",







                                        div { class: "mr-5 flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-emerald-500/5 font-bold text-emerald-500",



                                            components::LucideIcon { name: "file-check", class: "h-5 w-5" }



                                        }



                                        div { class: "w-56 shrink-0 pr-4",



                                            div { class: "text-[15px] font-semibold text-foreground", "{month_label}" }



                                            div { class: "mt-1 flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wider text-emerald-400",



                                                components::LucideIcon { name: "check-circle", class: "h-3.5 w-3.5 text-emerald-400" }



                                                "Attesterad"



                                            }



                                        }



                                        div { class: "flex min-w-0 flex-1 items-center gap-10 pr-4",



                                            div { class: "flex flex-col gap-1",



                                                div { class: "text-[10px] font-bold uppercase tracking-widest text-muted-foreground/50", "Work Time" }



                                                div { class: "font-mono text-[14px] font-bold text-foreground", "{report.hours}h" }



                                            }



                                            div { class: "flex flex-col gap-1",



                                                div { class: "text-[10px] font-bold uppercase tracking-widest text-muted-foreground/50", "OB Bonus" }



                                                div { class: "font-mono text-[14px] font-bold text-foreground", "0h" }



                                            }



                                            div { class: "flex flex-col gap-1",



                                                div { class: "text-[10px] font-bold uppercase tracking-widest text-rose-400/40", "Absence" }



                                                div { class: "font-mono text-[14px] font-bold text-foreground", "-" }



                                            }



                                        }



                                        div { class: "flex w-48 shrink-0 flex-col items-end justify-center pr-6",



                                            div { class: "mb-0.5 text-[10px] font-bold uppercase tracking-widest text-muted-foreground/50", "Lön (Netto)" }



                                            span { class: "text-[16px] font-bold text-foreground", "-" }



                                        }



                                        div { class: "flex w-8 shrink-0 items-center justify-end text-muted-foreground/30 transition-all group-hover:translate-x-1 group-hover:text-foreground",



                                            components::LucideIcon { name: "chevron-right", class: "h-5 w-5" }



                                        }



                                    }



                                }



                            }



                        }



                    }



                }



            } else {



                // Current Shifts layout



                div { class: "flex min-h-0 flex-1 flex-col",



                    // Table / Toolbar actions



                    div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border/50 px-8 bg-sidebar/50",



                        div { class: "flex items-center gap-5",



                            div {



                                class: "group flex w-8 cursor-pointer justify-center",



                                onclick: move |_| {



                                    let mut vec = selected_time_reports.read().clone();



                                    if is_all_selected {



                                        vec.retain(|id| !all_page_ids.contains(id));



                                    } else {



                                        for id in all_page_ids.iter() {



                                            if !vec.contains(id) {



                                                vec.push(id.clone());



                                            }



                                        }



                                    }



                                    selected_time_reports.set(vec);



                                },



                                div {



                                    class: format!("w-4.5 h-4.5 rounded-[5px] border-2 flex items-center justify-center transition-all {}", if is_all_selected { "border-primary bg-primary" } else { "border-border group-hover:border-primary/50" }),



                                    if is_all_selected {



                                        components::LucideIcon { name: "check", class: "h-3.5 w-3.5 stroke-[3] text-primary-foreground" }



                                    }



                                }



                            }







                            crate::components::Select {



                                trigger_class: "text-xs h-9 px-3 border border-border bg-white/[0.01]",



                                trigger_style: "max-width:180px;",



                                value: "{time_filter_status}",



                                onchange: move |val: String| {



                                    time_filter_status.set(val);



                                    selected_time_reports.set(Vec::new());



                                },



                                options: vec![



                                    ("all".to_string(), format!("Alla rapporter ({})", filtered_reports.len())),



                                    ("pending_attest".to_string(), format!("Väntar attest ({})", filtered_reports.iter().filter(|r| r.status == "pending_attest").count())),



                                    ("approved".to_string(), format!("Godkända ({})", filtered_reports.iter().filter(|r| r.status == "approved").count())),



                                    ("rejected".to_string(), format!("Avvisade ({})", filtered_reports.iter().filter(|r| r.status == "rejected").count())),



                                ],



                            }







                            if !selected_time_reports.read().is_empty() {



                                div { class: "flex items-center gap-2 border-l border-border/50 pl-5 duration-300 animate-in fade-in slide-in-from-left-2",



                                    if is_manager {



                                        button {



                                            class: "yntra-btn text-xs flex items-center gap-1.5 h-9 px-3 rounded-lg font-bold shadow-lg shadow-emerald-500/10 hover:bg-emerald-600 transition-colors cursor-pointer",



                                            style: "background: var(--success); border-color: var(--success); color: white;",



                                            onclick: move |_| {



                                                let ids = selected_time_reports.read().clone();



                                                let req_id = requester_id_approve.clone();



                                                spawn(async move {



                                                    for id in ids {



                                                        let _ = update_time_report_status(req_id.clone(), id, "approved".to_string()).await;



                                                    }



                                                });



                                                selected_time_reports.set(Vec::new());



                                                let current_trig = *db_trigger.read();



                                                db_trigger.set(current_trig + 1);



                                            },



                                            components::LucideIcon { name: "check-square", class: "h-4 w-4" }



                                            "Attestera valda ({selected_time_reports.read().len()})"



                                        }



                                    }



                                    if is_manager {



                                        button {



                                            class: "yntra-btn text-xs flex items-center gap-1.5 h-9 px-3 rounded-lg font-bold shadow-lg shadow-red-500/10 hover:bg-red-600 transition-colors cursor-pointer",



                                            style: "background: var(--danger); border-color: var(--danger); color: white;",



                                            onclick: move |_| {



                                                let ids = selected_time_reports.read().clone();



                                                let req_id = requester_id_reject.clone();



                                                spawn(async move {



                                                    for id in ids {



                                                        let _ = update_time_report_status(req_id.clone(), id, "rejected".to_string()).await;



                                                    }



                                                });



                                                selected_time_reports.set(Vec::new());



                                                let current_trig = *db_trigger.read();



                                                db_trigger.set(current_trig + 1);



                                            },



                                            components::LucideIcon { name: "slash", class: "h-4 w-4" }



                                            "Avvisa valda"



                                        }



                                    }



                                    button {



                                        class: "yntra-btn secondary h-9 w-9 text-muted-foreground transition-colors hover:bg-rose-500/10 hover:text-rose-500 cursor-pointer flex items-center justify-center p-0 rounded-lg border border-border",



                                        onclick: move |_| {



                                            let ids = selected_time_reports.read().clone();



                                            let req_id = requester_id_delete.clone();



                                            spawn(async move {



                                                for id in ids {



                                                    let _ = delete_time_report(req_id.clone(), id).await;



                                                }



                                            });



                                            selected_time_reports.set(Vec::new());



                                            let current_trig = *db_trigger.read();



                                            db_trigger.set(current_trig + 1);



                                        },



                                        components::LucideIcon { name: "trash", class: "h-4 w-4" }



                                    }



                                }



                            }



                        }







                        div { class: "flex items-center gap-5",



                            if !is_manager {



                                button {



                                    class: "yntra-btn h-9 text-xs flex items-center gap-1.5 font-bold shadow-lg cursor-pointer px-4 rounded-lg",



                                    onclick: move |_| show_report_modal.set(true),



                                    components::LucideIcon { name: "clock", class: "h-4 w-4" }



                                    "{t(\"timereports-report-time-btn\", &props.locale)}"



                                }



                            }



                            div { class: "relative hidden sm:block",



                                components::LucideIcon { name: "search", class: "absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground/40" }



                                crate::components::Input {



                                    class: "text-xs h-9 pl-9 rounded-lg border-none bg-white/[0.02] focus:outline-none focus:ring-1 focus:ring-primary/20",



                                    style: "padding-left: 2.25rem;",



                                    placeholder: t("timereports-search-shifts-placeholder", &props.locale),



                                    value: "{time_search_query}",



                                    oninput: move |e: FormEvent| time_search_query.set(e.value()),



                                }



                            }



                            div { class: "flex items-center gap-3 text-muted-foreground/60",



                                button {



                                    class: "yntra-btn secondary h-8 w-8 rounded-lg p-0 flex items-center justify-center border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer",



                                    disabled: current_page_val == 1,



                                    onclick: move |_| {



                                        if current_page_val > 1 {



                                            current_page.set(current_page_val - 1);



                                        }



                                    },



                                    components::LucideIcon { name: "chevron-left", class: "h-4 w-4" }



                                }



                                span { class: "text-[11px] font-bold tracking-widest text-foreground/80", "{current_page_val} / {total_pages}" }



                                button {



                                    class: "yntra-btn secondary h-8 w-8 rounded-lg p-0 flex items-center justify-center border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer",



                                    disabled: current_page_val == total_pages,



                                    onclick: move |_| {



                                        if current_page_val < total_pages {



                                            current_page.set(current_page_val + 1);



                                        }



                                    },



                                    components::LucideIcon { name: "chevron-right", class: "h-4 w-4" }



                                }



                            }



                        }



                    }







                    // The actual list of report items



                    div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",



                        if page_reports.is_empty() {



                            div { class: "text-center text-muted-foreground/60 py-16",



                                components::LucideIcon { name: "calendar", class: "h-12 w-12 mx-auto mb-3 opacity-20" }



                                p { class: "text-sm m-0", "Det finns inga pass rapporterade för denna vy." }



                            }



                        } else {



                            for report in page_reports.into_iter() {



                                {



                                    let is_checked = selected_time_reports.read().contains(&report.id);



                                    let user_name = users



                                        .iter()



                                        .find(|u| u.id == report.user_id)



                                        .and_then(|u| u.full_name.clone())



                                        .unwrap_or_else(|| "Unknown".to_string());



                                    let team_name = teams



                                        .iter()



                                        .find(|t| Some(t.id.clone()) == report.team_id)



                                        .map(|t| t.name.clone())



                                        .unwrap_or_else(|| "Unassigned Team".to_string());







                                    let display_title = if selected_team_id.read().is_some() {



                                        user_name.clone()



                                    } else {



                                        team_name.clone()



                                    };







                                    let (status_text, status_color, status_bg) = match report.status.as_str() {



                                        "approved" => ("Godkänt", "var(--success)", "rgba(16, 185, 129, 0.1)"),



                                        "rejected" => ("Avvisat", "var(--danger)", "rgba(239, 68, 68, 0.1)"),



                                        _ => ("Väntar attest", "var(--warning)", "rgba(245, 158, 11, 0.1)"),



                                    };







                                    let start_time_str = report.start_time.clone().unwrap_or_else(|| "08:00".to_string());



                                    let end_time_str = report.end_time.clone().unwrap_or_else(|| "17:00".to_string());








                                    let report_c = report.clone();
                                    rsx! {



                                        div {



                                            key: "{report.id}",




                                            oncontextmenu: move |evt| {
                                                evt.prevent_default();
                                                let coords = evt.client_coordinates();
                                                context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                context_menu_report.set(Some(report_c.clone()));
                                                context_menu_open.set(true);
                                            },
                                            class: "group flex items-center border-b border-border/30 px-8 py-3.5 transition-colors hover:bg-white/[0.015] list-item-hover",



                                            style: if is_checked { "background: rgba(255,255,255,0.02);" } else { "" },







                                            // Individual Row Checkbox selector



                                            div {



                                                class: "flex w-8 shrink-0 cursor-pointer justify-center p-1",



                                                onclick: move |_| {



                                                    let mut vec = selected_time_reports.read().clone();



                                                    if vec.contains(&report.id) {



                                                        vec.retain(|id| id != &report.id);



                                                    } else {



                                                        vec.push(report.id.clone());



                                                    }



                                                    selected_time_reports.set(vec);



                                                },



                                                div {



                                                    class: format!("w-4 h-4 rounded-[4px] border transition-all {}", if is_checked { "border-primary bg-primary" } else { "border-input bg-transparent group-hover:border-primary/40" }),



                                                    if is_checked {



                                                        components::LucideIcon { name: "check", class: "h-3 w-3 stroke-[3] text-primary-foreground" }



                                                    }



                                                }



                                            }







                                            div {



                                                class: "ml-4 w-48 shrink-0 truncate pr-4 text-sm font-semibold text-foreground md:w-64",



                                                div {



                                                    class: "transition-colors group-hover:text-primary",



                                                    "{display_title}"



                                                }



                                                div {



                                                    class: "truncate text-xs text-muted-foreground/60 font-normal mt-0.5",



                                                    "{report.note.clone().unwrap_or_default()}"



                                                }



                                            }







                                            div {



                                                class: "flex min-w-0 flex-1 items-center gap-6 pr-4",



                                                div {



                                                    class: "flex items-center gap-2 text-sm text-muted-foreground/80",



                                                    components::LucideIcon { name: "clock", class: "h-3.5 w-3.5 text-muted-foreground/40" }



                                                    span { class: "font-mono", "{start_time_str} - {end_time_str}" }



                                                }



                                                div {



                                                    class: "flex items-center gap-1 text-sm font-semibold",



                                                    span { "{report.hours}" }



                                                    span { class: "text-xs text-muted-foreground/60 font-normal", "h" }



                                                }



                                            }







                                            div {



                                                class: "flex w-40 shrink-0 items-center justify-end pr-4",



                                                span {



                                                    class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-[10px] font-bold tracking-wide uppercase",



                                                    style: "background: {status_bg}; color: {status_color}; border: 1px solid {status_color}20;",



                                                    "{status_text}"



                                                }



                                            }







                                            div {



                                                class: "flex w-[120px] shrink-0 items-center justify-end gap-3 text-right text-xs font-mono text-muted-foreground/60",



                                                "{report.date}"



                                            }



                                        }



                                    }



                                }



                            }



                        }



                    }



                }

            if let Some(report) = context_menu_report.read().clone() {
                {
                    let report_id = report.id.clone();
                    let report_status = report.status.clone();
                    let is_approved = report_status == "approved";
                    let is_rejected = report_status == "rejected";

                    let approve_uid = requester_id.clone();
                    let approve_id = report_id.clone();
                    let reject_uid = requester_id.clone();
                    let reject_id = report_id.clone();
                    let delete_uid = requester_id.clone();
                    let delete_id = report_id.clone();

                    let detail_str = format!(
                        "Shift Report
Date: {}
Hours: {}h
Note: {}",
                        report.date,
                        report.hours,
                        report.note.clone().unwrap_or_default()
                    );

                    let db_trig = db_trigger;

                    rsx! {
                        components::ContextMenu {
                            open: *context_menu_open.read(),
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            if is_manager {
                                if !is_approved {
                                    button {
                                        class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                        onclick: move |_| {
                                            let u_id = approve_uid.clone();
                                            let r_id = approve_id.clone();
                                            let mut d_trig = db_trig;
                                            spawn(async move {
                                                if update_time_report_status(u_id, r_id, "approved".to_string()).await.is_ok() {
                                                    let current = *d_trig.read();
                                                    d_trig.set(current + 1);
                                                }
                                            });
                                            context_menu_open.set(false);
                                        },
                                        components::LucideIcon { name: "check", size: "14" }
                                        "{t(\"time-action-approve-shift\", &region)}"
                                    }
                                }
                                if !is_rejected {
                                    button {
                                        class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                        onclick: move |_| {
                                            let u_id = reject_uid.clone();
                                            let r_id = reject_id.clone();
                                            let mut d_trig = db_trig;
                                            spawn(async move {
                                                if update_time_report_status(u_id, r_id, "rejected".to_string()).await.is_ok() {
                                                    let current = *d_trig.read();
                                                    d_trig.set(current + 1);
                                                }
                                            });
                                            context_menu_open.set(false);
                                        },
                                        components::LucideIcon { name: "x", size: "14" }
                                        "{t(\"time-action-reject-shift\", &region)}"
                                    }
                                }
                                button {
                                    class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-red-500 hover:text-red-400 flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                    onclick: move |_| {
                                        let u_id = delete_uid.clone();
                                        let r_id = delete_id.clone();
                                        let mut d_trig = db_trig;
                                        spawn(async move {
                                            if delete_time_report(u_id, r_id).await.is_ok() {
                                                let current = *d_trig.read();
                                                d_trig.set(current + 1);
                                            }
                                        });
                                        context_menu_open.set(false);
                                    },
                                    components::LucideIcon { name: "trash", size: "14" }
                                    "{t(\"time-action-delete-report\", &region)}"
                                }
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    let js = format!("navigator.clipboard.writeText({:?});", detail_str);
                                    let _ = dioxus::document::eval(&js);
                                    context_menu_open.set(false);
                                },
                                components::LucideIcon { name: "copy", size: "14" }
                                "{t(\"time-action-copy-details\", &region)}"
                            }
                        }
                    }
                }
            }




            }



        }



    }
}
