use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::JobTicket;
use super::ChecklistItem;

#[derive(Clone, PartialEq, Debug)]
pub struct MoveInventoryItem {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_category: String,
    pub item_name: String,
    pub quantity: i32,
    pub estimated_volume_m3: f64,
    pub handling_notes: Option<String>,
    pub updated_at: i64,
    pub sync_status: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MoveQuote {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub base_price: i64,
    pub distance_fee: i64,
    pub stairs_surcharge: i64,
    pub packing_supplies_fee: i64,
    pub total_price: i64,
    pub status: String,
    pub accepted_at: Option<i64>,
    pub updated_at: i64,
    pub sync_status: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct JobDetailsProps {
    pub job: JobTicket,
    pub active_user_id: String,
    pub region: String,
    pub checklist_state: Signal<Vec<ChecklistItem>>,
    pub completion_report_state: Signal<String>,
    pub inventories: Vec<MoveInventoryItem>,
    pub quote: Option<MoveQuote>,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn JobDetails(props: JobDetailsProps) -> Element {
    let job = props.job;
    let active_user_id = props.active_user_id;
    let region = props.region;
    let mut checklist_state = props.checklist_state;
    let mut completion_report_state = props.completion_report_state;
    let inventories = props.inventories;
    let quote = props.quote;

    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let modules_active_val: serde_json::Value = if let Some(ws) = workspace_opt {
        serde_json::from_str(&ws.modules_active).unwrap_or_default()
    } else {
        serde_json::from_str("{\"todos\":true,\"notes\":true,\"reporting\":true}").unwrap()
    };
    let todos_enabled = modules_active_val.get("todos").and_then(|v| v.as_bool()).unwrap_or(true);
    let reporting_enabled = modules_active_val.get("reporting").and_then(|v| v.as_bool()).unwrap_or(true);

    let job_id_status = job.id.clone();
    let job_id_submit = job.id.clone();
    let job_priority = job.priority.clone();
    let job_status = job.status.clone();
    let job_title = job.title.clone();
    let job_description = job.description.clone();
    let job_location = job.location_address.clone();
    let checklist = checklist_state.read().clone();
    
    let checklist_header = t("jobs-checklist-header", &region);
    let report_placeholder = t("jobs-report-placeholder", &region);

    let priority_color = match job_priority.as_str() {
        "critical" => "background: rgba(239, 68, 68, 0.15); color: hsl(0, 90.6%, 70.8%);",
        "high" => "background: rgba(245, 158, 11, 0.15); color: hsl(43.3, 96.4%, 56.3%);",
        "medium" => "background: rgba(59, 130, 246, 0.15); color: hsl(213.1, 93.9%, 67.8%);",
        _ => "background: rgba(156, 163, 175, 0.15); color: hsl(217.9, 10.6%, 64.9%);",
    };

    let total_vol: f64 = inventories.iter().map(|i| i.estimated_volume_m3 * i.quantity as f64).sum();

    rsx! {
        components::Card {
            class: "p-6 flex flex-col gap-5",
            
            // Header: Title & Badges
            div { class: "border-b border-border",
            style: "padding-bottom: 1rem;",
                div { class: "flex gap-2 items-center mb-2",
                    span {
                        style: format!("font-size: 0.7rem; font-weight: 800; text-transform: uppercase; padding: 0.2rem 0.5rem; border-radius: 4px; {}", priority_color),
                        "{job_priority}"
                    }
                    span {
                        class: "text-[11px] font-bold text-muted-foreground",
                        style: "background: rgba(107, 114, 128, 0.15); padding: 0.2rem 0.5rem; border-radius: 4px;",
                        "{job_status}"
                    }
                }
                h2 { class: "m-0 font-extrabold",
                style: "font-size: 1.4rem;", "{job_title}" }
                p { class: "text-sm text-muted-foreground",
                style: "margin: 0.5rem 0 0 0; line-height: 1.4;", "{job_description}" }
            }
            
            // Location panel
            if let (Some(ref origin), Some(ref dest)) = (job.origin_address.clone(), job.destination_address.clone()) {
                div { class: "flex flex-col gap-3 border border-border p-4 rounded-lg",
                    style: "background: var(--bg-app);",
                    div { class: "flex items-start gap-3",
                        components::LucideIcon { name: "circle", size: "14", class: "accent-text mt-1", }
                        div {
                            div { class: "text-[10px] font-bold text-muted-foreground uppercase", "Från (Ursprung)" }
                            div { class: "text-sm font-semibold", "{origin}" }
                            div { class: "text-xs text-muted-foreground mt-0.5",
                                "Våning: {job.origin_floor} • Hiss: "
                                if job.origin_has_elevator { "Ja" } else { "Nej" }
                                " • Parkeringstillstånd: "
                                if job.origin_parking_permit_needed { "Krävs" } else { "Ej krav" }
                            }
                        }
                    }
                    div { class: "h-px bg-border/40 ml-6", }
                    div { class: "flex items-start gap-3",
                        components::LucideIcon { name: "map-pin", size: "16", class: "text-primary mt-1", }
                        div {
                            div { class: "text-[10px] font-bold text-muted-foreground uppercase", "Till (Destination)" }
                            div { class: "text-sm font-semibold", "{dest}" }
                            div { class: "text-xs text-muted-foreground mt-0.5",
                                "Våning: {job.destination_floor} • Hiss: "
                                if job.destination_has_elevator { "Ja" } else { "Nej" }
                                " • Parkeringstillstånd: "
                                if job.destination_parking_permit_needed { "Krävs" } else { "Ej krav" }
                            }
                        }
                    }
                }
            } else {
                div { class: "flex items-center gap-2 border border-border p-3 rounded-lg",
                    style: "background: var(--bg-app);",
                    components::LucideIcon { name: "map-pin", size: "18", class: "accent-text", }
                    div {
                        div { class: "text-[11px] font-bold text-muted-foreground uppercase", "{t(\"jobs-location-label\", &region)}" }
                        div { class: "text-sm font-semibold", "{job_location}" }
                    }
                }
            }

            // Interactive Checklist
            if todos_enabled {
                div {
                    h3 { class: "text-sm font-extrabold flex items-center",
                    style: "margin: 0 0 0.75rem 0; gap: 0.35rem;",
                        components::LucideIcon { name: "check-square", size: "16" }
                        "{checklist_header}"
                    }
                    
                    div { class: "flex flex-col",
                    style: "gap: 0.65rem;",
                        {checklist.iter().enumerate().map(|(idx, item)| {
                            let is_done = item.done;
                            let item_text = item.text.clone();
                            rsx! {
                                div {
                                    key: "{idx}",
                                    class: "flex items-center gap-2 text-sm",
                                    components::Checkbox {
                                        checked: is_done,
                                        onchange: move |val| {
                                            let mut current = checklist_state.read().clone();
                                            current[idx].done = val;
                                            checklist_state.set(current);
                                        }
                                    }
                                    span {
                                        class: "cursor-pointer",
                                        style: format!(
                                            "transition: all 0.2s; {}",
                                            if is_done { "text-decoration: line-through; color: var(--text-secondary);" } else { "" }
                                        ),
                                        onclick: move |_| {
                                            let mut current = checklist_state.read().clone();
                                            current[idx].done = !is_done;
                                            checklist_state.set(current);
                                        },
                                        "{item_text}"
                                    }
                                }
                            }
                        })}
                    }
                }
            }

            // Moving Inventory list
            if !inventories.is_empty() {
                div { class: "border-t border-border/40 pt-4 flex flex-col gap-3",
                    h3 { class: "text-sm font-extrabold flex items-center gap-1.5",
                        style: "margin: 0;",
                        components::LucideIcon { name: "box", size: "16", class: "accent-text" }
                        "Flyttinventarie & Cargo ({total_vol:.1} m³)"
                    }
                    div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2",
                        for item in inventories.iter() {
                            div {
                                key: "{item.id}",
                                class: "flex items-center justify-between p-2.5 rounded-lg border border-border/20 bg-secondary/5",
                                div {
                                    div { class: "text-xs font-bold text-foreground", "{item.item_name}" }
                                    div { class: "text-[10px] text-muted-foreground", "{item.item_category}" }
                                }
                                div { class: "text-right",
                                    div { class: "text-xs font-extrabold text-primary", "{item.quantity} st" }
                                    div { class: "text-[10px] text-muted-foreground font-semibold", "{item.estimated_volume_m3 * item.quantity as f64} m³" }
                                }
                            }
                        }
                    }
                }
            }

            // Quote details card
            if let Some(ref q) = quote {
                div { class: "border-t border-border/40 pt-4 flex flex-col gap-3",
                    h3 { class: "text-sm font-extrabold flex items-center gap-1.5",
                        style: "margin: 0;",
                        components::LucideIcon { name: "credit-card", size: "16", class: "accent-text" }
                        "Flyttoffert"
                        span {
                            class: if q.status == "accepted" { "ml-2 px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-500" } else { "ml-2 px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500/10 text-amber-500" },
                            if q.status == "accepted" { "Godkänd" } else { "Skickad" }
                        }
                    }
                    div { class: "p-4 rounded-lg border border-border/20 bg-secondary/5 grid grid-cols-2 sm:grid-cols-4 gap-4",
                        div {
                            div { class: "text-[10px] text-muted-foreground font-semibold", "Baspris" }
                            div { class: "text-xs font-bold text-foreground mt-0.5", "{q.base_price} kr" }
                         }
                         div {
                             div { class: "text-[10px] text-muted-foreground font-semibold", "Distans" }
                             div { class: "text-xs font-bold text-foreground mt-0.5", "{q.distance_fee} kr" }
                         }
                         div {
                             div { class: "text-[10px] text-muted-foreground font-semibold", "Trappor" }
                             div { class: "text-xs font-bold text-foreground mt-0.5", "{q.stairs_surcharge} kr" }
                         }
                         div {
                             div { class: "text-[10px] text-muted-foreground font-semibold", "Material" }
                             div { class: "text-xs font-bold text-foreground mt-0.5", "{q.packing_supplies_fee} kr" }
                         }
                    }
                    div { class: "flex items-center justify-between text-xs font-bold text-foreground px-1",
                        span { "Totalt Offerterat Pris:" }
                        span { class: "text-sm text-primary font-extrabold", "{q.total_price} kr" }
                    }
                }
            }

            // Completion Report Textbox
            if reporting_enabled {
                div {
                    h3 { class: "text-sm font-extrabold flex items-center",
                    style: "margin: 0 0 0.5rem 0; gap: 0.35rem;",
                        components::LucideIcon { name: "file-edit", size: "16" }
                        "{t(\"jobs-report-header\", &region)}"
                    }
                    components::TextArea {
                        placeholder: report_placeholder.to_string(),
                        value: completion_report_state.read().clone(),
                        oninput: move |evt: FormEvent| completion_report_state.set(evt.value()),
                    }
                }
            }

            // Action Buttons
            div { class: "flex gap-3 border-t border-border pt-5",
                if job_status == "assigned" {
                    components::Button {
                        variant: components::ButtonVariant::Primary,
                        onclick: move |_| {
                            let job_id = job_id_status.clone();
                            let uid = active_user_id.clone();
                            spawn(async move {
                                let _ = yntra_core::update_job_status(uid, job_id, "in_progress".to_string()).await;
                            });
                        },
                        "{t(\"jobs-action-start\", &region)}"
                    }
                } else if job_status == "in_progress" {
                    components::Button {
                        variant: components::ButtonVariant::Primary,
                        onclick: move |_| {
                            let checklist_str = serde_json::to_string(&*checklist_state.read()).unwrap();
                            let report_str = completion_report_state.read().clone();
                            let job_id = job_id_submit.clone();
                            let uid = active_user_id.clone();
                            spawn(async move {
                                let _ = yntra_core::submit_job_completion(uid, job_id, checklist_str, report_str).await;
                            });
                        },
                        "{t(\"jobs-action-complete\", &region)}"
                    }
                } else {
                    div { class: "flex items-center text-sm font-bold",
                    style: "gap: 0.35rem; color: hsl(158.1, 64.4%, 51.6%);",
                        components::LucideIcon { name: "check-circle", size: "18" }
                        "{t(\"jobs-action-archived\", &region)}"
                    }
                }
            }
        }
    }
}
