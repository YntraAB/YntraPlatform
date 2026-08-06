#![allow(unused_imports)]
pub mod types;

pub use types::{trigger_download, JobDetailsProps};

use super::ChecklistItem;
use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    get_job_signature, save_job_signature, JobTicket, MoveInventoryItem, MoveQuote, MoveVehicle,
    WorkspaceUser,
};

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
            yntra_core::get_job_packaging_items(uid, jid)
                .await
                .unwrap_or_default()
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
        async move { yntra_core::get_users(uid).await.unwrap_or_default() }
    });
    let workspace_users = users_res.read().clone().unwrap_or_default();

    let uid_for_vehicles = active_user_id.clone();
    let vehicles_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = uid_for_vehicles.clone();
        async move { yntra_core::get_vehicles(uid).await.unwrap_or_default() }
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
                yntra_core::get_job_signature(uid, jid)
                    .await
                    .unwrap_or(None)
            }
        }
    });

    let mut selected_crew_user_id = use_signal(|| "999".to_string());
    let mut new_crew_role = use_signal(|| "Bärare".to_string());

    let mut show_override_form = use_signal(|| false);
    let mut manual_override_input = use_signal(|| {
        quote
            .as_ref()
            .and_then(|q| q.manual_price_override)
            .map(|v| v.to_string())
            .unwrap_or_default()
    });
    let mut discount_input = use_signal(|| {
        quote
            .as_ref()
            .and_then(|q| q.price_discount)
            .map(|v| v.to_string())
            .unwrap_or_default()
    });
    use_effect({
        let quote = quote.clone();
        move || {
            if let Some(ref q) = quote {
                manual_override_input.set(
                    q.manual_price_override
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                );
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
        async move { yntra_core::get_directions_url(uid, jid).await.ok() }
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

    let job_title = job.title.clone();
    let job_description = job.description.clone();

    rsx! {
        components::Card {
            class: "p-6 flex flex-col gap-5",

            div { class: "border-b border-border pb-4",
                h2 { class: "m-0 font-extrabold text-xl", "{job_title}" }
                p { class: "text-sm text-muted-foreground mt-1", "{job_description}" }
            }

            div { class: "flex items-center gap-2 text-xs font-semibold text-muted-foreground",
                span { "Job ID: {job.id}" }
                span { "•" }
                span { "Status: {job.status}" }
            }
        }
    }
}
