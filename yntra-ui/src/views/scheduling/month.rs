use super::utils::*;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TeamEvent;

#[derive(Props, Clone)]
pub struct MonthViewProps {
    pub cells: Vec<CalendarCell>,
    pub scheduled_events: Vec<TeamEvent>,
    pub events_sig: Signal<Vec<TeamEvent>>,
    pub selected_calendar_date: Signal<String>,
    pub dragged_event_id: Signal<Option<String>>,
    pub dragged_job_id: Signal<Option<String>>,
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub edit_mode: Signal<bool>,
    pub db_trigger: Signal<u32>,
    pub dragged_over_cell: Signal<Option<String>>,
    pub locale: String,
}

impl PartialEq for MonthViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn MonthView(props: MonthViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut selected_calendar_date = props.selected_calendar_date;
    let mut dragged_event_id = props.dragged_event_id;
    let dragged_job_id = props.dragged_job_id;
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let db_trigger = props.db_trigger;
    let events_sig = props.events_sig;
    let edit_mode = props.edit_mode;
    let mut dragged_over_cell = props.dragged_over_cell;

    rsx! {
        div { class: "scrollbar-dark flex-1 overflow-y-auto p-4 flex flex-col gap-1.5",

            // Day Labels
            div { class: "mb-2 grid grid-cols-7 gap-1",
                for label in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter() {
                    div { class: "py-2 text-center text-xs font-semibold text-muted-foreground/60 select-none",
                        "{label}"
                    }
                }
            }

            // Calendar grid
            div { class: "grid grid-cols-7 gap-1",
                for cell in props.cells.iter() {
                    {
                        let cell_date = format!("{:04}-{:02}-{:02}", cell.year, cell.month, cell.day);
                        let cell_date_clone = cell_date.clone();
                        let is_selected = cell.is_selected;
                        let is_today = cell.is_today;
                        let is_current = cell.is_current;
                        let is_dragged_over = Some(cell_date.clone()) == *dragged_over_cell.read();

                        let cell_bg_class = if is_current {
                            "bg-muted/30 border-border"
                        } else {
                            "bg-background/20 border-border/40 opacity-60"
                        };

                        let cell_selected_class = if is_selected {
                            "ring-2 ring-primary border-primary/50"
                        } else if is_dragged_over {
                            "bg-primary/10 border-2 border-dashed border-primary z-20"
                        } else {
                            "hover:border-primary/50"
                        };

                        let cell_events: Vec<TeamEvent> = props.scheduled_events
                            .iter()
                            .filter(|ev| {
                                if ev.start_time.starts_with(&cell_date) {
                                    true
                                } else { !ev.start_time.contains('-') && cell_date == "2026-06-30" }
                            })
                            .cloned()
                            .collect();

                        let cell_date_for_click = cell_date_clone.clone();
                        let cell_date_for_drop = cell_date_clone.clone();
                        let cell_date_for_enter = cell_date_clone.clone();

                        rsx! {
                            div {
                                key: "{cell_date}",
                                class: "min-h-[108px] cursor-pointer rounded-lg border p-2 transition-all duration-150 flex flex-col gap-1 {cell_bg_class} {cell_selected_class}",
                                onclick: move |_| {
                                    selected_calendar_date.set(cell_date_for_click.clone());
                                },
                                ondragover: move |e| {
                                    e.prevent_default();
                                },
                                ondragenter: move |e| {
                                    e.prevent_default();
                                    dragged_over_cell.set(Some(cell_date_for_enter.clone()));
                                },
                                ondragleave: move |_| {
                                    dragged_over_cell.set(None);
                                },
                                ondrop: {
                                    let cell_date_c = cell_date_for_drop.clone();
                                    let active_uid_sig = state.active_user_id;
                                     move |_| {
                                         dragged_over_cell.set(None);
                                         if let Some(event_id) = dragged_event_id.read().clone() {
                                             if let Some(ev) = events_sig.read().iter().find(|e| e.id == event_id) {
                                                 let new_start = if ev.start_time.starts_with("unscheduled") || ev.start_time.is_empty() {
                                                     format!("{} 09:00", cell_date_c)
                                                 } else {
                                                     update_date_in_time_str(&ev.start_time, &cell_date_c)
                                                 };
                                                 let new_end = if ev.end_time.starts_with("unscheduled") || ev.end_time.is_empty() {
                                                     format!("{} 10:00", cell_date_c)
                                                 } else {
                                                     update_date_in_time_str(&ev.end_time, &cell_date_c)
                                                 };
                                                 let mut db_trig = db_trigger;
                                                 let active_uid_sig_c = active_uid_sig.clone();
                                                 spawn(async move {
                                                     let active_uid = active_uid_sig_c.read().clone();
                                                     if yntra_core::update_event_time(active_uid, event_id, new_start, new_end).await.is_ok() {
                                                         let val = *db_trig.read();
                                                         db_trig.set(val + 1);
                                                     }
                                                 });
                                             }
                                         } else if let Some(job_id) = dragged_job_id.read().clone() {
                                             let mut db_trig = db_trigger;
                                             let active_uid_sig_c = active_uid_sig.clone();
                                             let target_date = cell_date_c.clone();
                                             spawn(async move {
                                                 let active_uid = active_uid_sig_c.read().clone();
                                                 if yntra_core::schedule_job_ticket(active_uid, job_id, target_date, None).await.is_ok() {
                                                     let val = *db_trig.read();
                                                     db_trig.set(val + 1);
                                                 }
                                             });
                                         }
                                     }
                                },

                                // Day Number
                                div {
                                    class: if is_today {
                                        "flex h-7 w-7 items-center justify-center rounded-full bg-primary text-white text-xs font-bold select-none"
                                    } else {
                                        "mb-1 text-xs font-semibold text-foreground/80 pl-1 select-none"
                                    },
                                    "{cell.day}"
                                }

                                // Cell Events
                                div { class: "space-y-1 flex-1 flex flex-col justify-start pointer-events-none",
                                    for item in cell_events.iter().take(3).map(|ev| {
                                        let ev_clone = ev.clone();
                                        let ev_id = ev.id.clone();
                                        let config = get_event_category_config(&ev.title, &ev.metadata);
                                        let metadata_obj = parse_metadata(&ev.metadata);
                                        let start_hm = ev.start_time.split('T').next_back().unwrap_or(&ev.start_time).chars().take(5).collect::<String>();
                                        let end_hm = ev.end_time.split('T').next_back().unwrap_or(&ev.end_time).chars().take(5).collect::<String>();
                                        let is_dragged = Some(ev_id.clone()) == *dragged_event_id.read();
                                        let drag_style = if is_dragged { "opacity: 0.35; transform: scale(0.95); cursor: grabbing;" } else { "" };
                                        (ev_clone, ev_id, config, metadata_obj, start_hm, end_hm, drag_style)
                                    }) {
                                        div {
                                            key: "{item.1}",
                                            class: "group relative block pointer-events-auto select-none",

                                            // Month Card
                                            div {
                                                class: "cursor-pointer truncate rounded px-1.5 py-0.5 text-[10px] transition-all duration-150 hover:brightness-110 block font-semibold border-l-2",
                                                style: "background-color: {item.2.bg_color}; color: {item.2.color}; border-left-color: {item.2.color}; {item.6}",
                                                draggable: *edit_mode.read(),
                                                ondragstart: {
                                                    let ev_id = item.1.clone();
                                                    move |_| {
                                                        dragged_event_id.set(Some(ev_id.clone()));
                                                    }
                                                },
                                                ondragend: move |_| {
                                                    dragged_event_id.set(None);
                                                    dragged_over_cell.set(None);
                                                },
                                                onclick: {
                                                    let ev_c = item.0.clone();
                                                    move |e| {
                                                        e.stop_propagation();
                                                        show_event_detail_modal.set(Some(ev_c.clone()));
                                                    }
                                                },
                                                div { class: "flex justify-between items-center gap-1 w-full",
                                                    span { class: "truncate", "{item.4} {item.0.title}" }
                                                    if *edit_mode.read() {
                                                        button {
                                                            r#type: "button",
                                                            class: "text-muted-foreground hover:text-destructive bg-transparent border-0 cursor-pointer p-0 rounded transition-colors shrink-0",
                                                                 onclick: {
                                                                     let ev_id = item.1.clone();
                                                                     let mut db_trig = db_trigger;
                                                                     let active_uid_sig = state.active_user_id;
                                                                     move |evt| {
                                                                         evt.stop_propagation();
                                                                         let target_ev_id = ev_id.clone();
                                                                         spawn(async move {
                                                                             let active_uid = active_uid_sig.read().clone();
                                                                             let _ = yntra_core::delete_event(active_uid, target_ev_id).await;
                                                                             let val = *db_trig.read();
                                                                             db_trig.set(val + 1);
                                                                         });
                                                                     }
                                                                 },
                                                            components::LucideIcon { name: "trash-2", class: "h-2.5 w-2.5" }
                                                        }
                                                    }
                                                }
                                            }

                                            // Hover Tooltip Popup (Pure CSS group-hover)
                                            div {
                                                class: "pointer-events-none absolute bottom-full left-0 z-[100] mb-2 hidden w-64 rounded-xl border border-border bg-background/95 p-4 shadow-2xl backdrop-blur-md duration-200 animate-in fade-in slide-in-from-bottom-1 group-hover:block",

                                                div { class: "mb-2 flex items-center justify-between text-[10px] text-muted-foreground",
                                                    span {
                                                        class: "rounded px-2 py-0.5 font-bold uppercase tracking-wider",
                                                        style: "background-color: {item.2.bg_color}; color: {item.2.color};",
                                                        "{t(item.2.label_key, &props.locale)}"
                                                    }
                                                    span { class: "flex items-center gap-1 font-semibold",
                                                        components::LucideIcon { name: "calendar", class: "h-3 w-3 text-primary" }
                                                        "{cell_date}"
                                                    }
                                                }
                                                h4 { class: "m-0 mb-1 text-sm font-bold leading-tight text-foreground", "{item.0.title}" }
                                                div { class: "mb-3 flex flex-col gap-1 text-[11px] text-muted-foreground",
                                                    div { class: "flex items-center gap-1.5",
                                                        span {
                                                            class: "h-1.5 w-1.5 rounded-full shrink-0",
                                                            style: "background-color: {item.2.color};"
                                                        }
                                                        "{item.4} - {item.5}"
                                                    }
                                                }
                                                if let Some(ref desc) = item.3.description {
                                                    if !desc.trim().is_empty() {
                                                        div { class: "mt-2 border-t border-border/50 pt-2",
                                                            p { class: "m-0 text-[11px] italic leading-relaxed text-muted-foreground", "{desc}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if cell_events.len() > 3 {
                                        div { class: "pl-1 text-[9px] font-bold text-muted-foreground select-none mt-0.5",
                                            "+{cell_events.len() - 3} more"
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
