use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::JobTicket;

mod details;

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
    let uid_for_inv = props.active_user_id.read().clone();
    let inventory_res = use_resource(move || {
        let _ = db_trig;
        let jid = selected_job_id_for_inv.clone();
        let uid = uid_for_inv.clone();
        async move {
            if jid.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_move_inventory(uid, jid).await.unwrap_or_default()
            }
        }
    });

    let selected_job_id_for_quote = selected_job_id.read().clone().unwrap_or_default();
    let uid_for_quote = props.active_user_id.read().clone();
    let quote_res = use_resource(move || {
        let _ = db_trig;
        let jid = selected_job_id_for_quote.clone();
        let uid = uid_for_quote.clone();
        async move {
            if jid.is_empty() {
                None
            } else {
                yntra_core::get_move_quote(uid, jid).await.unwrap_or(None)
            }
        }
    });

    let inventories = inventory_res.read().clone().unwrap_or_default();
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
                        details::JobDetails {
                            job: job,
                            active_user_id: props.active_user_id.read().clone(),
                            region: region.clone(),
                            checklist_state: checklist_state,
                            completion_report_state: completion_report_state,
                            inventories: inventories.clone(),
                            quote: quote.clone(),
                            db_trigger: db_trigger,
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
