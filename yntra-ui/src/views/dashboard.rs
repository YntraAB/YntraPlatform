use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::DailyNote;
use yntra_core::TeamEvent;
use yntra_core::TimeReport;
use yntra_core::WorkspaceUser;
use yntra_core::Team;
use yntra_core::ClientProfile;

#[derive(Props, Clone)]
pub struct DashboardViewProps {
    pub active_user: WorkspaceUser,
    pub events: Vec<TeamEvent>,
    pub unread_messages_count: usize,
    pub time_reports: Vec<TimeReport>,
    pub notes: Vec<DailyNote>,
    pub db_trigger: Signal<u32>,
    pub teams: Vec<Team>,
    pub clients: Vec<ClientProfile>,
    pub active_section: Signal<String>,
    pub auth_region: Signal<String>,
}

impl PartialEq for DashboardViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

fn calculate_hours(start: &str, end: &str, break_mins: &str) -> f64 {
    let parse_time = |t_str: &str| -> Option<(i32, i32)> {
        let parts: Vec<&str> = t_str.split(':').collect();
        if parts.len() == 2 {
            let h = parts[0].trim().parse::<i32>().ok()?;
            let m = parts[1].trim().parse::<i32>().ok()?;
            Some((h, m))
        } else {
            None
        }
    };

    if let (Some((sh, sm)), Some((eh, em))) = (parse_time(start), parse_time(end)) {
        let start_mins = sh * 60 + sm;
        let mut end_mins = eh * 60 + em;
        if end_mins < start_mins {
            end_mins += 24 * 60;
        }
        let break_m = break_mins.trim().parse::<i32>().unwrap_or(0);
        let duration_mins = end_mins - start_mins - break_m;
        if duration_mins > 0 {
            return (duration_mins as f64) / 60.0;
        }
    }
    8.0
}

fn get_formatted_today_date(today_str: &str, locale: &str) -> String {
    let parts: Vec<&str> = today_str.split('-').collect();
    if parts.len() != 3 {
        return today_str.to_string();
    }
    let year = parts[0].parse::<i32>().unwrap_or(2026);
    let month = parts[1].parse::<u32>().unwrap_or(6);
    let day = parts[2].parse::<u32>().unwrap_or(30);

    let is_leap = |y: i32| (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let days_in_month = |m: u32, y: i32| match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap(y) { 29 } else { 28 },
        _ => 30,
    };

    let mut total_days = 0;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    for m in 1..month {
        total_days += days_in_month(m, year);
    }
    total_days += day - 1;
    let weekday = (total_days + 4) % 7;

    let (wd_str, m_str) = match locale {
        "SE" => {
            let wd = match weekday {
                0 => "söndag",
                1 => "måndag",
                2 => "tisdag",
                3 => "onsdag",
                4 => "torsdag",
                5 => "fredag",
                6 => "lördag",
                _ => "",
            };
            let m = match month {
                1 => "januari",
                2 => "februari",
                3 => "mars",
                4 => "april",
                5 => "maj",
                6 => "juni",
                7 => "juli",
                8 => "augusti",
                9 => "september",
                10 => "oktober",
                11 => "november",
                12 => "december",
                _ => "",
            };
            (wd, m)
        }
        "NO" => {
            let wd = match weekday {
                0 => "søndag",
                1 => "mandag",
                2 => "tirsdag",
                3 => "onsdag",
                4 => "torsdag",
                5 => "fredag",
                6 => "lørdag",
                _ => "",
            };
            let m = match month {
                1 => "januar",
                2 => "februar",
                3 => "mars",
                4 => "april",
                5 => "mai",
                6 => "juni",
                7 => "juli",
                8 => "august",
                9 => "september",
                10 => "oktober",
                11 => "november",
                12 => "desember",
                _ => "",
            };
            (wd, m)
        }
        "DK" => {
            let wd = match weekday {
                0 => "søndag",
                1 => "mandag",
                2 => "tirsdag",
                3 => "onsdag",
                4 => "torsdag",
                5 => "fredag",
                6 => "lørdag",
                _ => "",
            };
            let m = match month {
                1 => "januar",
                2 => "februar",
                3 => "marts",
                4 => "april",
                5 => "maj",
                6 => "juni",
                7 => "juli",
                8 => "august",
                9 => "september",
                10 => "oktober",
                11 => "november",
                12 => "december",
                _ => "",
            };
            (wd, m)
        }
        _ => {
            let wd = match weekday {
                0 => "Sunday",
                1 => "Monday",
                2 => "Tuesday",
                3 => "Wednesday",
                4 => "Thursday",
                5 => "Friday",
                6 => "Saturday",
                _ => "",
            };
            let m = match month {
                1 => "January",
                2 => "February",
                3 => "March",
                4 => "April",
                5 => "May",
                6 => "June",
                7 => "July",
                8 => "August",
                9 => "September",
                10 => "October",
                11 => "November",
                12 => "December",
                _ => "",
            };
            (wd, m)
        }
    };

    if locale == "SE" || locale == "NO" || locale == "DK" {
        format!("{}, {} {} {}", wd_str, day, m_str, year)
    } else {
        format!("{}, {} {}, {}", wd_str, m_str, day, year)
    }
}

