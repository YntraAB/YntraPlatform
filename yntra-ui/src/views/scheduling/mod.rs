pub mod month;
pub mod week;
pub mod day;
pub mod agenda;
pub mod unscheduled;
pub mod utils;
pub mod sidebar;
pub mod add_event_modal;
pub mod time_off_modal;
pub mod detail_modal;

pub use month::MonthView;
pub use week::WeekView;
pub use day::DayView;
pub use agenda::AgendaView;
pub use unscheduled::UnscheduledBucket;
pub use utils::*;

use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TeamEvent;
use yntra_core::WorkspaceUser;
use yntra_core::Team;

use sidebar::SchedulingSidebar;
use add_event_modal::AddEventModal;
use time_off_modal::TimeOffModal;
use detail_modal::EventDetailModal;

#[derive(Props, Clone)]
pub struct SchedulingViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub teams: Vec<Team>,
    pub events: Vec<TeamEvent>,
    pub scheduling_sidebar_tab: Signal<String>,
    pub event_title: Signal<String>,
    pub event_team: Signal<String>,
    pub event_assignee: Signal<String>,
    pub event_recipient: Signal<String>,
    pub event_start: Signal<String>,
    pub event_end: Signal<String>,
    pub selected_calendar_date: Signal<String>,
    pub calendar_year: Signal<i32>,
    pub calendar_month: Signal<u32>,
    pub leave_type: Signal<String>,
    pub leave_start: Signal<String>,
    pub leave_end: Signal<String>,
    pub leave_reason: Signal<String>,
    pub leave_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub filter_categories: Signal<Vec<String>>,
    pub locale: String,
}

impl PartialEq for SchedulingViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SchedulingView(props: SchedulingViewProps) -> Element {
    let active_user = props.active_user.clone();
    let users = props.users.clone();
    let teams = props.teams.clone();
    let events = props.events.clone();
    let mut events_sig = use_signal(|| props.events.clone());
    use_effect(move || {
        events_sig.set(props.events.clone());
    });

    let mut edit_mode = use_signal(|| false);
    let mut calendar_view_mode = use_signal(|| "month".to_string());
    let dragged_event_id = use_signal(|| Option::<String>::None);
    let mut zoom_level = use_signal(|| 60.0f64);
    let _event_title = props.event_title;
    let mut event_team = props.event_team;
    let mut event_assignee = props.event_assignee;
    let mut event_recipient = props.event_recipient;
    let mut event_start = props.event_start;
    let mut event_end = props.event_end;
    let mut selected_calendar_date = props.selected_calendar_date;
    let mut calendar_year = props.calendar_year;
    let mut calendar_month = props.calendar_month;
    let leave_type = props.leave_type;
    let leave_start = props.leave_start;
    let leave_end = props.leave_end;
    let leave_reason = props.leave_reason;
    let leave_save_status = props.leave_save_status;
    let db_trigger = props.db_trigger;

    // Local state for sidebar filters and modal triggers
    let dragged_over_cell = use_signal(|| Option::<String>::None);
    let show_add_event_modal = use_signal(|| false);
    let show_time_off_modal = use_signal(|| false);
    let show_event_detail_modal = use_signal(|| Option::<TeamEvent>::None);
    let editing_event = use_signal(|| Option::<TeamEvent>::None);
    let filter_team_id = use_signal(|| "all".to_string());
    let filter_assignee_id = use_signal(|| "all".to_string());
    let filter_categories = props.filter_categories;

    let is_admin = active_user.role == "platform_admin" || active_user.role == "admin";

    let db_trig_val = *db_trigger.read();
    let courses_res = use_resource(move || {
        let _ = db_trig_val;
        async move {
            yntra_core::get_courses("user-1".to_string()).await.unwrap_or_default()
        }
    });
    let courses = courses_res.read().clone().unwrap_or_default();

    let req_id_timetable = active_user.id.clone();
    let slots_res = use_resource(move || {
        let _ = db_trig_val;
        let r_id = req_id_timetable.clone();
        async move {
            yntra_core::get_timetable_slots(r_id).await.unwrap_or_default()
        }
    });
    let mut slots = slots_res.read().clone().unwrap_or_default();
    slots.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    let mut selected_course_id = use_signal(String::new);
    let mut selected_day = use_signal(|| 1); // 1 = Monday
    let mut timetable_start_time = use_signal(|| "08:30".to_string());
    let mut timetable_end_time = use_signal(|| "09:45".to_string());
    let mut classroom_input = use_signal(String::new);
    let mut sync_success = use_signal(|| false);

