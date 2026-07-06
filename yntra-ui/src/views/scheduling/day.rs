use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TeamEvent;
use super::utils::*;

#[derive(Props, Clone)]
pub struct DayViewProps {
    pub selected_calendar_date: Signal<String>,
    pub scheduled_events: Vec<TeamEvent>,
    pub events_sig: Signal<Vec<TeamEvent>>,
    pub dragged_event_id: Signal<Option<String>>,
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub edit_mode: Signal<bool>,
    pub db_trigger: Signal<u32>,
    pub zoom_level: Signal<f64>,
    pub time_slots: Vec<u32>,
    pub is_today: bool,
    pub current_hour: u32,
    pub current_minute: u32,
    pub start_hour: u32,
    pub dragged_over_cell: Signal<Option<String>>,
    pub locale: String,
}

impl PartialEq for DayViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn DayView(props: DayViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user_id = state.active_user_id.read().clone();
    let selected_calendar_date = props.selected_calendar_date;
    let mut dragged_event_id = props.dragged_event_id;
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let db_trigger = props.db_trigger;
    let events_sig = props.events_sig;
    let edit_mode = props.edit_mode;
    let hour_height = *props.zoom_level.read();
    let selected_date_str = selected_calendar_date.read().clone();
    let mut dragged_over_cell = props.dragged_over_cell;

    let day_events: Vec<TeamEvent> = props.scheduled_events
        .iter()
        .filter(|ev| {
            if ev.start_time.starts_with(&selected_date_str) {
                true
            } else { !ev.start_time.contains('-') && selected_date_str == "2026-06-30" }
        })
        .cloned()
        .collect();

    let is_dragged_over = Some(selected_date_str.clone()) == *dragged_over_cell.read();
    let drag_over_class = if is_dragged_over { "bg-primary/5" } else { "" };

    let selected_date_for_enter = selected_date_str.clone();
    let selected_date_for_drop = selected_date_str.clone();

    rsx! {
        div {
            class: "flex-1 overflow-y-auto scrollbar-dark border border-border rounded-xl bg-background",
            style: "max-height: 600px;",
            div { class: "flex min-h-full",
                
                // Time column
                div { class: "w-16 flex-shrink-0 border-r border-border bg-sidebar",
                    div { class: "h-4 border-b border-border" }
                    div { class: "h-4 border-b border-border" }
                    for slot in props.time_slots.iter() {
                        div {
                            key: "{slot}",
                            style: "height: {hour_height}px;",
                            class: "relative border-b border-border select-none",
                            span { class: "absolute -top-2 right-2 text-[10px] text-muted-foreground font-medium",
                                "{slot:02}:00"
                            }
                        }
                    }
                }

                // Day column with events
                div { 
                    class: "relative flex-1 bg-background/20 {drag_over_class}",
                    ondragover: move |e| {
                        e.prevent_default();
                    },
                    ondragenter: move |e| {
                        e.prevent_default();
                        dragged_over_cell.set(Some(selected_date_for_enter.clone()));
                    },
                    ondragleave: move |_| {
                        dragged_over_cell.set(None);
                    },
                    ondrop: {
                        let cell_date_c = selected_date_for_drop.clone();
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
                                     let active_uid = active_user_id.clone();
                                     spawn(async move {
                                          if yntra_core::update_event_time(active_uid, event_id, new_start, new_end).await.is_ok() {
                                              let val = *db_trig.read();
                                              db_trig.set(val + 1);
                                          }
                                     });
                                }
                            }
                        }
                    },
                    div { class: "h-4 border-b border-border" }

                    // Current time indicator line
                    if props.is_today {
                        {
                            let indicator_top = ((props.current_hour as f64 - props.start_hour as f64 + props.current_minute as f64 / 60.0) * hour_height) + 16.0;
                            rsx! {
                                div {
                                    class: "pointer-events-none absolute left-0 right-0 z-20",
                                    style: "top: {indicator_top}px;",
                                    div { class: "flex items-center",
                                        div { class: "-ml-1 h-2.5 w-2.5 rounded-full bg-red-500 shadow-md" }
                                        div { class: "h-px flex-1 bg-red-500" }
                                    }
                                }
                            }
                        }
                    }

                    // Hour grid lines with 15-min sub-lines
                    for slot in props.time_slots.iter() {
                        div {
                            key: "{slot}",
                            style: "height: {hour_height}px;",
                            class: "border-b border-border relative group",
                            div { class: "absolute top-[25%] left-0 right-0 border-b border-border/10 dark:border-white/[0.03] pointer-events-none" }
                            div { class: "absolute top-[50%] left-0 right-0 border-b border-border/20 dark:border-white/[0.05] border-dashed pointer-events-none" }
                            div { class: "absolute top-[75%] left-0 right-0 border-b border-border/10 dark:border-white/[0.03] pointer-events-none" }
                        }
                    }

                    // Absolutely positioned events
                    div { class: "absolute inset-x-0 bottom-0 top-4 pointer-events-none",
                        for item in day_events.iter().map(|ev| {
                            let ev_clone = ev.clone();
                            let ev_id = ev.id.clone();
                            let top = calculate_event_top(&ev.start_time, hour_height, props.start_hour);
                            let height = calculate_event_height(&ev.start_time, &ev.end_time, hour_height);
                            let config = get_event_category_config(&ev.title, &ev.metadata);
                            let metadata_obj = parse_metadata(&ev.metadata);
                            let start_hm = ev.start_time.split('T').next_back().unwrap_or(&ev.start_time).chars().take(5).collect::<String>();
                            let end_hm = ev.end_time.split('T').next_back().unwrap_or(&ev.end_time).chars().take(5).collect::<String>();
                            let is_dragged = Some(ev_id.clone()) == *dragged_event_id.read();
                            let drag_style = if is_dragged { "opacity: 0.35; transform: scale(0.95); cursor: grabbing;" } else { "" };
                            let edit_mode_class = if *edit_mode.read() { "ring-1 ring-primary/30" } else { "" };
                            (ev_clone, ev_id, top, height, config, metadata_obj, start_hm, end_hm, drag_style, edit_mode_class)
                        }) {
                            div {
                                key: "{item.1}",
                                class: "group absolute left-1.5 right-1.5 z-10 rounded-lg p-2.5 text-xs transition-all duration-200 cursor-pointer pointer-events-auto border-l-4 shadow-sm hover:z-30 hover:shadow-lg hover:brightness-105 select-none {item.9}",
                                style: "top: {item.2}px; height: {(item.3 - 2.0).max(36.0)}px; background-color: {item.4.bg_color}; border-left-color: {item.4.color}; {item.8}",
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
                                div { class: "font-bold truncate flex justify-between items-center gap-1.5 w-full",
                                    span { style: "color: {item.4.color};", "{item.0.title}" }
                                    if *edit_mode.read() {
                                        button {
                                            r#type: "button",
                                            class: "text-muted-foreground hover:text-destructive bg-transparent border-0 cursor-pointer p-0.5 rounded transition-colors shrink-0",
                                             onclick: {
                                                 let ev_id = item.1.clone();
                                                 let mut db_trig = db_trigger;
                                                 let active_uid = active_user_id.clone();
                                                 move |evt| {
                                                     evt.stop_propagation();
                                                     let target_ev_id = ev_id.clone();
                                                     let active_uid = active_uid.clone();
                                                     spawn(async move {
                                                         let _ = yntra_core::delete_event(active_uid, target_ev_id).await;
                                                         let val = *db_trig.read();
                                                         db_trig.set(val + 1);
                                                     });
                                                 }
                                             },
                                            components::LucideIcon { name: "trash-2", class: "h-3 w-3" }
                                        }
                                    }
                                }
                                if item.3 > 40.0 {
                                    div { class: "text-[10px] text-muted-foreground mt-0.5 font-medium",
                                        "{item.6} - {item.7}"
                                    }
                                }

                                // Resize Handle - Top (Visual indicator)
                                if *edit_mode.read() {
                                    div { class: "absolute -top-1 left-0 right-0 h-1.5 cursor-ns-resize z-50 flex items-center justify-center",
                                        div { class: "w-8 h-0.5 bg-primary/40 rounded-full opacity-0 group-hover:opacity-100 transition-opacity" }
                                    }
                                }
                                // Resize Handle - Bottom (Visual indicator)
                                if *edit_mode.read() {
                                    div { class: "absolute -bottom-1 left-0 right-0 h-1.5 cursor-ns-resize z-50 flex items-center justify-center",
                                        div { class: "w-8 h-0.5 bg-primary/40 rounded-full opacity-0 group-hover:opacity-100 transition-opacity" }
                                    }
                                }

                                // Hover Information Popup (Top centered placement)
                                div { 
                                    class: "pointer-events-none absolute bottom-full left-1/2 z-[100] mb-2 -translate-x-1/2 hidden w-64 rounded-xl border border-border bg-background/95 p-4 shadow-2xl backdrop-blur-md duration-200 animate-in fade-in slide-in-from-bottom-2 group-hover:block",
                                    
                                    div { class: "mb-2 flex items-center justify-between text-[10px] text-muted-foreground",
                                        span {
                                            class: "rounded px-2 py-0.5 font-bold uppercase tracking-wider",
                                            style: "background-color: {item.4.bg_color}; color: {item.4.color};",
                                            "{t(item.4.label_key, &props.locale)}"
                                        }
                                        span { class: "flex items-center gap-1 font-semibold",
                                            components::LucideIcon { name: "calendar", class: "h-3 w-3 text-primary" }
                                            "{selected_date_str}"
                                        }
                                    }
                                    h4 { class: "m-0 mb-1 text-sm font-bold leading-tight text-foreground", "{item.0.title}" }
                                    div { class: "mb-3 flex flex-col gap-1 text-[11px] text-muted-foreground",
                                        div { class: "flex items-center gap-1.5",
                                            span {
                                                class: "h-1.5 w-1.5 rounded-full shrink-0",
                                                style: "background-color: {item.4.color};"
                                            }
                                            "{item.6} - {item.7}"
                                        }
                                    }
                                    if let Some(ref desc) = item.5.description {
                                        if !desc.trim().is_empty() {
                                            div { class: "mt-2 border-t border-border/50 pt-2",
                                                p { class: "m-0 text-[11px] italic leading-relaxed text-muted-foreground", "{desc}" }
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