#[component]
pub fn DashboardView(props: DashboardViewProps) -> Element {
    let active_user = props.active_user;
    let events = props.events;
    let unread_messages_count = props.unread_messages_count;
    let _time_reports = props.time_reports;
    let notes = props.notes;
    let mut db_trigger = props.db_trigger;
    let teams = props.teams.clone();
    let clients = props.clients.clone();
    let mut active_section = props.active_section;
    let auth_region = props.auth_region;

    let mut show_report_time_modal = use_signal(|| false);
    let mut selected_team_id = use_signal(|| {
        teams.first().map(|t| t.id.clone()).unwrap_or_else(|| "team-1".to_string())
    });
    let mut report_date = use_signal(|| {
        let now_str = yntra_core::infra::time::get_current_datetime_str();
        if now_str.len() >= 10 {
            now_str[..10].to_string()
        } else {
            "2026-07-02".to_string()
        }
    });
    let mut report_start = use_signal(|| "08:00".to_string());
    let mut report_end = use_signal(|| "16:00".to_string());
    let mut report_break = use_signal(|| "30".to_string());
    let mut report_note = use_signal(String::new);

    let is_client = active_user.role == "client";
    let locale = auth_region.read().clone();

    // Date formatting for the header greeting
    let today_prefix = {
        let now_str = yntra_core::infra::time::get_current_datetime_str();
        if now_str.len() >= 10 {
            now_str[..10].to_string()
        } else {
            "2026-07-02".to_string()
        }
    };
    let today_formatted = get_formatted_today_date(&today_prefix, &locale);

    // Translations
    let t_welcome = t("dashboard-welcome", &locale);
    let t_upcoming_events = t("dashboard-upcoming-events", &locale);
    let t_upcoming_events_desc = t("dashboard-upcoming-events-desc", &locale);
    let t_no_events = t("dashboard-no-events", &locale);
    let t_go_to_schedule = t("dashboard-go-to-schedule", &locale);
    let t_communications = t("dashboard-communications", &locale);
    let t_communications_desc = t("dashboard-communications-desc", &locale);
    let t_inbox = t("messages-inbox", &locale);
    let t_notes = t("section-notes", &locale);
    let t_report_time_btn = t("timereports-report-time-btn", &locale);

    let user_name = active_user.full_name.clone().unwrap_or_else(|| active_user.email.clone());

    // Filter events for today and limit to 3, sorted by time
    let mut today_events: Vec<TeamEvent> = events
        .iter()
        .filter(|e| e.start_time.starts_with(&today_prefix))
        .cloned()
        .collect();
    today_events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    today_events.truncate(3);

    rsx! {
        div {
            class: "flex flex-col gap-8 p-6 bg-background",
            
            // Header bar matching reference DashboardPage
            div {
                class: "mb-8 flex flex-col justify-between gap-4 sm:flex-row sm:items-end",
                div {
                    class: "flex flex-col gap-1",
                    p {
                        class: "text-sm font-semibold uppercase tracking-wider text-muted-foreground m-0",
                        "{today_formatted}"
                    }
                    h1 {
                        class: "text-3xl font-bold tracking-tight text-foreground m-0",
                        "{t_welcome}, {user_name}"
                    }
                }
                if !is_client {
                    button {
                        class: "shrink-0 gap-2 border-emerald-700 bg-emerald-600 font-bold text-white shadow-md hover:bg-emerald-700 flex items-center px-5 py-2.5 rounded-lg cursor-pointer transition-colors text-sm border-0",
                        onclick: move |_| show_report_time_modal.set(true),
                        components::LucideIcon { name: "time", class: "h-4 w-4", }
                        "{t_report_time_btn}"
                    }
                }
            }
 
            // Grid widgets layout matching reference md:grid-cols-2 lg:grid-cols-3
            div {
                class: "grid gap-6 md:grid-cols-2 lg:grid-cols-3",
                
                // UpcomingEventsWidget (col-span-2 equivalent)
                div {
                    class: "col-span-1 lg:col-span-2",
                    components::Card {
                        class: "flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3",
                            components::CardTitle {
                                class: "flex items-center gap-2 text-lg",
                                components::LucideIcon { name: "scheduling", class: "h-5 w-5 text-primary", }
                                "{t_upcoming_events}"
                            }
                            components::CardDescription {
                                "{t_upcoming_events_desc}"
                            }
                        }
                        components::CardContent {
                            class: "flex flex-1 flex-col justify-between",
                            div {
                                class: "space-y-4",
                                if today_events.is_empty() {
                                    p {
                                        class: "text-sm text-muted-foreground m-0 py-4",
                                        "{t_no_events}"
                                    }
                                } else {
                                    for event in today_events.iter() {
                                        {
                                            let start_time_part = if event.start_time.len() >= 16 { event.start_time[11..16].to_string() } else { event.start_time.clone() };
                                            let end_time_part = if event.end_time.len() >= 16 { event.end_time[11..16].to_string() } else { event.end_time.clone() };
                                            rsx! {
                                                div {
                                                    key: "{event.id}",
                                                    class: "flex flex-col gap-1 border border-border rounded-lg p-3 bg-white/[0.015] hover:bg-white/[0.03] transition-colors cursor-default",
                                                    span {
                                                        class: "font-semibold text-sm text-foreground",
                                                        "{event.title}"
                                                    }
                                                    span {
                                                        class: "text-xs text-muted-foreground font-medium",
                                                        "{start_time_part} - {end_time_part}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            button {
                                class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                onclick: move |_| active_section.set("scheduling".to_string()),
                                span { "{t_go_to_schedule}" }
                                components::LucideIcon { name: "arrow_right", class: "h-4 w-4", }
                            }
                        }
                    }
                }
 
                // CommunicationsWidget (col-span-1 equivalent)
                div {
                    class: "col-span-1",
                    components::Card {
                        class: "flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3",
                            components::CardTitle {
                                class: "flex items-center gap-2 text-lg",
                                components::LucideIcon { name: "messaging", class: "h-5 w-5 text-primary", }
                                "{t_communications}"
                            }
                            components::CardDescription {
                                "{t_communications_desc}"
                            }
                        }
                        components::CardContent {
                            class: "flex flex-1 flex-col justify-between",
                            div {
                                class: "flex flex-col gap-4 py-2",
                                div {
                                    class: "flex justify-between items-center",
                                    span {
                                        class: "text-sm font-medium",
                                        "{t_inbox}"
                                    }
                                    if unread_messages_count > 0 {
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{unread_messages_count}"
                                        }
                                    } else {
                                        span {
                                            class: "text-sm text-muted-foreground",
                                            "0"
                                        }
                                    }
                                }
                                div {
                                    class: "flex justify-between items-center",
                                    span {
                                        class: "text-sm font-medium",
                                        "{t_notes}"
                                    }
                                    if !notes.is_empty() {
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{notes.len()}"
                                        }
                                    } else {
                                        span {
                                            class: "text-sm text-muted-foreground",
                                            "0"
                                        }
                                    }
                                }
                            }
                            div {
                                class: "mt-4 flex gap-2",
                                button {
                                    class: "flex-1 flex justify-center text-sm py-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold rounded-lg cursor-pointer transition-colors shadow-sm",
                                    onclick: move |_| active_section.set("messaging".to_string()),
                                    "{t_inbox}"
                                }
                                button {
                                    class: "flex-1 flex justify-center text-sm py-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold rounded-lg cursor-pointer transition-colors shadow-sm",
                                    onclick: move |_| active_section.set("notes".to_string()),
                                    "{t_notes}"
                                }
                            }
                        }
                    }
                }
            }
        }

        if *show_report_time_modal.read() {
            components::Dialog {
                open: *show_report_time_modal.read(),
                title: t("timereports-report-time-btn", &locale),
                onclose: move |_| show_report_time_modal.set(false),
                div { class: "flex flex-col gap-5 text-sm",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-patient-team\", &locale)}"
                        }
                        select {
                            class: "yntra-input",
                            value: "{selected_team_id}",
                            onchange: move |e| selected_team_id.set(e.value()),
                            for t in teams.iter() {
                                option { value: "{t.id}", "{t.name}" }
                            }
                            for c in clients.iter() {
                                {
                                    let patient_lbl = t("common-patient", &locale);
                                    rsx! {
                                        option { value: "client-{c.id}", "{c.first_name} {c.last_name} ({patient_lbl})" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-date-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            r#type: "text",
                            value: "{report_date}",
                            oninput: move |e| report_date.set(e.value()),
                        }
                    }
                    div { class: "grid gap-4",
                    style: "grid-template-columns:1fr 1fr;",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                                "{t(\"timereports-start-time-label\", &locale)}"
                            }
                            input {
                                class: "yntra-input",
                                r#type: "text",
                                value: "{report_start}",
                                oninput: move |e| report_start.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                                "{t(\"timereports-end-time-label\", &locale)}"
                            }
                            input {
                                class: "yntra-input",
                                r#type: "text",
                                value: "{report_end}",
                                oninput: move |e| report_end.set(e.value()),
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-break-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            r#type: "text",
                            value: "{report_break}",
                            oninput: move |e| report_break.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-note-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            value: "{report_note}",
                            placeholder: t("timereports-note-placeholder", &locale),
                            oninput: move |e| report_note.set(e.value()),
                        }
                    }
                    div { class: "flex justify-end gap-3 mt-2",
                        button {
                            class: "yntra-btn secondary",
                            style: "padding:0.45rem 1rem;",
                            onclick: move |_| show_report_time_modal.set(false),
                            "{t(\"wizard-action-cancel\", &locale)}"
                        }
                        button {
                            class: "yntra-btn text-white",
                            style: "background: var(--accent-color); border-color: var(--accent-color); padding:0.45rem 1rem;",
                            onclick: move |_| {
                                let note = report_note.read().trim().to_string();
                                let start = report_start.read().trim().to_string();
                                let end = report_end.read().trim().to_string();
                                let brk = report_break.read().trim().to_string();
                                let hrs = calculate_hours(&start, &end, &brk);
                                let t_id = selected_team_id.read().clone();
                                let clean_team_id = if t_id.starts_with("client-") { None } else { Some(t_id) };
 
                                let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let user_id = active_user.id.clone();
                                let date_val = report_date.read().clone();
                                spawn(async move {
                                    let _ = yntra_core::add_time_report(
                                        user_id.clone(),
                                        workspace_id,
                                        user_id,
                                        clean_team_id,
                                        date_val,
                                        hrs,
                                        note,
                                        Some(start),
                                        Some(end),
                                    ).await;
                                });
                                let current_trig = *db_trigger.read();
                                db_trigger.set(current_trig + 1);
                                show_report_time_modal.set(false);
                                report_note.set(String::new());
                            },
                            "{t(\"timereports-report-time-btn\", &locale)}"
                        }
                    }
                }
            }
        }
    }
}

