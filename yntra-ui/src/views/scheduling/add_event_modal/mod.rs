pub mod template_inputs;
pub mod time_inputs;

use template_inputs::TemplateInputs;
use time_inputs::TimeInputs;

use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{Team, WorkspaceUser, TeamEvent};
use super::utils::*;

#[derive(Props, Clone)]
pub struct AddEventModalProps {
    pub show_add_event_modal: Signal<bool>,
    pub editing_event: Signal<Option<TeamEvent>>,
    pub selected_calendar_date: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub teams: Vec<Team>,
    pub users: Vec<WorkspaceUser>,
    pub workspace_id: String,
    pub locale: String,
}

impl PartialEq for AddEventModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AddEventModal(props: AddEventModalProps) -> Element {
    let mut show_add_event_modal = props.show_add_event_modal;
    let mut editing_event = props.editing_event;
    let db_trigger = props.db_trigger;
    let teams = props.teams.clone();
    let users = props.users.clone();
    let workspace_id = props.workspace_id.clone();

    // Fetch the active template type from core database
    let template_type_res = use_resource(move || {
        let ws_id = workspace_id.clone();
        async move {
            yntra_core::get_workspace_template_type(ws_id).await
        }
    });

    // Fetch courses list for School templates
    let courses_res = use_resource(move || {
        async move {
            yntra_core::get_courses("user-1".to_string()).await.unwrap_or_default()
        }
    });

    // Form inputs local states
    let mut category = use_signal(|| "assistance_time".to_string());
    let mut team_id = use_signal(String::new);
    let mut assignee_id = use_signal(String::new);
    let mut client_id = use_signal(|| "none".to_string());

    // School Extensions local states
    let mut selected_course_id = use_signal(|| "none".to_string());
    let mut classroom_text = use_signal(String::new);

    // Moving Extensions local states
    let mut vehicle_id_text = use_signal(String::new);
    let mut cargo_volume_text = use_signal(String::new);
    let mut destination_text = use_signal(String::new);
    let mut team_dropdown_open = use_signal(|| false);
    let mut assignee_dropdown_open = use_signal(|| false);
    let mut client_dropdown_open = use_signal(|| false);
    let mut start_date = use_signal(String::new);
    let mut start_time = use_signal(|| "09:00".to_string());
    let mut end_date = use_signal(String::new);
    let mut end_time = use_signal(|| "10:00".to_string());
    let mut description = use_signal(String::new);
    
    // Collapsible states
    let show_overlap = use_signal(|| false);
    let show_breaks = use_signal(|| false);

    // Metadata details states
    let mut waiting_from = use_signal(String::new);
    let mut waiting_to = use_signal(String::new);
    let mut active1_from = use_signal(String::new);
    let mut active1_to = use_signal(String::new);
    let mut active2_from = use_signal(String::new);
    let mut active2_to = use_signal(String::new);
    let mut active3_from = use_signal(String::new);
    let mut active3_to = use_signal(String::new);
    let mut break_from = use_signal(String::new);
    let mut break_to = use_signal(String::new);
    let mut is_break_paid = use_signal(|| false);

    let mut is_submitting = use_signal(|| false);

