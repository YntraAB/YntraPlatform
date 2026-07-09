use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::DailyNote;
use yntra_core::TeamEvent;
use yntra_core::TimeReport;
use yntra_core::WorkspaceUser;
use yntra_core::Team;
use yntra_core::ClientProfile;
use yntra_core::Workspace;

mod time_report_modal;

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
    pub workspace: Workspace,
}

impl PartialEq for DashboardViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
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
        "sv" => {
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
        "no" => {
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
        "da" => {
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

    if locale == "sv" || locale == "no" || locale == "da" {
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
    let time_reports = props.time_reports;
    let notes = props.notes;
    let db_trigger = props.db_trigger;
    let teams = props.teams.clone();
    let clients = props.clients.clone();
    let mut active_section = props.active_section;
    let auth_region = props.auth_region;
    let workspace = props.workspace.clone();

    let mut show_report_time_modal = use_signal(|| false);
    let mut show_customize_modal = use_signal(|| false);
    let mut todo_input = use_signal(|| String::new());

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

    // Resources for dynamic widget queries
    let user_id_todos = active_user.id.clone();
    let ws_id_todos = workspace.id.clone();
    let todos = use_resource(move || {
        let _trig = db_trigger.read();
        let user_id = user_id_todos.clone();
        let ws_id = ws_id_todos.clone();
        async move {
            yntra_core::get_todos(user_id, ws_id).await.unwrap_or_default()
        }
    });

    let user_id_jobs = active_user.id.clone();
    let job_tickets = use_resource(move || {
        let _trig = db_trigger.read();
        let user_id = user_id_jobs.clone();
        async move {
            yntra_core::get_job_tickets(user_id).await.unwrap_or_default()
        }
    });

    let user_id_courses = active_user.id.clone();
    let courses = use_resource(move || {
        let _trig = db_trigger.read();
        let user_id = user_id_courses.clone();
        async move {
            yntra_core::get_courses(user_id).await.unwrap_or_default()
        }
    });

    let user_id_timetable = active_user.id.clone();
    let timetable = use_resource(move || {
        let _trig = db_trigger.read();
        let user_id = user_id_timetable.clone();
        async move {
            yntra_core::get_timetable_slots(user_id).await.unwrap_or_default()
        }
    });

    let user_id_library = active_user.id.clone();
    let library_books = use_resource(move || {
        let _trig = db_trigger.read();
        let user_id = user_id_library.clone();
        async move {
            yntra_core::get_library_books(user_id).await.unwrap_or_default()
        }
    });

    // Parse active modules from workspace
    let modules_active: serde_json::Value =
        serde_json::from_str(&workspace.modules_active).unwrap_or_default();

    let is_module_active = |block_id: &str| -> bool {
        modules_active.get(block_id).and_then(|v| v.as_bool()).unwrap_or(false)
    };

    struct WidgetMeta {
        id: &'static str,
        block_id: &'static str,
    }

    let all_widgets = vec![
        WidgetMeta { id: "scheduling", block_id: "scheduling" },
        WidgetMeta { id: "messaging", block_id: "messaging" },
        WidgetMeta { id: "jobs", block_id: "jobs" },
        WidgetMeta { id: "todos", block_id: "todos" },
        WidgetMeta { id: "time", block_id: "time" },
        WidgetMeta { id: "assistance", block_id: "assistance" },
        WidgetMeta { id: "academics", block_id: "academics" },
        WidgetMeta { id: "library", block_id: "library" },
        WidgetMeta { id: "finance", block_id: "finance" },
    ];

    let allowed_widgets: Vec<WidgetMeta> = all_widgets
        .into_iter()
        .filter(|w| is_module_active(w.block_id))
        .collect();

    // Parse user preferences for active widgets
    let user_prefs: serde_json::Value =
        serde_json::from_str(&active_user.preferences).unwrap_or_default();

    let selected_widgets: Vec<String> = if let Some(arr) = user_prefs.get("dashboard_widgets").and_then(|v| v.as_array()) {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    } else {
        allowed_widgets.iter().map(|w| w.id.to_string()).collect()
    };

    // Widget custom translation helper
    let get_widget_label = |id: &str, locale: &str| -> String {
        let key = match id {
            "scheduling" => "dashboard-upcoming-events",
            "messaging" => "dashboard-communications",
            "jobs" => "section-jobs",
            "todos" => "section-todos",
            "time" => "section-time",
            "assistance" => "section-assistance",
            "academics" => "section-school-academics",
            "library" => "section-school-library",
            "finance" => "section-school-finance",
            _ => "",
        };
        let val = t(key, locale);
        if val == key || val.is_empty() {
            match id {
                "scheduling" => match locale { "sv" => "Kommande händelser", "no" => "Kommende hendelser", "da" => "Kommende begivenheder", _ => "Upcoming Events" }.to_string(),
                "messaging" => match locale { "sv" => "Kommunikation", "no" => "Kommunikasjon", "da" => "Kommunikation", _ => "Communications" }.to_string(),
                "jobs" => match locale { "sv" => "Arbetspass & Uppdrag", "no" => "Arbeidspass & Oppdrag", "da" => "Arbejdspas & Opgaver", _ => "Job Tickets" }.to_string(),
                "todos" => match locale { "sv" => "Uppgifter & Att göra", "no" => "Oppgaver & Gjøremål", "da" => "Opgaver & To-do", _ => "Todos" }.to_string(),
                "time" => match locale { "sv" => "Tidrapportering", "no" => "Tidsrapportering", "da" => "Tidsrapportering", _ => "Time Sheets" }.to_string(),
                "assistance" => match locale { "sv" => "Omsorg & Assistans", "no" => "Omsorg & Assistanse", "da" => "Omsorg & Assistance", _ => "Care & Assistance" }.to_string(),
                "academics" => match locale { "sv" => "Skola & Kurser", "no" => "Skole & Kurs", "da" => "Skole & Kurser", _ => "School Academics" }.to_string(),
                "library" => match locale { "sv" => "Skolbibliotek", "no" => "Skolebibliotek", "da" => "Skolebibliotek", _ => "School Library" }.to_string(),
                "finance" => match locale { "sv" => "Skolfakturering", "no" => "Skolefakturering", "da" => "Skolefakturering", _ => "School Billing" }.to_string(),
                _ => id.to_string(),
            }
        } else {
            val
        }
    };

    let t_customize = {
        let key = "dashboard-customize-btn";
        let val = t(key, &locale);
        if val == key {
            match locale.as_str() {
                "sv" => "Anpassa översikt".to_string(),
                "no" => "Tilpass oversikt".to_string(),
                "da" => "Tilpas oversigt".to_string(),
                _ => "Customize Dashboard".to_string(),
            }
        } else {
            val
        }
    };

    let t_welcome = t("dashboard-welcome", &locale);
    let t_report_time_btn = t("timereports-report-time-btn", &locale);
    let user_name = active_user.full_name.clone().unwrap_or_else(|| active_user.email.clone());

    struct RenderedEvent {
        id: String,
        title: String,
        time_range: String,
    }

    // Pre-calculate mapped data for the declarative layout
    let rendered_today_events = {
        let mut list: Vec<TeamEvent> = events
            .iter()
            .filter(|e| e.start_time.starts_with(&today_prefix))
            .cloned()
            .collect();
        list.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        list.truncate(3);
        
        let mapped: Vec<RenderedEvent> = list.iter().map(|event| {
            let start_time_part = if event.start_time.len() >= 16 { &event.start_time[11..16] } else { &event.start_time };
            let end_time_part = if event.end_time.len() >= 16 { &event.end_time[11..16] } else { &event.end_time };
            RenderedEvent {
                id: event.id.clone(),
                title: event.title.clone(),
                time_range: format!("{} - {}", start_time_part, end_time_part),
            }
        }).collect();
        mapped
    };

    let pending_tickets = {
        let tickets = job_tickets.read().clone().unwrap_or_default();
        let mut list: Vec<yntra_core::JobTicket> = tickets
            .iter()
            .filter(|t| t.status == "pending" || t.status == "assigned")
            .cloned()
            .collect();
        list.truncate(3);
        list
    };

    let active_todos = {
        let todo_list = todos.read().clone().unwrap_or_default();
        let mut list: Vec<yntra_core::TodoItem> = todo_list
            .iter()
            .filter(|t| !t.completed)
            .cloned()
            .collect();
        list.truncate(3);
        list
    };

    let today_hours: f64 = time_reports
        .iter()
        .filter(|r| r.date == today_prefix)
        .map(|r| r.hours)
        .sum();

    let course_count = courses.read().clone().unwrap_or_default().len();
    let slot_count = timetable.read().clone().unwrap_or_default().len();

    let (total_books, available_copies) = {
        let books = library_books.read().clone().unwrap_or_default();
        let copies: i32 = books.iter().map(|b| b.copies_available).sum();
        (books.len(), copies)
    };

    struct RenderedWidget {
        id: String,
        label: String,
        icon: String,
        checked: bool,
    }

    let rendered_allowed_widgets: Vec<RenderedWidget> = allowed_widgets.iter().map(|w| {
        RenderedWidget {
            id: w.id.to_string(),
            label: get_widget_label(w.id, &locale),
            icon: w.block_id.to_string(),
            checked: selected_widgets.contains(&w.id.to_string()),
        }
    }).collect();

    rsx! {
        div {
            class: "flex flex-col gap-8 p-6 bg-background",
            
            // Header bar
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
                div {
                    class: "flex items-center gap-3 shrink-0",
                    if !is_client && is_module_active("time") {
                        button {
                            class: "shrink-0 gap-2 border-emerald-700 bg-emerald-600 font-bold text-white shadow-md hover:bg-emerald-700 flex items-center px-5 py-2.5 rounded-lg cursor-pointer transition-colors text-sm border-0",
                            onclick: move |_| show_report_time_modal.set(true),
                            components::LucideIcon { name: "time", class: "h-4 w-4", }
                            "{t_report_time_btn}"
                        }
                    }
                    if !is_client {
                        button {
                            class: "shrink-0 gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground flex items-center px-4 py-2.5 rounded-lg cursor-pointer transition-colors text-sm border-0 font-bold shadow-sm",
                            onclick: move |_| show_customize_modal.set(true),
                            components::LucideIcon { name: "settings-2", class: "h-4 w-4", }
                            "{t_customize}"
                        }
                    }
                }
            }

            // Grid widgets layout
            div {
                class: "grid gap-6 md:grid-cols-2 lg:grid-cols-3",
                
                // 1. Scheduling Widget
                if selected_widgets.contains(&"scheduling".to_string()) && is_module_active("scheduling") {
                    div {
                        class: "col-span-1 lg:col-span-2",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "scheduling", class: "h-5 w-5 text-primary", }
                                    "{t(\"dashboard-upcoming-events\", &locale)}"
                                }
                                components::CardDescription {
                                    "{t(\"dashboard-upcoming-events-desc\", &locale)}"
                                }
                            }
                            components::CardContent {
                                class: "flex flex-1 flex-col justify-between",
                                div {
                                    class: "space-y-4",
                                    if rendered_today_events.is_empty() {
                                        p {
                                            class: "text-sm text-muted-foreground m-0 py-4",
                                            "{t(\"dashboard-no-events\", &locale)}"
                                        }
                                    } else {
                                        for event in rendered_today_events.iter() {
                                            div {
                                                key: "{event.id}",
                                                class: "flex flex-col gap-1 border border-border rounded-lg p-3 bg-white/[0.015] hover:bg-white/[0.03] transition-colors cursor-default",
                                                span {
                                                    class: "font-semibold text-sm text-foreground",
                                                    "{event.title}"
                                                }
                                                span {
                                                    class: "text-xs text-muted-foreground font-medium",
                                                    "{event.time_range}"
                                                }
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| active_section.set("scheduling".to_string()),
                                    span { "{t(\"dashboard-go-to-schedule\", &locale)}" }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 2. Communications Widget
                if selected_widgets.contains(&"messaging".to_string()) && (is_module_active("messaging") || is_module_active("notes")) {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "messaging", class: "h-5 w-5 text-primary", }
                                    "{t(\"dashboard-communications\", &locale)}"
                                }
                                components::CardDescription {
                                    "{t(\"dashboard-communications-desc\", &locale)}"
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
                                            "{t(\"messages-inbox\", &locale)}"
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
                                            "{t(\"section-notes\", &locale)}"
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
                                    if is_module_active("messaging") {
                                        button {
                                            class: "flex-1 flex justify-center text-sm py-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold rounded-lg cursor-pointer transition-colors shadow-sm",
                                            onclick: move |_| active_section.set("messaging".to_string()),
                                            "{t(\"messages-inbox\", &locale)}"
                                        }
                                    }
                                    if is_module_active("notes") {
                                        button {
                                            class: "flex-1 flex justify-center text-sm py-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold rounded-lg cursor-pointer transition-colors shadow-sm",
                                            onclick: move |_| active_section.set("notes".to_string()),
                                            "{t(\"section-notes\", &locale)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Job Tickets Widget
                if selected_widgets.contains(&"jobs".to_string()) && is_module_active("jobs") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "jobs", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"jobs\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Dina tilldelade uppdrag och arbetsordrar.",
                                        "no" => "Dine tildelte oppdrag og arbeidsordrer.",
                                        "da" => "Dine tildelte opgaver og arbejdsordrer.",
                                        _ => "Your assigned jobs and service tickets."
                                    }
                                }
                            }
                            components::CardContent {
                                class: "flex flex-1 flex-col justify-between",
                                div {
                                    class: "space-y-3",
                                    if pending_tickets.is_empty() {
                                        p { class: "text-sm text-muted-foreground m-0 py-4", 
                                            match locale.as_str() {
                                                "sv" => "Inga pågående uppdrag.",
                                                "no" => "Ingen pågående oppdrag.",
                                                "da" => "Ingen igangværende opgaver.",
                                                _ => "No active job tickets."
                                            }
                                        }
                                    } else {
                                        for ticket in pending_tickets.iter() {
                                            div {
                                                key: "{ticket.id}",
                                                class: "flex flex-col gap-0.5 border border-border/50 rounded-lg p-2.5 bg-white/[0.015]",
                                                span { class: "font-semibold text-xs text-foreground line-clamp-1", "{ticket.title}" }
                                                div { class: "flex items-center justify-between text-[10px] text-muted-foreground mt-1",
                                                    span { "{ticket.scheduled_date}" }
                                                    span { class: "uppercase font-bold text-primary", "{ticket.priority}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| active_section.set("jobs".to_string()),
                                    span { 
                                        match locale.as_str() {
                                            "sv" => "Visa alla uppdrag",
                                            "no" => "Vis alle oppdrag",
                                            "da" => "Vis alle opgaver",
                                            _ => "View all jobs"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 4. Quick Todos Widget
                if selected_widgets.contains(&"todos".to_string()) && is_module_active("todos") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "todos", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"todos\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Dina personliga uppgifter och kom-ihåg-lista.",
                                        "no" => "Dine personlige oppgaver og huskeliste.",
                                        "da" => "Dine personlige opgaver og huskeliste.",
                                        _ => "Your personal checklist and todo items."
                                    }
                                }
                            }
                            components::CardContent {
                                class: "flex flex-1 flex-col justify-between",
                                div {
                                    class: "space-y-3",
                                    if active_todos.is_empty() {
                                        p { class: "text-sm text-muted-foreground m-0 py-2",
                                            match locale.as_str() {
                                                "sv" => "Inga kvarstående uppgifter! 🎉",
                                                "no" => "Ingen gjenværende oppgaver! 🎉",
                                                "da" => "Ingen udestående opgaver! 🎉",
                                                _ => "All tasks completed! 🎉"
                                            }
                                        }
                                    } else {
                                        for item in active_todos.iter() {
                                            div {
                                                key: "{item.id}",
                                                class: "flex items-center gap-2 border border-border/40 rounded-lg p-2 bg-white/[0.015]",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: false,
                                                    class: "h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary cursor-pointer",
                                                    onchange: {
                                                        let active_user_id = active_user.id.clone();
                                                        let item_id = item.id.clone();
                                                        move |_| {
                                                            let active_user_id = active_user_id.clone();
                                                            let item_id = item_id.clone();
                                                            spawn(async move {
                                                                let _ = yntra_core::toggle_todo(active_user_id, item_id).await;
                                                            });
                                                        }
                                                    }
                                                }
                                                span { class: "text-xs font-semibold text-foreground line-clamp-1", "{item.text}" }
                                            }
                                        }
                                    }
                                    
                                    // Add Todo input field
                                    div { class: "flex items-center gap-2 mt-2 pt-2 border-t border-border/40",
                                        input {
                                            r#type: "text",
                                            placeholder: match locale.as_str() {
                                                "sv" => "Skriv ny uppgift...",
                                                "no" => "Skriv ny oppgave...",
                                                "da" => "Skriv ny opgave...",
                                                _ => "Write new todo..."
                                            },
                                            class: "flex-1 px-3 py-1.5 text-xs rounded-lg border border-border bg-white/[0.02] text-foreground outline-none focus:border-primary",
                                            value: "{todo_input}",
                                            oninput: move |e| todo_input.set(e.value()),
                                            onkeydown: {
                                                let mut todo_input = todo_input.clone();
                                                let active_user_id = active_user.id.clone();
                                                let ws_id = workspace.id.clone();
                                                move |e: KeyboardEvent| {
                                                    if e.key() == Key::Enter && !todo_input.read().trim().is_empty() {
                                                        let text = todo_input.read().clone();
                                                        todo_input.set(String::new());
                                                        let active_user_id = active_user_id.clone();
                                                        let ws_id = ws_id.clone();
                                                        spawn(async move {
                                                            let _ = yntra_core::add_todo(active_user_id, ws_id, text).await;
                                                        });
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

                // 5. Time Sheet Widget
                if selected_widgets.contains(&"time".to_string()) && is_module_active("time") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "time", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"time\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Registrerad arbetstid för idag.",
                                        "no" => "Registrert arbeidstid for i dag.",
                                        "da" => "Registreret arbejdstid for i dag.",
                                        _ => "Logged working hours for today."
                                    }
                                }
                            }
                            components::CardContent {
                                class: "flex flex-1 flex-col justify-between",
                                div {
                                    class: "flex flex-col items-center justify-center py-6 gap-2",
                                    span { class: "text-4xl font-extrabold text-foreground", "{today_hours}" }
                                    span { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                        match locale.as_str() {
                                            "sv" => "timmar rapporterade",
                                            "no" => "timer rapportert",
                                            "da" => "timer rapporteret",
                                            _ => "hours reported"
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| active_section.set("time".to_string()),
                                    span {
                                        match locale.as_str() {
                                            "sv" => "Visa tidrapport",
                                            "no" => "Vis tidsrapport",
                                            "da" => "Vis tidsrapport",
                                            _ => "View timesheet"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 6. Assistance Patient Logs Widget
                if selected_widgets.contains(&"assistance".to_string()) && is_module_active("assistance") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "assistance", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"assistance\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Snabböversikt över vårdtagare och omsorgsloggar.",
                                        "no" => "Hurtigoversikt over brukere og omsorgslogger.",
                                        "da" => "Hurtigt overblik over borgere og omsorgslogger.",
                                        _ => "Quick overview of clients and care logs."
                                    }
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
                                            match locale.as_str() {
                                                "sv" => "Antal vårdtagare",
                                                "no" => "Antall brukere",
                                                "da" => "Antal borgere",
                                                _ => "Active clients"
                                            }
                                        }
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{clients.len()}"
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| active_section.set("assistance".to_string()),
                                    span {
                                        match locale.as_str() {
                                            "sv" => "Öppna omsorgsportal",
                                            "no" => "Åpne omsorgsportal",
                                            "da" => "Åbn omsorgsportal",
                                            _ => "Open care portal"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 7. School Academics Widget
                if selected_widgets.contains(&"academics".to_string()) && is_module_active("academics") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "academics", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"academics\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Översikt av kurser och ditt schema.",
                                        "no" => "Oversikt over kurs og din timeplan.",
                                        "da" => "Oversigt over kurser og dit skema.",
                                        _ => "Overview of academic courses and timetable."
                                    }
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
                                            match locale.as_str() {
                                                "sv" => "Mina kurser",
                                                "no" => "Mine kurs",
                                                "da" => "Mine kurser",
                                                _ => "My courses"
                                            }
                                        }
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{course_count}"
                                        }
                                    }
                                    div {
                                        class: "flex justify-between items-center",
                                        span {
                                            class: "text-sm font-medium",
                                            match locale.as_str() {
                                                "sv" => "Schematillfällen",
                                                "no" => "Timeplantimer",
                                                "da" => "Skematimer",
                                                _ => "Timetable slots"
                                            }
                                        }
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{slot_count}"
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| active_section.set("school".to_string()),
                                    span {
                                        match locale.as_str() {
                                            "sv" => "Öppna skolportal",
                                            "no" => "Åpne skoleportal",
                                            "da" => "Åbn skoleportal",
                                            _ => "Open school portal"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 8. School Library Widget
                if selected_widgets.contains(&"library".to_string()) && is_module_active("library") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "library", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"library\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Status för skolbiblioteket.",
                                        "no" => "Status for skolebiblioteket.",
                                        "da" => "Status for skolebiblioteket.",
                                        _ => "Catalog status of school library."
                                    }
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
                                            match locale.as_str() {
                                                "sv" => "Böcker i katalog",
                                                "no" => "Bøker i katalog",
                                                "da" => "Bøger i katalog",
                                                _ => "Catalog books"
                                            }
                                        }
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{total_books}"
                                        }
                                    }
                                    div {
                                        class: "flex justify-between items-center",
                                        span {
                                            class: "text-sm font-medium",
                                            match locale.as_str() {
                                                "sv" => "Tillgängliga exemplar",
                                                "no" => "Tilgjengelige eksemplarer",
                                                "da" => "Tilgængelige eksemplarer",
                                                _ => "Available copies"
                                            }
                                        }
                                        span {
                                            class: "flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-bold text-primary-foreground",
                                            "{available_copies}"
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| {
                                        active_section.set("school".to_string());
                                    },
                                    span {
                                        match locale.as_str() {
                                            "sv" => "Hantera bibliotek",
                                            "no" => "Administrer bibliotek",
                                            "da" => "Administrer bibliotek",
                                            _ => "Manage library"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }

                // 9. School Finance Widget
                if selected_widgets.contains(&"finance".to_string()) && is_module_active("finance") {
                    div {
                        class: "col-span-1",
                        components::Card {
                            class: "flex flex-col h-full",
                            components::CardHeader {
                                class: "pb-3",
                                components::CardTitle {
                                    class: "flex items-center gap-2 text-lg",
                                    components::LucideIcon { name: "finance", class: "h-5 w-5 text-primary", }
                                    "{get_widget_label(\"finance\", &locale)}"
                                }
                                components::CardDescription {
                                    match locale.as_str() {
                                        "sv" => "Skolavgifter, fakturering och ekonomisk status.",
                                        "no" => "Skolepenger, fakturering og økonomisk status.",
                                        "da" => "Skolepenge, fakturering og økonomisk status.",
                                        _ => "Tuition invoices, billing, and account balances."
                                    }
                                }
                            }
                            components::CardContent {
                                class: "flex flex-1 flex-col justify-between",
                                div {
                                    class: "flex flex-col gap-4 py-2",
                                    p { class: "text-xs text-muted-foreground m-0 leading-relaxed",
                                        match locale.as_str() {
                                            "sv" => "Se fakturor, registrera inbetalningar och ställ ut nya fordringar.",
                                            "no" => "Se fakturaer, registrer innbetalinger og utsted nye krav.",
                                            "da" => "Se fakturaer, registrer indbetalinger og udsted nye krav.",
                                            _ => "Review school billing invoices, track incoming tuition, and dispatch claims."
                                        }
                                    }
                                }
                                button {
                                    class: "mt-4 w-full flex justify-center items-center gap-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground border-0 font-semibold py-2.5 px-4 rounded-lg cursor-pointer transition-colors text-sm shadow-sm",
                                    onclick: move |_| {
                                        active_section.set("school".to_string());
                                    },
                                    span {
                                        match locale.as_str() {
                                            "sv" => "Hantera ekonomi",
                                            "no" => "Administrer økonomi",
                                            "da" => "Administrer økonomi",
                                            _ => "Manage billing"
                                        }
                                    }
                                    components::LucideIcon { name: "arrow-right", class: "h-4 w-4", }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Time Report Modal
        time_report_modal::TimeReportModal {
            show_report_time_modal: show_report_time_modal,
            active_user: active_user.clone(),
            teams: teams,
            clients: clients,
            db_trigger: db_trigger,
            locale: locale.clone(),
        }

        // Custom Overview Dashboard Selection Modal
        if *show_customize_modal.read() {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4",
                components::Card {
                    class: "w-full max-w-md shadow-2xl border-border bg-card animate-in fade-in zoom-in-95 duration-200",
                    components::CardHeader {
                        components::CardTitle { class: "text-lg flex items-center gap-2",
                            components::LucideIcon { name: "settings-2", class: "h-5 w-5 text-primary" }
                            "{t_customize}"
                        }
                        components::CardDescription {
                            match locale.as_str() {
                                "sv" => "Välj vilka paneler som ska visas på din översiktssida.",
                                "no" => "Velg hvilke paneler som skal vises på oversiktssiden din.",
                                "da" => "Vælg hvilke paneler der skal vises på din oversigtsside.",
                                _ => "Choose which widgets you want to show on your overview dashboard."
                            }
                        }
                    }
                    components::CardContent {
                        class: "space-y-4 max-h-[60vh] overflow-y-auto py-2",
                        for w in rendered_allowed_widgets.iter() {
                            div {
                                key: "{w.id}",
                                class: "flex items-center justify-between p-3 rounded-lg border border-border/50 bg-white/[0.015] hover:bg-white/[0.03] transition-colors",
                                div { class: "flex items-center gap-3",
                                    div { class: "rounded-lg p-2 bg-primary/10 text-primary",
                                        components::LucideIcon { name: &w.icon, class: "h-4 w-4" }
                                    }
                                    span { class: "text-sm font-semibold text-foreground", "{w.label}" }
                                }
                                components::Switch {
                                    checked: w.checked,
                                    onchange: {
                                        let w_id = w.id.clone();
                                        let selected_widgets = selected_widgets.clone();
                                        let active_user = active_user.clone();
                                        move |val| {
                                            let mut next_selected = selected_widgets.clone();
                                            if val {
                                                if !next_selected.contains(&w_id) {
                                                    next_selected.push(w_id.clone());
                                                }
                                            } else {
                                                next_selected.retain(|x| x != &w_id);
                                            }
                                            
                                            let mut new_prefs: serde_json::Value = serde_json::from_str(&active_user.preferences).unwrap_or_default();
                                            new_prefs.as_object_mut().unwrap().insert(
                                                "dashboard_widgets".to_string(),
                                                serde_json::to_value(next_selected).unwrap()
                                            );
                                            
                                            let pref_json = serde_json::to_string(&new_prefs).unwrap_or_default();
                                            let req_id = active_user.id.clone();
                                            let u_id = active_user.id.clone();
                                            let u_name = active_user.full_name.clone();
                                            let u_phone = active_user.phone.clone();
                                            
                                            spawn(async move {
                                                let _ = yntra_core::update_user_profile(
                                                    req_id,
                                                    u_id,
                                                    u_name,
                                                    u_phone,
                                                    pref_json
                                                ).await;
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "p-4 border-t border-border flex justify-end",
                        components::Button {
                            variant: components::ButtonVariant::Secondary,
                            onclick: move |_| show_customize_modal.set(false),
                            match locale.as_str() {
                                "sv" => "Stäng",
                                "no" => "Lukk",
                                "da" => "Luk",
                                _ => "Close"
                            }
                        }
                    }
                }
            }
        }
    }
}
