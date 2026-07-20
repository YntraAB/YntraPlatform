use super::ChecklistItem;
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
    let uid_for_save = active_user_id.clone();
    let uid_for_calc = active_user_id.clone();
    let uid_for_delete = active_user_id.clone();
    let region = props.region;
    let mut checklist_state = props.checklist_state;
    let mut completion_report_state = props.completion_report_state;
    let inventories = props.inventories;
    let quote = props.quote;

    let quote_id_for_inv = quote.as_ref().map(|q| q.id.clone()).unwrap_or_default();
    let uid_for_inv = active_user_id.clone();
    let db_trig_val = *props.db_trigger.read();
    let invoice_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_inv.clone();
        let qid = quote_id_for_inv.clone();
        async move {
            if qid.is_empty() {
                None
            } else {
                yntra_core::get_move_invoice(uid, qid).await.unwrap_or(None)
            }
        }
    });
    let invoice = invoice_res.read().clone().flatten();

    let mut new_stop_address = use_signal(String::new);
    let uid_for_dir = active_user_id.clone();
    let jid_for_dir = job.id.clone();
    let directions_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_dir.clone();
        let jid = jid_for_dir.clone();
        async move {
            yntra_core::get_directions_url(uid, jid).await.ok()
        }
    });

    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let modules_active_val: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.modules_active).unwrap_or_default()
    } else {
        serde_json::from_str("{\"todos\":true,\"notes\":true,\"reporting\":true}").unwrap()
    };
    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume")
        .to_string();
    let hourly_rate = settings_json
        .get("moving_hourly_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(1200.0);
    let hours = quote.as_ref().map(|q| q.base_price as f64 / hourly_rate).unwrap_or(0.0);
    let todos_enabled = modules_active_val
        .get("todos")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let reporting_enabled = modules_active_val
        .get("reporting")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

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

    let total_vol: f64 = inventories
        .iter()
        .map(|i| i.estimated_volume_m3 * i.quantity as f64)
        .sum();

    let stops_str = job.route_stops_json.clone().unwrap_or_else(|| "[]".to_string());
    let stops: Vec<String> = serde_json::from_str(&stops_str).unwrap_or_default();

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

            // Location / Routing panel
            div { class: "flex flex-col gap-3 border border-border p-4 rounded-lg",
                style: "background: var(--bg-app);",

                if let (Some(ref origin), Some(ref dest)) = (job.origin_address.clone(), job.destination_address.clone()) {
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
                } else {
                    div { class: "flex items-start gap-3",
                        components::LucideIcon { name: "map-pin", size: "16", class: "accent-text mt-1", }
                        div {
                            div { class: "text-[10px] font-bold text-muted-foreground uppercase", "{t(\"jobs-location-label\", &region)}" }
                            div { class: "text-sm font-semibold", "{job_location}" }
                        }
                    }
                }

                // Intermediate stops list
                if !stops.is_empty() {
                    div { class: "h-px bg-border/40 ml-6", }
                    div { class: "flex flex-col gap-2 ml-4 pl-2 border-l-2 border-dashed border-border/60",
                        {stops.iter().enumerate().map(|(idx, stop)| {
                            let stop_val = stop.clone();
                            rsx! {
                                div { key: "{idx}", class: "flex justify-between items-center gap-2 p-2 rounded bg-muted/20 border border-border/20",
                                    div { class: "flex items-center gap-2",
                                        span { class: "w-4 h-4 rounded-full bg-accent/20 text-[9px] font-extrabold flex items-center justify-center text-accent", "{idx + 1}" }
                                        span { class: "text-xs font-medium text-foreground", "{stop}" }
                                    }
                                    div { class: "flex items-center gap-1",
                                        button {
                                            class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer disabled:opacity-30",
                                            disabled: idx == 0,
                                            onclick: {
                                                let uid = active_user_id.clone();
                                                let jid = job.id.clone();
                                                let mut stops_clone = stops.clone();
                                                let mut db_trigger = props.db_trigger;
                                                move |_| {
                                                    let uid = uid.clone();
                                                    let jid = jid.clone();
                                                    stops_clone.swap(idx, idx - 1);
                                                    let stops_param = stops_clone.clone();
                                                    spawn(async move {
                                                        if yntra_core::update_route_stops(uid, jid, stops_param).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "arrow-up", size: "12" }
                                        }
                                        button {
                                            class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer disabled:opacity-30",
                                            disabled: idx == stops.len() - 1,
                                            onclick: {
                                                let uid = active_user_id.clone();
                                                let jid = job.id.clone();
                                                let mut stops_clone = stops.clone();
                                                let mut db_trigger = props.db_trigger;
                                                move |_| {
                                                    let uid = uid.clone();
                                                    let jid = jid.clone();
                                                    stops_clone.swap(idx, idx + 1);
                                                    let stops_param = stops_clone.clone();
                                                    spawn(async move {
                                                        if yntra_core::update_route_stops(uid, jid, stops_param).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "arrow-down", size: "12" }
                                        }
                                        button {
                                            class: "p-1 rounded hover:bg-destructive/20 text-muted-foreground hover:text-destructive border-0 bg-transparent cursor-pointer",
                                            onclick: {
                                                let uid = active_user_id.clone();
                                                let jid = job.id.clone();
                                                let mut stops_clone = stops.clone();
                                                let mut db_trigger = props.db_trigger;
                                                move |_| {
                                                    let uid = uid.clone();
                                                    let jid = jid.clone();
                                                    stops_clone.remove(idx);
                                                    let stops_param = stops_clone.clone();
                                                    spawn(async move {
                                                        if yntra_core::update_route_stops(uid, jid, stops_param).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "trash-2", size: "12" }
                                        }
                                    }
                                }
                            }
                        })}
                    }
                }

                // Add stop controls
                div { class: "h-px bg-border/40 ml-6", }
                div { class: "flex gap-2 items-center ml-6",
                    input {
                        type: "text",
                        placeholder: "Lägg till delstopp (t.ex. Återvinning)...",
                        value: "{new_stop_address}",
                        oninput: move |e| new_stop_address.set(e.value()),
                        class: "flex-1 px-2.5 py-1.5 text-xs bg-muted/40 border border-border rounded text-foreground focus:outline-none focus:border-accent",
                    }
                    button {
                        class: "py-1.5 px-3 bg-secondary text-secondary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer transition-all",
                        onclick: {
                            let uid = active_user_id.clone();
                            let jid = job.id.clone();
                            let mut stops_clone = stops.clone();
                            let mut db_trigger = props.db_trigger;
                            move |_| {
                                let address = new_stop_address.read().trim().to_string();
                                if !address.is_empty() {
                                    let uid = uid.clone();
                                    let jid = jid.clone();
                                    let mut updated_stops = stops_clone.clone();
                                    updated_stops.push(address);
                                    spawn(async move {
                                        if yntra_core::update_route_stops(uid, jid, updated_stops).await.is_ok() {
                                            new_stop_address.set(String::new());
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        }
                                    });
                                }
                            }
                        },
                        "Lägg till"
                    }
                }

                // Destination Address
                if let (Some(_), Some(ref dest)) = (job.origin_address.clone(), job.destination_address.clone()) {
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

                // Map & Optimize buttons
                div { class: "flex gap-2 justify-end pt-2 border-t border-border/20 mt-2",
                    button {
                        class: "py-1.5 px-3 bg-accent text-accent-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                        disabled: stops.is_empty(),
                        onclick: {
                            let uid = active_user_id.clone();
                            let jid = job.id.clone();
                            let mut db_trigger = props.db_trigger;
                            move |_| {
                                let uid = uid.clone();
                                let jid = jid.clone();
                                spawn(async move {
                                    if yntra_core::optimize_job_route(uid, jid).await.is_ok() {
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                });
                            }
                        },
                        components::LucideIcon { name: "sparkles", size: "14" }
                        "Optimera rutt"
                    }
                    if let Some(Some(url)) = directions_res.read().as_ref() {
                        a {
                            href: "{url}",
                            target: "_blank",
                            class: "py-1.5 px-3 bg-primary text-primary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all text-decoration-none",
                            components::LucideIcon { name: "navigation", size: "14" }
                            "Öppna i Google Maps"
                        }
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

            // Moving Inventory & Cargo Section
            {
                let active_role = state.active_user_role.read().clone();
                let is_staff = active_role != "client" && active_role != "anonymous";
                let mut show_add_form = use_signal(|| false);
                let mut new_item_name = use_signal(String::new);
                let mut new_item_category = use_signal(|| "Möbler".to_string());
                let mut new_item_qty = use_signal(|| 1);
                let mut new_item_vol = use_signal(|| 0.5);
                let mut new_item_notes = use_signal(String::new);

                rsx! {
                    div { class: "border-t border-border/40 pt-4 flex flex-col gap-3",
                        h3 { class: "text-sm font-extrabold flex items-center gap-1.5",
                            style: "margin: 0;",
                            components::LucideIcon { name: "box", size: "16", class: "accent-text" }
                            "Flyttinventarie & Cargo ({total_vol:.1} m³)"
                        }
                        
                        if inventories.is_empty() {
                            p { class: "text-xs text-muted-foreground italic my-1", "Inga inventarier tillagda än." }
                        } else {
                            div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2",
                                for item in inventories.iter() {
                                    {
                                        let item_id = item.id.clone();
                                        let item_name = item.item_name.clone();
                                        let item_category = item.item_category.clone();
                                        let qty = item.quantity;
                                        let vol = item.estimated_volume_m3 * item.quantity as f64;
                                        let uid = uid_for_delete.clone();
                                        rsx! {
                                            div {
                                                key: "{item_id}",
                                                class: "flex items-center justify-between p-2.5 rounded-lg border border-border/20 bg-secondary/5",
                                                div {
                                                    div { class: "text-xs font-bold text-foreground", "{item_name}" }
                                                    div { class: "text-[10px] text-muted-foreground", "{item_category}" }
                                                }
                                                div { class: "flex items-center gap-3 text-right",
                                                    div {
                                                        div { class: "text-xs font-extrabold text-primary", "{qty} st" }
                                                        div { class: "text-[10px] text-muted-foreground font-semibold", "{vol} m³" }
                                                    }
                                                    if is_staff {
                                                        button {
                                                            onclick: move |_| {
                                                                let i_id = item_id.clone();
                                                                let uid_capture = uid.clone();
                                                                let mut db_trig = props.db_trigger;
                                                                spawn(async move {
                                                                    let _ = yntra_core::delete_move_inventory_item(uid_capture, i_id).await;
                                                                    let current = *db_trig.read();
                                                                    db_trig.set(current + 1);
                                                                });
                                                            },
                                                            class: "p-1 rounded text-red-500 hover:bg-red-500/10 border-0 bg-transparent cursor-pointer flex items-center justify-center",
                                                            title: "Ta bort",
                                                            components::LucideIcon { name: "trash-2", size: "14" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Inline Add Inventory Form for staff
                        if is_staff {
                            div { class: "mt-2 border border-border/30 rounded-lg p-3 bg-secondary/5",
                                if !*show_add_form.read() {
                                    button {
                                        onclick: move |_| show_add_form.set(true),
                                        class: "w-full py-2 border border-dashed border-border hover:border-primary/50 rounded-lg text-xs font-bold text-muted-foreground hover:text-primary bg-transparent cursor-pointer flex items-center justify-center gap-1.5 transition-all",
                                        components::LucideIcon { name: "plus", size: "14" }
                                        "Lägg till inventarie"
                                    }
                                } else {
                                    div { class: "flex flex-col gap-2.5",
                                        div { class: "flex justify-between items-center",
                                            span { class: "text-[11px] font-bold text-muted-foreground uppercase", "Ny flyttartikel" }
                                            button {
                                                onclick: move |_| show_add_form.set(false),
                                                class: "text-[10px] text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer",
                                                "Avbryt"
                                            }
                                        }
                                        
                                        div { class: "grid grid-cols-2 gap-2",
                                            div { class: "col-span-2 sm:col-span-1",
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Artikelnamn" }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "t.ex. Soffa, Kartong, Säng",
                                                    value: "{new_item_name}",
                                                    oninput: move |e: FormEvent| new_item_name.set(e.value()),
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                }
                                            }
                                            div { class: "col-span-2 sm:col-span-1",
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Kategori" }
                                                select {
                                                    value: "{new_item_category}",
                                                    onchange: move |e: FormEvent| new_item_category.set(e.value()),
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                    option { value: "Möbler", "Möbler" }
                                                    option { value: "Kartonger", "Kartonger" }
                                                    option { value: "Vitvaror", "Vitvaror" }
                                                    option { value: "Övrigt", "Övrigt" }
                                                }
                                            }
                                        }
                                        
                                        div { class: "grid grid-cols-2 gap-2",
                                            div {
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Antal" }
                                                input {
                                                    r#type: "number",
                                                    min: "1",
                                                    value: "{new_item_qty}",
                                                    oninput: move |e: FormEvent| new_item_qty.set(e.value().parse().unwrap_or(1)),
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                }
                                            }
                                            div {
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Volym per st (m³)" }
                                                input {
                                                    r#type: "number",
                                                    step: "0.05",
                                                    min: "0.01",
                                                    value: "{new_item_vol}",
                                                    oninput: move |e: FormEvent| new_item_vol.set(e.value().parse().unwrap_or(0.5)),
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                }
                                            }
                                        }

                                        div {
                                            label { class: "text-[10px] text-muted-foreground block mb-0.5", "Hanteringsanmärkningar" }
                                            input {
                                                r#type: "text",
                                                placeholder: "t.ex. Bräcklig, tung",
                                                value: "{new_item_notes}",
                                                oninput: move |e: FormEvent| new_item_notes.set(e.value()),
                                                class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                            }
                                        }

                                        button {
                                            onclick: {
                                                let j_id = job.id.clone();
                                                let uid = uid_for_save.clone();
                                                let mut db_trig = props.db_trigger;
                                                move |_| {
                                                    let name = new_item_name.read().trim().to_string();
                                                    if name.is_empty() { return; }
                                                    let cat = new_item_category.read().clone();
                                                    let qty = *new_item_qty.read();
                                                    let vol = *new_item_vol.read();
                                                    let notes_str = new_item_notes.read().trim().to_string();
                                                    let notes = if notes_str.is_empty() { None } else { Some(notes_str) };

                                                    let j_id = j_id.clone();
                                                    let uid = uid.clone();
                                                    
                                                    // reset inputs
                                                    new_item_name.set(String::new());
                                                    new_item_notes.set(String::new());
                                                    new_item_qty.set(1);
                                                    new_item_vol.set(0.5);
                                                    show_add_form.set(false);

                                                    spawn(async move {
                                                        let _ = yntra_core::create_move_inventory_item(
                                                            uid,
                                                            j_id,
                                                            cat,
                                                            name,
                                                            qty,
                                                            vol,
                                                            notes
                                                        ).await;
                                                        let current = *db_trig.read();
                                                        db_trig.set(current + 1);
                                                    });
                                                }
                                            },
                                            class: "w-full py-2 bg-primary hover:opacity-90 rounded-lg text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 transition-all",
                                            components::LucideIcon { name: "check", size: "14" }
                                            "Spara artikel"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Quote details card
            {
                let active_role = state.active_user_role.read().clone();
                let is_staff = active_role != "client" && active_role != "anonymous";
                rsx! {
                    div { class: "border-t border-border/40 pt-4 flex flex-col gap-3",
                        h3 { class: "text-sm font-extrabold flex items-center gap-1.5",
                            style: "margin: 0;",
                            components::LucideIcon { name: "credit-card", size: "16", class: "accent-text" }
                            "Flyttoffert"
                            if let Some(ref q) = quote {
                                span {
                                    class: if q.status == "accepted" { "ml-2 px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/10 text-emerald-500" } else { "ml-2 px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500/10 text-amber-500" },
                                    if q.status == "accepted" { "Godkänd" } else { "Skickad" }
                                }
                            }
                        }
                        if let Some(ref q) = quote {
                            div { class: "p-4 rounded-lg border border-border/20 bg-secondary/5 grid grid-cols-2 sm:grid-cols-4 gap-4",
                                  div {
                                     if pricing_model == "hourly" {
                                         div { class: "text-[10px] text-muted-foreground font-semibold", "Timpris ({hours:.1}h)" }
                                     } else {
                                         div { class: "text-[10px] text-muted-foreground font-semibold", "Baspris" }
                                     }
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
                            if let Some(ref inv) = invoice {
                                div { class: "mt-3 p-3 rounded bg-muted/20 border border-border/10 space-y-1.5",
                                    div { class: "text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground", "Fakturainformation" }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                        span { "RUT-avdrag:" }
                                        span { class: "font-semibold text-foreground", "{inv.rut_deduction} kr" }
                                    }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                        span { "Kundbelopp:" }
                                        span { class: "font-semibold text-foreground", "{inv.customer_amount} kr" }
                                    }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                        span { "Skatteverket (RUT):" }
                                        span { class: "font-semibold text-foreground", "{inv.tax_authority_amount} kr" }
                                    }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground pt-1 border-t border-border/10",
                                        span { "Fakturastatus:" }
                                        span {
                                            class: if inv.status == "paid" { "text-emerald-500 font-bold" } else { "text-amber-500 font-bold" },
                                            if inv.status == "paid" { "Betald" } else { "Obetald" }
                                        }
                                    }
                                    if inv.status == "paid" && inv.rut_deduction > 0.0 && is_staff {
                                        div { class: "flex gap-2 pt-2 border-t border-border/10",
                                            button {
                                                class: "flex-1 py-1 bg-primary/20 hover:bg-primary/30 rounded text-[10px] font-bold text-foreground border border-primary/20 cursor-pointer flex items-center justify-center gap-1",
                                                onclick: {
                                                    let inv_id = inv.id.clone();
                                                    let uid = active_user_id.clone();
                                                    move |_| {
                                                        let inv_id = inv_id.clone();
                                                        let uid = uid.clone();
                                                        spawn(async move {
                                                            match yntra_core::export_skatteverket_claims(uid, vec![inv_id.clone()], "xml".to_string()).await {
                                                                Ok(xml_content) => {
                                                                    let file_name = format!("Skatteverket_RUT_{}.xml", inv_id);
                                                                    trigger_download(&xml_content, &file_name);
                                                                }
                                                                Err(_) => {}
                                                            }
                                                        });
                                                    }
                                                },
                                                components::LucideIcon { name: "download", size: "10" }
                                                "XML (RUT)"
                                            }
                                            button {
                                                class: "flex-1 py-1 bg-primary/20 hover:bg-primary/30 rounded text-[10px] font-bold text-foreground border border-primary/20 cursor-pointer flex items-center justify-center gap-1",
                                                onclick: {
                                                    let inv_id = inv.id.clone();
                                                    let uid = active_user_id.clone();
                                                    move |_| {
                                                        let inv_id = inv_id.clone();
                                                        let uid = uid.clone();
                                                        spawn(async move {
                                                            match yntra_core::export_skatteverket_claims(uid, vec![inv_id.clone()], "csv".to_string()).await {
                                                                Ok(csv_content) => {
                                                                    let file_name = format!("Skatteverket_RUT_{}.csv", inv_id);
                                                                    trigger_download(&csv_content, &file_name);
                                                                }
                                                                Err(_) => {}
                                                            }
                                                        });
                                                    }
                                                },
                                                components::LucideIcon { name: "download", size: "10" }
                                                "CSV (RUT)"
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            p { class: "text-xs text-muted-foreground italic my-1", "Ingen offert skapad för detta uppdrag." }
                        }

                        if is_staff {
                            button {
                                onclick: {
                                    let j_id = job.id.clone();
                                    let uid = uid_for_calc.clone();
                                    let mut db_trig = props.db_trigger;
                                    move |_| {
                                        let j_id = j_id.clone();
                                        let uid = uid.clone();
                                        spawn(async move {
                                            let _ = yntra_core::calculate_and_save_move_quote(uid, j_id).await;
                                            let current = *db_trig.read();
                                            db_trig.set(current + 1);
                                        });
                                    }
                                },
                                class: "w-full py-2 bg-primary hover:opacity-90 rounded-lg text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1.5 transition-all",
                                components::LucideIcon { name: "calculator", size: "14" }
                                if quote.is_some() { "Beräkna & uppdatera offert" } else { "Beräkna offert automatiskt" }
                            }
                        }
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
            {
                let uid_for_start = active_user_id.clone();
                let uid_for_complete = active_user_id.clone();
                rsx! {
                    div { class: "flex gap-3 border-t border-border pt-5",
                        if job_status == "assigned" {
                            components::Button {
                                variant: components::ButtonVariant::Primary,
                                onclick: move |_| {
                                    let job_id = job_id_status.clone();
                                    let uid = uid_for_start.clone();
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
                                    let uid = uid_for_complete.clone();
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
    }
}
