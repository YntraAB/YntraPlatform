use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{JobTicket, MoveInventoryItem, MoveQuote};

fn trigger_download(content: &str, file_name: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let base64_str = crate::views::school::academics::utils::base64_encode(content.as_bytes());
        let js_code = format!(
            r#"
            (function() {{
                const base64 = "{}";
                const filename = "{}";
                const binString = atob(base64);
                const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                const blob = new Blob([bytes], {{ type: "application/octet-stream" }});
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = filename;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            }})();
            "#,
            base64_str, file_name
        );
        let _ = js_sys::eval(&js_code);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let home_dir = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let paths = vec![
            format!("{}/Desktop", home_dir),
            format!("{}/Downloads", home_dir),
            home_dir.clone(),
        ];
        for path in paths {
            let file_path = std::path::PathBuf::from(&path).join(file_name);
            if std::fs::write(&file_path, content).is_ok() {
                break;
            }
        }
    }
}

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
            match yntra_core::get_job_tickets(uid).await {
                Ok(list) => list,
                Err(_) => Vec::new(),
            }
        }
    });

    let jobs = jobs_resource.read().clone().unwrap_or_default();

    // Selected job state
    let mut selected_job_id = use_signal(|| Option::<String>::None);
    let selected_job = selected_job_id
        .read()
        .clone()
        .and_then(|id| jobs.iter().find(|j| j.id == id).cloned());

    // Selected job checklist & report states
    let mut checklist_state = use_signal(Vec::<ChecklistItem>::new);
    let mut completion_report_state = use_signal(String::new);
    let mut active_status_state = use_signal(|| "all".to_string());

    // Context Menu signals
    let mut job_context_menu_open = use_signal(|| false);
    let mut job_context_menu_pos = use_signal(|| (0, 0));
    let mut job_context_menu_val = use_signal(|| Option::<JobTicket>::None);

    // Reactive resources for moving company modules (Inventory & Quote)
    let details_resource = use_resource(move || {
        let uid = props.active_user_id.read().clone();
        let job_id_opt = selected_job_id.read().clone();
        async move {
            if let Some(job_id) = job_id_opt {
                let inv = yntra_core::get_move_inventory(uid.clone(), job_id.clone())
                    .await
                    .unwrap_or_default();
                let q = yntra_core::get_move_quote(uid, job_id)
                    .await
                    .unwrap_or(None);
                (inv, q)
            } else {
                (Vec::new(), None)
            }
        }
    });

    let details_val = details_resource.read().clone().unwrap_or((Vec::new(), None));
    let inventories: Vec<MoveInventoryItem> = details_val.0;
    let quote: Option<MoveQuote> = details_val.1;

    // Sync checklist/report when a new job is selected, reading directly from resource to avoid moves
    use_effect(use_reactive(&selected_job_id, move |selected_id| {
        if let Some(id) = selected_id.read().clone()
            && let Some(jobs_list) = jobs_resource.read().as_ref()
            && let Some(job) = jobs_list.iter().find(|j| j.id == id)
        {
            let items: Vec<ChecklistItem> =
                serde_json::from_str(&job.checklist_json).unwrap_or_default();
            checklist_state.set(items);
            completion_report_state.set(job.completion_report.clone().unwrap_or_default());
        }
    }));

    let rut_invoices_res = use_resource(move || {
        let trig = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            let _ = trig;
            yntra_core::get_rut_invoices(uid).await.unwrap_or_default()
        }
    });

    let selected_rut_invoices = use_signal(std::collections::HashSet::<String>::new);
    let list = rut_invoices_res.read().clone().unwrap_or_default();

    // Filter jobs by status tabs (All, Assigned, In Progress, Completed)
    let status_filter = active_status_state.read().clone();
    let filtered_jobs: Vec<JobTicket> = jobs
        .iter()
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
        components::tabs::TabItem {
            value: "rut_exports".to_string(),
            label: "RUT-avdrag".to_string(),
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

            // Two-column responsive layout or Bulk RUT View
            if *active_status_state.read() == "rut_exports" {
                components::Card {
                    class: "w-full p-6 flex flex-col gap-4",
                    div { class: "flex justify-between items-center pb-4 border-b border-border",
                        div {
                            h2 { class: "text-lg font-bold", "RUT-avdrag bulkhantering" }
                            p { class: "text-xs text-muted-foreground", "Markera betalda fakturor för att ladda ner XML- eller CSV-underlag för Skatteverket." }
                        }
                        div { class: "flex gap-2",
                            button {
                                class: "py-1.5 px-3 bg-primary text-primary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                                disabled: selected_rut_invoices.read().is_empty(),
                                onclick: {
                                    let uid = props.active_user_id.read().clone();
                                    move |_| {
                                        let uid = uid.clone();
                                        let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                        spawn(async move {
                                            if let Ok(xml_content) = yntra_core::export_skatteverket_claims(uid, ids, "xml".to_string()).await {
                                                trigger_download(&xml_content, "Skatteverket_RUT_Bulk.xml");
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "download", size: "14" }
                                "Exportera XML (Begäran)"
                            }
                            button {
                                class: "py-1.5 px-3 bg-secondary text-secondary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                                disabled: selected_rut_invoices.read().is_empty(),
                                onclick: {
                                    let uid = props.active_user_id.read().clone();
                                    move |_| {
                                        let uid = uid.clone();
                                        let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                        spawn(async move {
                                            if let Ok(csv_content) = yntra_core::export_skatteverket_claims(uid, ids, "csv".to_string()).await {
                                                trigger_download(&csv_content, "Skatteverket_RUT_Bulk.csv");
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "download", size: "14" }
                                "Exportera CSV"
                            }
                        }
                    }

                    if list.is_empty() {
                        div { class: "text-center p-8 text-muted-foreground",
                            components::LucideIcon { name: "inbox", size: "32", class: "icon-muted mb-2" }
                            p { class: "text-sm", "Inga betalda fakturor med RUT-avdrag hittades." }
                        }
                    } else {
                        table { class: "w-full text-left text-xs border-collapse",
                            thead { class: "bg-muted/30 border-b border-border/50 text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground",
                                tr {
                                    th { class: "p-3 w-10 text-center",
                                        input {
                                            type: "checkbox",
                                            checked: selected_rut_invoices.read().len() == list.len(),
                                            onchange: {
                                                let list_clone = list.clone();
                                                move |evt| {
                                                    let mut selected = selected_rut_invoices.clone();
                                                    if evt.value() == "true" {
                                                        let set: std::collections::HashSet<String> = list_clone.iter().map(|item| item.invoice_id.clone()).collect();
                                                        selected.set(set);
                                                    } else {
                                                        selected.set(std::collections::HashSet::new());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    th { class: "p-3", "Faktura ID" }
                                    th { class: "p-3", "Uppdrag" }
                                    th { class: "p-3", "Kund" }
                                    th { class: "p-3", "Personnummer" }
                                    th { class: "p-3", "Betaldatum" }
                                    th { class: "p-3 text-right", "RUT Belopp" }
                                    th { class: "p-3 text-center", "Status" }
                                }
                            }
                            tbody {
                                for item in list.iter() {
                                    {
                                        let is_checked = selected_rut_invoices.read().contains(&item.invoice_id);
                                        let inv_id = item.invoice_id.clone();
                                        rsx! {
                                            tr { key: "{item.invoice_id}", class: "border-b border-border/10 hover:bg-muted/10 transition-colors",
                                                td { class: "p-3 text-center",
                                                    input {
                                                        type: "checkbox",
                                                        checked: is_checked,
                                                        onchange: move |evt| {
                                                            let mut selected = selected_rut_invoices.clone();
                                                            let mut set = (*selected.read()).clone();
                                                            if evt.value() == "true" {
                                                                set.insert(inv_id.clone());
                                                            } else {
                                                                set.remove(&inv_id);
                                                            }
                                                            selected.set(set);
                                                        }
                                                    }
                                                }
                                                td { class: "p-3 font-semibold font-mono text-[10px]", "{item.invoice_id}" }
                                                td { class: "p-3 font-semibold", "{item.job_title}" }
                                                td { class: "p-3", "{item.customer_name}" }
                                                td { class: "p-3 font-mono text-[11px]", "{item.customer_pnum}" }
                                                td { class: "p-3", "{item.payment_date}" }
                                                td { class: "p-3 text-right font-bold text-emerald-500", "{item.rut_amount} kr" }
                                                td { class: "p-3 text-center",
                                                    span { class: "px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-500",
                                                        "Betald"
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
            } else {
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

                            let job_clone = job.clone();
                            rsx! {
                                div {
                                    key: "{job_id}",
                                    class: "cursor-pointer w-full",
                                    onclick: move |_| selected_job_id.set(Some(job_id.clone())),
                                    oncontextmenu: move |evt| {
                                        evt.prevent_default();
                                        let coords = evt.client_coordinates();
                                        job_context_menu_pos.set((coords.x as i32, coords.y as i32));
                                        job_context_menu_val.set(Some(job_clone.clone()));
                                        job_context_menu_open.set(true);
                                    },
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

        // Job Ticket Context Menu Overlay
        if let Some(job) = job_context_menu_val.read().clone() {
            {
                let job_id = job.id.clone();
                let active_uid = props.active_user_id.read().clone();
                let db_trig = db_trigger;
                let mut selected_id = selected_job_id;

                rsx! {
                    components::ContextMenu {
                        open: *job_context_menu_open.read(),
                        x: job_context_menu_pos.read().0,
                        y: job_context_menu_pos.read().1,
                        onclose: move |_| job_context_menu_open.set(false),

                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: {
                                let job_id = job_id.clone();
                                move |_| {
                                    selected_id.set(Some(job_id.clone()));
                                    job_context_menu_open.set(false);
                                }
                            },
                            components::LucideIcon { name: "info", size: "14" }
                            "View Details"
                        }
                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: {
                                let job_id = job_id.clone();
                                let active_uid = active_uid.clone();
                                move |_| {
                                    let j_id = job_id.clone();
                                    let u_id = active_uid.clone();
                                    let mut d_trig = db_trig;
                                    spawn(async move {
                                        let _ = yntra_core::update_job_status(u_id, j_id, "in_progress".to_string()).await;
                                        let current = *d_trig.read();
                                        d_trig.set(current + 1);
                                    });
                                    job_context_menu_open.set(false);
                                }
                            },
                            components::LucideIcon { name: "play", size: "14" }
                            "Mark In Progress"
                        }
                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: {
                                let job_id = job_id.clone();
                                let active_uid = active_uid.clone();
                                move |_| {
                                    let j_id = job_id.clone();
                                    let u_id = active_uid.clone();
                                    let mut d_trig = db_trig;
                                    spawn(async move {
                                        let _ = yntra_core::update_job_status(u_id, j_id, "completed".to_string()).await;
                                        let current = *d_trig.read();
                                        d_trig.set(current + 1);
                                    });
                                    job_context_menu_open.set(false);
                                }
                            },
                            components::LucideIcon { name: "check-circle", size: "14" }
                            "Mark Completed"
                        }
                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: {
                                let job_id = job_id.clone();
                                move |_| {
                                    let js = format!("navigator.clipboard.writeText({:?});", job_id);
                                    let _ = dioxus::document::eval(&js);
                                    job_context_menu_open.set(false);
                                }
                            },
                            components::LucideIcon { name: "copy", size: "14" }
                            "Copy Job ID"
                        }
                    }
                }
            }
        }
    }
}
