use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    JobTicket, MoveInventoryItem, MoveQuote, MoveVehicle,
    initiate_bankid_skatteverket_session, submit_skatteverket_claim_direct,
    SkatteverketSubmitResult, BankIdAuthSession,
};

fn trigger_download(toast: &dioxus_primitives::toast::Toasts, locale: &str, content: &str, file_name: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        toast.info(
            t("school-toast-download-started", locale),
            dioxus_primitives::toast::ToastOptions::new().description(t("school-toast-browser-download-desc", locale))
        );

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
        let file_path = rfd::FileDialog::new()
            .set_file_name(file_name)
            .save_file();
        if let Some(path) = file_path {
            if std::fs::write(&path, content).is_ok() {
                let desc = format!("{} {}", t("school-toast-saved-to", locale), path.display());
                toast.success(
                    t("school-toast-export-success", locale),
                    dioxus_primitives::toast::ToastOptions::new().description(desc)
                );
            } else {
                toast.error(
                    t("school-toast-export-failed", locale),
                    dioxus_primitives::toast::ToastOptions::new().description(t("school-toast-export-failed-desc", locale))
                );
            }
        }
    }
}

mod details;
mod dispatch;
mod public_widget;

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
    let toast = dioxus_primitives::toast::use_toast();
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let mut db_trigger = props.db_trigger;
    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let workspace_id = if let Some(ref ws) = workspace_opt {
        ws.id.clone()
    } else {
        "workspace-1".to_string()
    };
    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let show_rut = settings_json
        .get("show_rut_deduction")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| region == "SE");

    let has_skatteverket_cert = settings_json
        .get("skatteverket_corporate_cert")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

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

    // Load dynamic coordinate resolutions for all addresses in jobs
    let workspace_id_c = workspace_id.clone();
    let jobs_list_c = jobs.clone();
    let db_trig_val = db_trig;
    let coords_res = use_resource(move || {
        let ws_id = workspace_id_c.clone();
        let jobs_list = jobs_list_c.clone();
        let _trig = db_trig_val;
        async move {
            let mut coords_map = std::collections::HashMap::new();
            for job in jobs_list {
                let mut addresses = Vec::new();
                addresses.push(job.location_address.clone());
                if let Some(ref origin) = job.origin_address {
                    addresses.push(origin.clone());
                }
                if let Some(ref dest) = job.destination_address {
                    addresses.push(dest.clone());
                }

                for addr in addresses {
                    if !addr.trim().is_empty() && !coords_map.contains_key(&addr) {
                        let (lat, lon) = yntra_core::geocode(&ws_id, &addr).await;
                        coords_map.insert(addr, [lat, lon]);
                    }
                }
            }
            coords_map
        }
    });

    let coords = coords_res.read().clone().unwrap_or_default();

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
    let mut is_simulating = use_signal(|| false);
    
    // Skatteverket Direct Submission signals
    let mut show_skatteverket_modal = use_signal(|| false);
    let mut skatteverket_bankid_session = use_signal(|| Option::<BankIdAuthSession>::None);
    let mut skatteverket_submit_result = use_signal(|| Option::<SkatteverketSubmitResult>::None);
    let mut skatteverket_loading = use_signal(|| false);
    let mut bankid_personal_number = use_signal(|| "".to_string());

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

    // Vehicle signals and resource
    let mut new_vehicle_name = use_signal(String::new);
    let mut new_vehicle_plate = use_signal(String::new);
    let mut new_vehicle_capacity = use_signal(|| 15.0);
    let mut new_vehicle_gps_id = use_signal(String::new);

    let vehicles_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            yntra_core::get_vehicles(uid).await.unwrap_or_default()
        }
    });
    let vehicles = vehicles_res.read().clone().unwrap_or_default();

    // GPS Real-time Telemetry Simulation Loop
    use_effect(move || {
        let active_tab = active_status_state.read().clone();
        let sim_active = *is_simulating.read();
        let uid = props.active_user_id.read().clone();
        
        if sim_active && active_tab == "live_map" {
            spawn(async move {
                let mut tick = 0;
                loop {
                    // Check if simulation is still active
                    if !*is_simulating.read() || active_status_state.read().clone() != "live_map" {
                        break;
                    }
                    
                    if let Ok(v_list) = yntra_core::get_vehicles(uid.clone()).await {
                        for v in v_list {
                            if v.status == "active" {
                                let _ = yntra_core::simulate_vehicle_movement(uid.clone(), v.id, tick).await;
                            }
                        }
                    }
                    
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                    
                    tick += 1;
                    crate::utils::sleep_ms(3000).await;
                }
            });
        }
    });

    // Reactive effect to update map markers dynamically without iframe reload
    use_effect(move || {
        let v_data = vehicles_res.read().clone().unwrap_or_default();
        let j_data = jobs_resource.read().clone().unwrap_or_default();
        let c_data = coords_res.read().clone().unwrap_or_default();
        let v_json = serde_json::to_string(&v_data).unwrap_or_else(|_| "[]".to_string());
        let j_json = serde_json::to_string(&j_data).unwrap_or_else(|_| "[]".to_string());
        let c_json = serde_json::to_string(&c_data).unwrap_or_else(|_| "{}".to_string());
        let script = format!(
            r#"
            var iframe = document.querySelector('iframe');
            if (iframe && iframe.contentWindow && typeof iframe.contentWindow.updateMapData === 'function') {{
                iframe.contentWindow.updateMapData({}, {}, {});
            }}
            "#,
            v_json, j_json, c_json
        );
        let _ = dioxus::document::eval(&script);
    });

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

    let mut tabs_list = vec![
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

    if show_rut {
        tabs_list.push(components::tabs::TabItem {
            value: "rut_exports".to_string(),
            label: "RUT-avdrag".to_string(),
            icon: None,
        });
    }

    tabs_list.push(components::tabs::TabItem {
        value: "live_map".to_string(),
        label: "Livekarta".to_string(),
        icon: None,
    });

    tabs_list.push(components::tabs::TabItem {
        value: "fleet".to_string(),
        label: "Fordonsflotta".to_string(),
        icon: None,
    });

    tabs_list.push(components::tabs::TabItem {
        value: "dispatch".to_string(),
        label: "Resursplanering".to_string(),
        icon: None,
    });

    tabs_list.push(components::tabs::TabItem {
        value: "booking_widget".to_string(),
        label: "Embeddbar Bokning".to_string(),
        icon: None,
    });

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
                                    let toast_c = toast.clone();
                                    let region_c = region.clone();
                                    move |_| {
                                        let uid = uid.clone();
                                        let toast_c = toast_c.clone();
                                        let region_c = region_c.clone();
                                        let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                        spawn(async move {
                                            if let Ok(xml_content) = yntra_core::export_skatteverket_claims(uid, ids, "xml".to_string()).await {
                                                trigger_download(&toast_c, &region_c, &xml_content, "Skatteverket_RUT_Bulk.xml");
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
                                    let toast_c = toast.clone();
                                    let region_c = region.clone();
                                    move |_| {
                                        let uid = uid.clone();
                                        let toast_c = toast_c.clone();
                                        let region_c = region_c.clone();
                                        let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                        spawn(async move {
                                            if let Ok(csv_content) = yntra_core::export_skatteverket_claims(uid, ids, "csv".to_string()).await {
                                                trigger_download(&toast_c, &region_c, &csv_content, "Skatteverket_RUT_Bulk.csv");
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "download", size: "14" }
                                "Exportera CSV"
                            }
                            button {
                                class: "py-1.5 px-3 bg-blue-600 text-white hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                                disabled: selected_rut_invoices.read().is_empty(),
                                onclick: move |_| {
                                    skatteverket_bankid_session.set(None);
                                    skatteverket_submit_result.set(None);
                                    skatteverket_loading.set(false);
                                    show_skatteverket_modal.set(true);
                                },
                                components::LucideIcon { name: "send", size: "14" }
                                "Skicka Direkt"
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
                                                    {
                                                        let (badge_text, badge_style) = match item.status.as_str() {
                                                            "claimed" | "submitted" => ("Inskickad", "bg-blue-500/10 text-blue-500"),
                                                            _ => ("Klar för inskick", "bg-emerald-500/10 text-emerald-500"),
                                                        };
                                                        rsx! {
                                                            span { class: "px-1.5 py-0.5 rounded text-[10px] font-bold {badge_style}",
                                                                "{badge_text}"
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
            } else if *active_status_state.read() == "live_map" {
                {
                    let (center_lat, center_lon, zoom) = match region.as_str() {
                        "US" => (37.7749, -122.4194, 12),
                        "DE" => (52.5200, 13.4050, 12),
                        _ => (59.3293, 18.0686, 12),
                    };

                    let coords_json = serde_json::to_string(&coords).unwrap_or_else(|_| "{}".to_string());
                    let vehicles_json = serde_json::to_string(&vehicles).unwrap_or_else(|_| "[]".to_string());
                    let jobs_json = serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string());

                    let map_html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <style>
        html, body, #map {{
            width: 100%;
            height: 100%;
            margin: 0;
            padding: 0;
            background: #0f172a;
        }}
        .leaflet-popup-content-wrapper {{
            background: rgba(15, 23, 42, 0.95);
            backdrop-filter: blur(8px);
            color: #f8fafc;
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 8px;
            font-family: system-ui, -apple-system, sans-serif;
            box-shadow: 0 10px 25px -5px rgba(0,0,0,0.5);
        }}
        .leaflet-popup-tip {{
            background: rgba(15, 23, 42, 0.95);
        }}
        .vehicle-popup {{
            font-size: 11px;
            line-height: 1.5;
            padding: 4px;
        }}
        .vehicle-popup h4 {{
            margin: 0 0 6px 0;
            font-size: 12px;
            color: #38bdf8;
            font-weight: 700;
            font-family: ui-monospace, SFMono-Regular, monospace;
        }}
        
        /* Minimalist Enterprise Vehicle Pin */
        .enterprise-vehicle-pin {{
            display: flex;
            align-items: center;
            gap: 6px;
            background: #1e293b;
            border: 1.5px solid #334155;
            border-radius: 6px;
            padding: 3px 8px;
            box-shadow: 0 4px 6px -1px rgba(0,0,0,0.3);
            white-space: nowrap;
            box-sizing: border-box;
        }}
        .status-dot {{
            width: 7px;
            height: 7px;
            border-radius: 50%;
            background: #94a3b8;
            transition: background 0.3s ease;
        }}
        .enterprise-vehicle-pin.active .status-dot {{
            background: #10b981;
            box-shadow: 0 0 6px #10b981;
        }}
        .pin-plate {{
            color: #f8fafc;
            font-size: 10px;
            font-weight: 700;
            font-family: ui-monospace, SFMono-Regular, monospace;
            letter-spacing: 0.5px;
        }}

        /* Minimalist Letter Pins for Job Stops */
        .job-stop-pin {{
            display: flex;
            align-items: center;
            justify-content: center;
            width: 20px;
            height: 20px;
            border-radius: 50%;
            font-size: 11px;
            font-weight: 800;
            font-family: system-ui, sans-serif;
            color: #ffffff;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
        }}
        .job-stop-pin.origin {{
            background: #ef4444;
            border: 2px solid #ffffff;
        }}
        .job-stop-pin.destination {{
            background: #3b82f6;
            border: 2px solid #ffffff;
        }}
    </style>
</head>
<body>
    <div id="map"></div>
    <script>
        var map = L.map('map', {{
            zoomControl: false
        }}).setView([{center_lat}, {center_lon}], {zoom});

        L.control.zoom({{
            position: 'bottomright'
        }}).addTo(map);

        L.tileLayer('https://{{s}}.basemaps.cartocdn.com/dark_all/{{z}}/{{x}}/{{y}}{{r}}.png', {{
            attribution: '&copy; OpenStreetMap contributors &copy; CARTO'
        }}).addTo(map);

        var addressCoords = {{
            "Vasagatan 12, Stockholm": [59.3315, 18.0583],
            "Kungsgatan 3, Stockholm": [59.3352, 18.0682],
            "Location St 20": [59.3242, 18.0722],
            "Sveavägen 45, Stockholm": [59.3385, 18.0599],
            "Odengatan 12, Stockholm": [59.3444, 18.0611],
            "Karlavägen 8, Stockholm": [59.3421, 18.0763],
            "Valhallavägen 100, Stockholm": [59.3465, 18.0722]
        }};

        var sfCoords = {{
            "Market St, San Francisco": [37.7891, -122.4014],
            "Mission St, San Francisco": [37.7682, -122.4143],
            "Geary Blvd, San Francisco": [37.7858, -122.4345],
            "Fell St, San Francisco": [37.7760, -122.4284]
        }};

        var berlinCoords = {{
            "Alexanderplatz, Berlin": [52.5219, 13.4132],
            "Friedrichstraße, Berlin": [52.5162, 13.3889],
            "Kurfürstendamm, Berlin": [52.5012, 13.3289],
            "Potsdamer Platz, Berlin": [52.5096, 13.3759]
        }};

        var dynamicCoords = {coords_json};
        var coordsMap = Object.assign({{}}, addressCoords, sfCoords, berlinCoords, dynamicCoords);

        var vehicleMarkers = {{}};
        var routeLines = [];
        var jobMarkers = [];

        var vehicleGroup = L.layerGroup().addTo(map);
        var jobGroup = L.layerGroup().addTo(map);

        function animateMarker(marker, oldLat, oldLng, newLat, newLng, duration) {{
            var start = performance.now();
            function tick(now) {{
                var elapsed = now - start;
                var progress = Math.min(elapsed / duration, 1);
                var lat = oldLat + (newLat - oldLat) * progress;
                var lng = oldLng + (newLng - oldLng) * progress;
                marker.setLatLng([lat, lng]);
                if (progress < 1) {{
                    requestAnimationFrame(tick);
                }}
            }}
            requestAnimationFrame(tick);
        }}

        window.updateMapData = function(vehicles, jobs, newCoords) {{
            if (newCoords) {{
                Object.assign(coordsMap, newCoords);
            }}
            vehicles.forEach(function(v) {{
                if (v.latitude !== null && v.longitude !== null) {{
                    var popupContent = '<div class="vehicle-popup">' +
                        '<h4>' + v.license_plate + ' (' + v.name + ')</h4>' +
                        '<b>Lastkapacitet:</b> ' + v.capacity_m3 + ' m³<br/>' +
                        '<b>Status:</b> ' + v.status + '<br/>' +
                        (v.last_ping ? '<b>Senast spårad:</b> ' + new Date(v.last_ping).toLocaleTimeString() : '') +
                        '</div>';

                    if (vehicleMarkers[v.id]) {{
                        var marker = vehicleMarkers[v.id];
                        var oldLatLng = marker.getLatLng();
                        if (oldLatLng.lat !== v.latitude || oldLatLng.lng !== v.longitude) {{
                            animateMarker(marker, oldLatLng.lat, oldLatLng.lng, v.latitude, v.longitude, 1200);
                        }}
                        marker.setPopupContent(popupContent);
                        
                        // Update active class on element
                        var el = marker.getElement();
                        if (el) {{
                            var pin = el.querySelector('.enterprise-vehicle-pin');
                            if (pin) {{
                                if (v.status === 'active') {{
                                    pin.classList.add('active');
                                }} else {{
                                    pin.classList.remove('active');
                                }}
                            }}
                        }}
                    }} else {{
                        var iconHtml = '<div class="enterprise-vehicle-pin' + (v.status === 'active' ? ' active' : '') + '">' +
                            '<div class="status-dot"></div>' +
                            '<span class="pin-plate">' + v.license_plate + '</span>' +
                            '</div>';

                        var customIcon = L.divIcon({{
                            html: iconHtml,
                            className: 'custom-div-icon',
                            iconSize: [80, 24],
                            iconAnchor: [40, 12]
                        }});

                        var marker = L.marker([v.latitude, v.longitude], {{ icon: customIcon }});
                        marker.bindPopup(popupContent);
                        marker.addTo(vehicleGroup);
                        vehicleMarkers[v.id] = marker;
                    }}
                }}
            }});

            for (var id in vehicleMarkers) {{
                if (!vehicles.find(function(v) {{ return v.id === id; }})) {{
                    vehicleGroup.removeLayer(vehicleMarkers[id]);
                    delete vehicleMarkers[id];
                }}
            }}

            jobMarkers.forEach(function(m) {{ jobGroup.removeLayer(m); }});
            routeLines.forEach(function(l) {{ jobGroup.removeLayer(l); }});
            jobMarkers = [];
            routeLines = [];

            jobs.forEach(function(j) {{
                var origin = coordsMap[j.origin_address] || coordsMap[j.location_address];
                var dest = coordsMap[j.destination_address];

                if (origin) {{
                    var originIcon = L.divIcon({{
                        html: '<div class="job-stop-pin origin">A</div>',
                        className: 'job-pin-container',
                        iconSize: [20, 20],
                        iconAnchor: [10, 10]
                    }});
                    var m = L.marker(origin, {{ icon: originIcon }}).bindPopup('<b>Startpunkt (A):</b> ' + (j.origin_address || j.location_address) + '<br/><b>Jobb:</b> ' + j.title);
                    m.addTo(jobGroup);
                    jobMarkers.push(m);
                }}

                if (dest) {{
                    var destIcon = L.divIcon({{
                        html: '<div class="job-stop-pin destination">B</div>',
                        className: 'job-pin-container',
                        iconSize: [20, 20],
                        iconAnchor: [10, 10]
                    }});
                    var m = L.marker(dest, {{ icon: destIcon }}).bindPopup('<b>Slutdestination (B):</b> ' + j.destination_address + '<br/><b>Jobb:</b> ' + j.title);
                    m.addTo(jobGroup);
                    jobMarkers.push(m);
                }}

                if (origin && dest) {{
                    var line = L.polyline([origin, dest], {{
                        color: '#6366f1',
                        weight: 3,
                        opacity: 0.8
                    }}).bindPopup('<b>Planerad Rutt:</b> ' + j.title);
                    line.addTo(jobGroup);
                    routeLines.push(line);
                }}
            }});
        }};

        var initialVehicles = {vehicles_json};
        var initialJobs = {jobs_json};
        window.updateMapData(initialVehicles, initialJobs);

        var bounds = L.latLngBounds();
        var hasBounds = false;
        for (var id in vehicleMarkers) {{
            bounds.extend(vehicleMarkers[id].getLatLng());
            hasBounds = true;
        }}
        jobMarkers.forEach(function(m) {{
            bounds.extend(m.getLatLng());
            hasBounds = true;
        }});
        if (hasBounds) {{
            map.fitBounds(bounds, {{ padding: [60, 60] }});
        }}
    </script>
</body>
</html>
"#, center_lat=center_lat, center_lon=center_lon, zoom=zoom, vehicles_json=vehicles_json, jobs_json=jobs_json, coords_json=coords_json);

                    rsx! {
                        div { class: "relative w-full h-[620px] rounded-3xl border border-border overflow-hidden bg-background shadow-2xl animate-in fade-in zoom-in duration-500",
                            // 1. Fullscreen map iframe
                            iframe {
                                srcdoc: "{map_html}",
                                style: "width: 100%; height: 100%; border: none; display: block;",
                                class: "bg-slate-900"
                            }

                            // 2. Glassmorphic Sidebar Overlay (Left)
                            div { 
                                style: "position: absolute; top: 16px; left: 16px; z-index: 1000; width: 290px; max-height: calc(100% - 32px); display: flex; flex-direction: column; background: rgba(15, 23, 42, 0.82); backdrop-filter: blur(16px) saturate(180%); -webkit-backdrop-filter: blur(16px) saturate(180%); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 20px; box-shadow: 0 20px 40px -10px rgba(0,0,0,0.7); box-sizing: border-box; overflow: hidden; padding: 1.25rem;",
                                div { class: "flex items-center gap-2.5 pb-3 border-b border-white/10",
                                    div { class: "rounded-xl bg-primary/20 p-2 text-primary",
                                        components::LucideIcon { name: "truck", class: "h-5 w-5 animate-pulse" }
                                    }
                                    div {
                                        h4 { class: "text-sm font-bold text-white m-0", "Fordonsspårning" }
                                        p { class: "text-[10px] text-slate-400 m-0", "Realtidsstatus för flyttbilar" }
                                    }
                                }

                                div { class: "flex-1 overflow-y-auto space-y-2 mt-3 pr-1 scrollbar-thin",
                                    if vehicles.is_empty() {
                                        p { class: "text-xs text-slate-500 italic text-center py-8 m-0", "Inga fordon tillgängliga" }
                                    } else {
                                        for v in vehicles.iter() {
                                            div {
                                                class: "p-2.5 rounded-xl bg-white/5 border border-white/5 flex items-center gap-3 transition-all hover:bg-white/10",
                                                div {
                                                    class: format!("w-2.5 h-2.5 rounded-full flex-shrink-0 {}", if v.status == "active" { "bg-emerald-400 shadow-[0_0_8px_#34d399] animate-pulse" } else { "bg-slate-500" })
                                                }
                                                div { class: "flex-1 min-w-0",
                                                    p { class: "text-xs font-bold text-white m-0 truncate", "{v.name}" }
                                                    p { class: "text-[9px] text-slate-400 m-0 flex items-center gap-1 mt-0.5",
                                                        components::LucideIcon { name: "credit-card", size: "9" }
                                                        "{v.license_plate} • {v.capacity_m3}m³"
                                                    }
                                                }
                                                if let (Some(lat), Some(lng)) = (v.latitude, v.longitude) {
                                                    div { class: "text-[8px] font-mono text-emerald-400 text-right flex-shrink-0 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/10",
                                                        div { "{lat:.4}°N" }
                                                        div { "{lng:.4}°E" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 3. Glassmorphic GPS Integration Panel (Right)
                            div {
                                style: "position: absolute; top: 16px; right: 16px; z-index: 1000; background: rgba(15, 23, 42, 0.82); backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 16px; box-shadow: 0 15px 30px -8px rgba(0,0,0,0.6); padding: 0.75rem 1rem; display: flex; flex-direction: column; gap: 0.5rem; max-width: 280px;",
                                div { class: "flex items-center gap-2.5",
                                    div { class: "rounded-lg bg-emerald-500/20 p-1.5 text-emerald-400",
                                        components::LucideIcon { name: "rss", class: "h-4 w-4" }
                                    }
                                    div {
                                        h5 { class: "text-xs font-bold text-white m-0", "GPS Gateway Status" }
                                        p { class: "text-[9px] text-emerald-400 font-semibold m-0 flex items-center gap-1",
                                            span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" }
                                            "Aktiv (Traccar / REST API)"
                                        }
                                    }
                                }
                                div { class: "border-t border-white/10 pt-2 text-[9px] text-slate-300 space-y-1 flex flex-col gap-1.5",
                                    p { class: "m-0", "Mottagare: " span { class: "font-mono text-white/80", "https://api.yntra.se/v1/gps/ping" } }
                                    p { class: "m-0 text-slate-400", "Enheter identifieras via IMEI / GPS Tracker ID." }
                                    button {
                                        class: format!("mt-1 py-1.5 px-3 rounded text-[10px] font-bold border-0 cursor-pointer transition-all {}",
                                            if *is_simulating.read() { "bg-rose-500/20 text-rose-400 hover:bg-rose-500/30" } else { "bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30" }
                                        ),
                                        onclick: move |_| {
                                            let current = *is_simulating.read();
                                            is_simulating.set(!current);
                                        },
                                        if *is_simulating.read() { "Stoppa GPS-simulering" } else { "Starta GPS-simulering" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if *active_status_state.read() == "fleet" {
                {
                    let active_uid_c = props.active_user_id.read().clone();
                    rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-3 gap-6 w-full animate-in fade-in zoom-in duration-300",
                            // Left/Main Card: Fordonslista
                            div { class: "md:col-span-2",
                                components::Card {
                                    class: "border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                                    components::CardHeader {
                                        class: "pb-3 border-b border-border/40",
                                        div { class: "flex items-center gap-3",
                                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                                components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                            }
                                            div {
                                                components::CardTitle { class: "text-lg font-bold", "Registrerade Fordon" }
                                                components::CardDescription { class: "text-xs", "Aktuell fordonsflotta och lastkapacitet." }
                                            }
                                        }
                                    }
                                    components::CardContent {
                                        class: "pt-4 flex flex-1 flex-col",
                                        if vehicles.is_empty() {
                                            div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10 my-auto",
                                                components::LucideIcon { name: "truck", class: "h-8 w-8 opacity-20 mb-2" }
                                                p { class: "text-sm font-semibold m-0", "Inga fordon registrerade." }
                                            }
                                        } else {
                                            div { class: "space-y-3",
                                                for vehicle in vehicles.iter() {
                                                    {
                                                        let v_id = vehicle.id.clone();
                                                        let v_name = vehicle.name.clone();
                                                        let v_plate = vehicle.license_plate.clone();
                                                        let v_capacity = vehicle.capacity_m3;
                                                        let v_status = vehicle.status.clone();
                                                        let uid_del = active_uid_c.clone();
                                                        rsx! {
                                                            div {
                                                                key: "{v_id}",
                                                                class: "flex items-center justify-between p-3.5 rounded-xl border border-border/20 bg-background/50 hover:bg-secondary/20 transition-all",
                                                                div { class: "flex items-center gap-3",
                                                                    div { class: "flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary",
                                                                        components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                                                    }
                                                                    div {
                                                                        div { class: "text-sm font-bold text-foreground", "{v_name}" }
                                                                        div { class: "text-xs text-muted-foreground flex items-center gap-1.5 mt-0.5",
                                                                            span { class: "font-semibold bg-muted px-1.5 py-0.5 rounded text-[10px]", "{v_plate}" }
                                                                            span { "•" }
                                                                            span { "Kapacitet: {v_capacity} m³" }
                                                                        }
                                                                    }
                                                                }
                                                                div { class: "flex items-center gap-3",
                                                                    span {
                                                                        class: "px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-500",
                                                                        "{v_status}"
                                                                    }
                                                                    button {
                                                                        onclick: move |_| {
                                                                            let vehicle_id = v_id.clone();
                                                                            let uid = uid_del.clone();
                                                                            let mut db_trigger = db_trigger;
                                                                            spawn(async move {
                                                                                if yntra_core::delete_vehicle(uid, vehicle_id).await.is_ok() {
                                                                                    let current = *db_trigger.read();
                                                                                    db_trigger.set(current + 1);
                                                                                }
                                                                            });
                                                                        },
                                                                        class: "p-2 rounded-lg text-muted-foreground hover:text-red-500 hover:bg-red-500/10 border-0 bg-transparent cursor-pointer transition-all flex items-center justify-center",
                                                                        title: "Ta bort fordon",
                                                                        components::LucideIcon { name: "trash-2", size: "16" }
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
                            // Right Card: Registrera Fordon Form
                            div { class: "col-span-1",
                                components::Card {
                                    class: "border border-border bg-sidebar p-1 shadow-md h-full flex flex-col",
                                    components::CardHeader {
                                        class: "pb-3 border-b border-border/40",
                                        div { class: "flex items-center gap-3",
                                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                                components::LucideIcon { name: "plus", class: "h-5 w-5" }
                                            }
                                            div {
                                                components::CardTitle { class: "text-lg font-bold", "Registrera Fordon" }
                                                components::CardDescription { class: "text-xs", "Lägg till ett nytt fordon i flottan." }
                                            }
                                        }
                                    }
                                    components::CardContent {
                                        class: "pt-6 space-y-4 flex-1 flex flex-col justify-between",
                                        div { class: "space-y-4",
                                            div {
                                                label { class: "text-xs font-bold text-muted-foreground block mb-1", "Fordonsnamn / Modell" }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "t.ex. Volvo FL6, Ford Transit",
                                                    value: "{new_vehicle_name}",
                                                    oninput: move |e| new_vehicle_name.set(e.value()),
                                                    class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                }
                                            }
                                            div {
                                                label { class: "text-xs font-bold text-muted-foreground block mb-1", "Registreringsnummer" }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "t.ex. ABC-123",
                                                    value: "{new_vehicle_plate}",
                                                    oninput: move |e| new_vehicle_plate.set(e.value()),
                                                    class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                }
                                            }
                                            div {
                                                label { class: "text-xs font-bold text-muted-foreground block mb-1", "Kapacitet (m³)" }
                                                input {
                                                    r#type: "number",
                                                    step: "0.5",
                                                    min: "0.1",
                                                    value: "{new_vehicle_capacity}",
                                                    oninput: move |e| new_vehicle_capacity.set(e.value().parse().unwrap_or(15.0)),
                                                    class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                }
                                            }
                                            div {
                                                label { class: "text-xs font-bold text-muted-foreground block mb-1", "GPS Tracker ID (IMEI)" }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "t.ex. 35241509...",
                                                    value: "{new_vehicle_gps_id}",
                                                    oninput: move |e| new_vehicle_gps_id.set(e.value()),
                                                    class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                }
                                            }
                                        }
                                        button {
                                            onclick: {
                                                let uid = active_uid_c.clone();
                                                let mut db_trigger = db_trigger;
                                                move |_| {
                                                    let name = new_vehicle_name.read().trim().to_string();
                                                    let plate = new_vehicle_plate.read().trim().to_string();
                                                    let cap = *new_vehicle_capacity.read();
                                                    let gps_id = new_vehicle_gps_id.read().trim().to_string();
                                                    if name.is_empty() || plate.is_empty() {
                                                        return;
                                                    }
                                                    let uid = uid.clone();
                                                    
                                                    // Reset inputs
                                                    new_vehicle_name.set(String::new());
                                                    new_vehicle_plate.set(String::new());
                                                    new_vehicle_capacity.set(15.0);
                                                    new_vehicle_gps_id.set(String::new());
                                                    
                                                    let gps_opt = if gps_id.is_empty() { None } else { Some(gps_id) };
                                                    
                                                    spawn(async move {
                                                        if yntra_core::create_vehicle(uid, name, plate, cap, gps_opt).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            class: "w-full rounded-lg bg-primary py-2.5 text-xs font-bold text-primary-foreground shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center justify-center gap-2 mt-4",
                                            components::LucideIcon { name: "check", class: "h-4 w-4" }
                                            "Registrera Fordon"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if *active_status_state.read() == "dispatch" {
                dispatch::DispatchBoard {
                    active_user_id: props.active_user_id.read().clone(),
                    region: region.clone(),
                    db_trigger: db_trigger,
                    vehicles: vehicles.clone(),
                    jobs: jobs.clone(),
                }
            } else if *active_status_state.read() == "booking_widget" {
                public_widget::PublicBookingPreview {
                    workspace_id: workspace_id.clone(),
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

        // Skatteverket Direct Submission Modal
        if *show_skatteverket_modal.read() {
            {
                let selected_ids_list: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                let selected_amount_sum: f64 = list.iter()
                    .filter(|item| selected_ids_list.contains(&item.invoice_id))
                    .map(|item| item.rut_amount)
                    .sum();
                    
                rsx! {
                    div {
                        class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200",
                        div {
                            class: "w-full max-w-md rounded-2xl border border-border bg-sidebar p-6 shadow-2xl animate-in zoom-in-95 duration-200 flex flex-col gap-4 text-foreground",
                            
                            // Header
                            div { class: "flex items-center justify-between border-b border-border/40 pb-3",
                                div { class: "flex items-center gap-2",
                                    div { class: "rounded-lg bg-blue-500/10 p-2 text-blue-500",
                                        components::LucideIcon { name: "send", class: "h-5 w-5" }
                                    }
                                    div {
                                        h3 { class: "text-sm font-extrabold m-0", "Direktsändning Skatteverket" }
                                        p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", "Elektronisk insändning via BankID" }
                                    }
                                }
                                button {
                                    onclick: move |_| show_skatteverket_modal.set(false),
                                    class: "p-1 rounded-lg text-muted-foreground hover:bg-muted hover:text-foreground border-0 bg-transparent cursor-pointer transition-colors",
                                    components::LucideIcon { name: "x", class: "h-4 w-4" }
                                }
                            }

                            if let Some(ref result) = *skatteverket_submit_result.read() {
                                // Result View
                                div { class: "flex flex-col gap-4 py-3 text-center",
                                    if result.status == "accepted" {
                                        div { class: "mx-auto rounded-full bg-emerald-500/10 p-3 text-emerald-500 w-fit",
                                            components::LucideIcon { name: "check-circle", class: "h-8 w-8" }
                                        }
                                        h4 { class: "text-sm font-extrabold text-foreground m-0", "Sändning Godkänd" }
                                        p { class: "text-xs text-muted-foreground m-0 px-2", "{result.message}" }
                                        
                                        div { class: "mt-2 p-3 rounded-lg bg-background/50 border border-border/20 text-left text-xs space-y-1.5",
                                            div { class: "flex justify-between",
                                                span { class: "text-muted-foreground", "Referensnummer:" }
                                                span { class: "font-mono font-bold text-foreground", "{result.reference_number}" }
                                            }
                                            div { class: "flex justify-between",
                                                span { class: "text-muted-foreground", "Antal ärenden:" }
                                                span { class: "font-bold text-foreground", "{result.total_claims} st" }
                                            }
                                            div { class: "flex justify-between",
                                                span { class: "text-muted-foreground", "Totalt belopp:" }
                                                span { class: "font-bold text-emerald-500", "{result.total_amount} kr" }
                                            }
                                        }
                                    } else {
                                        div { class: "mx-auto rounded-full bg-red-500/10 p-3 text-red-500 w-fit",
                                            components::LucideIcon { name: "alert-triangle", class: "h-8 w-8" }
                                        }
                                        h4 { class: "text-sm font-extrabold text-foreground m-0", "Sändning Misslyckades" }
                                        p { class: "text-xs text-red-500 m-0 px-2", "{result.message}" }
                                    }
                                    
                                    button {
                                        onclick: move |_| {
                                            show_skatteverket_modal.set(false);
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        },
                                        class: "w-full py-2 bg-primary text-primary-foreground hover:opacity-90 rounded-lg text-xs font-bold border-0 cursor-pointer transition-all mt-2",
                                        "Klar"
                                    }
                                }
                            } else if let Some(ref session) = *skatteverket_bankid_session.read() {
                                // Loading/Signing View
                                div { class: "flex flex-col items-center justify-center gap-4 py-6 text-center select-none",
                                    div { class: "relative flex items-center justify-center w-24 h-24 rounded-2xl bg-white p-2 border border-border/30",
                                        img {
                                            src: "https://www.bankid.com/assets/bankid-logo.svg",
                                            class: "w-16 h-16 animate-pulse"
                                        }
                                    }
                                    
                                    div { class: "space-y-1",
                                        h4 { class: "text-xs font-bold text-foreground m-0 animate-pulse", "Öppna BankID-säkerhetsapp" }
                                        p { class: "text-[10px] text-muted-foreground m-0", "Signerar Begäran om RUT-avdrag till Skatteverket." }
                                    }

                                    // Progress Bar simulation
                                    div { class: "w-full bg-muted rounded-full h-1.5 mt-2 overflow-hidden",
                                        div {
                                            class: "bg-blue-500 h-1.5 rounded-full transition-all duration-300",
                                            style: "width: {session.progress}%"
                                        }
                                    }
                                    
                                    p { class: "text-[10px] font-mono text-muted-foreground", "Svarar från BankID..." }
                                }
                            } else {
                                // Initial BankID Prompt
                                div { class: "flex flex-col gap-4 py-2",
                                    div { class: "p-3 rounded-lg bg-background/50 border border-border/20 text-xs space-y-1.5",
                                        div { class: "flex justify-between",
                                            span { class: "text-muted-foreground", "Valda fakturor:" }
                                            span { class: "font-bold text-foreground", "{selected_ids_list.len()} st" }
                                        }
                                        div { class: "flex justify-between",
                                            span { class: "text-muted-foreground", "Totalt belopp att ansöka:" }
                                            span { class: "font-bold text-emerald-500", "{selected_amount_sum} kr" }
                                        }
                                    }
                                    
                                    if !has_skatteverket_cert {
                                        div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-3 text-xs text-red-500 flex gap-2 items-start",
                                            components::LucideIcon { name: "alert-octagon", class: "h-4 w-4 shrink-0 mt-0.5" }
                                            span {
                                                "Error: Skatteverket corporate certificate is missing in Workspace Settings. Submissions will fail."
                                            }
                                        }
                                    }
                                    
                                    div { class: "space-y-1.5",
                                        label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Signerande Personnummer" }
                                        input {
                                            r#type: "text",
                                            placeholder: "YYYYMMDD-XXXX",
                                            value: "{bankid_personal_number}",
                                            oninput: move |e: FormEvent| bankid_personal_number.set(e.value()),
                                            class: "w-full text-xs p-2 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary"
                                        }
                                    }

                                    button {
                                        disabled: *skatteverket_loading.read() || bankid_personal_number.read().trim().is_empty(),
                                        onclick: move |_| {
                                            skatteverket_loading.set(true);
                                            let uid = props.active_user_id.read().clone();
                                            let ids_c = selected_ids_list.clone();
                                            
                                            spawn(async move {
                                                if let Ok(session) = initiate_bankid_skatteverket_session(uid.clone()).await {
                                                    skatteverket_bankid_session.set(Some(session.clone()));
                                                    
                                                    // Simulate BankID progress ticking
                                                    let mut prog = 0.0;
                                                    for _ in 1..=4 {
                                                        crate::utils::sleep_ms(700).await;
                                                        prog += 25.0;
                                                        let mut updated = session.clone();
                                                        updated.progress = prog;
                                                        skatteverket_bankid_session.set(Some(updated));
                                                    }
                                                    
                                                    // Execute final Skatteverket Direct API Submission
                                                    match submit_skatteverket_claim_direct(uid, session.id, ids_c).await {
                                                        Ok(res) => {
                                                            skatteverket_submit_result.set(Some(res));
                                                        }
                                                        Err(e) => {
                                                            skatteverket_submit_result.set(Some(SkatteverketSubmitResult {
                                                                reference_number: "".to_string(),
                                                                total_claims: 0,
                                                                total_amount: 0.0,
                                                                status: "failed".to_string(),
                                                                message: e.to_string(),
                                                            }));
                                                        }
                                                    }
                                                    skatteverket_bankid_session.set(None);
                                                    skatteverket_loading.set(false);
                                                } else {
                                                    skatteverket_loading.set(false);
                                                }
                                            });
                                        },
                                        class: "w-full py-2.5 bg-blue-600 hover:opacity-90 rounded-lg text-xs font-bold text-white border-0 cursor-pointer flex items-center justify-center gap-1.5 transition-all disabled:opacity-50",
                                        components::LucideIcon { name: "send", size: "14" }
                                        "Starta BankID-signering & Skicka"
                                    }
                                }
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
