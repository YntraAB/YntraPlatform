#![allow(unused_imports)]
use super::ChecklistItem;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    JobTicket, MoveInventoryItem, MoveQuote, MoveVehicle, WorkspaceUser,
    save_job_signature, get_job_signature,
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
    let toast = dioxus_primitives::toast::use_toast();
    let active_user_id = props.active_user_id;
    let uid_for_save = active_user_id.clone();
    let uid_for_calc = active_user_id.clone();
    let uid_for_delete = active_user_id.clone();
    let region = props.region;
    let mut checklist_state = props.checklist_state;
    let mut completion_report_state = props.completion_report_state;
    let inventories = props.inventories;
    let quote = props.quote;
    let db_trigger = props.db_trigger;

    let state = use_context::<crate::state::AppState>();
    let active_role = state.active_user_role.read().clone();
    let is_staff = active_role != "client" && active_role != "anonymous";

    let mut show_bol_modal = use_signal(|| false);
    let mut show_condition_modal = use_signal(|| false);
    let mut show_pos_modal = use_signal(|| false);
    let mut show_sit_modal = use_signal(|| false);
    let mut show_payroll_modal = use_signal(|| false);
    let mut show_erp_modal = use_signal(|| false);
    let mut show_live_tracking_modal = use_signal(|| false);
    let mut show_field_crew_view = use_signal(|| false);
    let mut show_eld_modal = use_signal(|| false);
    let mut show_dispatch_alerts_modal = use_signal(|| false);
    let mut show_hvac_modal = use_signal(|| false);
    let mut show_edit_surcharges = use_signal(|| false);
    let mut edit_long_carry = use_signal(|| job.long_carry_meters);
    let mut edit_toll_fees = use_signal(|| job.toll_fees);

    let mut show_adjust_invoice = use_signal(|| false);
    let mut actual_hours_input = use_signal(|| String::new());
    let mut additional_charges_input = use_signal(|| String::new());
    let mut adjustment_notes_input = use_signal(|| String::new());

    let mut show_add_pack_form = use_signal(|| false);
    let mut new_pack_preset = use_signal(|| "Flyttkartong".to_string());
    let mut new_pack_name = use_signal(|| "Flyttkartong".to_string());
    let mut new_pack_qty = use_signal(|| 10);
    let mut new_pack_price = use_signal(|| 30.0);
    let mut new_pack_leased = use_signal(|| true);

    let uid_for_pack = active_user_id.clone();
    let jid_for_pack = job.id.clone();
    let db_trig_val = *props.db_trigger.read();
    let packaging_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_pack.clone();
        let jid = jid_for_pack.clone();
        async move {
            yntra_core::get_job_packaging_items(uid, jid).await.unwrap_or_default()
        }
    });

    let workspace_opt = state.workspace.read().clone();
    let modules_val: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.modules_active).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let is_hvac_plumbing = modules_val
        .get("hvac_plumbing")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let show_rut = settings_json
        .get("show_rut_deduction")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| region == "SE");
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

    // Crew & Vehicle resources
    let jid_for_crew = job.id.clone();
    let uid_for_crew = active_user_id.clone();
    let crew_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_crew.clone();
        let jid = jid_for_crew.clone();
        async move {
            if jid.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_job_crew(uid, jid).await.unwrap_or_default()
            }
        }
    });
    let crew_list = crew_res.read().clone().unwrap_or_default();

    let uid_for_users = active_user_id.clone();
    let users_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_users.clone();
        async move {
            yntra_core::get_users(uid).await.unwrap_or_default()
        }
    });
    let workspace_users = users_res.read().clone().unwrap_or_default();

    let uid_for_vehicles = active_user_id.clone();
    let vehicles_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_vehicles.clone();
        async move {
            yntra_core::get_vehicles(uid).await.unwrap_or_default()
        }
    });
    let vehicles_list = vehicles_res.read().clone().unwrap_or_default();

    let jid_for_sig = job.id.clone();
    let uid_for_sig = active_user_id.clone();
    let signature_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_sig.clone();
        let jid = jid_for_sig.clone();
        async move {
            if jid.is_empty() {
                None
            } else {
                yntra_core::get_job_signature(uid, jid).await.unwrap_or(None)
            }
        }
    });
    let signature_opt = signature_res.read().clone().flatten();

    // Form inputs for signature
    let mut signer_name = use_signal(String::new);

    // The signature pad event listeners are managed inside the SignatureCanvas subcomponent
    // to prevent canvas resets during parent re-renders.

    let mut selected_crew_user_id = use_signal(|| "999".to_string());
    let mut new_crew_role = use_signal(|| "Bärare".to_string());

    let mut show_override_form = use_signal(|| false);
    let mut manual_override_input = use_signal(|| {
        quote.as_ref().and_then(|q| q.manual_price_override).map(|v| v.to_string()).unwrap_or_default()
    });
    let mut discount_input = use_signal(|| {
        quote.as_ref().and_then(|q| q.price_discount).map(|v| v.to_string()).unwrap_or_default()
    });
    use_effect({
        let quote = quote.clone();
        move || {
            if let Some(ref q) = quote {
                manual_override_input.set(q.manual_price_override.map(|v| v.to_string()).unwrap_or_default());
                discount_input.set(q.price_discount.map(|v| v.to_string()).unwrap_or_default());
            } else {
                manual_override_input.set("".to_string());
                discount_input.set("".to_string());
            }
        }
    });

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
    let surcharge_piano = settings_json.get("surcharge_piano").and_then(|v| v.as_f64()).unwrap_or(1500.0);
    let surcharge_safe = settings_json.get("surcharge_safe").and_then(|v| v.as_f64()).unwrap_or(2000.0);
    let surcharge_jacuzzi = settings_json.get("surcharge_jacuzzi").and_then(|v| v.as_f64()).unwrap_or(2500.0);
    let surcharge_fragile = settings_json.get("surcharge_fragile").and_then(|v| v.as_f64()).unwrap_or(500.0);
    let specialty_surcharge: f64 = inventories.iter().map(|item| {
        let item_fee = yntra_core::calculate_item_specialty_surcharge(
            item.item_category.clone(),
            item.item_name.clone(),
            item.handling_notes.clone(),
            surcharge_piano,
            surcharge_safe,
            surcharge_jacuzzi,
            surcharge_fragile,
        );
        item_fee * item.quantity as f64
    }).sum();
    let total_vol: f64 = inventories
        .iter()
        .map(|i| i.estimated_volume_m3 * i.quantity as f64)
        .sum();
    let total_weight: f64 = inventories
        .iter()
        .map(|i| i.estimated_weight_kg * i.quantity as f64)
        .sum();
    let recommended_crew_size = if total_vol > 35.0 || total_weight > 800.0 {
        4
    } else if total_vol > 15.0 || total_weight > 350.0 {
        3
    } else {
        2
    };

    let hours = quote.as_ref().map(|q| {
        let labor_base = (q.base_price as f64 - specialty_surcharge).max(0.0);
        labor_base / hourly_rate
    }).unwrap_or(0.0);
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

    let has_specialty_item = inventories.iter().any(|item| {
        yntra_core::calculate_item_specialty_surcharge(
            item.item_category.clone(),
            item.item_name.clone(),
            item.handling_notes.clone(),
            surcharge_piano,
            surcharge_safe,
            surcharge_jacuzzi,
            surcharge_fragile,
        ) > 0.0
    });

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
                    button {
                        class: "ml-auto px-3 py-1 text-xs font-semibold rounded bg-emerald-600/10 text-emerald-400 hover:bg-emerald-600/20 border border-emerald-500/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_bol_modal.set(true),
                        components::LucideIcon { name: "file-text", size: "14" }
                        "Bill of Lading (BOL)"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-amber-600/10 text-amber-400 hover:bg-amber-600/20 border border-amber-500/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_condition_modal.set(true),
                        components::LucideIcon { name: "clipboard-check", size: "14" }
                        "Condition & Pre-Move Waiver"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-primary/10 text-primary hover:bg-primary/20 border border-primary/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_sit_modal.set(true),
                        components::LucideIcon { name: "warehouse", size: "14" }
                        "SIT & Lagermagasin"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-emerald-600/10 text-emerald-400 hover:bg-emerald-600/20 border border-emerald-500/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_payroll_modal.set(true),
                        components::LucideIcon { name: "coins", size: "14" }
                        "Tips & Förarlön"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-primary/10 text-primary hover:bg-primary/20 border border-primary/20 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_erp_modal.set(true),
                        components::LucideIcon { name: "refresh-cw", size: "14" }
                        "Bokföring & ERP"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-primary/10 text-primary hover:bg-primary/20 border border-primary/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_live_tracking_modal.set(true),
                        components::LucideIcon { name: "navigation", size: "14" }
                        "Live GPS Spårning"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-bold rounded bg-amber-500/20 text-amber-300 hover:bg-amber-500/30 border border-amber-500/50 cursor-pointer flex items-center gap-1.5 transition-colors shadow-xs",
                        onclick: move |_| show_field_crew_view.set(true),
                        components::LucideIcon { name: "smartphone", size: "14" }
                        "Fältläge (Touch-UI)"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-primary/10 text-primary hover:bg-primary/20 border border-primary/20 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_eld_modal.set(true),
                        components::LucideIcon { name: "truck", size: "14" }
                        "ELD & DOT Efterlevnad"
                    }
                    button {
                        class: "px-3 py-1 text-xs font-semibold rounded bg-primary/10 text-primary hover:bg-primary/20 border border-primary/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                        onclick: move |_| show_dispatch_alerts_modal.set(true),
                        components::LucideIcon { name: "message-square", size: "14" }
                        "Dispatch SMS & Omdömen"
                    }
                    if is_hvac_plumbing {
                        button {
                            class: "px-3 py-1 text-xs font-semibold rounded bg-teal-600/10 text-teal-400 hover:bg-teal-600/20 border border-teal-500/30 cursor-pointer flex items-center gap-1.5 transition-colors",
                            onclick: move |_| show_hvac_modal.set(true),
                            components::LucideIcon { name: "zap", size: "14" }
                            "{t(\"hvac-btn-title\", &region)}"
                        }
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

                // Carry & Tolls Info Panel
                div { class: "mt-1 pt-2 border-t border-border/20 flex flex-wrap gap-x-4 gap-y-1.5 text-xs text-muted-foreground",
                    div { class: "flex items-center gap-1",
                        components::LucideIcon { name: "footprints", size: "12", class: "accent-text" }
                        span { class: "font-semibold text-foreground", "Bärlängd:" }
                        span { "{job.long_carry_meters} m" }
                    }
                    div { class: "flex items-center gap-1",
                        components::LucideIcon { name: "coins", size: "12", class: "accent-text" }
                        span { class: "font-semibold text-foreground", "Vägavgifter:" }
                        span { "{job.toll_fees} kr" }
                    }
                    if is_staff {
                        button {
                            class: "ml-auto text-[10px] text-accent font-bold hover:underline bg-transparent border-0 cursor-pointer p-0",
                            onclick: move |_| {
                                edit_long_carry.set(job.long_carry_meters);
                                edit_toll_fees.set(job.toll_fees);
                                show_edit_surcharges.set(true);
                            },
                            "Ändra surcharges"
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

            // Crew & Vehicle Planning Section
            div { class: "border-t border-border/40 pt-4 flex flex-col gap-4 animate-in fade-in duration-300",
                h3 { class: "text-sm font-extrabold flex items-center gap-1.5 m-0",
                    components::LucideIcon { name: "users", size: "16", class: "accent-text" }
                    "Bemannings- & Fordonsplanering"
                }

                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 p-4 rounded-xl border border-border/20 bg-secondary/5",
                    
                    // Vehicle Assignment Column
                    div { class: "space-y-3",
                        label { class: "text-[11px] font-bold text-muted-foreground uppercase tracking-wider block", "Tilldelat Fordon" }
                        
                        // Select element for assigning vehicle
                        select {
                            value: if let Some(ref vid) = job.assigned_vehicle_id { vid.clone() } else { "none".to_string() },
                            onchange: {
                                let jid = job.id.clone();
                                let uid = active_user_id.clone();
                                let mut db_trigger = props.db_trigger;
                                move |e: FormEvent| {
                                    let jid = jid.clone();
                                    let uid = uid.clone();
                                    let selected_vid = e.value();
                                    let arg_vid = if selected_vid == "none" { None } else { Some(selected_vid) };
                                    spawn(async move {
                                        if yntra_core::assign_vehicle_to_job(uid, jid, arg_vid).await.is_ok() {
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        }
                                    });
                                }
                            },
                            class: "w-full text-xs p-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                            option { value: "none", "Inget fordon tilldelat" }
                            for veh in vehicles_list.iter() {
                                option { value: "{veh.id}", "{veh.name} ({veh.license_plate}) • {veh.capacity_m3}m³" }
                            }
                        }
                    }

                    // Crew Assignment Column
                    div { class: "space-y-3",
                        label { class: "text-[11px] font-bold text-muted-foreground uppercase tracking-wider block", "Lägg till bemanning" }
                        
                        div { class: "flex flex-col gap-2",
                            // Dropdown of users
                            select {
                                value: "{selected_crew_user_id}",
                                onchange: move |e: FormEvent| {
                                    selected_crew_user_id.set(e.value());
                                },
                                class: "w-full text-xs p-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                option { value: "999", disabled: true, "Välj medarbetare..." }
                                for user in workspace_users.iter() {
                                    if !crew_list.iter().any(|c| c.id == user.id) {
                                        option { value: "{user.id}", "{user.full_name.clone().unwrap_or_else(|| user.email.clone())} ({user.role})" }
                                    }
                                }
                            }

                            // Role selection
                            div { class: "flex gap-2",
                                select {
                                    value: "{new_crew_role}",
                                    onchange: move |e: FormEvent| {
                                        new_crew_role.set(e.value());
                                    },
                                    class: "flex-1 text-xs p-2 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                    option { value: "Bärare", "Bärare (Packer)" }
                                    option { value: "Förare", "Förare (Driver)" }
                                    option { value: "Arbetsledare", "Arbetsledare (Supervisor)" }
                                }
                                button {
                                    class: "py-2 px-4 bg-primary hover:opacity-90 rounded-lg text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 transition-all disabled:opacity-50",
                                    disabled: *selected_crew_user_id.read() == "999",
                                    onclick: {
                                        let jid = job.id.clone();
                                        let uid = active_user_id.clone();
                                        let mut db_trigger = props.db_trigger;
                                        move |_| {
                                            let selected_uid = selected_crew_user_id.read().clone();
                                            if selected_uid == "999" { return; }
                                            let role = new_crew_role.read().clone();
                                            let jid = jid.clone();
                                            let uid = uid.clone();
                                            
                                            // Reset dropdown
                                            selected_crew_user_id.set("999".to_string());
                                            
                                            spawn(async move {
                                                if yntra_core::add_crew_member(uid, jid, selected_uid, role).await.is_ok() {
                                                    let current = *db_trigger.read();
                                                    db_trigger.set(current + 1);
                                                }
                                            });
                                        }
                                    },
                                    components::LucideIcon { name: "plus", size: "14" }
                                    "Lägg till"
                                }
                            }
                        }
                    }
                }

                // Assigned Crew Members List
                if !crew_list.is_empty() {
                    div { class: "space-y-2 pl-2 border-l-2 border-dashed border-border/60",
                        label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "Tilldelad Bemanning" }
                        div { class: "grid grid-cols-1 sm:grid-cols-2 gap-2",
                            for member in crew_list.iter() {
                                {
                                    let m_id = member.id.clone();
                                    let m_name = member.full_name.clone().unwrap_or_else(|| member.email.clone());
                                    let m_email = member.email.clone();
                                    let m_role = member.role.clone();
                                    let jid = job.id.clone();
                                    let uid = active_user_id.clone();
                                    let mut db_trigger = props.db_trigger;
                                    rsx! {
                                        div {
                                            key: "{m_id}",
                                            class: "flex items-center justify-between p-2.5 rounded-lg border border-border/10 bg-background/50",
                                            div { class: "min-w-0 flex-1",
                                                div { class: "text-xs font-bold text-foreground truncate", "{m_name}" }
                                                div { class: "text-[10px] text-muted-foreground flex items-center gap-1 truncate mt-0.5",
                                                    span { class: "text-primary font-semibold", "{m_role}" }
                                                    span { "•" }
                                                    span { "{m_email}" }
                                                }
                                            }
                                            button {
                                                onclick: move |_| {
                                                    let member_id = m_id.clone();
                                                    let jid = jid.clone();
                                                    let uid = uid.clone();
                                                    spawn(async move {
                                                        if yntra_core::remove_crew_member(uid, jid, member_id).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                },
                                                class: "p-1 rounded text-red-500 hover:bg-red-500/10 border-0 bg-transparent cursor-pointer flex items-center justify-center transition-all",
                                                title: "Ta bort från bemanning",
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

            // Packaging supplies inventory section
            {
                let items = packaging_res.read().clone().unwrap_or_default();
                let uid_for_add = active_user_id.clone();
                let jid_for_add = job.id.clone();
                
                rsx! {
                    div { class: "border-t border-border/40 pt-4 flex flex-col gap-3",
                        h3 { class: "text-sm font-extrabold flex items-center gap-1.5",
                            style: "margin: 0;",
                            components::LucideIcon { name: "package", size: "16", class: "accent-text" }
                            "Förpackningsmaterial & Materialinventarie"
                        }
                        
                        if items.is_empty() {
                            p { class: "text-xs text-muted-foreground italic my-1", "Inga förpackningsmaterial registrerade för detta flyttuppdrag." }
                        } else {
                            div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2",
                                for item in items.iter() {
                                    {
                                        let item_id = item.id.clone();
                                        let item_name = item.item_name.clone();
                                        let qty = item.quantity;
                                        let price = item.price_per_unit;
                                        let is_leased = item.is_leased;
                                        let returned = item.returned_quantity;
                                        let total_cost = price * qty as f64;
                                        let uid = active_user_id.clone();
                                        let jid = job.id.clone();
                                        
                                        rsx! {
                                            div {
                                                key: "{item_id}",
                                                class: "flex flex-col gap-1.5 p-3 rounded-lg border border-border/20 bg-secondary/5",
                                                div { class: "flex items-center justify-between",
                                                    div {
                                                        div { class: "text-xs font-bold text-foreground", "{item_name}" }
                                                        div { class: "text-[10px] text-muted-foreground font-semibold flex items-center gap-1",
                                                            if is_leased {
                                                                span { class: "px-1 py-0.25 bg-amber-500/10 text-amber-500 rounded-[3px] text-[8px] font-extrabold uppercase", "Hyra" }
                                                            } else {
                                                                span { class: "px-1 py-0.25 bg-emerald-500/10 text-emerald-500 rounded-[3px] text-[8px] font-extrabold uppercase", "Köp" }
                                                            }
                                                            " {price} kr/st"
                                                        }
                                                    }
                                                    div { class: "text-right",
                                                        div { class: "text-xs font-extrabold text-primary", "{total_cost} kr" }
                                                        div { class: "text-[10px] text-muted-foreground", "{qty} st" }
                                                    }
                                                }
                                                
                                                if is_leased {
                                                    div { class: "flex items-center justify-between border-t border-border/10 pt-1.5 mt-1",
                                                        div { class: "text-[10px] font-bold text-muted-foreground",
                                                            "Återlämnat: {returned} av {qty}"
                                                        }
                                                        if is_staff && returned < qty {
                                                            button {
                                                                onclick: {
                                                                    let i_id = item_id.clone();
                                                                    let uid_capture = active_user_id.clone();
                                                                    let mut db_trig = props.db_trigger;
                                                                    move |_| {
                                                                        let i_id = i_id.clone();
                                                                        let uid_capture = uid_capture.clone();
                                                                        let next_returned = returned + 1;
                                                                        spawn(async move {
                                                                            let _ = yntra_core::update_job_packaging_item_returned(
                                                                                uid_capture,
                                                                                i_id,
                                                                                next_returned
                                                                            ).await;
                                                                            let current = *db_trig.read();
                                                                            db_trig.set(current + 1);
                                                                        });
                                                                    }
                                                                },
                                                                class: "px-2 py-0.5 rounded bg-primary text-primary-foreground hover:opacity-90 border-0 cursor-pointer text-[9px] font-extrabold transition-all",
                                                                "Markera återlämnad (+1)"
                                                            }
                                                        } else if returned >= qty {
                                                            span { class: "text-[9px] font-bold text-emerald-500 flex items-center gap-0.5",
                                                                components::LucideIcon { name: "check-circle", size: "10" }
                                                                "Helt återlämnat"
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                if is_staff {
                                                    div { class: "flex justify-end gap-1.5 border-t border-border/10 pt-1.5 mt-1",
                                                        button {
                                                            onclick: {
                                                                let i_id = item_id.clone();
                                                                let uid_capture = active_user_id.clone();
                                                                let mut db_trig = props.db_trigger;
                                                                move |_| {
                                                                    let i_id = i_id.clone();
                                                                    let uid_capture = uid_capture.clone();
                                                                    spawn(async move {
                                                                        let _ = yntra_core::remove_job_packaging_item(uid_capture, i_id).await;
                                                                        let current = *db_trig.read();
                                                                        db_trig.set(current + 1);
                                                                    });
                                                                }
                                                             },
                                                             class: "px-2 py-1 rounded text-red-500 hover:bg-red-500/10 border-0 bg-transparent cursor-pointer flex items-center justify-center gap-1 text-[9px] font-bold",
                                                             components::LucideIcon { name: "trash-2", size: "10" }
                                                             "Ta bort"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        if is_staff {
                            div { class: "mt-2 border border-border/30 rounded-lg p-3 bg-secondary/5",
                                if !*show_add_pack_form.read() {
                                    button {
                                        onclick: move |_| show_add_pack_form.set(true),
                                        class: "w-full py-2 border border-dashed border-border hover:border-primary/50 rounded-lg text-xs font-bold text-muted-foreground hover:text-primary bg-transparent cursor-pointer flex items-center justify-center gap-1.5 transition-all",
                                        components::LucideIcon { name: "plus", size: "14" }
                                        "Lägg till material"
                                    }
                                } else {
                                    div { class: "flex flex-col gap-2.5",
                                        div { class: "flex justify-between items-center",
                                            span { class: "text-[11px] font-bold text-muted-foreground uppercase", "Nytt förpackningsmaterial" }
                                            button {
                                                onclick: move |_| show_add_pack_form.set(false),
                                                class: "text-[10px] text-muted-foreground hover:text-foreground border-0 bg-transparent cursor-pointer",
                                                "Avbryt"
                                            }
                                        }
                                        
                                        div { class: "grid grid-cols-2 gap-2",
                                            div { class: "col-span-2 sm:col-span-1",
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Materialtyp (Preset)" }
                                                select {
                                                    value: "{new_pack_preset}",
                                                    onchange: move |e: FormEvent| {
                                                        let preset = e.value();
                                                        new_pack_preset.set(preset.clone());
                                                        match preset.as_str() {
                                                            "Flyttkartong" => {
                                                                new_pack_name.set("Flyttkartong".to_string());
                                                                new_pack_price.set(30.0);
                                                                new_pack_leased.set(true);
                                                            }
                                                            "Packtejp" => {
                                                                new_pack_name.set("Packtejp".to_string());
                                                                new_pack_price.set(50.0);
                                                                new_pack_leased.set(false);
                                                            }
                                                            "Bubbelplast" => {
                                                                new_pack_name.set("Bubbelplast".to_string());
                                                                new_pack_price.set(90.0);
                                                                new_pack_leased.set(false);
                                                            }
                                                            "Garderobskartong" => {
                                                                new_pack_name.set("Garderobskartong".to_string());
                                                                new_pack_price.set(120.0);
                                                                new_pack_leased.set(true);
                                                            }
                                                            _ => {
                                                                new_pack_name.set(String::new());
                                                                new_pack_price.set(0.0);
                                                                new_pack_leased.set(false);
                                                            }
                                                        }
                                                    },
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                    option { value: "Flyttkartong", "Flyttkartong (Hyra, 30 kr)" }
                                                    option { value: "Packtejp", "Packtejp (Köp, 50 kr)" }
                                                    option { value: "Bubbelplast", "Bubbelplast (Köp, 90 kr)" }
                                                    option { value: "Garderobskartong", "Garderobskartong (Hyra, 120 kr)" }
                                                    option { value: "Custom", "Annan (Ange själv...)" }
                                                }
                                            }
                                            
                                            div { class: "col-span-2 sm:col-span-1",
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Namn" }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "t.ex. Packpapper",
                                                    value: "{new_pack_name}",
                                                    oninput: move |e: FormEvent| new_pack_name.set(e.value()),
                                                    disabled: *new_pack_preset.read() != "Custom",
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary disabled:opacity-60",
                                                }
                                            }
                                        }
                                        
                                        div { class: "grid grid-cols-3 gap-2",
                                            div {
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Antal" }
                                                input {
                                                    r#type: "number",
                                                    min: "1",
                                                    value: "{new_pack_qty}",
                                                    oninput: move |e: FormEvent| new_pack_qty.set(e.value().parse().unwrap_or(1)),
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                }
                                            }
                                            div {
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Pris per st" }
                                                input {
                                                    r#type: "number",
                                                    min: "0",
                                                    value: "{new_pack_price}",
                                                    oninput: move |e: FormEvent| new_pack_price.set(e.value().parse().unwrap_or(0.0)),
                                                    disabled: *new_pack_preset.read() != "Custom",
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary disabled:opacity-60",
                                                }
                                            }
                                            div {
                                                label { class: "text-[10px] text-muted-foreground block mb-0.5", "Transaktionstyp" }
                                                select {
                                                    value: if *new_pack_leased.read() { "lease" } else { "sell" },
                                                    onchange: move |e: FormEvent| new_pack_leased.set(e.value() == "lease"),
                                                    disabled: *new_pack_preset.read() != "Custom",
                                                    class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary disabled:opacity-60",
                                                    option { value: "lease", "Hyra" }
                                                    option { value: "sell", "Köp" }
                                                }
                                            }
                                        }
                                        
                                        button {
                                            onclick: {
                                                let j_id = jid_for_add.clone();
                                                let uid = uid_for_add.clone();
                                                let mut db_trig = props.db_trigger;
                                                move |_| {
                                                    let name = new_pack_name.read().trim().to_string();
                                                    if name.is_empty() { return; }
                                                    let qty = *new_pack_qty.read();
                                                    let price = *new_pack_price.read();
                                                    let leased = *new_pack_leased.read();
                                                    
                                                    let j_id = j_id.clone();
                                                    let uid = uid.clone();
                                                    
                                                    // reset inputs
                                                    new_pack_qty.set(10);
                                                    show_add_pack_form.set(false);
                                                    
                                                    spawn(async move {
                                                        let _ = yntra_core::add_job_packaging_item(
                                                            uid,
                                                            j_id,
                                                            name,
                                                            qty,
                                                            price,
                                                            leased
                                                        ).await;
                                                        let current = *db_trig.read();
                                                        db_trig.set(current + 1);
                                                    });
                                                }
                                            },
                                            class: "w-full py-2 bg-primary hover:opacity-90 rounded-lg text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 transition-all",
                                            components::LucideIcon { name: "check", size: "14" }
                                            "Spara material"
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
                let (is_weekend_date, is_peak_date) = if let Ok(date) = chrono::NaiveDate::parse_from_str(&job.scheduled_date, "%Y-%m-%d") {
                    use chrono::Datelike;
                    let wd = date.weekday();
                    let is_we = wd == chrono::Weekday::Sat || wd == chrono::Weekday::Sun;
                    let day = date.day();
                    let is_pk = day >= 25 || day <= 3;
                    (is_we, is_pk)
                } else {
                    (false, false)
                };
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
                                             div { class: "text-[10px] text-muted-foreground font-semibold", "Timpris ({hours:.1}h • {recommended_crew_size}-man)" }
                                         } else {
                                             div { class: "text-[10px] text-muted-foreground font-semibold", "Baspris ({recommended_crew_size}-man)" }
                                         }
                                         div { class: "text-xs font-bold text-foreground mt-0.5", "{q.base_price} kr" }
                                      }
                                     div {
                                         div { class: "text-[10px] text-muted-foreground font-semibold", "Distans" }
                                         div { class: "text-xs font-bold text-foreground mt-0.5", "{q.distance_fee} kr" }
                                     }
                                     div {
                                         div { class: "text-[10px] text-muted-foreground font-semibold", "Trappor & Bärning" }
                                         div { class: "text-xs font-bold text-foreground mt-0.5", "{q.stairs_surcharge} kr" }
                                     }
                                     div {
                                         div { class: "text-[10px] text-muted-foreground font-semibold", "Material" }
                                         div { class: "text-xs font-bold text-foreground mt-0.5", "{q.packing_supplies_fee} kr" }
                                     }
                                 }

                                // Itemized Dynamic Surcharge & Surge Rules Panel
                                div { class: "p-3 rounded-lg border border-primary/20 bg-primary/5 flex flex-col gap-2 my-1",
                                    div { class: "text-[10px] font-extrabold text-primary uppercase tracking-wider flex items-center gap-1",
                                        components::LucideIcon { name: "calculator", size: "12" }
                                        "Dynamiska taxor & Tilläggsberäkning"
                                    }
                                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-2 text-xs",
                                        div { class: "p-2 rounded bg-background border border-border/30 flex flex-col justify-between",
                                            span { class: "text-[10px] text-muted-foreground font-bold", "Bemannings- & Taxenivå" }
                                            span { class: "font-extrabold text-foreground mt-1", "{recommended_crew_size}-Manna taxa (Multi-tier)" }
                                        }
                                        div { class: "p-2 rounded bg-background border border-border/30 flex flex-col justify-between",
                                            span { class: "text-[10px] text-muted-foreground font-bold", "Bärlängdstillägg (>25m / 75ft)" }
                                            if job.long_carry_meters > 0 {
                                                span { class: "font-extrabold text-amber-600 mt-1", "{job.long_carry_meters} meter ({job.long_carry_meters * 40} kr)" }
                                            } else {
                                                span { class: "font-semibold text-muted-foreground mt-1", "Standard distans (0 kr)" }
                                            }
                                        }
                                        div { class: "p-2 rounded bg-background border border-border/30 flex flex-col justify-between",
                                            span { class: "text-[10px] text-muted-foreground font-bold", "Högsäsong & Helgtaxa" }
                                            div { class: "flex items-center gap-1 mt-1 flex-wrap",
                                                if is_weekend_date {
                                                    span { class: "px-1.5 py-0.2 rounded bg-amber-500/20 text-amber-600 text-[9px] font-extrabold", "Helg +25%" }
                                                }
                                                if is_peak_date {
                                                    span { class: "px-1.5 py-0.2 rounded bg-rose-500/20 text-rose-600 text-[9px] font-extrabold", "Månadsskifte +15%" }
                                                }
                                                if !is_weekend_date && !is_peak_date {
                                                    span { class: "text-muted-foreground font-semibold text-[11px]", "Normal vardagstaxa" }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(over) = q.manual_price_override {
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground px-1",
                                        span { "{t(\"jobs-manual-override\", &region)}" }
                                        span { class: "font-semibold text-foreground", "{over} kr" }
                                    }
                                }
                                if let Some(disc) = q.price_discount {
                                    if disc > 0.0 {
                                        div { class: "flex items-center justify-between text-[11px] text-muted-foreground px-1",
                                            span { "{t(\"jobs-discount\", &region)}" }
                                            span { class: "font-semibold text-rose-500", "-{disc} kr" }
                                        }
                                    }
                                }
                                div { class: "flex items-center justify-between text-xs font-bold text-foreground px-1 border-t border-border/10 pt-1.5 mt-1",
                                    span { "{t(\"jobs-total-quoted-price\", &region)}" }
                                    span { class: "text-sm text-primary font-extrabold", "{q.total_price} kr" }
                                }
                                if is_staff {
                                 div { class: "border-t border-border/10 pt-2.5 flex flex-col gap-2",
                                     button {
                                         class: "text-[11px] font-bold text-primary hover:underline cursor-pointer bg-transparent border-0 self-start p-0 flex items-center gap-1",
                                         onclick: move |_| {
                                             let current = *show_override_form.read();
                                             show_override_form.set(!current);
                                         },
                                         components::LucideIcon { name: if *show_override_form.read() { "chevron-up" } else { "chevron-down" }, size: "12" }
                                         "Ange manuell priskorrigering / rabatt"
                                     }
                                     if *show_override_form.read() {
                                         div { class: "grid grid-cols-2 gap-3 p-3 rounded bg-secondary/10 border border-border/10",
                                             div { class: "flex flex-col gap-1",
                                                 label { class: "text-[10px] font-semibold text-muted-foreground", "Manuell totalt pris (kr)" }
                                                 input {
                                                     r#type: "text",
                                                     placeholder: "T.ex. 5000",
                                                     value: "{manual_override_input}",
                                                     oninput: move |e| manual_override_input.set(e.value().clone()),
                                                     class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground w-full",
                                                 }
                                             }
                                             div { class: "flex flex-col gap-1",
                                                 label { class: "text-[10px] font-semibold text-muted-foreground", "Rabatt (kr)" }
                                                 input {
                                                     r#type: "text",
                                                     placeholder: "T.ex. 500",
                                                     value: "{discount_input}",
                                                     oninput: move |e| discount_input.set(e.value().clone()),
                                                     class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground w-full",
                                                 }
                                             }
                                             button {
                                                 class: "col-span-2 py-1.5 bg-primary hover:opacity-90 rounded text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 mt-1",
                                                 onclick: {
                                                     let j_id = job.id.clone();
                                                     let uid = active_user_id.clone();
                                                     let mut db_trig = props.db_trigger;
                                                     move |_| {
                                                         let j_id = j_id.clone();
                                                         let uid = uid.clone();
                                                         let override_val = manual_override_input.read().parse::<f64>().ok();
                                                         let discount_val = discount_input.read().parse::<f64>().ok();
                                                         spawn(async move {
                                                             let _ = yntra_core::update_move_quote_price_adjustments(uid, j_id, override_val, discount_val).await;
                                                             let current = *db_trig.read();
                                                             db_trig.set(current + 1);
                                                         });
                                                     }
                                                 },
                                                 components::LucideIcon { name: "check-circle", size: "12" }
                                                 "Spara priskorrigering"
                                             }
                                         }
                                     }
                                 }
                             }
                            if has_specialty_item {
                                div { class: "text-[10px] text-amber-600 dark:text-amber-500 font-bold bg-amber-500/10 border border-amber-500/20 px-2.5 py-1.5 rounded-lg mt-1 text-center flex items-center justify-center gap-1",
                                    components::LucideIcon { name: "shield-alert", size: "12" }
                                    "Baspriset inkluderar hanteringstillägg för tunga/känsliga föremål."
                                }
                            }
                            if let Some(ref inv) = invoice {
                                div { class: "mt-3 p-3 rounded bg-muted/20 border border-border/10 space-y-1.5",
                                    div { class: "text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground", "Fakturainformation" }
                                    if show_rut {
                                        div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                            span { "RUT-avdrag:" }
                                            span { class: "font-semibold text-foreground", "{inv.rut_deduction} kr" }
                                        }
                                    }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                        span { "{t(\"jobs-customer-amount\", &region)}" }
                                        span { class: "font-semibold text-foreground", "{inv.customer_amount} kr" }
                                    }
                                    if show_rut {
                                        div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                            span { "{t(\"jobs-tax-authority-rut\", &region)}" }
                                            span { class: "font-semibold text-foreground", "{inv.tax_authority_amount} kr" }
                                        }
                                    }
                                    div { class: "flex items-center justify-between text-[11px] text-muted-foreground pt-1 border-t border-border/10",
                                        span { "{t(\"jobs-invoice-status\", &region)}" }
                                        {
                                            let status_text = if inv.status == "paid" { t("jobs-status-paid", &region) } else { t("jobs-status-unpaid", &region) };
                                            rsx! {
                                                span {
                                                    class: if inv.status == "paid" { "text-emerald-500 font-bold" } else { "text-amber-500 font-bold" },
                                                    "{status_text}"
                                                }
                                            }
                                        }
                                    }
                                    if let Some(hrs) = inv.actual_hours {
                                        div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                            span { "{t(\"jobs-worked-hours\", &region)}" }
                                            span { class: "font-semibold text-foreground", "{hrs} h" }
                                        }
                                    }
                                    if let Some(charges) = inv.additional_charges {
                                        div { class: "flex items-center justify-between text-[11px] text-muted-foreground",
                                            span { "{t(\"jobs-additional-charges\", &region)}" }
                                            span { class: "font-semibold text-foreground", "{charges} kr" }
                                        }
                                    }
                                    if let Some(ref notes) = inv.adjustment_notes {
                                        if !notes.is_empty() {
                                            div { class: "text-[10px] text-muted-foreground italic border-t border-border/10 pt-1 mt-1",
                                                "Anteckning: {notes}"
                                            }
                                        }
                                    }
                                    if is_staff {
                                        div { class: "pt-2 border-t border-border/10 space-y-2",
                                            button {
                                                class: "w-full py-1 bg-secondary/80 hover:bg-secondary rounded text-[10px] font-bold text-secondary-foreground border border-border/20 cursor-pointer flex items-center justify-center gap-1 transition-all",
                                                onclick: {
                                                    let hrs_opt = inv.actual_hours;
                                                    let charges_opt = inv.additional_charges;
                                                    let notes_opt = inv.adjustment_notes.clone();
                                                    move |_| {
                                                        if actual_hours_input.read().is_empty() {
                                                            if let Some(hrs) = hrs_opt {
                                                                actual_hours_input.set(hrs.to_string());
                                                            }
                                                        }
                                                        if additional_charges_input.read().is_empty() {
                                                            if let Some(charges) = charges_opt {
                                                                additional_charges_input.set(charges.to_string());
                                                            }
                                                        }
                                                        if adjustment_notes_input.read().is_empty() {
                                                            if let Some(ref notes) = notes_opt {
                                                                adjustment_notes_input.set(notes.clone());
                                                            }
                                                        }
                                                        let curr = *show_adjust_invoice.read();
                                                        show_adjust_invoice.set(!curr);
                                                    }
                                                },
                                                components::LucideIcon { name: "edit-3", size: "10" }
                                                if *show_adjust_invoice.read() { "Dölj fakturajustering" } else { "Justera faktura (faktiska timmar/tillägg)" }
                                            }
                                            button {
                                                class: "px-2 py-1 text-[11px] font-bold rounded bg-emerald-600/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-600/20 cursor-pointer flex items-center gap-1 transition-colors",
                                                onclick: {
                                                    let active_uid = active_user_id.clone();
                                                    let j_id = job.id.clone();
                                                    move |_| {
                                                        let uid = active_uid.clone();
                                                        let jid = j_id.clone();
                                                        spawn(async move {
                                                            if let Ok(html) = yntra_core::generate_printable_invoice_html(uid, jid.clone()).await {
                                                                super::printable_exporter::trigger_print_or_pdf_download(&toast, &html, &format!("invoice_{}.html", jid));
                                                            }
                                                        });
                                                    }
                                                },
                                                components::LucideIcon { name: "printer", size: "11" }
                                                "Print Invoice PDF"
                                            }
                                            button {
                                                class: "px-2 py-1 text-[11px] font-bold rounded bg-primary/10 text-primary border border-primary/20 hover:bg-primary/20 cursor-pointer flex items-center gap-1 transition-colors",
                                                onclick: move |_| show_pos_modal.set(true),
                                                components::LucideIcon { name: "credit-card", size: "11" }
                                                "On-Site Kortterminal / POS"
                                            }
                                            if *show_adjust_invoice.read() {
                                                div { class: "p-2.5 rounded bg-background border border-border/30 space-y-2 text-left",
                                                    div { class: "text-[11px] font-bold text-foreground flex items-center gap-1",
                                                        components::LucideIcon { name: "sliders", size: "12" }
                                                        "Justering för faktiskt utfall"
                                                    }
                                                    div { class: "grid grid-cols-2 gap-2",
                                                        div { class: "flex flex-col gap-1",
                                                            label { class: "text-[10px] font-semibold text-muted-foreground", "Arbetade timmar (h)" }
                                                            input {
                                                                r#type: "number",
                                                                step: "0.5",
                                                                placeholder: "T.ex. 4.5",
                                                                value: "{actual_hours_input}",
                                                                oninput: move |e| actual_hours_input.set(e.value().clone()),
                                                                class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground w-full",
                                                            }
                                                        }
                                                        div { class: "flex flex-col gap-1",
                                                            label { class: "text-[10px] font-semibold text-muted-foreground", "Extra tillägg (kr)" }
                                                            input {
                                                                r#type: "number",
                                                                step: "10",
                                                                placeholder: "T.ex. 250",
                                                                value: "{additional_charges_input}",
                                                                oninput: move |e| additional_charges_input.set(e.value().clone()),
                                                                class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground w-full",
                                                            }
                                                        }
                                                    }
                                                    div { class: "flex flex-col gap-1",
                                                        label { class: "text-[10px] font-semibold text-muted-foreground", "Anteckning om justeringen" }
                                                        input {
                                                            r#type: "text",
                                                            placeholder: "T.ex. Extra timme pga tunga trappor...",
                                                            value: "{adjustment_notes_input}",
                                                            oninput: move |e| adjustment_notes_input.set(e.value().clone()),
                                                            class: "px-2 py-1 text-xs border border-border bg-background rounded text-foreground w-full",
                                                        }
                                                    }
                                                    button {
                                                        class: "w-full py-1.5 bg-primary hover:opacity-90 rounded text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 mt-1 transition-all",
                                                        onclick: {
                                                            let inv_id = inv.id.clone();
                                                            let uid = active_user_id.clone();
                                                            let mut db_trig = props.db_trigger;
                                                            let toast_c = toast.clone();
                                                            move |_| {
                                                                let inv_id = inv_id.clone();
                                                                let uid = uid.clone();
                                                                let toast_c = toast_c.clone();
                                                                let hrs_val = actual_hours_input.read().parse::<f64>().ok();
                                                                let charges_val = additional_charges_input.read().parse::<f64>().ok();
                                                                let notes_val = if adjustment_notes_input.read().trim().is_empty() {
                                                                    None
                                                                } else {
                                                                    Some(adjustment_notes_input.read().trim().to_string())
                                                                };
                                                                spawn(async move {
                                                                    match yntra_core::adjust_invoice_for_actuals(uid, inv_id, hrs_val, charges_val, notes_val).await {
                                                                        Ok(_) => {
                                                                            let current = *db_trig.read();
                                                                            db_trig.set(current + 1);
                                                                            show_adjust_invoice.set(false);
                                                                            toast_c.success(
                                                                                "Faktura omberäknad".to_string(),
                                                                                dioxus_primitives::toast::ToastOptions::new().description("Fakturan har uppdaterats med faktiskt utfall.")
                                                                            );
                                                                        }
                                                                        Err(e) => {
                                                                            toast_c.error(
                                                                                "Justering misslyckades".to_string(),
                                                                                dioxus_primitives::toast::ToastOptions::new().description(e.to_string())
                                                                            );
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        components::LucideIcon { name: "check-circle", size: "12" }
                                                        "Spara & beräkna om faktura"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if show_rut && inv.status == "paid" && inv.rut_deduction > 0.0 && is_staff {
                                        div { class: "flex gap-2 pt-2 border-t border-border/10",
                                            button {
                                                class: "flex-1 py-1 bg-primary/20 hover:bg-primary/30 rounded text-[10px] font-bold text-foreground border border-primary/20 cursor-pointer flex items-center justify-center gap-1",
                                                onclick: {
                                                    let inv_id = inv.id.clone();
                                                    let uid = active_user_id.clone();
                                                    let toast_c = toast.clone();
                                                    let region_c = region.clone();
                                                    move |_| {
                                                        let inv_id = inv_id.clone();
                                                        let uid = uid.clone();
                                                        let toast_c = toast_c.clone();
                                                        let region_c = region_c.clone();
                                                        spawn(async move {
                                                            match yntra_core::export_skatteverket_claims(uid, vec![inv_id.clone()], "xml".to_string()).await {
                                                                Ok(xml_content) => {
                                                                    let file_name = format!("Skatteverket_RUT_{}.xml", inv_id);
                                                                    trigger_download(&toast_c, &region_c, &xml_content, &file_name);
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
                                                    let toast_c = toast.clone();
                                                    let region_c = region.clone();
                                                    move |_| {
                                                        let inv_id = inv_id.clone();
                                                        let uid = uid.clone();
                                                        let toast_c = toast_c.clone();
                                                        let region_c = region_c.clone();
                                                        spawn(async move {
                                                            match yntra_core::export_skatteverket_claims(uid, vec![inv_id.clone()], "csv".to_string()).await {
                                                                Ok(csv_content) => {
                                                                    let file_name = format!("Skatteverket_RUT_{}.csv", inv_id);
                                                                    trigger_download(&toast_c, &region_c, &csv_content, &file_name);
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

            // Signature capture panel for in_progress status
            if job_status == "in_progress" {
                div { class: "border-t border-border/40 pt-4 flex flex-col gap-4 animate-in fade-in duration-300",
                    h3 { class: "text-sm font-extrabold flex items-center gap-1.5 m-0",
                        components::LucideIcon { name: "pen-tool", size: "16", class: "accent-text" }
                        "Kundens signatur och godkännande"
                    }
                    div { class: "p-4 rounded-xl border border-border/20 bg-secondary/5 space-y-4 max-w-md",
                        div {
                            label { class: "text-[11px] font-bold text-muted-foreground uppercase tracking-wider block mb-1", "Namnförtydligande (Kund)" }
                            input {
                                r#type: "text",
                                placeholder: "t.ex. Anna Andersson",
                                value: "{signer_name}",
                                oninput: move |e| signer_name.set(e.value()),
                                class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                            }
                        }
                        div { class: "space-y-1.5",
                            label { class: "text-[11px] font-bold text-muted-foreground uppercase tracking-wider block", "Rita signatur" }
                            SignatureCanvas {
                                job_id: job.id.clone()
                            }
                            div { class: "flex justify-end",
                                button {
                                    onclick: move |_| {
                                        let _ = dioxus::document::eval(r#"
                                            const canvas = document.getElementById('signature-pad');
                                            if (canvas) {
                                                const ctx = canvas.getContext('2d');
                                                ctx.clearRect(0, 0, canvas.width, canvas.height);
                                            }
                                        "#);
                                    },
                                    class: "py-1.5 px-3 rounded-lg text-xs font-semibold text-muted-foreground bg-transparent hover:bg-secondary/20 hover:text-foreground border border-border/40 cursor-pointer transition-all",
                                    "Rensa"
                                }
                            }
                        }
                    }
                }
            }

            // Display captured signature if it exists
            if let Some(ref sig) = signature_opt {
                {
                    let formatted_time = chrono::DateTime::from_timestamp(sig.signed_at / 1000, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_default();
                    rsx! {
                        div { class: "border-t border-border/40 pt-4 flex flex-col gap-3 animate-in fade-in duration-300",
                            h3 { class: "text-sm font-extrabold flex items-center gap-1.5 m-0",
                                components::LucideIcon { name: "check-square", size: "16", class: "text-emerald-500" }
                                "Godkänd & Signerad"
                            }
                            div { class: "p-4 rounded-xl border border-border/20 bg-emerald-500/5 flex flex-col gap-2 max-w-sm",
                                div { class: "bg-white border border-border rounded-lg p-2 flex items-center justify-center shadow-inner",
                                    img {
                                        src: "{sig.signature_data_base64}",
                                        class: "h-[80px] object-contain select-none max-w-full",
                                        alt: "Digital signatur"
                                    }
                                }
                                div { class: "text-xs font-bold text-foreground",
                                    "Signerat av: {sig.signer_name}"
                                }
                                div { class: "text-[10px] text-muted-foreground",
                                    "Tid: {formatted_time}"
                                }
                            }
                        }
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
                                disabled: signer_name.read().trim().is_empty(),
                                onclick: {
                                    let checklist_state = checklist_state.clone();
                                    let completion_report_state = completion_report_state.clone();
                                    let job_id = job_id_submit.clone();
                                    let uid = uid_for_complete.clone();
                                    let signer_name_sig = signer_name.clone();
                                    let mut db_trigger = props.db_trigger;
                                    move |_| {
                                        let checklist_str = serde_json::to_string(&*checklist_state.read()).unwrap();
                                        let report_str = completion_report_state.read().clone();
                                        let job_id = job_id.clone();
                                        let uid = uid.clone();
                                        let signer = signer_name_sig.read().trim().to_string();
                                        
                                        spawn(async move {
                                            // 1. Capture base64 signature from canvas
                                            let mut eval = dioxus::document::eval(r#"
                                                const canvas = document.getElementById('signature-pad');
                                                if (canvas) {
                                                    const dataUrl = canvas.toDataURL('image/png');
                                                    dioxus.send(dataUrl);
                                                } else {
                                                    dioxus.send("");
                                                }
                                            "#);
                                            
                                            let signature_data = eval.recv::<String>().await.unwrap_or_default();
                                            
                                            // 2. Save signature if we captured one
                                            if !signature_data.is_empty() && !signer.is_empty() {
                                                let _ = yntra_core::save_job_signature(
                                                    uid.clone(),
                                                    job_id.clone(),
                                                    signer,
                                                    signature_data,
                                                ).await;
                                            }
                                            
                                            // 3. Complete the job
                                            if yntra_core::submit_job_completion(uid, job_id, checklist_str, report_str).await.is_ok() {
                                                let current = *db_trigger.read();
                                                db_trigger.set(current + 1);
                                            }
                                        });
                                    }
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

            if *show_edit_surcharges.read() {
                div { class: "fixed inset-0 bg-black/60 backdrop-blur-sm z-[100] flex items-center justify-center p-4",
                    div { class: "bg-background border border-border rounded-xl p-6 max-w-sm w-full shadow-2xl flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150",
                        h3 { class: "text-base font-extrabold flex items-center gap-1.5 m-0 text-foreground",
                            components::LucideIcon { name: "settings", size: "18", class: "accent-text" }
                            "Ändra bärlängd & vägavgifter"
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Bärlängd (meter från lastbil till dörr)" }
                            input {
                                r#type: "number",
                                value: "{edit_long_carry}",
                                oninput: move |e: FormEvent| {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        edit_long_carry.set(val);
                                    }
                                },
                                class: "w-full px-3 py-2 text-sm bg-muted/20 border border-border/80 rounded-lg text-foreground focus:outline-none focus:border-accent",
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Väg- & broavgifter (kr)" }
                            input {
                                r#type: "number",
                                value: "{edit_toll_fees}",
                                oninput: move |e: FormEvent| {
                                    if let Ok(val) = e.value().parse::<f64>() {
                                        edit_toll_fees.set(val);
                                    }
                                },
                                class: "w-full px-3 py-2 text-sm bg-muted/20 border border-border/80 rounded-lg text-foreground focus:outline-none focus:border-accent",
                            }
                        }
                        div { class: "flex gap-2 justify-end mt-2 pt-2 border-t border-border/40",
                            button {
                                class: "px-3 py-2 bg-secondary text-secondary-foreground hover:opacity-90 rounded-lg text-xs font-bold border-0 cursor-pointer transition-all",
                                onclick: move |_| {
                                    show_edit_surcharges.set(false);
                                },
                                "Avbryt"
                            }
                            button {
                                class: "px-4 py-2 bg-primary text-primary-foreground hover:opacity-90 rounded-lg text-xs font-bold border-0 cursor-pointer transition-all flex items-center gap-1.5",
                                onclick: {
                                    let uid = active_user_id.clone();
                                    let jid = job.id.clone();
                                    let mut db_trigger = props.db_trigger;
                                    move |_| {
                                        let uid = uid.clone();
                                        let jid = jid.clone();
                                        let carry = *edit_long_carry.read();
                                        let tolls = *edit_toll_fees.read();
                                        spawn(async move {
                                            if yntra_core::update_job_moving_surcharges(uid, jid, carry, tolls).await.is_ok() {
                                                show_edit_surcharges.set(false);
                                                let current = *db_trigger.read();
                                                db_trigger.set(current + 1);
                                            }
                                        });
                                    }
                                },
                                components::LucideIcon { name: "save", size: "14" }
                                "Spara"
                            }
                        }
                    }
                }
            }

            if *show_bol_modal.read() {
                super::bol_modal::BillOfLadingModal {
                    job_id: job.id.clone(),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_bol_modal.set(false),
                }
            }

            if *show_condition_modal.read() {
                super::condition_modal::InventoryConditionModal {
                    job_id: job.id.clone(),
                    active_user_id: active_user_id.clone(),
                    inventories: inventories.clone(),
                    on_close: move |_| show_condition_modal.set(false),
                }
            }

            if *show_pos_modal.read() {
                {
                    let inv_opt = invoice_res.read().clone().flatten();
                    let inv_id = inv_opt.as_ref().map(|i| i.id.clone()).unwrap_or_default();
                    let amount = inv_opt.as_ref().map(|i| i.customer_amount).unwrap_or(0.0);
                    rsx! {
                        super::pos_modal::PosTerminalModal {
                            active_user_id: active_user_id.clone(),
                            invoice_id: inv_id,
                            amount_sek: amount,
                            db_trigger: props.db_trigger,
                            on_close: move |_| show_pos_modal.set(false),
                        }
                    }
                }
            }

            if *show_sit_modal.read() {
                super::sit_modal::WarehouseSitModal {
                    job_id: job.id.clone(),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_sit_modal.set(false),
                }
            }

            if *show_payroll_modal.read() {
                super::payroll_modal::CrewPayrollModal {
                    job_id: job.id.clone(),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_payroll_modal.set(false),
                }
            }

            if *show_erp_modal.read() {
                {
                    let erp_inv_id = invoice_res.read().clone().flatten().map(|i| i.id);
                    rsx! {
                        super::erp_modal::ErpSyncModal {
                            job_id: job.id.clone(),
                            invoice_id: erp_inv_id,
                            active_user_id: active_user_id.clone(),
                            on_close: move |_| show_erp_modal.set(false),
                        }
                    }
                }
            }

            if *show_live_tracking_modal.read() {
                super::live_tracking_modal::CustomerLiveTrackingModal {
                    job_id: job.id.clone(),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_live_tracking_modal.set(false),
                }
            }

            if *show_field_crew_view.read() {
                super::field_crew_view::FieldCrewView {
                    job_id: job.id.clone(),
                    job: Some(job.clone()),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_field_crew_view.set(false),
                    on_open_condition_modal: move |_| show_condition_modal.set(true),
                    on_open_pos_modal: move |_| show_pos_modal.set(true),
                }
            }

            if *show_eld_modal.read() {
                super::eld_modal::EldDotComplianceModal {
                    vehicle_id: job.assigned_vehicle_id.clone().unwrap_or_else(|| "vh-default".to_string()),
                    on_close_handler: move |_| show_eld_modal.set(false),
                }
            }

            if *show_dispatch_alerts_modal.read() {
                super::dispatch_alerts_modal::DispatchAlertsModal {
                    job_id: job.id.clone(),
                    customer_id: job.assigned_user_id.clone(),
                    active_user_id: active_user_id.clone(),
                    on_close: move |_| show_dispatch_alerts_modal.set(false),
                }
            }

            super::hvac_modal::HvacDiagnosticModal {
                show: show_hvac_modal,
                job_id: job.id.clone(),
                active_user_id: active_user_id.clone(),
                db_trigger: db_trigger,
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct SignatureCanvasProps {
    pub job_id: String,
}

#[component]
pub fn SignatureCanvas(props: SignatureCanvasProps) -> Element {
    use_effect(move || {
        let script = r#"
            (function() {
                const canvas = document.getElementById('signature-pad');
                if (!canvas) return;

                let drawing = false;
                let lastX = 0;
                let lastY = 0;

                function getPos(e) {
                    const rect = canvas.getBoundingClientRect();
                    const clientX = e.touches ? e.touches[0].clientX : e.clientX;
                    const clientY = e.touches ? e.touches[0].clientY : e.clientY;
                    return {
                        x: clientX - rect.left,
                        y: clientY - rect.top
                    };
                }

                function startDraw(e) {
                    drawing = true;
                    const pos = getPos(e);
                    lastX = pos.x;
                    lastY = pos.y;
                    
                    const ctx = canvas.getContext('2d');
                    ctx.beginPath();
                    ctx.moveTo(lastX, lastY);
                    if (e.cancelable) e.preventDefault();
                }

                function draw(e) {
                    if (!drawing) return;
                    const pos = getPos(e);
                    const ctx = canvas.getContext('2d');
                    ctx.lineTo(pos.x, pos.y);
                    ctx.stroke();
                    lastX = pos.x;
                    lastY = pos.y;
                    if (e.cancelable) e.preventDefault();
                }

                // Clone to clear any old event listeners
                const clone = canvas.cloneNode(true);
                canvas.replaceWith(clone);

                const ctx = clone.getContext('2d');
                const isDark = document.documentElement.classList.contains('dark') || document.body.classList.contains('dark') || (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
                ctx.strokeStyle = isDark ? '#f8fafc' : '#0f172a';
                ctx.lineWidth = 3;
                ctx.lineCap = 'round';
                ctx.lineJoin = 'round';

                clone.addEventListener('mousedown', startDraw);
                clone.addEventListener('mousemove', draw);
                clone.addEventListener('mouseup', function() { drawing = false; });
                clone.addEventListener('mouseleave', function() { drawing = false; });

                clone.addEventListener('touchstart', startDraw, { passive: false });
                clone.addEventListener('touchmove', draw, { passive: false });
                clone.addEventListener('touchend', function() { drawing = false; });
            })();
        "#;
        let _ = dioxus::document::eval(script);
    });

    rsx! {
        canvas {
            id: "signature-pad",
            width: "400",
            height: "150",
            class: "border border-border bg-background rounded-lg cursor-crosshair w-full h-[150px] shadow-sm select-none touch-none",
        }
    }
}