    // Sync input states when opening modal or changing event
    let effect_teams = teams.clone();
    let effect_users = users.clone();
    use_effect(move || {
        let is_add_open = *show_add_event_modal.read();
        let edit_opt = editing_event.read();
        
        if let Some(ref ev) = *edit_opt {
            let meta = parse_metadata(&ev.metadata);
            category.set(meta.category.clone().unwrap_or_else(|| "assistance_time".to_string()));
            description.set(meta.description.clone().unwrap_or_default());
            team_id.set(ev.team_id.clone().unwrap_or_default());
            assignee_id.set(ev.assignee_id.clone().unwrap_or_default());
            client_id.set(ev.user_id.clone().unwrap_or_else(|| "none".to_string()));
            
            selected_course_id.set(meta.course_id.clone().unwrap_or_else(|| "none".to_string()));
            classroom_text.set(meta.classroom.clone().unwrap_or_default());
            vehicle_id_text.set(meta.vehicle_id.clone().unwrap_or_default());
            cargo_volume_text.set(meta.cargo_volume.clone().unwrap_or_default());
            destination_text.set(meta.destination.clone().unwrap_or_default());
            
            let parts_start: Vec<&str> = ev.start_time.split(' ').collect();
            if parts_start.len() == 2 {
                start_date.set(parts_start[0].to_string());
                start_time.set(parts_start[1].to_string());
            } else {
                start_date.set(props.selected_calendar_date.read().clone());
                start_time.set(ev.start_time.clone());
            }
            
            let parts_end: Vec<&str> = ev.end_time.split(' ').collect();
            if parts_end.len() == 2 {
                end_date.set(parts_end[0].to_string());
                end_time.set(parts_end[1].to_string());
            } else {
                end_date.set(props.selected_calendar_date.read().clone());
                end_time.set(ev.end_time.clone());
            }
            
            if let Some(ref w) = meta.waiting_time {
                waiting_from.set(w.from.clone());
                waiting_to.set(w.to.clone());
            } else {
                waiting_from.set(String::new());
                waiting_to.set(String::new());
            }
            
            if let Some(ref acts) = meta.active_times {
                if let Some(act) = acts.first() {
                    active1_from.set(act.from.clone());
                    active1_to.set(act.to.clone());
                } else {
                    active1_from.set(String::new());
                    active1_to.set(String::new());
                }
                if let Some(act) = acts.get(1) {
                    active2_from.set(act.from.clone());
                    active2_to.set(act.to.clone());
                } else {
                    active2_from.set(String::new());
                    active2_to.set(String::new());
                }
                if let Some(act) = acts.get(2) {
                    active3_from.set(act.from.clone());
                    active3_to.set(act.to.clone());
                } else {
                    active3_from.set(String::new());
                    active3_to.set(String::new());
                }
            } else {
                active1_from.set(String::new());
                active1_to.set(String::new());
                active2_from.set(String::new());
                active2_to.set(String::new());
                active3_from.set(String::new());
                active3_to.set(String::new());
            }
            
            if let Some(ref b) = meta.r#break {
                break_from.set(b.from.clone());
                break_to.set(b.to.clone());
                is_break_paid.set(b.is_paid);
            } else {
                break_from.set(String::new());
                break_to.set(String::new());
                is_break_paid.set(false);
            }
        } else if is_add_open {
            let default_cat = match *template_type_res.read() {
                Some(Ok(yntra_core::WorkspaceTemplateType::School)) => "lectures",
                Some(Ok(yntra_core::WorkspaceTemplateType::MovingCompany)) => "packing",
                Some(Ok(yntra_core::WorkspaceTemplateType::Care)) => "assistance_time",
                _ => "meeting",
            };
            category.set(default_cat.to_string());
            description.set(String::new());
            if !effect_teams.is_empty() {
                team_id.set(effect_teams[0].id.clone());
            } else {
                team_id.set(String::new());
            }
            let non_clients: Vec<&WorkspaceUser> = effect_users.iter().filter(|u| u.role != "client").collect();
            if !non_clients.is_empty() {
                assignee_id.set(non_clients[0].id.clone());
            } else {
                assignee_id.set(String::new());
            }
            client_id.set("none".to_string());
            
            selected_course_id.set("none".to_string());
            classroom_text.set(String::new());
            vehicle_id_text.set(String::new());
            cargo_volume_text.set(String::new());
            destination_text.set(String::new());
            start_date.set(props.selected_calendar_date.read().clone());
            end_date.set(props.selected_calendar_date.read().clone());
            start_time.set("09:00".to_string());
            end_time.set("10:00".to_string());
            
            waiting_from.set(String::new());
            waiting_to.set(String::new());
            active1_from.set(String::new());
            active1_to.set(String::new());
            active2_from.set(String::new());
            active2_to.set(String::new());
            active3_from.set(String::new());
            active3_to.set(String::new());
            break_from.set(String::new());
            break_to.set(String::new());
            is_break_paid.set(false);
        }
    });

    let clients: Vec<WorkspaceUser> = users.iter().filter(|u| u.role == "client").cloned().collect();
    let staff_users: Vec<WorkspaceUser> = users.iter().filter(|u| u.role != "client").cloned().collect();

    let template = template_type_res.read().as_ref().and_then(|r| r.as_ref().ok().copied()).unwrap_or(yntra_core::WorkspaceTemplateType::General);
    let cats_list = get_categories_for_template(template);
    let courses = courses_res.read().clone().unwrap_or_default();
    let quick_cats = match template {
        yntra_core::WorkspaceTemplateType::Care => vec!["assistance_time", "on_call", "administrative_hours", "meeting", "other"],
        yntra_core::WorkspaceTemplateType::School => vec!["lectures", "lab_slots", "grading_hours", "meeting", "other"],
        yntra_core::WorkspaceTemplateType::MovingCompany => vec!["packing", "loading", "transport", "unloading", "other"],
        yntra_core::WorkspaceTemplateType::General => vec!["meeting", "administrative_hours", "training", "other"],
    };

    let is_open = *show_add_event_modal.read() || editing_event.read().is_some();
    let title_key = if editing_event.read().is_some() { "scheduler-edit-event" } else { "scheduler-new-event" };

    rsx! {
        components::Dialog {
            open: is_open,
            onclose: move |_| {
                show_add_event_modal.set(false);
                editing_event.set(None);
            },
            title: t(title_key, &props.locale),
            div { 
                class: "flex flex-col gap-5 text-left max-h-[75vh] overflow-y-auto pr-1 scrollbar-dark",
                style: "min-width: 440px; box-sizing: border-box; padding: 0.5rem;",
                
                // Category Select
                div { class: "flex flex-col gap-2",
                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                        "{t(\"common-category\", &props.locale)}"
                    }
                    select {
                        class: "yntra-input border-border bg-muted/50 w-full",
                        value: "{category}",
                        onchange: move |e| category.set(e.value()),
                        for cat in cats_list.iter() {
                            option { value: "{cat.id}", "{t(cat.label_key, &props.locale)}" }
                        }
                    }
                    div { class: "flex flex-wrap gap-2 pt-1",
                        for cat in quick_cats.iter() {
                            {
                                let cat_str = cat.to_string();
                                let is_active = *category.read() == cat_str;
                                let active_class = if is_active {
                                    "border border-primary/50 bg-primary/20 text-primary shadow-sm font-semibold"
                                } else {
                                    "border border-transparent bg-muted/50 text-muted-foreground hover:bg-muted/80"
                                };
                                let config = get_category_config(&cat_str);
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "rounded-lg px-3 py-1.5 text-[11px] font-medium transition-all cursor-pointer {active_class}",
                                        onclick: move |_| category.set(cat_str.clone()),
                                        "{t(config.label_key, &props.locale)}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Team & Staff Row
                div { class: "grid grid-cols-2 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"common-team\", &props.locale)}"
                        }
                        {
                            let current_team_name = if let Some(t) = teams.iter().find(|t| t.id == *team_id.read()) {
                                t.name.clone()
                            } else if let Some(first_t) = teams.first() {
                                first_t.name.clone()
                            } else {
                                "Select Team...".to_string()
                            };

                            rsx! {
                                components::Dropdown {
                                    label: current_team_name,
                                    open: *team_dropdown_open.read(),
                                    ontoggle: move |_| {
                                        let cur = *team_dropdown_open.read();
                                        team_dropdown_open.set(!cur);
                                    },
                                    for t in teams.iter() {
                                        {
                                            let t_id = t.id.clone();
                                            let t_name = t.name.clone();
                                            rsx! {
                                                components::DropdownItem {
                                                    label: t_name,
                                                    onclick: move |_| {
                                                        team_id.set(t_id.clone());
                                                        team_dropdown_open.set(false);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-staff\", &props.locale)}"
                        }
                        {
                            let current_assignee_name = if let Some(u) = staff_users.iter().find(|u| u.id == *assignee_id.read()) {
                                u.full_name.clone().unwrap_or_else(|| u.email.clone())
                            } else if let Some(first_u) = staff_users.first() {
                                first_u.full_name.clone().unwrap_or_else(|| first_u.email.clone())
                            } else {
                                "Select Staff...".to_string()
                            };

                            rsx! {
                                components::Dropdown {
                                    label: current_assignee_name,
                                    open: *assignee_dropdown_open.read(),
                                    ontoggle: move |_| {
                                        let cur = *assignee_dropdown_open.read();
                                        assignee_dropdown_open.set(!cur);
                                    },
                                    for u in staff_users.iter() {
                                        {
                                            let u_id = u.id.clone();
                                            let u_name = u.full_name.clone().unwrap_or_else(|| u.email.clone());
                                            rsx! {
                                                components::DropdownItem {
                                                    label: u_name,
                                                    onclick: move |_| {
                                                        assignee_id.set(u_id.clone());
                                                        assignee_dropdown_open.set(false);
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

                // Client Select (Optional)
                if !clients.is_empty() {
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-client-optional\", &props.locale)}"
                        }
                        {
                            let current_client_name = if *client_id.read() == "none" {
                                t("scheduler-no-specific-client", &props.locale)
                            } else if let Some(c) = clients.iter().find(|c| c.id == *client_id.read()) {
                                c.full_name.clone().unwrap_or_else(|| c.email.clone())
                            } else {
                                t("scheduler-no-specific-client", &props.locale)
                            };

                            rsx! {
                                components::Dropdown {
                                    label: current_client_name,
                                    open: *client_dropdown_open.read(),
                                    ontoggle: move |_| {
                                        let cur = *client_dropdown_open.read();
                                        client_dropdown_open.set(!cur);
                                    },
                                    components::DropdownItem {
                                        label: t("scheduler-no-specific-client", &props.locale),
                                        onclick: move |_| {
                                            client_id.set("none".to_string());
                                            client_dropdown_open.set(false);
                                        }
                                    }
                                    for c in clients.iter() {
                                        {
                                            let c_id = c.id.clone();
                                            let c_name = c.full_name.clone().unwrap_or_else(|| c.email.clone());
                                            rsx! {
                                                components::DropdownItem {
                                                    label: c_name,
                                                    onclick: move |_| {
                                                        client_id.set(c_id.clone());
                                                        client_dropdown_open.set(false);
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

                // Template-specific inputs (School / Moving Company)
                TemplateInputs {
                    template,
                    selected_course_id,
                    classroom_text,
                    vehicle_id_text,
                    cargo_volume_text,
                    destination_text,
                    courses,
                    locale: props.locale.clone(),
                }

                // Date & Time Grid, Care Overlap & Breaks Collapsibles
                TimeInputs {
                    template,
                    start_date,
                    start_time,
                    end_date,
                    end_time,
                    show_overlap,
                    show_breaks,
                    waiting_from,
                    waiting_to,
                    active1_from,
                    active1_to,
                    active2_from,
                    active2_to,
                    active3_from,
                    active3_to,
                    break_from,
                    break_to,
                    is_break_paid,
                    locale: props.locale.clone(),
                }

                // Description
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                        "{t(\"common-description\", &props.locale)}"
                    }
                    textarea {
                        class: "min-h-[90px] w-full resize-none rounded-lg border border-border bg-muted/50 px-3 py-2 text-foreground text-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{description}",
                        placeholder: t("scheduler-description-placeholder", &props.locale),
                        oninput: move |e| description.set(e.value())
                    }
                }

                // Dialog Action Buttons
                div { class: "flex justify-end gap-3 mt-4 pt-4 border-t border-border/50",
                    button {
                        r#type: "button",
                        class: "yntra-btn secondary text-xs px-4 py-2 cursor-pointer",
                        onclick: move |_| {
                            show_add_event_modal.set(false);
                            editing_event.set(None);
                        },
                        "{t(\"common-cancel\", &props.locale)}"
                    }
                    button {
                        r#type: "button",
                        class: "yntra-btn text-xs px-5 py-2 cursor-pointer font-semibold bg-primary text-white hover:bg-primary/90",
                        disabled: *is_submitting.read() || team_id.read().is_empty() || assignee_id.read().is_empty(),
                        onclick: move |_| {
                            is_submitting.set(true);
                            let cat_val = category.read().clone();
                            let desc_val = description.read().clone();
                            let t_id = if team_id.read().is_empty() { None } else { Some(team_id.read().clone()) };
                            let a_id = if assignee_id.read().is_empty() { None } else { Some(assignee_id.read().clone()) };
                            let r_id = if *client_id.read() == "none" { None } else { Some(client_id.read().clone()) };
                            
                            let start_dt = format!("{} {}", *start_date.read(), *start_time.read());
                            let end_dt = format!("{} {}", *end_date.read(), *end_time.read());

                            let waiting_val = if !waiting_from.read().is_empty() || !waiting_to.read().is_empty() {
                                Some(TimeRange {
                                    from: waiting_from.read().clone(),
                                    to: waiting_to.read().clone(),
                                })
                            } else {
                                None
                            };

                            let mut active_list = Vec::new();
                            if !active1_from.read().is_empty() || !active1_to.read().is_empty() {
                                active_list.push(TimeRange { from: active1_from.read().clone(), to: active1_to.read().clone() });
                            }
                            if !active2_from.read().is_empty() || !active2_to.read().is_empty() {
                                active_list.push(TimeRange { from: active2_from.read().clone(), to: active2_to.read().clone() });
                            }
                            if !active3_from.read().is_empty() || !active3_to.read().is_empty() {
                                active_list.push(TimeRange { from: active3_from.read().clone(), to: active3_to.read().clone() });
                            }
                            let active_val = if active_list.is_empty() { None } else { Some(active_list) };

                            let break_val = if !break_from.read().is_empty() || !break_to.read().is_empty() {
                                Some(BreakInfo {
                                    from: break_from.read().clone(),
                                    to: break_to.read().clone(),
                                    is_paid: *is_break_paid.read(),
                                })
                            } else {
                                None
                            };

                            let course_val = if *selected_course_id.read() == "none" { None } else { Some(selected_course_id.read().clone()) };
                            let classroom_val = if classroom_text.read().is_empty() { None } else { Some(classroom_text.read().clone()) };
                            
                            let vehicle_val = if vehicle_id_text.read().is_empty() { None } else { Some(vehicle_id_text.read().clone()) };
                            let volume_val = if cargo_volume_text.read().is_empty() { None } else { Some(cargo_volume_text.read().clone()) };
                            let dest_val = if destination_text.read().is_empty() { None } else { Some(destination_text.read().clone()) };

                            let metadata_obj = EventMetadata {
                                category: Some(cat_val.clone()),
                                description: Some(desc_val),
                                waiting_time: waiting_val,
                                active_times: active_val,
                                r#break: break_val,
                                course_id: course_val.clone(),
                                classroom: classroom_val,
                                vehicle_id: vehicle_val,
                                cargo_volume: volume_val,
                                destination: dest_val,
                            };

                            let metadata_str = serde_json::to_string(&metadata_obj).unwrap_or_else(|_| "{}".to_string());
                            
                            let courses_list = courses_res.read().clone().unwrap_or_default();
                            let title_val = if let Some(ref cid) = course_val {
                                if let Some(course) = courses_list.iter().find(|c| &c.id == cid) {
                                    format!("{} - {}", course.name, t(&format!("scheduler-categories-{}", cat_val), &props.locale))
                                } else {
                                    t(&format!("scheduler-categories-{}", cat_val), &props.locale)
                                }
                            } else {
                                t(&format!("scheduler-categories-{}", cat_val), &props.locale)
                            };

                            let state = use_context::<crate::state::AppState>();
                            let active_user_id = state.active_user_id.read().clone();
                            let mut db_trig = db_trigger;
                            let edit_opt = editing_event.read().clone();
                            let workspace_id = props.workspace_id.clone();
                            spawn(async move {
                                if let Some(ref ev) = edit_opt {
                                    if yntra_core::update_event(
                                        active_user_id,
                                        ev.id.clone(),
                                        title_val,
                                        start_dt,
                                        end_dt,
                                        t_id,
                                        a_id,
                                        r_id,
                                        metadata_str,
                                    ).await.is_ok() {
                                        let current_val = *db_trig.read();
                                        db_trig.set(current_val + 1);
                                    }
                                } else {
                                    if yntra_core::add_event_with_metadata(
                                        active_user_id,
                                        workspace_id,
                                        title_val,
                                        start_dt,
                                        end_dt,
                                        t_id,
                                        a_id,
                                        r_id,
                                        metadata_str,
                                    ).await.is_ok() {
                                        let current_val = *db_trig.read();
                                        db_trig.set(current_val + 1);
                                    }
                                }
                                show_add_event_modal.set(false);
                                editing_event.set(None);
                                is_submitting.set(false);
                            });
                        },
                        if *is_submitting.read() { "{t(\"common-saving\", &props.locale)}" } else { "{t(\"common-save\", &props.locale)}" }
                    }
                }
            }
        }
    }
}
