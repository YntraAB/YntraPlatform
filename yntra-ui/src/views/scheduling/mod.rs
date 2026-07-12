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

use sidebar::SchedulingSidebar;
use add_event_modal::AddEventModal;
use time_off_modal::TimeOffModal;
use detail_modal::EventDetailModal;

#[derive(Props, Clone)]
pub struct SchedulingViewProps {
    pub active_user: WorkspaceUser,
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
    let state = use_context::<crate::state::AppState>();
    let active_user = props.active_user.clone();
    let users = state.users.read().clone().unwrap_or_default();
    let teams = state.teams.read().clone().unwrap_or_default();
    let events = state.events.read().clone().unwrap_or_default();
    let mut events_sig = use_signal(|| events.clone());
    use_effect(move || {
        let evs = state.events.read().clone().unwrap_or_default();
        events_sig.set(evs);
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
                                for item in ["month", "week", "day", "agenda"].iter().map(|mode| {
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


