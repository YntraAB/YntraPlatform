use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{Team, WorkspaceUser, TeamEvent, delete_event};
use super::utils::*;

#[derive(Props, Clone)]
pub struct EventDetailModalProps {
    pub show_event_detail_modal: Signal<Option<TeamEvent>>,
    pub editing_event: Signal<Option<TeamEvent>>,
    pub db_trigger: Signal<u32>,
    pub teams: Vec<Team>,
    pub users: Vec<WorkspaceUser>,
    pub locale: String,
    pub is_admin: bool,
}

impl PartialEq for EventDetailModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn EventDetailModal(props: EventDetailModalProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user_id = state.active_user_id.read().clone();
    let mut show_event_detail_modal = props.show_event_detail_modal;
    let mut editing_event = props.editing_event;
    let mut db_trigger = props.db_trigger;
    let teams = props.teams.clone();
    let users = props.users.clone();

    // Fetch active template type dynamically based on the event's workspace
    let show_val = show_event_detail_modal.read().clone();
    let template_type_res = use_resource(move || {
        let ws_id = show_val.as_ref().map(|e| e.workspace_id.clone()).unwrap_or_else(|| "workspace-1".to_string());
        async move {
            yntra_core::get_workspace_template_type(ws_id).await
        }
    });



    if let Some(ref ev) = *show_event_detail_modal.read() {
        let ev_id = ev.id.clone();
        let ev_clone = ev.clone();
        
        let category_config = get_event_category_config(&ev.title, &ev.metadata);
        let metadata_obj = parse_metadata(&ev.metadata);

        let template = template_type_res.read().as_ref().and_then(|r| r.as_ref().ok().copied()).unwrap_or(yntra_core::WorkspaceTemplateType::General);
        let course_name: Option<String> = None;
        
        let classroom = metadata_obj.classroom.clone().filter(|r| !r.trim().is_empty());
        let vehicle = metadata_obj.vehicle_id.clone().filter(|v| !v.trim().is_empty());
        let volume = metadata_obj.cargo_volume.clone().filter(|v| !v.trim().is_empty());
        let destination = metadata_obj.destination.clone().filter(|d| !d.trim().is_empty());

        let assignee = users
            .iter()
            .find(|u| Some(u.id.clone()) == ev.assignee_id)
            .map(|u| u.full_name.clone().unwrap_or_else(|| u.email.clone()))
            .unwrap_or_else(|| "Unassigned".to_string());

        let team_name = teams
            .iter()
            .find(|t| Some(t.id.clone()) == ev.team_id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "No Team".to_string());

        // Parse nice times
        let parts_start: Vec<&str> = ev.start_time.split(' ').collect();
        let date_label = if parts_start.len() == 2 {
            parts_start[0].to_string()
        } else {
            ev.start_time.clone()
        };
        let time_range_label = format_time_range(&ev.start_time, &ev.end_time);

        rsx! {
            components::Dialog {
                open: show_event_detail_modal.read().is_some(),
                onclose: move |_| show_event_detail_modal.set(None),
                title: t("scheduler-shift-details", &props.locale),
                div { 
                    class: "flex flex-col gap-5 text-left",
                    style: "min-width: 380px; box-sizing: border-box; padding: 0.25rem;",
                    
                    // Title and category indicator dot
                    div { class: "flex items-center gap-3 border-b border-border/40 pb-3",
                        span {
                            class: "h-3.5 w-3.5 rounded-full shadow-sm shrink-0",
                            style: "background-color: {category_config.color};"
                        }
                        div { class: "font-bold text-lg text-foreground truncate", "{ev.title}" }
                    }

                    // Time & Date
                    div { class: "flex items-start gap-3.5",
                        div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                            components::LucideIcon { name: "clock", class: "h-4 w-4 text-muted-foreground" }
                        }
                        div { class: "flex flex-col",
                            span { class: "text-sm font-semibold text-foreground", "{date_label}" }
                            span { class: "text-xs text-muted-foreground mt-0.5", "{time_range_label}" }
                        }
                    }

                    // Description
                    if let Some(ref desc) = metadata_obj.description {
                        if !desc.trim().is_empty() {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "align-left", class: "h-4 w-4 text-muted-foreground" }
                                }
                                p { class: "m-0 text-sm leading-relaxed text-muted-foreground pt-1.5", "{desc}" }
                            }
                        }
                    }

                    // Category
                    div { class: "flex items-start gap-3.5",
                        div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                            components::LucideIcon { name: "tag", class: "h-4 w-4 text-muted-foreground" }
                        }
                        div { class: "pt-1.5",
                            span {
                                class: "rounded-full px-2.5 py-1 text-[10px] font-bold tracking-wide shadow-sm border border-border/20",
                                style: "background-color: {category_config.bg_color}; color: {category_config.color}; border-color: {category_config.color}22",
                                "{t(category_config.label_key, &props.locale)}"
                            }
                        }
                    }

                    // Team
                    div { class: "flex items-start gap-3.5",
                        div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                            components::LucideIcon { name: "users", class: "h-4 w-4 text-muted-foreground" }
                        }
                        div { class: "pt-1.5 text-sm font-semibold text-foreground", "{team_name}" }
                    }

                    // Assistant
                    div { class: "flex items-start gap-3.5",
                        div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                            components::LucideIcon { name: "user", class: "h-4 w-4 text-muted-foreground" }
                        }
                        div { class: "pt-1.5 text-sm text-muted-foreground",
                            "{t(\"scheduler-assigned-to\", &props.locale)}: "
                            span { class: "font-bold text-foreground", "{assignee}" }
                        }
                    }

                    // Template-specific metadata fields rendering
                    if template == yntra_core::WorkspaceTemplateType::School {
                        if let Some(name) = course_name {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "book-open", class: "h-4 w-4 text-muted-foreground" }
                                }
                                div { class: "pt-1.5 text-sm text-muted-foreground",
                                    "{t(\"scheduler-course\", &props.locale)}: "
                                    span { class: "font-bold text-foreground", "{name}" }
                                }
                            }
                        }
                        if let Some(room) = classroom {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "map-pin", class: "h-4 w-4 text-muted-foreground" }
                                }
                                div { class: "pt-1.5 text-sm text-muted-foreground",
                                    "{t(\"scheduler-classroom\", &props.locale)}: "
                                    span { class: "font-bold text-foreground", "{room}" }
                                }
                            }
                        }
                    }

                    if template == yntra_core::WorkspaceTemplateType::MovingCompany {
                        if let Some(veh) = vehicle {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "truck", class: "h-4 w-4 text-muted-foreground" }
                                }
                                div { class: "pt-1.5 text-sm text-muted-foreground",
                                    "{t(\"scheduler-vehicle\", &props.locale)}: "
                                    span { class: "font-bold text-foreground", "{veh}" }
                                }
                            }
                        }
                        if let Some(vol) = volume {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "package", class: "h-4 w-4 text-muted-foreground" }
                                }
                                div { class: "pt-1.5 text-sm text-muted-foreground",
                                    "{t(\"scheduler-volume\", &props.locale)}: "
                                    span { class: "font-bold text-foreground", "{vol} m³" }
                                }
                            }
                        }
                        if let Some(dest) = destination {
                            div { class: "flex items-start gap-3.5",
                                div { class: "mt-0.5 rounded-lg bg-muted/60 p-2 border border-border/30",
                                    components::LucideIcon { name: "navigation", class: "h-4 w-4 text-muted-foreground" }
                                }
                                div { class: "pt-1.5 text-sm text-muted-foreground",
                                    "{t(\"scheduler-destination\", &props.locale)}: "
                                    span { class: "font-bold text-foreground", "{dest}" }
                                }
                            }
                        }
                    }

                    // Dialog Footer Buttons
                    div { class: "flex justify-end gap-3 mt-4 pt-4 border-t border-border/50",
                        if props.is_admin {
                            button {
                                class: "yntra-btn btn-danger text-xs px-3.5 py-2 flex items-center gap-1.5 cursor-pointer bg-red-500/10 text-red-500 hover:bg-red-500 hover:text-white border-0",
                                onclick: move |_| {
                                    let event_id = ev_id.clone();
                                    let active_uid = active_user_id.clone();
                                    spawn(async move {
                                        let _ = delete_event(active_uid, event_id).await;
                                    });
                                    let current_val = *db_trigger.read();
                                    db_trigger.set(current_val + 1);
                                    show_event_detail_modal.set(None);
                                },
                                components::LucideIcon { name: "trash", class: "h-3.5 w-3.5" }
                                "{t(\"common-delete\", &props.locale)}"
                            }
                            button {
                                class: "yntra-btn text-xs px-4 py-2 flex items-center gap-1.5 cursor-pointer bg-primary text-white hover:bg-primary/90",
                                onclick: move |_| {
                                    editing_event.set(Some(ev_clone.clone()));
                                    show_event_detail_modal.set(None);
                                },
                                components::LucideIcon { name: "edit", class: "h-3.5 w-3.5" }
                                "{t(\"common-edit\", &props.locale)}"
                            }
                        }
                        button {
                            class: "yntra-btn secondary text-xs px-4 py-2 cursor-pointer",
                            onclick: move |_| show_event_detail_modal.set(None),
                            "{t(\"common-close\", &props.locale)}"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}
