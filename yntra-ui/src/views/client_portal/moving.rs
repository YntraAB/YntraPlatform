use crate::components;
use dioxus::prelude::*;
use yntra_core::{
    AdyenPaymentSession, StripePaymentSession, SwishPaymentSession, accept_move_quote,
    accept_move_quote_with_rut, accept_move_quote_with_deposit,
    calculate_and_save_move_quote, check_swish_payment_status, create_move_inventory_item,
    delete_move_inventory_item, generate_move_invoice, get_job_tickets, get_move_inventory,
    get_move_invoice, get_move_quote, initiate_adyen_payment, initiate_stripe_payment,
    initiate_swish_payment, pay_move_invoice,
};

#[derive(Clone, Copy, PartialEq)]
struct InventoryTemplate {
    name_key: &'static str,
    category_key: &'static str,
    default_name: &'static str,
    default_category: &'static str,
    volume: f64,
}

const TEMPLATES: &[InventoryTemplate] = &[
    InventoryTemplate {
        name_key: "inventory-item-moving-box",
        category_key: "inventory-category-boxes",
        default_name: "Flyttkartong",
        default_category: "Kartonger",
        volume: 0.1,
    },
    InventoryTemplate {
        name_key: "inventory-item-bed-single",
        category_key: "inventory-category-furniture",
        default_name: "Säng (enkel)",
        default_category: "Möbler",
        volume: 1.2,
    },
    InventoryTemplate {
        name_key: "inventory-item-bed-double",
        category_key: "inventory-category-furniture",
        default_name: "Säng (dubbel)",
        default_category: "Möbler",
        volume: 2.4,
    },
    InventoryTemplate {
        name_key: "inventory-item-sofa-3p",
        category_key: "inventory-category-furniture",
        default_name: "Soffa (3-sits)",
        default_category: "Möbler",
        volume: 1.8,
    },
    InventoryTemplate {
        name_key: "inventory-item-dining-table",
        category_key: "inventory-category-furniture",
        default_name: "Matbord",
        default_category: "Möbler",
        volume: 1.2,
    },
    InventoryTemplate {
        name_key: "inventory-item-chair",
        category_key: "inventory-category-furniture",
        default_name: "Stol",
        default_category: "Möbler",
        volume: 0.2,
    },
    InventoryTemplate {
        name_key: "inventory-item-wardrobe",
        category_key: "inventory-category-furniture",
        default_name: "Garderob",
        default_category: "Möbler",
        volume: 2.0,
    },
    InventoryTemplate {
        name_key: "inventory-item-bookshelf",
        category_key: "inventory-category-furniture",
        default_name: "Bokhylla",
        default_category: "Möbler",
        volume: 0.8,
    },
    InventoryTemplate {
        name_key: "inventory-item-dresser",
        category_key: "inventory-category-furniture",
        default_name: "Byrå",
        default_category: "Möbler",
        volume: 0.7,
    },
    InventoryTemplate {
        name_key: "inventory-item-fridge-freezer",
        category_key: "inventory-category-appliances",
        default_name: "Kyl/Frys",
        default_category: "Vitvaror",
        volume: 1.5,
    },
    InventoryTemplate {
        name_key: "inventory-item-washing-machine",
        category_key: "inventory-category-appliances",
        default_name: "Tvättmaskin",
        default_category: "Vitvaror",
        volume: 0.6,
    },
];

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Props, Clone, PartialEq)]
pub struct MovingPortalProps {
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn MovingPortal(props: MovingPortalProps) -> Element {
    let mut db_trigger = props.db_trigger;

    let state = use_context::<crate::state::AppState>();
    let locale = state.auth_region.read().clone();
    let workspace_opt = state.workspace.read().clone();
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

    let mut active_tab = use_signal(|| "moving_jobs".to_string());

    let active_uid_for_jobs = props.active_user_id.clone();
    let active_uid_for_inv = props.active_user_id.clone();
    let active_uid_for_quote = props.active_user_id.clone();

    let jobs_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = active_uid_for_jobs.clone();
        async move {
            match get_job_tickets(uid).await {
                Ok(list) => list,
                Err(_) => Vec::new(),
            }
        }
    });
    let mut selected_job_id = use_signal(|| Option::<String>::None);
    let jobs = jobs_res.read().clone().unwrap_or_default();

    use_effect(move || {
        let current_jobs = jobs_res.read().clone().unwrap_or_default();
        if !current_jobs.is_empty() {
            let sel = selected_job_id.read().clone();
            if sel.is_none()
                || !current_jobs
                    .iter()
                    .any(|j| j.id == sel.as_deref().unwrap_or(""))
            {
                selected_job_id.set(Some(current_jobs[0].id.clone()));
            }
        }
    });

    let active_job_id = selected_job_id
        .read()
        .clone()
        .or_else(|| jobs.first().map(|j| j.id.clone()))
        .unwrap_or_default();
    let active_job = jobs.iter().find(|j| j.id == active_job_id).cloned();

    let active_job_id_for_inv = active_job_id.clone();
    let uid_for_inv = active_uid_for_inv.clone();
    let inventory_res = use_resource(move || {
        let _trig = db_trigger.read();
        let jid = active_job_id_for_inv.clone();
        let uid = uid_for_inv.clone();
        async move {
            if jid.is_empty() {
                Vec::new()
            } else {
                get_move_inventory(uid, jid).await.unwrap_or_default()
            }
        }
    });

    let active_job_id_for_quote = active_job_id.clone();
    let uid_for_quote = active_uid_for_quote.clone();
    let quote_res = use_resource(move || {
        let _trig = db_trigger.read();
        let jid = active_job_id_for_quote.clone();
        let uid = uid_for_quote.clone();
        async move {
            if jid.is_empty() {
                None
            } else {
                get_move_quote(uid, jid).await.unwrap_or(None)
            }
        }
    });

    let inventories = inventory_res.read().clone().unwrap_or_default();
    let total_volume: f64 = inventories
        .iter()
        .map(|i| i.estimated_volume_m3 * i.quantity as f64)
        .sum();
    let quote = quote_res.read().clone().flatten();
    let mut use_rut = use_signal(|| false);
    let mut personal_number = use_signal(String::new);
    let mut show_add_form = use_signal(|| false);
    let mut selected_template_idx = use_signal(|| 999);
    let mut new_item_name = use_signal(String::new);
    let mut new_item_category = use_signal(|| "Möbler".to_string());
    let mut new_item_qty = use_signal(|| 1);
    let mut new_item_vol = use_signal(|| 0.5);
    let mut new_item_notes = use_signal(String::new);
    let mut new_item_length = use_signal(|| "".to_string());
    let mut new_item_width = use_signal(|| "".to_string());
    let mut new_item_height = use_signal(|| "".to_string());
    let mut update_volume_from_dims = move |l_str: String, w_str: String, h_str: String| {
        let l = l_str.parse::<f64>().unwrap_or(0.0);
        let w = w_str.parse::<f64>().unwrap_or(0.0);
        let h = h_str.parse::<f64>().unwrap_or(0.0);
        if l > 0.0 && w > 0.0 && h > 0.0 {
            let vol = (l * w * h) / 1_000_000.0;
            let rounded = (vol * 1000.0).round() / 1000.0;
            new_item_vol.set(rounded);
        }
    };
    let mut show_swish_modal = use_signal(|| Option::<SwishPaymentSession>::None);
    let mut show_adyen_modal = use_signal(|| Option::<AdyenPaymentSession>::None);
    let mut show_stripe_modal = use_signal(|| Option::<StripePaymentSession>::None);
    let mut swish_polling_seconds = use_signal(|| 0i32);
    let mut swish_payment_status = use_signal(|| "pending".to_string());
    let mut show_tracking_job_id = use_signal(|| Option::<String>::None);

    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            settings_json
                .get("company_country")
                .and_then(|v| v.as_str())
                .unwrap_or("SE")
        })
        .to_uppercase();
    let show_rut = settings_json
        .get("show_rut_deduction")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| target_region == "SE");

    let configured_currency = settings_json
        .get("currency")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());

    let configured_currency_symbol = settings_json
        .get("currency_symbol")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let configured_gateway = settings_json
        .get("payment_gateway")
        .or_else(|| settings_json.get("payment_method"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let currency_suffix = if let Some(ref sym) = configured_currency_symbol {
        format!(" {}", sym)
    } else if let Some(ref curr) = configured_currency {
        match curr.as_str() {
            "USD" => " $".to_string(),
            "EUR" => " €".to_string(),
            "GBP" => " £".to_string(),
            "SEK" | "NOK" | "DKK" => " kr".to_string(),
            c => format!(" {}", c),
        }
    } else {
        match target_region.as_str() {
            "US" => " $".to_string(),
            "DE" | "FR" | "ES" | "IT" | "NL" | "AT" | "FI" => " €".to_string(),
            "GB" => " £".to_string(),
            _ => " kr".to_string(),
        }
    };

    let dynamic_tax_rate = settings_json
        .get("tax_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| {
            if target_region == "US" {
                settings_json
                    .get("sales_tax_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.08)
            } else if target_region == "DE" {
                settings_json
                    .get("vat_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.19)
            } else {
                0.0
            }
        });

    let active_gateway = if let Some(ref gw) = configured_gateway {
        gw.clone()
    } else if target_region == "US" || configured_currency.as_deref() == Some("USD") {
        "stripe".to_string()
    } else if target_region == "DE" || configured_currency.as_deref() == Some("EUR") {
        "adyen".to_string()
    } else if target_region == "SE" || configured_currency.as_deref() == Some("SEK") {
        "swish".to_string()
    } else {
        "stripe".to_string()
    };

    let is_english = target_region != "SE";
    let current_sw_status = swish_payment_status.read().clone();

    let payment_method_label = match active_gateway.as_str() {
        "stripe" | "card" => {
            if is_english {
                "Pay with Card (Stripe)".to_string()
            } else {
                "Betala med Stripe (Kort)".to_string()
            }
        }
        "adyen" => {
            if is_english {
                "Pay with Adyen".to_string()
            } else {
                "Betala med Adyen".to_string()
            }
        }
        "swish" => {
            if is_english {
                "Pay with Swish".to_string()
            } else {
                "Betala med Swish".to_string()
            }
        }
        gw => {
            if is_english {
                format!("Pay with {}", gw)
            } else {
                format!("Betala med {}", gw)
            }
        }
    };

    let tax_label = match (target_region.as_str(), configured_currency.as_deref()) {
        // Nordics
        ("SE", _) | (_, Some("SEK")) => "Skatteverket RUT-avdrag (söks av oss):".to_string(),
        ("NO", _) | (_, Some("NOK")) => "MVA / Merverdiavgift (25%):".to_string(),
        ("DK", _) | (_, Some("DKK")) => "Moms / Merværdiafgift (25%):".to_string(),
        ("FI", _) => "ALV / Arvonlisävero (25.5%):".to_string(),
        ("IS", _) | (_, Some("ISK")) => "VSK / Virðisaukaskattur (24%):".to_string(),

        // North America
        ("US", _) | (_, Some("USD")) => "Sales Tax:".to_string(),
        ("CA", _) | (_, Some("CAD")) => "GST / HST / PST:".to_string(),
        ("MX", _) | (_, Some("MXN")) => "IVA / Impuesto al Valor Agregado (16%):".to_string(),

        // UK, Ireland, Commonwealth & Oceania
        ("GB", _) | ("UK", _) | (_, Some("GBP")) => "VAT (20%):".to_string(),
        ("IE", _) => "VAT / Value Added Tax (23%):".to_string(),
        ("AU", _) | (_, Some("AUD")) => "GST / Goods and Services Tax (10%):".to_string(),
        ("NZ", _) | (_, Some("NZD")) => "GST / Goods and Services Tax (15%):".to_string(),

        // DACH & Western Europe
        ("DE", _) => "MwSt / Umsatzsteuer (19%):".to_string(),
        ("AT", _) => "MwSt / USt (20%):".to_string(),
        ("CH", _) | (_, Some("CHF")) => "MWST / TVA / IVA (8.1%):".to_string(),
        ("NL", _) => "Btw / Omzetbelasting (21%):".to_string(),
        ("BE", _) => "TVA / BTW (21%):".to_string(),
        ("FR", _) => "TVA / Taxe sur la valeur ajoutée (20%):".to_string(),
        ("ES", _) => "IVA / Impuesto sobre el Valor Añadido (21%):".to_string(),
        ("IT", _) => "IVA / Imposta sul Valore Aggiunto (22%):".to_string(),
        ("PT", _) => "IVA / Imposto sobre o Valor Acrescentado (23%):".to_string(),

        // Asia Pacific & Middle East & LatAm
        ("JP", _) | (_, Some("JPY")) => "消費税 / Consumption Tax (10%):".to_string(),
        ("SG", _) | (_, Some("SGD")) => "GST / Goods and Services Tax (9%):".to_string(),
        ("AE", _) | (_, Some("AED")) => "VAT / Value Added Tax (5%):".to_string(),
        ("SA", _) | (_, Some("SAR")) => "VAT / Value Added Tax (15%):".to_string(),
        ("BR", _) | (_, Some("BRL")) => "Impostos / ICMS / ISS:".to_string(),

        // Fallback for EUR or general international
        (_, Some("EUR")) => "VAT (MwSt / TVA / Btw):".to_string(),
        _ => "VAT / Sales Tax:".to_string(),
    };

    let quote_id_for_inv = quote.as_ref().map(|q| q.id.clone()).unwrap_or_default();
    let active_uid_for_invoice = props.active_user_id.clone();
    let db_trig_val = *db_trigger.read();
    let invoice_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = active_uid_for_invoice.clone();
        let qid = quote_id_for_inv.clone();
        async move {
            if qid.is_empty() {
                None
            } else {
                get_move_invoice(uid, qid).await.unwrap_or(None)
            }
        }
    });
    let invoice = invoice_res.read().clone().flatten();

    let active_uid_for_accept_c = props.active_user_id.clone();
    let on_accept_quote = move |quote_id: String| {
        let uid = active_uid_for_accept_c.clone();
        let apply_rut = *use_rut.read();
        let pnum_str = personal_number.read().clone();
        spawn(async move {
            let pnum_opt = if pnum_str.trim().is_empty() { None } else { Some(pnum_str) };
            if accept_move_quote_with_rut(uid.clone(), quote_id.clone(), apply_rut, pnum_opt)
                .await
                .is_ok()
            {
                let _ = generate_move_invoice(uid, quote_id, apply_rut).await;
                let current_val = *db_trigger.read();
                db_trigger.set(current_val + 1);
            }
        });
    };

    let active_uid_for_swish_complete = props.active_user_id.clone();
    let on_complete_swish_payment = move |invoice_id: String| {
        let uid = active_uid_for_swish_complete.clone();
        spawn(async move {
            if pay_move_invoice(uid, invoice_id).await.is_ok() {
                let current_val = *db_trigger.read();
                db_trigger.set(current_val + 1);
            }
            show_swish_modal.set(None);
        });
    };

    let active_uid_for_stripe_complete = props.active_user_id.clone();
    let on_complete_stripe_payment = move |invoice_id: String| {
        let uid = active_uid_for_stripe_complete.clone();
        spawn(async move {
            if pay_move_invoice(uid, invoice_id).await.is_ok() {
                let current_val = *db_trigger.read();
                db_trigger.set(current_val + 1);
            }
            show_stripe_modal.set(None);
        });
    };

    let active_uid_for_adyen_complete = props.active_user_id.clone();
    let on_complete_adyen_payment = move |invoice_id: String| {
        let uid = active_uid_for_adyen_complete.clone();
        spawn(async move {
            if pay_move_invoice(uid, invoice_id).await.is_ok() {
                let current_val = *db_trigger.read();
                db_trigger.set(current_val + 1);
            }
            show_adyen_modal.set(None);
        });
    };

    let active_uid_for_swish_retry = props.active_user_id.clone();
    let on_retry_swish = move |invoice_id: String| {
        let uid = active_uid_for_swish_retry.clone();
        spawn(async move {
            swish_payment_status.set("pending".to_string());
            swish_polling_seconds.set(0);
            if let Ok(session) = initiate_swish_payment(uid.clone(), invoice_id.clone()).await {
                show_swish_modal.set(Some(session.clone()));
                let invoice_id_c = invoice_id.clone();
                let uid_c = uid.clone();
                let token_c = session.token.clone();
                spawn(async move {
                    let mut resolved = false;
                    for iteration in 1..=60 {
                        crate::utils::sleep_ms(2000).await;
                        if show_swish_modal.read().is_none() {
                            resolved = true;
                            break;
                        }
                        swish_polling_seconds.set(iteration * 2);
                        if let Ok(status) = check_swish_payment_status(
                            uid_c.clone(),
                            invoice_id_c.clone(),
                            token_c.clone(),
                        )
                        .await
                        {
                            if status == "paid" {
                                resolved = true;
                                swish_payment_status.set("paid".to_string());
                                swish_polling_seconds.set(120);
                                let current_val = *db_trigger.read();
                                db_trigger.set(current_val + 1);
                                crate::utils::sleep_ms(1500).await;
                                show_swish_modal.set(None);
                                break;
                            } else if status == "failed"
                                || status == "declined"
                                || status == "cancelled"
                            {
                                resolved = true;
                                swish_payment_status.set(status);
                                break;
                            }
                        }
                    }
                    if !resolved && show_swish_modal.read().is_some() {
                        swish_payment_status.set("timeout".to_string());
                    }
                });
            } else {
                swish_payment_status.set("failed".to_string());
            }
        });
    };

    let active_uid_for_card_fallback = props.active_user_id.clone();
    let on_fallback_to_card = move |invoice_id: String| {
        let uid = active_uid_for_card_fallback.clone();
        spawn(async move {
            show_swish_modal.set(None);
            if let Ok(session) = initiate_stripe_payment(uid, invoice_id).await {
                show_stripe_modal.set(Some(session));
            }
        });
    };

    let active_uid_for_manual_check = props.active_user_id.clone();
    let on_manual_swish_check = move |invoice_id: String| {
        let uid = active_uid_for_manual_check.clone();
        let session_token = show_swish_modal
            .read()
            .as_ref()
            .map(|s| s.token.clone())
            .unwrap_or_default();
        spawn(async move {
            if let Ok(status) = check_swish_payment_status(uid, invoice_id, session_token).await {
                if status == "paid" {
                    swish_payment_status.set("paid".to_string());
                    let current_val = *db_trigger.read();
                    db_trigger.set(current_val + 1);
                    crate::utils::sleep_ms(1500).await;
                    show_swish_modal.set(None);
                } else {
                    swish_payment_status.set(status);
                }
            }
        });
    };

    let active_uid_for_pay_c = props.active_user_id.clone();
    let gateway_for_pay = active_gateway.clone();
    let on_pay_invoice = move |invoice_id: String| {
        let uid = active_uid_for_pay_c.clone();
        let gateway = gateway_for_pay.clone();
        spawn(async move {
            if gateway == "stripe" || gateway == "card" {
                if let Ok(session) = initiate_stripe_payment(uid, invoice_id).await {
                    show_stripe_modal.set(Some(session));
                }
            } else if gateway == "adyen" {
                if let Ok(session) = initiate_adyen_payment(uid, invoice_id).await {
                    show_adyen_modal.set(Some(session));
                }
            } else {
                if let Ok(session) = initiate_swish_payment(uid.clone(), invoice_id.clone()).await {
                    show_swish_modal.set(Some(session.clone()));
                    swish_polling_seconds.set(0);
                    swish_payment_status.set("pending".to_string());

                    let invoice_id_c = invoice_id.clone();
                    let uid_c = uid.clone();
                    let token_c = session.token.clone();
                    spawn(async move {
                        let mut resolved = false;
                        // Poll Swish payment status every 2 seconds for up to 120 seconds (60 iterations)
                        for iteration in 1..=60 {
                            crate::utils::sleep_ms(2000).await;
                            if show_swish_modal.read().is_none() {
                                resolved = true;
                                break;
                            }
                            swish_polling_seconds.set(iteration * 2);

                            if let Ok(status) = check_swish_payment_status(
                                uid_c.clone(),
                                invoice_id_c.clone(),
                                token_c.clone(),
                            )
                            .await
                            {
                                if status == "paid" {
                                    resolved = true;
                                    swish_payment_status.set("paid".to_string());
                                    swish_polling_seconds.set(120);
                                    let current_val = *db_trigger.read();
                                    db_trigger.set(current_val + 1);

                                    // Pause to let the user see the success state
                                    crate::utils::sleep_ms(1500).await;
                                    show_swish_modal.set(None);
                                    break;
                                } else if status == "failed"
                                    || status == "declined"
                                    || status == "cancelled"
                                {
                                    resolved = true;
                                    swish_payment_status.set(status);
                                    break;
                                }
                            }
                        }
                        if !resolved && show_swish_modal.read().is_some() {
                            swish_payment_status.set("timeout".to_string());
                        }
                    });
                }
            }
        });
    };

    let swish_session_opt = show_swish_modal.read().clone();
    let show_swish = swish_session_opt.is_some();
    let qr_code_base64 = swish_session_opt
        .as_ref()
        .map(|s| s.qr_code_base64.clone())
        .unwrap_or_default();
    let swish_amount = swish_session_opt.as_ref().map(|s| s.amount).unwrap_or(0.0);
    let swish_url = swish_session_opt
        .as_ref()
        .map(|s| s.swish_url.clone())
        .unwrap_or_default();

    let elapsed_sec = *swish_polling_seconds.read();
    let progress_percent = ((elapsed_sec as f64 / 120.0) * 100.0).min(100.0) as i64;
    let current_inv_id = invoice_res
        .read()
        .clone()
        .flatten()
        .map(|i| i.id)
        .unwrap_or_default();

    rsx! {

        // 2. Moving Company Portal Widgets
        div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",

            // Left/Main Card: Inbokade Flyttar
            div { class: "md:col-span-2",
                components::Card {
                    class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                    components::CardHeader {
                        class: "pb-3 border-b border-border/40",
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                components::LucideIcon { name: "truck", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg font-bold", "Inbokade Flyttar" }
                                components::CardDescription { class: "text-xs", "Aktuell status för dina flyttuppdrag." }
                            }
                        }
                    }
                    components::CardContent {
                        class: "pt-4 flex flex-1 flex-col justify-between",
                        if jobs.is_empty() {
                            div { class: "flex flex-col items-center justify-center py-8 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/20 my-auto",
                                components::LucideIcon { name: "truck", class: "h-8 w-8 opacity-20 mb-2" }
                                p { class: "text-sm m-0", "Inga flyttar inbokade för tillfället." }
                            }
                        } else {
                            div { class: "space-y-4",
                                for job in jobs.iter() {
                                    div {
                                        key: "{job.id}",
                                        class: "flex flex-col gap-3 rounded-xl border border-border/30 bg-background/50 p-4 transition-all hover:bg-secondary/20",
                                        div { class: "flex items-start gap-3",
                                            div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary font-bold",
                                                components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                            }
                                            div { class: "flex-1 min-w-0",
                                                h4 { class: "text-sm font-bold text-foreground truncate m-0", "{job.title}" }
                                                p { class: "text-xs text-muted-foreground mt-0.5 line-clamp-2 m-0", "{job.description}" }
                                                div { class: "flex items-center gap-1.5 text-xs text-primary font-semibold mt-1",
                                                    components::LucideIcon { name: "calendar", class: "h-3.5 w-3.5" }
                                                    span { "{job.scheduled_date}" }
                                                }
                                            }
                                        }
                                        if let (Some(ref origin), Some(ref dest)) = (job.origin_address.clone(), job.destination_address.clone()) {
                                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 border-t border-border/20 pt-3 mt-1",
                                                div { class: "space-y-1",
                                                    div { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Från (Ursprung)" }
                                                    div { class: "text-xs font-semibold text-foreground", "{origin}" }
                                                    div { class: "text-[10px] text-muted-foreground",
                                                        "Våning {job.origin_floor} • Hiss: "
                                                        if job.origin_has_elevator { "Ja" } else { "Nej" }
                                                        " • Parkering: "
                                                        if job.origin_parking_permit_needed { "Krävs" } else { "Ej krav" }
                                                    }
                                                }
                                                div { class: "space-y-1",
                                                    div { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Till (Destination)" }
                                                    div { class: "text-xs font-semibold text-foreground", "{dest}" }
                                                    div { class: "text-[10px] text-muted-foreground",
                                                        "Våning {job.destination_floor} • Hiss: "
                                                        if job.destination_has_elevator { "Ja" } else { "Nej" }
                                                        " • Parkering: "
                                                        if job.destination_parking_permit_needed { "Krävs" } else { "Ej krav" }
                                                    }
                                                }
                                            }
                                        } else {
                                            div { class: "flex items-center gap-1 text-xs text-muted-foreground border-t border-border/20 pt-2",
                                                components::LucideIcon { name: "map-pin", class: "h-3.5 w-3.5 text-primary" }
                                                span { "{job.location_address}" }
                                            }
                                        }
                                        div { class: "pt-2 flex items-center justify-between border-t border-border/20",
                                            {
                                                let j_id_select = job.id.clone();
                                                let is_active = job.id == active_job_id;
                                                rsx! {
                                                    button {
                                                        class: if is_active {
                                                            "px-3 py-1.5 bg-primary text-primary-foreground text-xs font-semibold rounded-lg shadow-sm flex items-center gap-1.5 cursor-default"
                                                        } else {
                                                            "px-3 py-1.5 bg-secondary hover:bg-secondary/80 text-foreground text-xs font-semibold rounded-lg transition-colors border border-border flex items-center gap-1.5"
                                                        },
                                                        onclick: move |_| selected_job_id.set(Some(j_id_select.clone())),
                                                        components::LucideIcon { name: "check-circle", class: "h-3.5 w-3.5" }
                                                        if is_active { "Vald Aktiv Flytt" } else { "Välj denna flytt" }
                                                    }
                                                }
                                            }
                                            {
                                                let j_id = job.id.clone();
                                                rsx! {
                                                    button {
                                                        class: "px-3 py-1.5 bg-primary hover:bg-primary text-white text-xs font-semibold rounded-lg transition-colors flex items-center gap-1.5 shadow-sm",
                                                        onclick: move |_| show_tracking_job_id.set(Some(j_id.clone())),
                                                        components::LucideIcon { name: "navigation", class: "h-3.5 w-3.5" }
                                                        "Spåra Flytt (Live GPS)"
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

            // Right Card: Ditt Offer / Price Calculation or Flyttöversikt
            div { class: "col-span-1",
                if let Some(ref q) = quote {
                    components::Card {
                        class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3 border-b border-border/40",
                            div { class: "flex items-center gap-3",
                                div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                    components::LucideIcon { name: "credit-card", class: "h-5 w-5" }
                                }
                                div {
                                    components::CardTitle { class: "text-lg font-bold", "Ditt Priserbjudande" }
                                    components::CardDescription { class: "text-xs", "Prisberäkning för din flytt." }
                                }
                            }
                               components::CardContent {
                            class: "pt-6 space-y-4 flex-1 flex flex-col justify-between",
                            if q.status != "accepted" {
                                {
                                    let is_rut = *use_rut.read() && show_rut;
                                    let labor_cost = q.base_price + q.stairs_surcharge;

                                    let (tax_amount, final_total) = match target_region.as_str() {
                                        "US" | "DE" => {
                                            let tax = q.total_price * dynamic_tax_rate;
                                            (tax, q.total_price + tax)
                                        }
                                        _ => {
                                            if dynamic_tax_rate > 0.0 && !is_rut {
                                                let tax = q.total_price * dynamic_tax_rate;
                                                (tax, q.total_price + tax)
                                            } else {
                                                let rut = if is_rut { 0.5 * labor_cost } else { 0.0 };
                                                (rut, q.total_price - rut)
                                            }
                                        }
                                    };

                                    let hours = q.base_price as f64 / hourly_rate;
                                    rsx! {
                                        div { class: "space-y-3",
                                            div { class: "space-y-1.5",
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    if pricing_model == "hourly" {
                                                        span { "Baspris ({hours:.1} tim à {hourly_rate}{currency_suffix}/tim):" }
                                                    } else {
                                                        span { "Baspris (arbete/tid):" }
                                                    }
                                                    span { class: "font-semibold text-foreground", "{q.base_price}{currency_suffix}" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Distansavgift:" }
                                                    span { class: "font-semibold text-foreground", "{q.distance_fee}{currency_suffix}" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Trapptillägg:" }
                                                    span { class: "font-semibold text-foreground", "{q.stairs_surcharge}{currency_suffix}" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Packmaterial & utrustning:" }
                                                    span { class: "font-semibold text-foreground", "{q.packing_supplies_fee}{currency_suffix}" }
                                                }
                                                if show_rut && is_rut {
                                                    div { class: "flex items-center justify-between text-xs text-emerald-500 font-semibold",
                                                        span { "Preliminärt RUT-avdrag (50%):" }
                                                        span { "-{tax_amount}{currency_suffix}" }
                                                    }
                                                }
                                                if target_region != "SE" && tax_amount > 0.0 {
                                                    div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                        span { "{tax_label}" }
                                                        span { "+{tax_amount}{currency_suffix}" }
                                                    }
                                                }
                                            }

                                            if show_rut {
                                                div { class: "flex flex-col gap-2 pt-2.5 border-t border-border/20 text-xs text-muted-foreground",
                                                    div { class: "flex items-center gap-2",
                                                        input {
                                                            r#type: "checkbox",
                                                            id: "rut_checkbox",
                                                            checked: is_rut,
                                                            onclick: move |_| {
                                                                let val = *use_rut.read();
                                                                use_rut.set(!val);
                                                            },
                                                            class: "rounded border-border bg-background text-primary focus:ring-primary cursor-pointer"
                                                        }
                                                        label { r#for: "rut_checkbox", class: "font-bold cursor-pointer select-none text-foreground/80 hover:text-foreground", "Ansök om RUT-avdrag (50% på arbetskostnad)" }
                                                    }
                                                    if is_rut {
                                                        div { class: "pl-6 space-y-1",
                                                            input {
                                                                r#type: "text",
                                                                placeholder: "Personnummer (ÅÅÅÅMMDD-XXXX)",
                                                                value: "{personal_number.read()}",
                                                                oninput: move |evt| personal_number.set(evt.value()),
                                                                class: "w-full rounded-md border border-input bg-background px-3 py-1.5 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            div { class: "h-px bg-border/40 my-2", }
                                            div { class: "flex items-center justify-between text-sm font-extrabold",
                                                span { if is_rut { "Ditt pris efter RUT:" } else { "Totalt pris:" } }
                                                span { class: "text-primary text-base", "{final_total}{currency_suffix}" }
                                            }

                                            div { class: "pt-4",
                                                button {
                                                    onclick: {
                                                        let q_id = q.id.clone();
                                                        move |_| on_accept_quote(q_id.clone())
                                                    },
                                                    class: "w-full rounded-lg bg-primary py-2.5 text-xs font-bold text-primary-foreground shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center justify-center gap-2",
                                                    components::LucideIcon { name: "thumbs-up", class: "h-4 w-4" }
                                                    "Godkänn Flytoffert"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "space-y-3",
                                    if let Some(ref inv) = invoice {
                                        div { class: "space-y-2",
                                            div { class: "flex items-center justify-center gap-2 p-2.5 rounded-lg bg-emerald-500/10 text-emerald-500 text-xs font-bold text-center border border-emerald-500/20 mb-2 select-none",
                                                components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                                span { "Offert & Avtal Godkända" }
                                            }
                                            div { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground/60 mt-1 select-none", "Fakturaspecifikation" }
                                            div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                span { if show_rut { "Totalsumma (exkl. RUT):" } else { "Subtotal:" } }
                                                span { class: "font-semibold text-foreground", "{inv.subtotal}{currency_suffix}" }
                                            }
                                            if show_rut && inv.rut_deduction > 0.0 {
                                                div { class: "flex items-center justify-between text-xs text-emerald-500 font-semibold",
                                                    span { "RUT-avdrag (skattereduktion):" }
                                                    span { "-{inv.rut_deduction}{currency_suffix}" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Skatteverket (söks av oss):" }
                                                    span { "{inv.tax_authority_amount}{currency_suffix}" }
                                                }
                                            }
                                            if target_region != "SE" && inv.tax_authority_amount > 0.0 {
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "{tax_label}" }
                                                    span { "{inv.tax_authority_amount}{currency_suffix}" }
                                                }
                                            }
                                            div { class: "h-px bg-border/40 my-1", }
                                            div { class: "flex items-center justify-between text-sm font-extrabold",
                                                span { "Att betala (Kund):" }
                                                span { class: "text-primary text-base", "{inv.customer_amount}{currency_suffix}" }
                                            }
                                            div { class: "flex items-center justify-between text-[10px] text-muted-foreground/75 pt-2",
                                                span { "Fakturadatum:" }
                                                span { "{inv.invoice_date}" }
                                            }
                                            div { class: "flex items-center justify-between text-[10px] text-muted-foreground/75",
                                                span { "Förfallodatum:" }
                                                span { "{inv.due_date}" }
                                            }

                                            div { class: "pt-4",
                                                if inv.status == "paid" {
                                                    div { class: "flex items-center justify-center gap-2 p-2.5 rounded-lg bg-primary/10 text-blue-500 text-xs font-bold text-center border border-primary/20 select-none",
                                                        components::LucideIcon { name: "credit-card", class: "h-4 w-4" }
                                                        span { "Faktura Betald" }
                                                    }
                                                } else {
                                                    button {
                                                        onclick: {
                                                            let inv_id = inv.id.clone();
                                                            move |_| on_pay_invoice(inv_id.clone())
                                                        },
                                                        class: "w-full rounded-lg bg-emerald-500 text-white py-2.5 text-xs font-bold shadow hover:opacity-90 hover:scale-[1.01] transition-all border-0 cursor-pointer flex items-center justify-center gap-2 select-none",
                                                        components::LucideIcon { name: "credit-card", class: "h-4 w-4" }
                                                        "{payment_method_label}"
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "flex items-center justify-center gap-2 p-2.5 rounded-lg bg-emerald-500/10 text-emerald-500 text-xs font-bold text-center border border-emerald-500/20 select-none",
                                            components::LucideIcon { name: "check-circle", class: "h-4 w-4" }
                                            span { "Offert Godkänd" }
                                        }
                                    }
                                }
                            }
                        }                     }
                    }
                } else {
                    components::Card {
                        class: "overflow-hidden border border-border bg-sidebar p-1 shadow-md flex flex-col h-full",
                        components::CardHeader {
                            class: "pb-3 border-b border-border/40",
                            div { class: "flex items-center gap-3",
                                div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                                    components::LucideIcon { name: "package", class: "h-5 w-5" }
                                }
                                div {
                                    components::CardTitle { class: "text-lg font-bold", "Flyttöversikt" }
                                    components::CardDescription { class: "text-xs", "Bokningsstatistik för din flytt." }
                                }
                            }
                        }
                        components::CardContent {
                            class: "pt-6 space-y-4 flex-1 flex flex-col justify-center",
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Antal flyttuppdrag" }
                                span { class: "text-sm font-bold text-foreground", "{jobs.len()} st" }
                            }
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Volymuppskattning" }
                                span { class: "text-sm font-bold text-foreground", "0 m³" }
                            }
                            div { class: "flex items-center justify-between p-3 rounded-lg bg-background/40 border border-border/30",
                                span { class: "text-xs font-semibold text-muted-foreground", "Offertstatus" }
                                span { class: "text-sm font-bold text-muted-foreground", "Ej skapad" }
                            }
                        }
                    }
                }
            }
        }

        // Tabs Section for Moving
        div { class: "w-full animate-in fade-in zoom-in duration-300",
            div { class: "grid w-full max-w-xl grid-cols-3 bg-muted/50 rounded-xl p-1 mb-6 border border-border",
                button {
                    onclick: move |_| active_tab.set("moving_jobs".to_string()),
                    class: if *active_tab.read() == "moving_jobs" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "check-square", class: "h-3.5 w-3.5" }
                    "Mina Flyttuppdrag"
                }
                button {
                    onclick: move |_| active_tab.set("inventory_builder".to_string()),
                    class: if *active_tab.read() == "inventory_builder" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-primary text-white shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "package", class: "h-3.5 w-3.5" }
                    "Möbelberäknare 🛋️"
                }
                button {
                    onclick: move |_| active_tab.set("moving_agreement".to_string()),
                    class: if *active_tab.read() == "moving_agreement" {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center bg-background text-foreground shadow border-0 cursor-pointer"
                    } else {
                        "rounded-lg gap-2 text-xs font-bold py-2 flex items-center justify-center text-muted-foreground hover:bg-secondary/50 hover:text-foreground border-0 bg-transparent cursor-pointer"
                    },
                    components::LucideIcon { name: "file-text", class: "h-3.5 w-3.5" }
                    "Avtal & Detaljer"
                }
            }

            if *active_tab.read() == "moving_jobs" {
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                    // Checklist: Left/Main 2 cols
                    div { class: "md:col-span-2",
                        components::Card {
                            class: "border border-border bg-sidebar shadow-md h-full",
                            components::CardHeader {
                                class: "pb-3 border-b border-border/40",
                                components::CardTitle { class: "text-base font-bold", "Checklista Flyttdag" }
                                components::CardDescription { class: "text-xs", "Uppgifter att slutföra inför och under flyttdagen." }
                            },
                            components::CardContent {
                                class: "pt-6",
                                if jobs.is_empty() {
                                    div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10",
                                        components::LucideIcon { name: "check-square", class: "h-8 w-8 opacity-20 mb-2" }
                                        p { class: "text-sm font-semibold m-0", "Inga checklistor tillgängliga." }
                                    }
                                } else {
                                    div { class: "space-y-3",
                                        for job in jobs.iter() {
                                            {
                                                let checklist_items: Vec<ChecklistItem> = serde_json::from_str(&job.checklist_json).unwrap_or_default();
                                                rsx! {
                                                    div {
                                                        key: "{job.id}",
                                                        class: "space-y-2",
                                                        for item in checklist_items.iter() {
                                                            div { class: "flex items-center gap-3 p-3 rounded-lg bg-background/50 border border-border/20 transition-all hover:bg-secondary/10",
                                                                components::LucideIcon {
                                                                    name: if item.done { "check-circle" } else { "circle" },
                                                                    class: if item.done { "h-4 w-4 text-emerald-500" } else { "h-4 w-4 text-muted-foreground/75" }
                                                                }
                                                                span {
                                                                    class: "text-xs font-medium",
                                                                    style: if item.done { "text-decoration: line-through; color: var(--text-secondary);" } else { "" },
                                                                    "{item.text}"
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
                    },
                    // Inventory: Right 1 col
                    div { class: "col-span-1",
                        components::Card {
                            class: "border border-border bg-sidebar shadow-md h-full flex flex-col",
                            components::CardHeader {
                                class: "pb-3 border-b border-border/40",
                                components::CardTitle { class: "text-base font-bold", "Flyttinventarie & Volym" }
                                components::CardDescription { class: "text-xs", "Bohag uppdelat per kategori." }
                            },
                            components::CardContent {
                                class: "pt-6 flex-1 flex flex-col justify-between h-full",
                                {
                                    let is_editable = quote.as_ref().map(|q| q.status.as_str()) != Some("accepted");
                                    rsx! {
                                        if inventories.is_empty() {
                                            div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10 my-auto",
                                                components::LucideIcon { name: "box", class: "h-8 w-8 opacity-20 mb-2" }
                                                p { class: "text-sm font-semibold m-0", "Inget inventarie tillagt." }
                                            }
                                        } else {
                                            div { class: "space-y-3 flex-1 overflow-y-auto max-h-[300px] pr-1",
                                                for item in inventories.iter() {
                                                    div {
                                                        key: "{item.id}",
                                                        class: "flex items-start justify-between p-2.5 rounded-lg bg-background/50 border border-border/10",
                                                        div { class: "min-w-0 flex-1",
                                                            div { class: "text-xs font-bold text-foreground truncate", "{item.item_name}" }
                                                            div { class: "text-[10px] text-muted-foreground", "{item.item_category} • {item.estimated_volume_m3} m³" }
                                                        }
                                                        div { class: "flex items-center gap-2 text-right pl-2",
                                                            div {
                                                                div { class: "text-xs font-extrabold text-primary", "{item.quantity}x" }
                                                                div { class: "text-[10px] text-muted-foreground font-semibold", "{item.estimated_volume_m3 * item.quantity as f64} m³" }
                                                            }
                                                            if is_editable {
                                                                {
                                                                    let item_id = item.id.clone();
                                                                    let jid = active_job_id.clone();
                                                                    let uid = props.active_user_id.clone();
                                                                    rsx! {
                                                                        button {
                                                                            onclick: move |_| {
                                                                                let i_id = item_id.clone();
                                                                                let uid_val = uid.clone();
                                                                                let j_id = jid.clone();
                                                                                spawn(async move {
                                                                                    let _ = delete_move_inventory_item(uid_val.clone(), i_id).await;
                                                                                    let _ = calculate_and_save_move_quote(uid_val, j_id).await;
                                                                                    let val = *db_trigger.read();
                                                                                    db_trigger.set(val + 1);
                                                                                });
                                                                            },
                                                                            class: "p-1 rounded text-red-500 hover:bg-red-500/10 border-0 bg-transparent cursor-pointer flex items-center justify-center transition-all",
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

                                            div { class: "border-t border-border/30 pt-3 mt-2 flex items-center justify-between",
                                                span { class: "text-xs font-bold text-muted-foreground", "Total cargo-volym:" }
                                                span { class: "text-sm font-extrabold text-primary", "{total_volume:.1} m³" }
                                            }
                                        }

                                        if is_editable && !active_job_id.is_empty() {
                                            div { class: "mt-3 border border-border/30 rounded-lg p-3 bg-secondary/5",
                                                if !*show_add_form.read() {
                                                    button {
                                                        onclick: move |_| show_add_form.set(true),
                                                        class: "w-full py-2 border border-dashed border-border hover:border-primary/50 rounded-lg text-xs font-bold text-muted-foreground hover:text-primary bg-transparent cursor-pointer flex items-center justify-center gap-1.5 transition-all",
                                                        components::LucideIcon { name: "plus", size: "14" }
                                                        "Lägg till bohag / kartong"
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

                                                        // Dropdown of presets
                                                        div {
                                                            label { class: "text-[10px] text-muted-foreground block mb-0.5", "{crate::locales::t(\"inventory-select-type\", &locale)}" }
                                                            select {
                                                                value: "{selected_template_idx}",
                                                                onchange: {
                                                                    let locale_val = locale.clone();
                                                                    move |e: FormEvent| {
                                                                        let idx_str = e.value();
                                                                        let idx: usize = idx_str.parse().unwrap_or(0);
                                                                        selected_template_idx.set(idx);
                                                                        if idx < TEMPLATES.len() {
                                                                            let t_name = crate::locales::t(TEMPLATES[idx].name_key, &locale_val);
                                                                            let t_cat = crate::locales::t(TEMPLATES[idx].category_key, &locale_val);
                                                                            let name_str = if t_name == TEMPLATES[idx].name_key { TEMPLATES[idx].default_name.to_string() } else { t_name };
                                                                            let cat_str = if t_cat == TEMPLATES[idx].category_key { TEMPLATES[idx].default_category.to_string() } else { t_cat };
                                                                            new_item_name.set(name_str);
                                                                            new_item_category.set(cat_str);
                                                                            new_item_vol.set(TEMPLATES[idx].volume);
                                                                        } else {
                                                                            // Custom selection
                                                                            let default_cat = crate::locales::t("inventory-category-furniture", &locale_val);
                                                                            let cat_str = if default_cat == "inventory-category-furniture" { "Möbler".to_string() } else { default_cat };
                                                                            new_item_name.set(String::new());
                                                                            new_item_category.set(cat_str);
                                                                            new_item_vol.set(0.5);
                                                                        }
                                                                        new_item_length.set(String::new());
                                                                        new_item_width.set(String::new());
                                                                        new_item_height.set(String::new());
                                                                    }
                                                                },
                                                                class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                option { value: "999", "{crate::locales::t(\"inventory-option-custom\", &locale)}" }
                                                                {TEMPLATES.iter().enumerate().map(|(i, t_item)| {
                                                                    let raw_name = crate::locales::t(t_item.name_key, &locale);
                                                                    let display_name = if raw_name == t_item.name_key { t_item.default_name.to_string() } else { raw_name };
                                                                    rsx! {
                                                                        option { key: "{i}", value: "{i}", "{display_name} ({t_item.volume} m³)" }
                                                                    }
                                                                })}
                                                            }
                                                        }

                                                        // If Custom selected, show custom inputs
                                                        if *selected_template_idx.read() == 999 {
                                                            div { class: "flex flex-col gap-2",
                                                                div { class: "grid grid-cols-2 gap-2",
                                                                    div { class: "col-span-2 sm:col-span-1",
                                                                        label { class: "text-[10px] text-muted-foreground block mb-0.5", "{crate::locales::t(\"inventory-label-item-name\", &locale)}" }
                                                                        input {
                                                                            r#type: "text",
                                                                            placeholder: "{crate::locales::t(\"inventory-placeholder-custom-name\", &locale)}",
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
                                                                            option { value: "Möbler", "{crate::locales::t(\"inventory-category-furniture\", &locale)}" }
                                                                            option { value: "Kartonger", "{crate::locales::t(\"inventory-category-boxes\", &locale)}" }
                                                                            option { value: "Vitvaror", "{crate::locales::t(\"inventory-category-appliances\", &locale)}" }
                                                                            option { value: "Övrigt", "{crate::locales::t(\"inventory-category-other\", &locale)}" }
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
                                                                div { class: "grid grid-cols-3 gap-2 mt-1",
                                                                    div {
                                                                        label { class: "text-[9px] text-muted-foreground block mb-0.5", "Längd (cm)" }
                                                                        input {
                                                                            r#type: "number",
                                                                            placeholder: "t.ex. 120",
                                                                            value: "{new_item_length}",
                                                                            oninput: move |e: FormEvent| {
                                                                                let val = e.value();
                                                                                new_item_length.set(val.clone());
                                                                                update_volume_from_dims(val, new_item_width.read().clone(), new_item_height.read().clone());
                                                                            },
                                                                            class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                        }
                                                                    }
                                                                    div {
                                                                        label { class: "text-[9px] text-muted-foreground block mb-0.5", "Bredd (cm)" }
                                                                        input {
                                                                            r#type: "number",
                                                                            placeholder: "t.ex. 60",
                                                                            value: "{new_item_width}",
                                                                            oninput: move |e: FormEvent| {
                                                                                let val = e.value();
                                                                                new_item_width.set(val.clone());
                                                                                update_volume_from_dims(new_item_length.read().clone(), val, new_item_height.read().clone());
                                                                            },
                                                                            class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                        }
                                                                    }
                                                                    div {
                                                                        label { class: "text-[9px] text-muted-foreground block mb-0.5", "Höjd (cm)" }
                                                                        input {
                                                                            r#type: "number",
                                                                            placeholder: "t.ex. 80",
                                                                            value: "{new_item_height}",
                                                                            oninput: move |e: FormEvent| {
                                                                                let val = e.value();
                                                                                new_item_height.set(val.clone());
                                                                                update_volume_from_dims(new_item_length.read().clone(), new_item_width.read().clone(), val);
                                                                            },
                                                                            class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            // Predefined item: just quantity selector
                                                            div { class: "grid grid-cols-2 gap-2",
                                                                div { class: "col-span-2",
                                                                    label { class: "text-[10px] text-muted-foreground block mb-0.5", "Antal" }
                                                                    input {
                                                                        r#type: "number",
                                                                        min: "1",
                                                                        value: "{new_item_qty}",
                                                                        oninput: move |e: FormEvent| new_item_qty.set(e.value().parse().unwrap_or(1)),
                                                                        class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        div {
                                                            label { class: "text-[10px] text-muted-foreground block mb-0.5", "Anmärkningar (valfritt)" }
                                                            input {
                                                                r#type: "text",
                                                                placeholder: "t.ex. bräcklig, tung",
                                                                value: "{new_item_notes}",
                                                                oninput: move |e: FormEvent| new_item_notes.set(e.value()),
                                                                class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                            }
                                                        }

                                                        button {
                                                            onclick: {
                                                                let j_id = active_job_id.clone();
                                                                let uid = props.active_user_id.clone();
                                                                move |_| {
                                                                    let name = new_item_name.read().trim().to_string();
                                                                    if name.is_empty() { return; }
                                                                    let cat = new_item_category.read().clone();
                                                                    let qty = *new_item_qty.read();
                                                                    let vol = *new_item_vol.read();
                                                                    let notes_str = new_item_notes.read().trim().to_string();
                                                                    let notes = if notes_str.is_empty() { None } else { Some(notes_str) };

                                                                    let j_id = j_id.clone();
                                                                    let uid_val = uid.clone();

                                                                    // reset inputs
                                                                    new_item_name.set(String::new());
                                                                    new_item_notes.set(String::new());
                                                                    new_item_qty.set(1);
                                                                    new_item_vol.set(0.5);
                                                                    new_item_length.set(String::new());
                                                                    new_item_width.set(String::new());
                                                                    new_item_height.set(String::new());
                                                                    selected_template_idx.set(999);
                                                                    show_add_form.set(false);

                                                                    spawn(async move {
                                                                        let _ = create_move_inventory_item(
                                                                            uid_val.clone(),
                                                                            j_id.clone(),
                                                                            cat,
                                                                            name,
                                                                            qty,
                                                                            vol,
                                                                            notes
                                                                        ).await;
                                                                        let _ = calculate_and_save_move_quote(uid_val, j_id).await;
                                                                        let val = *db_trigger.read();
                                                                        db_trigger.set(val + 1);
                                                                    });
                                                                }
                                                            },
                                                            class: "w-full py-2 bg-primary hover:opacity-90 rounded-lg text-xs font-bold text-primary-foreground border-0 cursor-pointer flex items-center justify-center gap-1 transition-all",
                                                            components::LucideIcon { name: "check", size: "14" }
                                                            "Lägg till i flytten"
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
            } else if *active_tab.read() == "inventory_builder" {
                super::inventory_builder::InteractiveSelfServiceInventoryBuilder {
                    active_user_id: props.active_user_id.clone(),
                    job_id: active_job_id.clone(),
                    on_inventory_updated: move |_| {
                        let val = *db_trigger.read();
                        db_trigger.set(val + 1);
                    }
                }
            } else {
                components::Card {
                    class: "border border-border bg-sidebar shadow-md",
                    components::CardContent {
                        class: "pt-6 space-y-6 flex flex-col gap-4",
                        div { class: "border-b border-border/30 pb-4",
                            h3 { class: "text-base font-bold text-foreground m-0", "Standardavtal för Flyttjänster" }
                            p { class: "text-xs text-muted-foreground mt-1 m-0", "Senast uppdaterat: 2026-07-04" }
                        }
                        div { class: "text-sm text-foreground leading-relaxed whitespace-pre-line bg-background/30 p-4 rounded-xl border border-border/20 m-0",
                            "1. OMFATTNING\nTjänsten omfattar flytt av bohag enligt den godkända inventarielistan. Packningsmaterial tillhandahålls av flyttfirman om detta överenskommits.\n\n2. ANSVAR & FÖRSÄKRING\nFöretaget ansvarar för skador som uppkommer på grund av vårdslöshet från personalens sida. Kunden uppmanas att teckna en kompletterande hemförsäkring med flyttskydd. Allt bohag är försäkrat under transporten upp till 500 000 SEK.\n\n3. AVBOKNINGSREGLER\nAvbokning kostnadsfritt fram till 5 arbetsdagar innan bokat flyttdatum. Därefter debiteras 50% av den estimerade arbestaytan."
                        }
                    }
                }
            }
        }

        if show_swish {
            div {
                class: "fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200",
                        div {
                            class: "bg-sidebar border border-border rounded-2xl w-full max-w-sm p-6 shadow-2xl flex flex-col items-center text-center gap-5 relative animate-in zoom-in-95 duration-200",
                            button {
                                onclick: move |_| {
                                    show_swish_modal.set(None);
                                },
                                class: "absolute top-4 right-4 p-1 rounded-full hover:bg-muted border-0 bg-transparent cursor-pointer text-muted-foreground transition-all",
                                components::LucideIcon { name: "x", size: "16" }
                            }
                            div {
                                class: "w-16 h-16 rounded-2xl bg-[#EB2B7F] flex items-center justify-center text-white shadow-lg shadow-[#EB2B7F]/20 select-none",
                                style: "font-family: 'Outfit', sans-serif; font-weight: 800; font-size: 1.5rem; letter-spacing: -1px;",
                                "swish"
                            }
                            div { class: "space-y-1.5",
                                h3 { class: "text-lg font-bold text-foreground m-0", "Betala med Swish" }
                                p { class: "text-xs text-muted-foreground m-0", "Öppna Swish i din mobil och godkänn betalningen." }
                            }
                            div {
                                class: "p-4 rounded-2xl bg-white border border-border shadow-inner flex items-center justify-center relative group",
                                img {
                                    src: "{qr_code_base64}",
                                    class: "w-44 h-44 select-none",
                                    alt: "Swish Betalning QR Kod"
                                }
                            }
                            div { class: "w-full space-y-2 bg-muted/40 p-3.5 rounded-xl border border-border/30 text-xs text-left",
                                div { class: "flex justify-between",
                                    span { class: "text-muted-foreground", "Mottagare:" }
                                    span { class: "font-semibold text-foreground", "Yntra Flytt AB" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-muted-foreground", "Belopp:" }
                                    span { class: "font-bold text-foreground", "{swish_amount}{currency_suffix}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-muted-foreground", "Referens:" }
                                    span { class: "font-mono font-semibold text-foreground/80", "{current_inv_id}" }
                                }
                            }
                            div { class: "w-full space-y-2 pt-1",
                        if current_sw_status == "paid" {
                            div { class: "flex items-center justify-center gap-2 text-xs font-semibold text-emerald-500",
                                components::LucideIcon { name: "check-circle", class: "h-4 w-4 text-emerald-500 animate-bounce" }
                                span { "Betalning Godkänd!" }
                            }
                        } else if current_sw_status == "timeout" {
                            div { class: "flex flex-col items-center justify-center gap-1 text-xs font-semibold text-amber-500 text-center",
                                div { class: "flex items-center gap-1.5",
                                    components::LucideIcon { name: "clock", class: "h-4 w-4 text-amber-500" }
                                    span { "Tidsgränsen passerades (120s)" }
                                }
                                span { class: "text-[11px] font-normal text-muted-foreground", "Ingen bekräftelse mottogs i tid. Kontrollera din Swish-app eller försök igen." }
                            }
                        } else if current_sw_status == "failed" || current_sw_status == "declined" || current_sw_status == "cancelled" {
                            div { class: "flex flex-col items-center justify-center gap-1 text-xs font-semibold text-destructive text-center",
                                div { class: "flex items-center gap-1.5",
                                    components::LucideIcon { name: "alert-circle", class: "h-4 w-4 text-destructive" }
                                    span { "Betalningen avbröts eller misslyckades" }
                                }
                                span { class: "text-[11px] font-normal text-muted-foreground", "Var god försök igen eller välj ett annat betalsätt." }
                            }
                        } else {
                            div { class: "flex items-center justify-center gap-2 text-xs font-semibold text-primary",
                                div { class: "w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin" }
                                span { "Väntar på BankID-signering... (max 120s)" }
                            }
                        }
                        div { class: "w-full h-1.5 bg-muted rounded-full overflow-hidden border border-border/10",
                            div {
                                class: "h-full bg-primary transition-all duration-300 rounded-full",
                                style: "width: {progress_percent}%;"
                            }
                        }
                    }
                    div { class: "w-full pt-1 flex flex-col gap-2",
                        if current_sw_status == "timeout" || current_sw_status == "failed" || current_sw_status == "declined" || current_sw_status == "cancelled" {
                            button {
                                onclick: {
                                    let inv_id = current_inv_id.clone();
                                    move |_| on_retry_swish(inv_id.clone())
                                },
                                class: "w-full py-2.5 rounded-xl bg-primary hover:opacity-90 text-xs font-bold text-primary-foreground text-center border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5 shadow",
                                components::LucideIcon { name: "refresh-cw", size: "14" }
                                "Generera Ny Swish QR-kod (Försök igen)"
                            }
                            button {
                                onclick: {
                                    let inv_id = current_inv_id.clone();
                                    move |_| on_manual_swish_check(inv_id.clone())
                                },
                                class: "w-full py-2 rounded-xl bg-secondary hover:bg-secondary/80 text-xs font-semibold text-secondary-foreground text-center border border-border/40 cursor-pointer transition-all flex items-center justify-center gap-1.5",
                                components::LucideIcon { name: "search", size: "14" }
                                "Kontrollera status igen (Manuell sökning)"
                            }
                            button {
                                onclick: {
                                    let inv_id = current_inv_id.clone();
                                    move |_| on_fallback_to_card(inv_id.clone())
                                },
                                class: "w-full py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-xs font-semibold text-white text-center border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5 shadow-sm",
                                components::LucideIcon { name: "credit-card", size: "14" }
                                "Växla till Kortbetalning (Stripe)"
                            }
                            button {
                                onclick: move |_| show_swish_modal.set(None),
                                class: "w-full py-1.5 rounded-xl bg-transparent hover:bg-muted/40 text-[11px] font-medium text-muted-foreground border-0 cursor-pointer transition-all",
                                "Stäng fönstret"
                            }
                        } else {
                            a {
                                href: "{swish_url}",
                                class: "w-full py-2.5 rounded-xl bg-primary hover:opacity-90 text-xs font-bold text-primary-foreground text-center no-underline border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5",
                                components::LucideIcon { name: "smartphone", size: "14" }
                                "Öppna Swish-appen"
                            }
                            button {
                                onclick: {
                                    let inv_id = current_inv_id.clone();
                                    move |_| on_complete_swish_payment(inv_id.clone())
                                },
                                class: "w-full py-2 rounded-xl bg-secondary hover:bg-secondary/80 text-[10px] font-bold text-secondary-foreground border-0 cursor-pointer transition-all",
                                "Simulera Godkännande (Test)"
                            }
                        }
                    }
                }
            }
        }

        if let Some(ref stripe_sess) = *show_stripe_modal.read() {
            {
                let invoice_id = current_inv_id.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200",
                        div {
                            class: "bg-sidebar border border-border rounded-2xl w-full max-w-md p-6 shadow-2xl flex flex-col gap-5 relative animate-in zoom-in-95 duration-200",
                            button {
                                onclick: move |_| {
                                    show_stripe_modal.set(None);
                                },
                                class: "absolute top-4 right-4 p-1 rounded-full hover:bg-muted border-0 bg-transparent cursor-pointer text-muted-foreground transition-all",
                                components::LucideIcon { name: "x", size: "16" }
                            }
                            div { class: "flex items-center gap-3 border-b border-border/40 pb-4",
                                div { class: "p-2 rounded-xl bg-primary/10 text-blue-500",
                                    components::LucideIcon { name: "credit-card", class: "h-6 w-6" }
                                }
                                div {
                                    h3 { class: "text-base font-bold text-foreground m-0", "Betala med Stripe" }
                                    p { class: "text-xs text-muted-foreground m-0", "Fyll i dina kortuppgifter för att slutföra betalningen." }
                                }
                            }
                            div { class: "space-y-4 pt-2",
                                div { class: "space-y-1",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "Kortinnehavare" }
                                    input {
                                        r#type: "text",
                                        placeholder: "t.ex. Anna Andersson",
                                        class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                    }
                                }
                                div { class: "space-y-1",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "Kortnummer" }
                                    div { class: "relative",
                                        input {
                                            r#type: "text",
                                            placeholder: "4111 2222 3333 4444",
                                            class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all pl-9",
                                        }
                                        div { class: "absolute left-3 top-2.5 text-muted-foreground",
                                            components::LucideIcon { name: "credit-card", size: "16" }
                                        }
                                    }
                                }
                                div { class: "grid grid-cols-2 gap-4",
                                    div { class: "space-y-1",
                                        label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "Utgångsdatum" }
                                        input {
                                            r#type: "text",
                                            placeholder: "MM/ÅÅ",
                                            class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                        }
                                    }
                                    div { class: "space-y-1",
                                        label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "CVC" }
                                        input {
                                            r#type: "password",
                                            placeholder: "•••",
                                            maxlength: "3",
                                            class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                        }
                                    }
                                }
                            }
                            div { class: "bg-muted/40 p-4 rounded-xl border border-border/30 text-xs flex justify-between items-center mt-2",
                                span { class: "text-muted-foreground", "Att betala:" }
                                span { class: "font-bold text-foreground text-sm", "{stripe_sess.amount}{currency_suffix}" }
                            }
                            button {
                                onclick: {
                                    let inv_id = invoice_id.clone();
                                    move |_| on_complete_stripe_payment(inv_id.clone())
                                },
                                class: "w-full py-3 rounded-xl bg-primary hover:bg-primary/90 text-xs font-bold text-white shadow border-0 cursor-pointer transition-all flex items-center justify-center gap-2 mt-2",
                                components::LucideIcon { name: "check", size: "16" }
                                "Slutför kortbetalning"
                            }
                        }
                    }
                }
            }
        }

        if let Some(ref adyen_sess) = *show_adyen_modal.read() {
            {
                let invoice_id = current_inv_id.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200",
                        div {
                            class: "bg-sidebar border border-border rounded-2xl w-full max-w-md p-6 shadow-2xl flex flex-col gap-5 relative animate-in zoom-in-95 duration-200",
                            button {
                                onclick: move |_| {
                                    show_adyen_modal.set(None);
                                },
                                class: "absolute top-4 right-4 p-1 rounded-full hover:bg-muted border-0 bg-transparent cursor-pointer text-muted-foreground transition-all",
                                components::LucideIcon { name: "x", size: "16" }
                            }
                            div { class: "flex items-center gap-3 border-b border-border/40 pb-4",
                                div { class: "p-2 rounded-xl bg-[#00112C] text-white flex items-center justify-center font-bold text-xs w-10 h-10 select-none",
                                    "Adyen"
                                }
                                div {
                                    h3 { class: "text-base font-bold text-foreground m-0", "Betala med Adyen" }
                                    p { class: "text-xs text-muted-foreground m-0", "Koppla upp säkert mot din internetbank via Adyen." }
                                }
                            }
                            div { class: "space-y-4 pt-2",
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "Välj din bank" }
                                    select {
                                        class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                        option { "Deutsche Bank" }
                                        option { "Commerzbank" }
                                        option { "Sparkasse" }
                                        option { "Volksbank" }
                                    }
                                }
                                div { class: "space-y-1",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider block", "IBAN" }
                                    input {
                                        r#type: "text",
                                        placeholder: "DE89 3704 0044 0532 0130 00",
                                        class: "w-full text-xs p-2.5 rounded-lg border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                    }
                                }
                            }
                            div { class: "bg-muted/40 p-4 rounded-xl border border-border/30 text-xs flex justify-between items-center mt-2",
                                span { class: "text-muted-foreground", "Att betala:" }
                                span { class: "font-bold text-foreground text-sm", "{adyen_sess.amount}{currency_suffix}" }
                            }
                            button {
                                onclick: {
                                    let inv_id = invoice_id.clone();
                                    move |_| on_complete_adyen_payment(inv_id.clone())
                                },
                                class: "w-full py-3 rounded-xl bg-[#00112C] hover:opacity-90 text-xs font-bold text-white shadow border-0 cursor-pointer transition-all flex items-center justify-center gap-2 mt-2",
                                components::LucideIcon { name: "check", size: "16" }
                                "Slutför bankbetalning"
                            }
                        }
                    }
                }
            }
        }
        if let Some(ref j_id) = *show_tracking_job_id.read() {
            crate::views::jobs::live_tracking_modal::CustomerLiveTrackingModal {
                job_id: j_id.clone(),
                active_user_id: props.active_user_id.clone(),
                on_close: move |_| show_tracking_job_id.set(None),
            }
        }
    }
}