    use_effect(move || {
        let cs = courses_res.read().clone().unwrap_or_default();
        if selected_course_id.read().is_empty() && !cs.is_empty() {
            selected_course_id.set(cs[0].id.clone());
        }
    });

    // Setup initial select defaults if empty
    let effect_teams = teams.clone();
    let effect_users = users.clone();
    use_effect(move || {
        if event_team.read().is_empty() && !effect_teams.is_empty() {
            event_team.set(effect_teams[0].id.clone());
        }
        if event_assignee.read().is_empty() && !effect_users.is_empty() {
            event_assignee.set(effect_users[0].id.clone());
        }
        let clients: Vec<&WorkspaceUser> = effect_users.iter().filter(|u| u.role == "client").collect();
        if event_recipient.read().is_empty() && !clients.is_empty() {
            event_recipient.set(clients[0].id.clone());
        }
        if event_start.read().is_empty() {
            event_start.set("09:00".to_string());
        }
        if event_end.read().is_empty() {
            event_end.set("17:00".to_string());
        }
    });

    // Filter events locally
    let filtered_events: Vec<TeamEvent> = events
        .iter()
        .filter(|ev| {
            let matches_team = if *filter_team_id.read() == "all" {
                true
            } else {
                ev.team_id.as_deref() == Some(filter_team_id.read().as_str())
            };
            let matches_assignee = if *filter_assignee_id.read() == "all" {
                true
            } else {
                ev.assignee_id.as_deref() == Some(filter_assignee_id.read().as_str())
            };
            let _config = get_event_category_config(&ev.title, &ev.metadata);
            let matches_category = filter_categories.read().contains(&"schedule".to_string());
            
            matches_team && matches_assignee && matches_category
        })
        .cloned()
        .collect();

    // Partition events into scheduled and unscheduled buckets
    let mut scheduled_events = Vec::new();
    let mut unscheduled_events = Vec::new();
    for ev in filtered_events.iter() {
        if ev.start_time.is_empty() || ev.start_time.starts_with("unscheduled") {
            unscheduled_events.push(ev.clone());
        } else {
            scheduled_events.push(ev.clone());
        }
    }

    // Upcoming events list (next 3 shifts on or after today "2026-06-30")
    let upcoming_events: Vec<TeamEvent> = {
        let mut list: Vec<TeamEvent> = scheduled_events
            .iter()
            .filter(|ev| ev.start_time.as_str() >= "2026-06-30 00:00")
            .cloned()
            .collect();
        list.sort_by_key(|e| e.start_time.clone());
        list.truncate(3);
        list
    };

    // Calendar grid cells calculations
    let selected_date_str = selected_calendar_date.read().clone();
    let curr_year = *calendar_year.read();
    let curr_month = *calendar_month.read();

    let prev_year = if curr_month == 1 { curr_year - 1 } else { curr_year };
    let prev_month = if curr_month == 1 { 12 } else { curr_month - 1 };

    let next_year = if curr_month == 12 { curr_year + 1 } else { curr_year };
    let next_month = if curr_month == 12 { 1 } else { curr_month + 1 };

    let days_in_prev = get_days_in_month(prev_year, prev_month);
    let days_in_curr = get_days_in_month(curr_year, curr_month);
    let first_day_wd = get_first_day_of_week(curr_year, curr_month);

    let leading_days = if first_day_wd == 0 { 6 } else { first_day_wd - 1 };

    let mut cells = Vec::new();

    for i in (days_in_prev - leading_days + 1)..=days_in_prev {
        let date_str = format!("{:04}-{:02}-{:02}", prev_year, prev_month, i);
        let is_today = date_str == "2026-06-30";
        let is_selected = date_str == selected_date_str;
        cells.push(CalendarCell {
            year: prev_year,
            month: prev_month,
            day: i,
            is_current: false,
            is_today,
            is_selected,
        });
    }
    for i in 1..=days_in_curr {
        let date_str = format!("{:04}-{:02}-{:02}", curr_year, curr_month, i);
        let is_today = date_str == "2026-06-30";
        let is_selected = date_str == selected_date_str;
        cells.push(CalendarCell {
            year: curr_year,
            month: curr_month,
            day: i,
            is_current: true,
            is_today,
            is_selected,
        });
    }
    let remaining = 42 - cells.len();
    for i in 1..=remaining {
        let date_str = format!("{:04}-{:02}-{:02}", next_year, next_month, i);
        let is_today = date_str == "2026-06-30";
        let is_selected = date_str == selected_date_str;
        cells.push(CalendarCell {
            year: next_year,
            month: next_month,
            day: i as u32,
            is_current: false,
            is_today,
            is_selected,
        });
    }

