use super::utils::*;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::Team;
use yntra_core::TeamEvent;
use yntra_core::WorkspaceUser;

#[derive(Props, Clone)]
pub struct SchedulingSidebarProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub teams: Vec<Team>,
    pub filtered_events: Vec<TeamEvent>,
    pub upcoming_events: Vec<TeamEvent>,
    pub selected_calendar_date: Signal<String>,
    pub calendar_year: Signal<i32>,
    pub calendar_month: Signal<u32>,
    pub filter_team_id: Signal<String>,
    pub filter_assignee_id: Signal<String>,
    pub show_add_event_modal: Signal<bool>,
    pub show_time_off_modal: Signal<bool>,
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub leave_save_status: Signal<String>,
    pub locale: String,
    pub is_admin: bool,
}

impl PartialEq for SchedulingSidebarProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SchedulingSidebar(props: SchedulingSidebarProps) -> Element {
    let active_user = props.active_user.clone();
    let users = props.users.clone();
    let teams = props.teams.clone();
    let filtered_events = props.filtered_events.clone();
    let upcoming_events = props.upcoming_events.clone();

    let mut selected_calendar_date = props.selected_calendar_date;
    let mut calendar_year = props.calendar_year;
    let mut calendar_month = props.calendar_month;
    let mut filter_team_id = props.filter_team_id;
    let mut filter_assignee_id = props.filter_assignee_id;
    let mut show_add_event_modal = props.show_add_event_modal;
    let mut show_time_off_modal = props.show_time_off_modal;
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let mut leave_save_status = props.leave_save_status;

