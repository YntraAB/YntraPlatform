use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TeamEvent;
use super::utils::*;

#[derive(Props, Clone)]
pub struct AgendaViewProps {
    pub scheduled_events: Vec<TeamEvent>,
    pub calendar_year: i32,
    pub calendar_month: u32,
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub locale: String,
}

impl PartialEq for AgendaViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

struct GroupedAgenda {
    date_str: String,
    month_label: String,
    day_label: String,
    day_of_week_label: String,
    is_today: bool,
    events: Vec<TeamEvent>,
}

#[component]
pub fn AgendaView(props: AgendaViewProps) -> Element {
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let mut agenda_events = props.scheduled_events.clone();
    agenda_events.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    let mut groups: Vec<GroupedAgenda> = Vec::new();

    for ev in agenda_events.iter() {
        let date_part = if ev.start_time.len() >= 10 {
            ev.start_time[..10].to_string()
        } else {
            "2026-06-30".to_string()
        };
        
        let is_today = date_part == "2026-06-30";

        if let Some(group) = groups.iter_mut().find(|g| g.date_str == date_part) {
            group.events.push(ev.clone());
        } else {
            let parts: Vec<&str> = date_part.split('-').collect();
            let mut month_label = "Jun".to_string();
            let mut day_label = "30".to_string();
            if parts.len() == 3 {
                if let Ok(m) = parts[1].parse::<u32>() {
                    month_label = get_month_name(m).chars().take(3).collect::<String>();
                }
                day_label = parts[2].to_string();
            }

            let weekday_str = if is_today {
                t("common-today", &props.locale)
            } else {
                date_part.clone()
            };

            groups.push(GroupedAgenda {
                date_str: date_part,
                month_label,
                day_label,
                day_of_week_label: weekday_str,
                is_today,
                events: vec![ev.clone()],
            });
        }
    }

    rsx! {
        div { class: "scrollbar-dark flex-1 overflow-y-auto p-4 select-none",
            if groups.is_empty() {
                div { class: "flex h-full flex-col items-center justify-center text-muted-foreground/60 py-16",
                    components::LucideIcon { name: "calendar", class: "mb-4 h-12 w-12 opacity-30 text-muted-foreground" }
                    p { class: "text-lg font-semibold m-0", "No upcoming events" }
                    p { class: "text-xs mt-1", "Your scheduled events will appear here" }
                }
            } else {
                div { class: "mx-auto max-w-3xl space-y-6",
                    for group in groups.iter() {
                        div {
                            key: "{group.date_str}",
                            
                            // Date Header
                            div { class: "mb-3 flex items-center gap-3",
                                div {
                                    class: if group.is_today {
                                        "flex h-10 w-10 flex-col items-center justify-center rounded-lg text-xs bg-primary text-white font-bold shadow-sm"
                                    } else {
                                        "flex h-10 w-10 flex-col items-center justify-center rounded-lg text-xs bg-muted text-muted-foreground border border-border/50 font-medium"
                                    },
                                    span { class: "text-[9px] uppercase tracking-wider font-semibold opacity-85", "{group.month_label}" }
                                    span { class: "font-bold text-sm", "{group.day_label}" }
                                }
                                h3 {
                                    class: if group.is_today {
                                        "text-lg font-bold text-foreground m-0"
                                    } else {
                                        "text-lg font-bold text-muted-foreground/80 m-0"
                                    },
                                    "{group.day_of_week_label}"
                                }
                            }

                            // Events list
                            div { class: "ml-12 space-y-2",
                                for item in group.events.iter().map(|ev| {
                                    let ev_clone = ev.clone();
                                    let config = get_event_category_config(&ev.title, &ev.metadata);
                                    let metadata_obj = parse_metadata(&ev.metadata);
                                    let start_hm = ev.start_time.split('T').next_back().unwrap_or(&ev.start_time).chars().take(5).collect::<String>();
                                    let end_hm = ev.end_time.split('T').next_back().unwrap_or(&ev.end_time).chars().take(5).collect::<String>();
                                    (ev_clone, config, metadata_obj, start_hm, end_hm)
                                }) {
                                    div {
                                        key: "{item.0.id}",
                                        onclick: {
                                            let ev_c = item.0.clone();
                                            move |_| {
                                                show_event_detail_modal.set(Some(ev_c.clone()));
                                            }
                                        },
                                        class: "flex cursor-pointer items-center gap-4 rounded-lg border border-border bg-muted p-3 transition-all duration-150 hover:border-primary/50 hover:bg-background",
                                        
                                        // Time
                                        div { class: "w-20 shrink-0 text-right font-medium",
                                            div { class: "text-sm text-foreground", "{item.3}" }
                                            div { class: "text-xs text-muted-foreground mt-0.5", "{item.4}" }
                                        }

                                        // Category vertical line
                                        div {
                                            class: "h-10 w-1 shrink-0 rounded-full",
                                            style: "background-color: {item.1.color};"
                                        }

                                        // Details
                                        div { class: "min-w-0 flex-1",
                                            div { class: "truncate font-bold text-sm text-foreground", "{item.0.title}" }
                                            if let Some(ref desc) = item.2.description {
                                                if !desc.trim().is_empty() {
                                                    div { class: "truncate text-xs text-muted-foreground mt-0.5", "{desc}" }
                                                }
                                            }
                                        }

                                        // Category Badge
                                        span {
                                            class: "shrink-0 rounded px-2 py-1 text-[10px] font-bold uppercase tracking-wider border border-border/20",
                                            style: "background-color: {item.1.bg_color}; color: {item.1.color}; border-color: {item.1.color}22",
                                            "{t(item.1.label_key, &props.locale)}"
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