    // Find the week cells containing the selected_date
    let week_cells = if let Some(pos) = cells.iter().position(|c| {
        let cell_date = format!("{:04}-{:02}-{:02}", c.year, c.month, c.day);
        cell_date == selected_date_str
    }) {
        let start_idx = (pos / 7) * 7;
        let end_idx = start_idx + 7;
        cells[start_idx..end_idx].to_vec()
    } else {
        cells[0..7].to_vec()
    };

    // Calculate dates formatted label for toolbar
    let date_range_header = match calendar_view_mode.read().as_str() {
        "day" => {
            // e.g. "Tuesday, June 30, 2026"
            selected_date_str.clone()
        }
        "week" => {
            // e.g. "Jun 29 - Jul 5, 2026"
            if !week_cells.is_empty() {
                let first = &week_cells[0];
                let last = &week_cells[6];
                format!(
                    "{} {} - {} {}, {}",
                    get_month_name(first.month).chars().take(3).collect::<String>(),
                    first.day,
                    get_month_name(last.month).chars().take(3).collect::<String>(),
                    last.day,
                    first.year
                )
            } else {
                selected_date_str.clone()
            }
        }
        "agenda" => {
            t("scheduler-upcoming-events", &props.locale)
        }
        _ => {
            format!("{} {}", get_month_name(curr_month), curr_year)
        }
    };