    // MiniCalendar cells calculations
    let mini_cells: Vec<CalendarCell> = {
        let curr_year = *calendar_year.read();
        let curr_month = *calendar_month.read();
        let selected_date_str = selected_calendar_date.read().clone();

        let prev_year = if curr_month == 1 {
            curr_year - 1
        } else {
            curr_year
        };
        let prev_month = if curr_month == 1 { 12 } else { curr_month - 1 };

        let next_year = if curr_month == 12 {
            curr_year + 1
        } else {
            curr_year
        };
        let next_month = if curr_month == 12 { 1 } else { curr_month + 1 };

        let days_in_prev = get_days_in_month(prev_year, prev_month);
        let days_in_curr = get_days_in_month(curr_year, curr_month);
        let first_day_wd = get_first_day_of_week(curr_year, curr_month);

        let leading_days = if first_day_wd == 0 {
            6
        } else {
            first_day_wd - 1
        };

        let mut list = Vec::new();

        for i in (days_in_prev - leading_days + 1)..=days_in_prev {
            let date_str = format!("{:04}-{:02}-{:02}", prev_year, prev_month, i);
            let is_today = date_str == "2026-06-30";
            let is_selected = date_str == selected_date_str;
            list.push(CalendarCell {
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
            list.push(CalendarCell {
                year: curr_year,
                month: curr_month,
                day: i,
                is_current: true,
                is_today,
                is_selected,
            });
        }
        let remaining = 42 - list.len();
        for i in 1..=remaining {
            let date_str = format!("{:04}-{:02}-{:02}", next_year, next_month, i);
            let is_today = date_str == "2026-06-30";
            let is_selected = date_str == selected_date_str;
            list.push(CalendarCell {
                year: next_year,
                month: next_month,
                day: i as u32,
                is_current: false,
                is_today,
                is_selected,
            });
        }
        list
    };

    rsx! {
        div {
            class: "scrollbar-dark w-72 shrink-0 overflow-y-auto border-r border-border bg-sidebar p-4 flex flex-col gap-6",
            style: "box-sizing: border-box;",

            // MiniCalendar Widget Card
            div { class: "rounded-lg bg-sidebar p-1 flex flex-col gap-3",

                // Month navigation header
                div { class: "flex items-center justify-between mb-2",
                    button {
                        class: "rounded p-1 transition-colors hover:bg-muted border-0 bg-transparent cursor-pointer flex items-center justify-center",
                        onclick: move |_| {
                            let m = *calendar_month.read();
                            if m == 1 {
                                calendar_month.set(12);
                                let prev = *calendar_year.read();
                                calendar_year.set(prev - 1);
                            } else {
                                calendar_month.set(m - 1);
                            }
                        },
                        components::LucideIcon { name: "chevron-left", class: "h-4 w-4 text-muted-foreground" }
                    }
                    span { class: "text-sm font-medium text-foreground",
                        "{get_month_name(*calendar_month.read())} {calendar_year}"
                    }
                    button {
                        class: "rounded p-1 transition-colors hover:bg-muted border-0 bg-transparent cursor-pointer flex items-center justify-center",
                        onclick: move |_| {
                            let m = *calendar_month.read();
                            if m == 12 {
                                calendar_month.set(1);
                                let next = *calendar_year.read();
                                calendar_year.set(next + 1);
                            } else {
                                calendar_month.set(m + 1);
                            }
                        },
                        components::LucideIcon { name: "chevron-right", class: "h-4 w-4 text-muted-foreground" }
                    }
                }

                // Weekdays header
                div { class: "grid grid-cols-7 gap-1 text-center mb-1",
                    for label in ["M", "T", "W", "T", "F", "S", "S"].iter() {
                        div { class: "py-1 text-xs font-medium text-muted-foreground/60 select-none", "{label}" }
                    }
                }

                // Mini cells grid
                div { class: "grid grid-cols-7 gap-1 text-center",
                    for cell in mini_cells.iter() {
                        {
                            let cell_date = format!("{:04}-{:02}-{:02}", cell.year, cell.month, cell.day);
                            let cell_date_clone = cell_date.clone();
                            let is_selected = cell.is_selected;
                            let is_today = cell.is_today;
                            let is_current = cell.is_current;
                            let has_cell_events = filtered_events.iter().any(|ev| ev.start_time.starts_with(&cell_date));

                            let cell_btn_class = if is_selected {
                                "bg-primary text-white hover:bg-primary/80 font-bold"
                            } else if is_today {
                                "text-primary ring-1 ring-primary font-bold hover:bg-muted"
                            } else if !is_current {
                                "text-muted-foreground/30 hover:bg-muted"
                            } else {
                                "text-foreground hover:bg-muted"
                            };

                            rsx! {
                                button {
                                    key: "{cell_date}",
                                    onclick: move |_| {
                                        selected_calendar_date.set(cell_date_clone.clone());
                                    },
                                    class: "relative mx-auto flex h-8 w-8 items-center justify-center rounded-full text-xs transition-all duration-150 border-0 bg-transparent cursor-pointer {cell_btn_class}",

                                    span { "{cell.day}" }
                                    if has_cell_events && !is_selected {
                                        span { class: "absolute bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-primary" }
                                    }
                                }
                            }
                        }
                    }
                }

                // "+ New Shift" button at the bottom of the MiniCalendar
                if props.is_admin {
                    button {
                        class: "mt-4 flex w-full items-center justify-center gap-2 rounded-md bg-muted px-4 py-2 text-xs font-medium text-foreground transition-colors hover:bg-muted border-0 cursor-pointer",
                        onclick: move |_| show_add_event_modal.set(true),
                        span { class: "text-primary font-bold text-sm", "+" }
                        "{t(\"scheduler-new-event\", &props.locale)}"
                    }
                }
            }

            // Care Team Filter Dropdown Select
            if teams.len() > 1 {
                div { class: "flex flex-col gap-2 border-t border-border/50 pt-4",
                    h3 { class: "m-0 text-xs font-semibold uppercase tracking-wider text-muted-foreground/60 select-none",
                        "{t(\"scheduler-active-schedule\", &props.locale)}"
                    }
                    crate::components::Select {
                        trigger_class: "h-[38px] w-full rounded-lg border border-border bg-muted px-3 text-sm text-foreground outline-none cursor-pointer",
                        value: "{filter_team_id}",
                        onchange: move |val: String| filter_team_id.set(val),
                        options: {
                            let mut opts = vec![("all".to_string(), t("scheduler-all-teams", &props.locale))];
                            for t in teams.iter() {
                                opts.push((t.id.clone(), t.name.clone()));
                            }
                            opts
                        },
                    }
                }
            }

            // Staff / Assistants Filter Dropdown Select
            div { class: "flex flex-col gap-2 border-t border-border/50 pt-4",
                h3 { class: "m-0 text-xs font-semibold uppercase tracking-wider text-muted-foreground/60 select-none",
                    "{t(\"scheduler-staff-assistants\", &props.locale)}"
                }
                crate::components::Select {
                    trigger_class: "h-[38px] w-full rounded-lg border border-border bg-muted px-3 text-sm text-foreground outline-none cursor-pointer",
                    value: "{filter_assignee_id}",
                    onchange: move |val: String| filter_assignee_id.set(val),
                    options: {
                        let mut opts = vec![("all".to_string(), t("scheduler-all-assistants", &props.locale))];
                        if props.is_admin {
                            for u in users.iter().filter(|u| u.role != "client") {
                                opts.push((u.id.clone(), u.full_name.clone().unwrap_or_else(|| u.email.clone())));
                            }
                        } else {
                            opts.push((active_user.id.clone(), t("scheduler-only-my-shifts", &props.locale)));
                        }
                        opts
                    },
                }
            }

            // Request Time Off Button
            div { class: "border-t border-border/50 pt-4",
                button {
                    class: "w-full flex items-center justify-start gap-2 border border-primary/20 bg-primary/5 hover:bg-primary/10 hover:text-primary transition-all rounded-lg px-4 py-2 text-sm font-medium text-foreground cursor-pointer",
                    onclick: move |_| {
                        leave_save_status.set("idle".to_string());
                        show_time_off_modal.set(true);
                    },
                    components::LucideIcon { name: "calendar", class: "h-4 w-4 text-primary" }
                    "{t(\"reporting-leave_request\", &props.locale)}"
                }
            }

            // Upcoming Shifts List
            div { class: "flex flex-col gap-3 border-t border-border/50 pt-4",
                h3 { class: "m-0 text-xs font-semibold uppercase tracking-wider text-muted-foreground/60 select-none",
                    "{t(\"common-upcoming\", &props.locale)}"
                }
                if upcoming_events.is_empty() {
                    div { class: "rounded-lg border border-dashed border-border p-3 text-center text-xs italic text-muted-foreground/50 bg-muted/5 select-none",
                        "{t(\"scheduler-no-upcoming-shifts\", &props.locale)}"
                    }
                } else {
                    div { class: "flex flex-col gap-2",
                        for item in upcoming_events.iter().map(|ev| {
                            let ev_clone = ev.clone();
                            let parts_start: Vec<&str> = ev.start_time.split(' ').collect();
                            let date_str = parts_start.first().unwrap_or(&"").to_string();
                            let time_str = parts_start.get(1).unwrap_or(&"").to_string();
                            (ev_clone, date_str, time_str)
                        }) {
                            div {
                                key: "{item.0.id}",
                                onclick: {
                                    let ev_c = item.0.clone();
                                    move |_| show_event_detail_modal.set(Some(ev_c.clone()))
                                },
                                class: "cursor-pointer rounded-lg bg-secondary/50 p-3 transition-colors hover:bg-muted border border-border/40 select-none",

                                div { class: "font-medium text-sm text-foreground truncate", "{item.0.title}" }
                                div { class: "text-xs text-muted-foreground mt-1",
                                    "{item.1} · {item.2}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
