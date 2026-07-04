use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::JobTicket;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Props, Clone)]
pub struct JobsViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for JobsViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn JobsView(props: JobsViewProps) -> Element {
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let db_trigger = props.db_trigger;

    // Fetch jobs from the FFI service
    let jobs_resource = use_resource(move || {
        let _ = db_trig; // trigger reload on db updates
        let uid = props.active_user_id.read().clone();
        async move {
            yntra_core::get_job_tickets(uid).await.unwrap_or_default()
        }
    });

    let jobs = jobs_resource.read().clone().unwrap_or_default();

    // Selected job state
    let mut selected_job_id = use_signal(|| Option::<String>::None);
    let selected_job = selected_job_id.read().clone()
        .and_then(|id| jobs.iter().find(|j| j.id == id).cloned());

    // Selected job checklist & report states
    let mut checklist_state = use_signal(Vec::<ChecklistItem>::new);
    let mut completion_report_state = use_signal(String::new);
    let mut active_status_state = use_signal(|| "all".to_string());

    // Reactive resources for moving company modules (Inventory & Quote)
    let selected_job_id_for_inv = selected_job_id.read().clone().unwrap_or_default();
    let inventory_res = use_resource(move || {
        let _ = db_trig;
        let jid = selected_job_id_for_inv.clone();
        async move {
            if jid.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_move_inventory(jid).await.unwrap_or_default()
            }
        }
    });

    let selected_job_id_for_quote = selected_job_id.read().clone().unwrap_or_default();
    let quote_res = use_resource(move || {
        let _ = db_trig;
        let jid = selected_job_id_for_quote.clone();
        async move {
            if jid.is_empty() {
                None
            } else {
                yntra_core::get_move_quote(jid).await.unwrap_or(None)
            }
        }
    });

    let inventories = inventory_res.read().clone().unwrap_or_default();
    let total_vol: f64 = inventories.iter().map(|i| i.estimated_volume_m3 * i.quantity as f64).sum();
    let quote = quote_res.read().clone().flatten();

    // Sync checklist/report when a new job is selected, reading directly from resource to avoid moves
    use_effect(use_reactive(&selected_job_id, move |selected_id| {
        if let Some(id) = selected_id.read().clone()
            && let Some(jobs_list) = jobs_resource.read().as_ref()
                && let Some(job) = jobs_list.iter().find(|j| j.id == id) {
                    let items: Vec<ChecklistItem> = serde_json::from_str(&job.checklist_json).unwrap_or_default();
                    checklist_state.set(items);
                    completion_report_state.set(job.completion_report.clone().unwrap_or_default());
                }
    }));

    // Filter jobs by status tabs (All, Assigned, In Progress, Completed)
    let status_filter = active_status_state.read().clone();
    let filtered_jobs: Vec<JobTicket> = jobs.iter()
        .filter(|j| {
            if status_filter == "all" {
                true
            } else {
                j.status == status_filter
            }
        })
        .cloned()
        .collect();

    // Localized Headers & Labels
    let _view_title = t("jobs-view-title", &region);
    let checklist_header = t("jobs-checklist-header", &region);
    let report_placeholder = t("jobs-report-placeholder", &region);

    let tabs_list = vec![
        components::tabs::TabItem {
            value: "all".to_string(),
            label: t("jobs-filter-all", &region),
            icon: None,
        },
        components::tabs::TabItem {
            value: "assigned".to_string(),
            label: t("jobs-filter-assigned", &region),
            icon: None,
        },
        components::tabs::TabItem {
            value: "in_progress".to_string(),
            label: t("jobs-filter-in-progress", &region),
            icon: None,
        },
        components::tabs::TabItem {
            value: "completed".to_string(),
            label: t("jobs-filter-completed", &region),
            icon: None,
        },
    ];

    rsx! {
        div {
            class: "mx-auto w-full max-w-5xl",
            style: "padding: 2rem; display: flex; flex-direction: column; gap: 1.5rem; box-sizing: border-box;",
            // Header Section
            div { class: "flex flex-col gap-1.5",
                components::Tabs {
                    tabs: tabs_list,
                    active_tab: active_status_state.read().clone(),
                    onchange: move |val| active_status_state.set(val),
                }
            }

            // Two-column responsive layout
            div {
                style: "display: flex; gap: 1.5rem; align-items: start; width: 100%; flex-wrap: wrap; box-sizing: border-box;",
                
                // Left Column: Job Tickets List
                div {
                    style: "display: flex; flex-direction: column; gap: 0.75rem; width: 340px; flex-shrink: 0; min-width: 280px;",
                    if filtered_jobs.is_empty() {
                        components::Card {
                            class: "text-center p-8 text-muted-foreground",
                            components::LucideIcon { name: "inbox", size: "32", class: "icon-muted", }
                            p { class: "mt-2 text-sm", "{t(\"jobs-empty-filter\", &region)}" }
                        }
                    }
                    
                    {filtered_jobs.into_iter().map(|job| {
                        let job_id = job.id.clone();
                        let is_selected = selected_job_id.read().as_ref() == Some(&job_id);
                        
                        let priority_color = match job.priority.as_str() {
                            "critical" => "background: rgba(239, 68, 68, 0.15); color: hsl(0, 90.6%, 70.8%); border: 1px solid rgba(239, 68, 68, 0.2);",
                            "high" => "background: rgba(245, 158, 11, 0.15); color: hsl(43.3, 96.4%, 56.3%); border: 1px solid rgba(245, 158, 11, 0.2);",
                            "medium" => "background: rgba(59, 130, 246, 0.15); color: hsl(213.1, 93.9%, 67.8%); border: 1px solid rgba(59, 130, 246, 0.2);",
                            _ => "background: rgba(156, 163, 175, 0.15); color: hsl(217.9, 10.6%, 64.9%); border: 1px solid rgba(156, 163, 175, 0.2);",
                        };

                        let status_color = match job.status.as_str() {
                            "completed" => "background: rgba(16, 185, 129, 0.15); color: hsl(158.1, 64.4%, 51.6%);",
                            "in_progress" => "background: rgba(245, 158, 11, 0.15); color: hsl(43.3, 96.4%, 56.3%);",
                            _ => "background: rgba(107, 114, 128, 0.15); color: hsl(216, 12.2%, 83.9%);",
                        };

                        rsx! {
                            div {
                                key: "{job_id}",
                                class: "cursor-pointer w-full",
                                onclick: move |_| selected_job_id.set(Some(job_id.clone())),
                                components::Card {
                                    style: format!(
                                        "padding: 1rem; border-color: {}; transition: all 0.2s;",
                                        if is_selected { "var(--accent)" } else { "var(--border-color)" }
                                    ),
                                    
                                    // Top Row: Priority & Status Badges
                                    div { class: "flex justify-between items-center mb-2",
                                        span {
                                            style: format!("font-size: 0.65rem; text-transform: uppercase; font-weight: 800; padding: 0.15rem 0.4rem; border-radius: 4px; {}", priority_color),
                                            "{job.priority}"
                                        }
                                        span {
                                            style: format!("font-size: 0.65rem; font-weight: 700; padding: 0.15rem 0.4rem; border-radius: 4px; {}", status_color),
                                            "{job.status}"
                                        }
                                    }
                                    
                                    h3 { class: "text-sm font-bold",
                    style: "margin: 0 0 0.25rem 0;", "{job.title}" }
                                    p { class: "text-xs text-muted-foreground",
                    style: "margin: 0 0 0.5rem 0; line-height: 1.3;", "{job.description}" }
                                    
                                    // Bottom Metadata
                                    div { class: "flex items-center text-xs text-muted-foreground",
                    style: "gap: 0.35rem;",
                                        components::LucideIcon { name: "map-pin", size: "12" }
                                        span { "{job.location_address}" }
                                    }
                                }
                            }
                        }
                    })}
                }

                // Right Column: Active Job Ticket Detail View
                div {
                    style: "flex: 1 1 0%; min-width: 320px;",
                    if let Some(job) = selected_job {
                        {
                            let job_id_status = job.id.clone();
                            let job_id_submit = job.id.clone();
                            let job_priority = job.priority.clone();
                            let job_status = job.status.clone();
                            let job_title = job.title.clone();
                            let job_description = job.description.clone();
                            let job_location = job.location_address.clone();
                            let checklist = checklist_state.read().clone();
                            
                            let priority_color = match job_priority.as_str() {
                                "critical" => "background: rgba(239, 68, 68, 0.15); color: hsl(0, 90.6%, 70.8%);",
                                "high" => "background: rgba(245, 158, 11, 0.15); color: hsl(43.3, 96.4%, 56.3%);",
                                "medium" => "background: rgba(59, 130, 246, 0.15); color: hsl(213.1, 93.9%, 67.8%);",
                                _ => "background: rgba(156, 163, 175, 0.15); color: hsl(217.9, 10.6%, 64.9%);",
                            };

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

                                    // Action Buttons
                                    div { class: "flex gap-3 border-t border-border pt-5",
                                        if job_status == "assigned" {
                                            components::Button {
                                                variant: components::ButtonVariant::Primary,
                                                onclick: move |_| {
                                                    let job_id = job_id_status.clone();
                                                    let mut trigger = db_trigger;
                                                    spawn(async move {
                                                        if yntra_core::update_job_status(job_id, "in_progress".to_string()).await.is_ok() {
                                                            let current = *trigger.read();
                                                            trigger.set(current + 1);
                                                        }
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
                                                    let mut trigger = db_trigger;
                                                    spawn(async move {
                                                        if yntra_core::submit_job_completion(job_id, checklist_str, report_str).await.is_ok() {
                                                            let current = *trigger.read();
                                                            trigger.set(current + 1);
                                                        }
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
                    } else {
                        components::Card {
                            class: "flex flex-col items-center justify-center text-center text-muted-foreground",
                    style: "min-height: 380px;",
                            components::LucideIcon { name: "wrench", size: "48", class: "icon-muted", }
                            h2 { class: "text-lg font-extrabold text-foreground",
                    style: "margin: 1rem 0 0.25rem 0;", "{t(\"jobs-detail-empty-title\", &region)}" }
                            p { class: "text-sm m-0",
                    style: "max-width: 280px; line-height: 1.4;", "{t(\"jobs-detail-empty-desc\", &region)}" }
                        }
                    }
                }
            }
        }
    }
}