    rsx! {
        div { class: "flex h-full flex-1 overflow-hidden bg-background",
            
            // Renders sidebar
            if *calendar_view_mode.read() != "timetable" {
                SchedulingSidebar {
                    active_user: active_user.clone(),
                    users: users.clone(),
                    teams: teams.clone(),
                    filtered_events: filtered_events.clone(),
                    upcoming_events,
                    selected_calendar_date,
                    calendar_year,
                    calendar_month,
                    filter_team_id,
                    filter_assignee_id,
                    show_add_event_modal,
                    show_time_off_modal,
                    show_event_detail_modal,
                    leave_save_status,
                    locale: props.locale.clone(),
                    is_admin,
                }
            }

            // Main View Area (with Unscheduled drawer and Calendar pane)
            div { class: "flex-1 min-w-0 flex overflow-hidden",
                
                // Collapsible Unscheduled Bucket Drawer
                if *calendar_view_mode.read() != "timetable" {
                    UnscheduledBucket {
                        unscheduled_events,
                        edit_mode,
                        dragged_event_id,
                        db_trigger,
                        show_event_detail_modal,
                        is_admin,
                        workspace_id: props.active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string()),
                        locale: props.locale.clone(),
                    }
                }

                // Main content: calendar grid and controls
                div { class: "flex flex-col flex-1 min-w-0 bg-background",
                    
                    // Toolbar Header
                    div { class: "flex items-center justify-between border-b border-border px-4 py-3 bg-background/95 backdrop-blur-sm z-30 sticky top-0",
                        // Left side: Navigation and Date range display
                        div { class: "flex items-center gap-4",
                            div { class: "flex items-center gap-1",
                                button {
                                    class: "rounded-md p-2 transition-colors hover:bg-muted border-0 bg-transparent cursor-pointer flex items-center justify-center",
                                    onclick: move |_| {
                                        match calendar_view_mode.read().as_str() {
                                            "month" => {
                                                let m = *calendar_month.read();
                                                if m == 1 {
                                                    calendar_month.set(12);
                                                    let prev = *calendar_year.read();
                                                    calendar_year.set(prev - 1);
                                                } else {
                                                    calendar_month.set(m - 1);
                                                }
                                            }
                                            "week" => {
                                                let date_c = add_days_to_date(&selected_calendar_date.read(), -7);
                                                let (y, m, _) = parse_date(&date_c);
                                                selected_calendar_date.set(date_c);
                                                calendar_year.set(y);
                                                calendar_month.set(m);
                                            }
                                            _ => {
                                                let date_c = add_days_to_date(&selected_calendar_date.read(), -1);
                                                let (y, m, _) = parse_date(&date_c);
                                                selected_calendar_date.set(date_c);
                                                calendar_year.set(y);
                                                calendar_month.set(m);
                                            }
                                        }
                                    },
                                    components::LucideIcon { name: "chevron-left", class: "h-5 w-5 text-muted-foreground" }
                                }
                                button {
                                    class: "rounded-md px-3 py-1.5 text-sm font-medium text-foreground transition-colors hover:bg-muted border border-border bg-background cursor-pointer",
                                    onclick: move |_| {
                                        calendar_year.set(2026);
                                        calendar_month.set(6);
                                        selected_calendar_date.set("2026-06-30".to_string());
                                    },
                                    "{t(\"common-today\", &props.locale)}"
                                }
                                button {
                                    class: "rounded-md p-2 transition-colors hover:bg-muted border-0 bg-transparent cursor-pointer flex items-center justify-center",
                                    onclick: move |_| {
                                        match calendar_view_mode.read().as_str() {
                                            "month" => {
                                                let m = *calendar_month.read();
                                                if m == 12 {
                                                    calendar_month.set(1);
                                                    let next = *calendar_year.read();
                                                    calendar_year.set(next + 1);
                                                } else {
                                                    calendar_month.set(m + 1);
                                                }
                                            }
                                            "week" => {
                                                let date_c = add_days_to_date(&selected_calendar_date.read(), 7);
                                                let (y, m, _) = parse_date(&date_c);
                                                selected_calendar_date.set(date_c);
                                                calendar_year.set(y);
                                                calendar_month.set(m);
                                            }
                                            _ => {
                                                let date_c = add_days_to_date(&selected_calendar_date.read(), 1);
                                                let (y, m, _) = parse_date(&date_c);
                                                selected_calendar_date.set(date_c);
                                                calendar_year.set(y);
                                                calendar_month.set(m);
                                            }
                                        }
                                    },
                                    components::LucideIcon { name: "chevron-right", class: "h-5 w-5 text-muted-foreground" }
                                }
                            }
                            
                            div { class: "flex items-center gap-2 select-none",
                                components::LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                                span { class: "font-bold text-foreground text-base", "{date_range_header}" }
                            }
                        }

                        // Right side: Edit mode toggle, View selector, Zoom controls
                        div { class: "flex items-center gap-4",
                            // Edit Mode Toggle
                            {
                                let edit_icon_class = if *edit_mode.read() {
                                    "h-3.5 w-3.5 animate-pulse text-white"
                                } else {
                                    "h-3.5 w-3.5 text-muted-foreground"
                                };
                                let edit_label = if *edit_mode.read() {
                                    t("scheduler-exit-edit-mode", &props.locale)
                                } else {
                                    t("scheduler-edit-mode", &props.locale)
                                };
                                rsx! {
                                    button {
                                        onclick: move |_| {
                                            let curr = *edit_mode.read();
                                            edit_mode.set(!curr);
                                        },
                                        class: if *edit_mode.read() {
                                            "flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs font-bold transition-all duration-200 cursor-pointer bg-primary text-white shadow-lg shadow-primary/25 scale-105 border-0"
                                        } else {
                                            "flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs font-bold transition-all duration-200 cursor-pointer bg-secondary/80 text-muted-foreground hover:bg-muted hover:text-foreground border border-border/50"
                                        },
                                        components::LucideIcon { name: "edit", class: edit_icon_class }
                                        "{edit_label}"
                                    }
                                }
                            }

                            // View mode selector
                            div { class: "flex items-center rounded-lg bg-secondary/30 p-1 border border-border/50",
                                for item in ["month", "week", "day", "agenda", "timetable"].iter().map(|mode| {
                                    let mode_str = mode.to_string();
                                    let is_active = *calendar_view_mode.read() == mode_str;
                                    let active_class = if is_active {
                                        "bg-background text-foreground shadow-sm font-bold animate-in fade-in"
                                    } else {
                                        "text-muted-foreground hover:bg-muted hover:text-foreground"
                                    };
                                    let label = t(&format!("scheduler-views-{}", mode_str), &props.locale);
                                    (mode_str, active_class, label)
                                }) {
                                    button {
                                        key: "{item.0}",
                                        onclick: {
                                            let m_str = item.0.clone();
                                            move |_| calendar_view_mode.set(m_str.clone())
                                        },
                                        class: "rounded-md px-3 py-1.5 text-xs transition-all duration-150 border-0 bg-transparent cursor-pointer {item.1}",
                                        "{item.2}"
                                    }
                                }
                            }

                            // Zoom Controls
                            if *calendar_view_mode.read() == "day" || *calendar_view_mode.read() == "week" {
                                div { 
                                    class: "flex items-center rounded-lg border border-border/50 bg-secondary/30 px-2 py-1 gap-1.5 select-none",
                                    span { class: "text-muted-foreground/60 font-bold uppercase text-[9px] mr-1", "Zoom: {zoom_level.read():.0}%" }
                                    button {
                                        class: "h-5 w-5 rounded bg-muted hover:bg-muted/80 text-xs font-bold border-0 cursor-pointer flex items-center justify-center",
                                        onclick: move |_| {
                                            let val = *zoom_level.read();
                                            zoom_level.set((val - 10.0).max(40.0));
                                        },
                                        "-"
                                    }
                                    button {
                                        class: "h-5 w-5 rounded bg-muted hover:bg-muted/80 text-xs font-bold border-0 cursor-pointer flex items-center justify-center",
                                        onclick: move |_| {
                                            let val = *zoom_level.read();
                                            zoom_level.set((val + 10.0).min(160.0));
                                        },
                                        "+"
                                    }
                                }
                            }
                        }
                    }

                    // View-specific header (Day / Week column headers)
                    {
                        if *calendar_view_mode.read() == "day" {
                            rsx! {
                                div { class: "flex border-b border-border sticky top-[61px] z-20 bg-background/95 backdrop-blur-sm select-none",
                                    div { class: "w-16 flex-shrink-0 border-r border-border" }
                                    div { class: "flex-1 px-2 py-3 text-center",
                                        div { class: "text-xs uppercase tracking-wider text-primary font-bold",
                                            "{selected_date_str}"
                                        }
                                    }
                                }
                            }
                        } else if *calendar_view_mode.read() == "week" {
                            rsx! {
                                div { class: "flex border-b border-border sticky top-[61px] z-20 bg-background/95 backdrop-blur-sm select-none",
                                    div { class: "w-16 flex-shrink-0 border-r border-border bg-sidebar" }
                                    div { 
                                        class: "grid flex-1 divide-x divide-border",
                                        style: "grid-template-columns: repeat(7, minmax(0, 1fr));",
                                        for cell in week_cells.iter() {
                                            {
                                                let date_str = format!("{:04}-{:02}-{:02}", cell.year, cell.month, cell.day);
                                                let is_today = cell.is_today;
                                                let is_weekend = cell.day % 2 == 0;
                                                let cell_label_class = if is_today { "text-primary font-bold" } else { "text-muted-foreground/80" };
                                                let cell_bg = if is_weekend { "bg-muted/30 dark:bg-[#0F1115]" } else { "" };
                                                
                                                rsx! {
                                                    div {
                                                        key: "{date_str}",
                                                        class: "border-r border-border px-2 py-3 text-center last:border-r-0 {cell_bg}",
                                                        div { class: "text-xs uppercase tracking-wider {cell_label_class}",
                                                            "{get_month_name(cell.month).chars().take(3).collect::<String>()} {cell.day}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }

                    // Main view body
                    div {
                        class: "flex-1 overflow-hidden flex flex-col",
                        onwheel: move |e| {
                            if e.modifiers().ctrl() {
                                e.prevent_default();
                                let delta_y = e.delta().strip_units().y;
                                let change = if delta_y > 0.0 { -10.0f64 } else { 10.0f64 };
                                let current = *zoom_level.read();
                                zoom_level.set((current + change).clamp(40.0f64, 160.0f64));
                            }
                        },
                        {
                            let hour_height = zoom_level;
                            let start_hour = 0u32;
                            let end_hour = 23u32;
                            let time_slots: Vec<u32> = (start_hour..=end_hour).collect();
                            let today_prefix = {
                                let now_str = yntra_core::infra::time::get_current_datetime_str();
                                if now_str.len() >= 10 {
                                    now_str[..10].to_string()
                                } else {
                                    "2026-07-02".to_string()
                                }
                            };
                            let current_time_hm = yntra_core::infra::time::get_current_time_str_hm();
                            let (current_hour, current_minute) = if let Some((h, m)) = parse_time_from_str(&current_time_hm) {
                                (h, m)
                            } else {
                                (12, 0)
                            };

                            match calendar_view_mode.read().as_str() {
                                "timetable" => rsx! {
                                    div { class: "flex-1 flex flex-col gap-6 p-6 overflow-y-auto scrollbar-dark",
                                        div { class: "flex flex-col gap-1 border-b border-border pb-4",
                                            h3 { class: "text-lg font-black text-foreground flex items-center gap-2 m-0",
                                                components::LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                                                "Weekly Timetable Slots"
                                            }
                                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed font-medium",
                                                "Configure recurring weekly lessons and class slots, then sync them to generate calendar events."
                                            }
                                        }
                                        
                                        // Weekly Grid + Add Form Row
                                        div { class: "grid gap-6 lg:grid-cols-4 items-start w-full",
                                            // Left 3 columns: Weekly schedule board
                                            div { class: "lg:col-span-3 flex flex-col gap-4 w-full",
                                                div { class: "grid gap-3 grid-cols-1 md:grid-cols-5 w-full",
                                                    for day_idx in 1..=5 {
                                                        {
                                                            let day_name = match day_idx {
                                                                1 => "Monday",
                                                                2 => "Tuesday",
                                                                3 => "Wednesday",
                                                                4 => "Thursday",
                                                                5 => "Friday",
                                                                _ => "Unknown"
                                                            };
                                                            let day_slots: Vec<yntra_core::TimetableSlot> = slots.iter().filter(|s| s.day_of_week == day_idx).cloned().collect();
                                                            
                                                            rsx! {
                                                                div { class: "border border-border/40 bg-sidebar/20 rounded-xl p-3 flex flex-col gap-2 min-h-[300px] w-full",
                                                                    h4 { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider m-0 text-center border-b border-border/30 pb-1.5", "{day_name}" }
                                                                    
                                                                    if day_slots.is_empty() {
                                                                        div { class: "flex-1 flex flex-col items-center justify-center text-center opacity-30 p-2",
                                                                            components::LucideIcon { name: "calendar", size: "18", class: "mb-1" }
                                                                            span { class: "text-[9px] font-bold", "No slots" }
                                                                        }
                                                                    } else {
                                                                        div { class: "flex flex-col gap-2",
                                                                            for s in day_slots.iter() {
                                                                                div { 
                                                                                    key: "{s.id}",
                                                                                    class: "group relative border border-border/30 p-2 rounded-lg bg-sidebar/40 hover:border-primary/40 hover:bg-sidebar/60 transition-all flex flex-col gap-1",
                                                                                    
                                                                                    // Delete overlay button on hover
                                                                                    button {
                                                                                        class: "absolute top-1 right-1 opacity-0 group-hover:opacity-100 p-0.5 rounded bg-destructive/10 text-destructive hover:bg-destructive/20 transition-all border-0 cursor-pointer flex items-center justify-center",
                                                                                        onclick: {
                                                                                            let s_id = s.id.clone();
                                                                                            let db_trig = db_trigger;
                                                                                            move |_| {
                                                                                                let id_clone = s_id.clone();
                                                                                                let mut db_trig_inner = db_trig;
                                                                                                spawn(async move {
                                                                                                    if yntra_core::delete_timetable_slot("user-1".to_string(), id_clone).await.is_ok() {
                                                                                                        let current = *db_trig_inner.read();
                                                                                                        db_trig_inner.set(current + 1);
                                                                                                    }
                                                                                                });
                                                                                            }
                                                                                        },
                                                                                        components::LucideIcon { name: "x", size: "10" }
                                                                                    }
                                                                                    
                                                                                    span { class: "text-[9px] font-black uppercase text-primary px-1.5 py-0.5 bg-primary/10 rounded self-start tracking-wider",
                                                                                        {
                                                                                            let c = courses.iter().find(|c| c.id == s.course_id);
                                                                                            c.map(|c| c.subject.clone()).unwrap_or_else(|| "Class".to_string())
                                                                                        }
                                                                                    }
                                                                                    span { class: "text-xs font-bold text-foreground leading-tight truncate",
                                                                                        {
                                                                                            let c = courses.iter().find(|c| c.id == s.course_id);
                                                                                            c.map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string())
                                                                                        }
                                                                                    }
                                                                                    div { class: "flex items-center gap-1 text-[9px] text-muted-foreground font-semibold mt-0.5",
                                                                                        components::LucideIcon { name: "clock", size: "9" }
                                                                                        span { "{s.start_time} - {s.end_time}" }
                                                                                    }
                                                                                    if let Some(ref room) = s.classroom {
                                                                                        if !room.is_empty() {
                                                                                            div { class: "flex items-center gap-1 text-[9px] text-muted-foreground font-semibold",
                                                                                                components::LucideIcon { name: "map-pin", size: "9" }
                                                                                                span { "{room}" }
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
                                            }
                                            
                                            // Right column: Add Weekly Slot form
                                            div { class: "border border-border/50 bg-card/30 rounded-xl p-4 flex flex-col gap-4 shadow-sm",
                                                h4 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0 border-b border-border/30 pb-2", "Add Weekly Slot" }
                                                
                                                if courses.is_empty() {
                                                    div { class: "py-4 text-center text-xs text-muted-foreground",
                                                        "No courses found. Add courses in Courses & Grading first."
                                                    }
                                                } else {
                                                    div { class: "flex flex-col gap-3.5",
                                                        div { class: "flex flex-col gap-1.5",
                                                            label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Subject / Course" }
                                                            select {
                                                                class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                                                value: "{selected_course_id}",
                                                                onchange: move |e| selected_course_id.set(e.value()),
                                                                for c in courses.iter() {
                                                                    option { value: "{c.id}", "{c.name}" }
                                                                }
                                                            }
                                                        }
                                                        
                                                        div { class: "flex flex-col gap-1.5",
                                                            label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Day of Week" }
                                                            select {
                                                                class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                                                value: "{selected_day}",
                                                                onchange: move |e| {
                                                                    if let Ok(d) = e.value().parse::<i32>() {
                                                                        selected_day.set(d);
                                                                    }
                                                                },
                                                                option { value: "1", "Monday" }
                                                                option { value: "2", "Tuesday" }
                                                                option { value: "3", "Wednesday" }
                                                                option { value: "4", "Thursday" }
                                                                option { value: "5", "Friday" }
                                                            }
                                                        }
                                                        
                                                        div { class: "grid grid-cols-2 gap-2",
                                                            div { class: "flex flex-col gap-1.5",
                                                                label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Start Time" }
                                                                input {
                                                                    r#type: "text",
                                                                    class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                                                    value: "{timetable_start_time}",
                                                                    oninput: move |e| timetable_start_time.set(e.value()),
                                                                }
                                                            }
                                                            div { class: "flex flex-col gap-1.5",
                                                                label { class: "text-[10px] font-bold text-muted-foreground uppercase", "End Time" }
                                                                input {
                                                                    r#type: "text",
                                                                    class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                                                    value: "{timetable_end_time}",
                                                                    oninput: move |e| timetable_end_time.set(e.value()),
                                                                }
                                                            }
                                                        }
                                                        
                                                        div { class: "flex flex-col gap-1.5",
                                                            label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Classroom" }
                                                            input {
                                                                r#type: "text",
                                                                class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                                                placeholder: "Room 204",
                                                                value: "{classroom_input}",
                                                                oninput: move |e| classroom_input.set(e.value()),
                                                            }
                                                        }
                                                        
                                                        button {
                                                            class: "yntra-btn mt-1 text-xs py-2 w-full",
                                                            onclick: {
                                                                let ws_id = props.active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                                                let db_trig = db_trigger;
                                                                move |_| {
                                                                    let cid = selected_course_id.read().clone();
                                                                    let day = *selected_day.read();
                                                                    let start = timetable_start_time.read().clone();
                                                                    let end = timetable_end_time.read().clone();
                                                                    let room = if classroom_input.read().is_empty() { None } else { Some(classroom_input.read().clone()) };
                                                                    let ws = ws_id.clone();
                                                                    let mut db_trig_inner = db_trig;
                                                                    
                                                                    spawn(async move {
                                                                        if yntra_core::save_timetable_slot(
                                                                            "user-1".to_string(),
                                                                            ws,
                                                                            cid,
                                                                            day,
                                                                            start,
                                                                            end,
                                                                            room
                                                                        ).await.is_ok() {
                                                                            classroom_input.set(String::new());
                                                                            let current = *db_trig_inner.read();
                                                                            db_trig_inner.set(current + 1);
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            "Add Slot"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Sync button
                                        div { class: "flex justify-end pt-2",
                                            button {
                                                class: "yntra-btn text-xs font-bold py-2.5 px-5 flex items-center gap-2 shadow-md",
                                                onclick: {
                                                    let ws_id = props.active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                                    let db_trig = db_trigger;
                                                    move |_| {
                                                        let ws_clone = ws_id.clone();
                                                        let mut db_trig_inner = db_trig;
                                                        spawn(async move {
                                                            if yntra_core::sync_timetable_to_calendar(ws_clone, "user-1".to_string()).await.is_ok() {
                                                                sync_success.set(true);
                                                                let current = *db_trig_inner.read();
                                                                db_trig_inner.set(current + 1);
                                                            }
                                                        });
                                                    }
                                                },
                                                components::LucideIcon { name: "refresh-cw", size: "14" }
                                                if *sync_success.read() { "Synced to Calendar Successfully!" } else { "Sync Slots to Calendar" }
                                            }
                                        }
                                    }
                                },
                                "week" => {
                                    let week_has_today = week_cells.iter().any(|c| c.is_today);
                                    rsx! {
                                        WeekView {
                                            week_cells,
                                            scheduled_events: scheduled_events.clone(),
                                            events_sig,
                                            selected_calendar_date,
                                            dragged_event_id,
                                            show_event_detail_modal,
                                            edit_mode,
                                            db_trigger,
                                            zoom_level: hour_height,
                                            time_slots,
                                            week_has_today,
                                            current_hour,
                                            current_minute,
                                            start_hour,
                                            dragged_over_cell,
                                            locale: props.locale.clone(),
                                        }
                                    }
                                },
                                "day" => {
                                    let is_today = selected_date_str == today_prefix;
                                    rsx! {
                                        DayView {
                                            selected_calendar_date,
                                            scheduled_events: scheduled_events.clone(),
                                            events_sig,
                                            dragged_event_id,
                                            show_event_detail_modal,
                                            edit_mode,
                                            db_trigger,
                                            zoom_level: hour_height,
                                            time_slots,
                                            is_today,
                                            current_hour,
                                            current_minute,
                                            start_hour,
                                            dragged_over_cell,
                                            locale: props.locale.clone(),
                                        }
                                    }
                                },
                                "agenda" => rsx! {
                                    AgendaView {
                                        scheduled_events: scheduled_events.clone(),
                                        calendar_year: *calendar_year.read(),
                                        calendar_month: *calendar_month.read(),
                                        show_event_detail_modal,
                                        locale: props.locale.clone(),
                                    }
                                },
                                _ => rsx! {
                                    MonthView {
                                        cells,
                                        scheduled_events: scheduled_events.clone(),
                                        events_sig,
                                        selected_calendar_date,
                                        dragged_event_id,
                                        show_event_detail_modal,
                                        edit_mode,
                                        db_trigger,
                                        dragged_over_cell,
                                        locale: props.locale.clone(),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Dialog Modal: AddEventModal (Create Shift)
        AddEventModal {
            show_add_event_modal,
            editing_event,
            selected_calendar_date,
            db_trigger,
            teams: teams.clone(),
            users: users.clone(),
            workspace_id: props.active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string()),
            locale: props.locale.clone(),
        }

        // Dialog Modal: TimeOffRequestModal (Request Leave)
        TimeOffModal {
            show_time_off_modal,
            leave_type,
            leave_start,
            leave_end,
            leave_reason,
            leave_save_status,
            db_trigger,
            active_user: active_user.clone(),
            users: users.clone(),
            locale: props.locale.clone(),
        }

        // Dialog Modal: EventDetailModal
        EventDetailModal {
            show_event_detail_modal,
            editing_event,
            db_trigger,
            teams,
            users,
            locale: props.locale.clone(),
            is_admin,
        }
    }
}

// Helpers for date calculations
fn parse_date(date_str: &str) -> (i32, u32, u32) {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        (
            parts[0].parse().unwrap_or(2026),
            parts[1].parse().unwrap_or(6),
            parts[2].parse().unwrap_or(30),
        )
    } else {
        (2026, 6, 30)
    }
}

fn add_days_to_date(date_str: &str, days: i32) -> String {
    let (mut y, mut m, mut d) = parse_date(date_str);
    let mut total_days = d as i32 + days;
    
    if days > 0 {
        while total_days > get_days_in_month(y, m) as i32 {
            total_days -= get_days_in_month(y, m) as i32;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        d = total_days as u32;
    } else {
        while total_days <= 0 {
            m -= 1;
            if m == 0 {
                m = 12;
                y -= 1;
            }
            total_days += get_days_in_month(y, m) as i32;
        }
        d = total_days as u32;
    }
    
    format!("{:04}-{:02}-{:02}", y, m, d)
}

