use dioxus::prelude::*;
use yntra_core::{
    get_job_tickets, get_move_inventory, get_move_quote, accept_move_quote,
    generate_move_invoice, get_move_invoice, pay_move_invoice,
    create_move_inventory_item, delete_move_inventory_item,
    calculate_and_save_move_quote, initiate_swish_payment, SwishPaymentSession,
};
use crate::components;

#[derive(Clone, Copy, PartialEq)]
struct InventoryTemplate {
    name: &'static str,
    category: &'static str,
    volume: f64,
}

const TEMPLATES: &[InventoryTemplate] = &[
    InventoryTemplate { name: "Flyttkartong", category: "Kartonger", volume: 0.1 },
    InventoryTemplate { name: "Säng (enkel)", category: "Möbler", volume: 0.6 },
    InventoryTemplate { name: "Säng (dubbel)", category: "Möbler", volume: 1.2 },
    InventoryTemplate { name: "Soffa (3-sits)", category: "Möbler", volume: 1.5 },
    InventoryTemplate { name: "Matbord", category: "Möbler", volume: 0.8 },
    InventoryTemplate { name: "Stol", category: "Möbler", volume: 0.2 },
    InventoryTemplate { name: "Garderob", category: "Möbler", volume: 1.0 },
    InventoryTemplate { name: "Bokhylla", category: "Möbler", volume: 0.8 },
    InventoryTemplate { name: "Byrå", category: "Möbler", volume: 0.5 },
    InventoryTemplate { name: "Kyl/Frys", category: "Vitvaror", volume: 1.0 },
    InventoryTemplate { name: "Tvättmaskin", category: "Vitvaror", volume: 0.5 },
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
    let jobs = jobs_res.read().clone().unwrap_or_default();

    let active_job = jobs.first().cloned();
    let active_job_id = active_job
        .as_ref()
        .map(|j| j.id.clone())
        .unwrap_or_default();

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
    let mut show_add_form = use_signal(|| false);
    let mut selected_template_idx = use_signal(|| 999);
    let mut new_item_name = use_signal(String::new);
    let mut new_item_category = use_signal(|| "Möbler".to_string());
    let mut new_item_qty = use_signal(|| 1);
    let mut new_item_vol = use_signal(|| 0.5);
    let mut new_item_notes = use_signal(String::new);
    let mut show_swish_modal = use_signal(|| Option::<SwishPaymentSession>::None);
    let mut swish_polling_seconds = use_signal(|| 0);


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
        spawn(async move {
            if accept_move_quote(uid.clone(), quote_id.clone()).await.is_ok() {
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

    let active_uid_for_pay_c = props.active_user_id.clone();
    let on_pay_invoice = move |invoice_id: String| {
        let uid = active_uid_for_pay_c.clone();
        spawn(async move {
            if let Ok(session) = initiate_swish_payment(uid.clone(), invoice_id.clone()).await {
                show_swish_modal.set(Some(session));
                swish_polling_seconds.set(0);
                
                let invoice_id_c = invoice_id.clone();
                let uid_c = uid.clone();
                spawn(async move {
                    for sec in 1..=4 {
                        crate::utils::sleep_ms(1000).await;
                        if show_swish_modal.read().is_none() {
                            break;
                        }
                        swish_polling_seconds.set(sec);
                        if sec == 4 {
                            if pay_move_invoice(uid_c.clone(), invoice_id_c.clone()).await.is_ok() {
                                let current_val = *db_trigger.read();
                                db_trigger.set(current_val + 1);
                            }
                            show_swish_modal.set(None);
                        }
                    }
                });
            }
        });
    };

    let swish_session_opt = show_swish_modal.read().clone();
    let show_swish = swish_session_opt.is_some();
    let qr_code_base64 = swish_session_opt.as_ref().map(|s| s.qr_code_base64.clone()).unwrap_or_default();
    let swish_amount = swish_session_opt.as_ref().map(|s| s.amount).unwrap_or(0.0);
    let swish_url = swish_session_opt.as_ref().map(|s| s.swish_url.clone()).unwrap_or_default();
    
    let elapsed_sec = *swish_polling_seconds.read();
    let progress_percent = (elapsed_sec as f64 / 4.0 * 100.0) as i64;
    let current_inv_id = invoice.as_ref().map(|i| i.id.clone()).unwrap_or_default();

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
                                    let is_rut = *use_rut.read();
                                    let labor_cost = q.base_price + q.stairs_surcharge;
                                    let rut_reduction = if is_rut { (0.5 * labor_cost as f64) as i64 } else { 0 };
                                    let final_total = q.total_price - rut_reduction;
                                    let hours = q.base_price as f64 / hourly_rate;
                                    rsx! {
                                        div { class: "space-y-3",
                                            div { class: "space-y-1.5",
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    if pricing_model == "hourly" {
                                                        span { "Baspris ({hours:.1} tim à {hourly_rate} kr/tim):" }
                                                    } else {
                                                        span { "Baspris (arbete/tid):" }
                                                    }
                                                    span { class: "font-semibold text-foreground", "{q.base_price} kr" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Distansavgift:" }
                                                    span { class: "font-semibold text-foreground", "{q.distance_fee} kr" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Trapptillägg:" }
                                                    span { class: "font-semibold text-foreground", "{q.stairs_surcharge} kr" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Packmaterial & utrustning:" }
                                                    span { class: "font-semibold text-foreground", "{q.packing_supplies_fee} kr" }
                                                }
                                                if is_rut {
                                                    div { class: "flex items-center justify-between text-xs text-emerald-500 font-semibold",
                                                        span { "Preliminärt RUT-avdrag (50%):" }
                                                        span { "-{rut_reduction} kr" }
                                                    }
                                                }
                                            }

                                            div { class: "flex items-center gap-2 pt-2.5 border-t border-border/20 text-xs text-muted-foreground",
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
                                                label { r#for: "rut_checkbox", class: "font-bold cursor-pointer select-none text-foreground/80 hover:text-foreground", "Ansök om RUT-avdrag" }
                                            }

                                            div { class: "h-px bg-border/40 my-2", }
                                            div { class: "flex items-center justify-between text-sm font-extrabold",
                                                span { if is_rut { "Ditt pris efter RUT:" } else { "Totalt pris:" } }
                                                span { class: "text-primary text-base", "{final_total} kr" }
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
                                                span { "Totalsumma (exkl. RUT):" }
                                                span { class: "font-semibold text-foreground", "{inv.subtotal} kr" }
                                            }
                                            if inv.rut_deduction > 0.0 {
                                                div { class: "flex items-center justify-between text-xs text-emerald-500 font-semibold",
                                                    span { "RUT-avdrag (skattereduktion):" }
                                                    span { "-{inv.rut_deduction} kr" }
                                                }
                                                div { class: "flex items-center justify-between text-xs text-muted-foreground",
                                                    span { "Skatteverket (söks av oss):" }
                                                    span { "{inv.tax_authority_amount} kr" }
                                                }
                                            }
                                            div { class: "h-px bg-border/40 my-1", }
                                            div { class: "flex items-center justify-between text-sm font-extrabold",
                                                span { "Att betala (Kund):" }
                                                span { class: "text-primary text-base", "{inv.customer_amount} kr" }
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
                                                    div { class: "flex items-center justify-center gap-2 p-2.5 rounded-lg bg-blue-500/10 text-blue-500 text-xs font-bold text-center border border-blue-500/20 select-none",
                                                        components::LucideIcon { name: "credit-card", class: "h-4 w-4" }
                                                        span { "Faktura Betald (Swish)" }
                                                    }
                                                } else {
                                                    button {
                                                        onclick: {
                                                            let inv_id = inv.id.clone();
                                                            move |_| on_pay_invoice(inv_id.clone())
                                                        },
                                                        class: "w-full rounded-lg bg-emerald-500 text-white py-2.5 text-xs font-bold shadow hover:opacity-90 hover:scale-[1.01] transition-all border-0 cursor-pointer flex items-center justify-center gap-2 select-none",
                                                        components::LucideIcon { name: "credit-card", class: "h-4 w-4" }
                                                        "Betala med Swish"
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
            div { class: "grid w-full max-w-md grid-cols-2 bg-muted/50 rounded-xl p-1 mb-6 border border-border",
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
                                                            label { class: "text-[10px] text-muted-foreground block mb-0.5", "Välj typ av föremål" }
                                                            select {
                                                                value: "{selected_template_idx}",
                                                                onchange: move |e: FormEvent| {
                                                                    let idx_str = e.value();
                                                                    let idx: usize = idx_str.parse().unwrap_or(0);
                                                                    selected_template_idx.set(idx);
                                                                    if idx < TEMPLATES.len() {
                                                                        new_item_name.set(TEMPLATES[idx].name.to_string());
                                                                        new_item_category.set(TEMPLATES[idx].category.to_string());
                                                                        new_item_vol.set(TEMPLATES[idx].volume);
                                                                    } else {
                                                                        // Custom selection
                                                                        new_item_name.set(String::new());
                                                                        new_item_category.set("Möbler".to_string());
                                                                        new_item_vol.set(0.5);
                                                                    }
                                                                },
                                                                class: "w-full text-xs p-1.5 rounded border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                                option { value: "999", "Annan möbel eller låda (anpassad)..." }
                                                                for (i, t) in TEMPLATES.iter().enumerate() {
                                                                    option { value: "{i}", "{t.name} ({t.volume} m³)" }
                                                                }
                                                            }
                                                        }

                                                        // If Custom selected, show custom inputs
                                                        if *selected_template_idx.read() == 999 {
                                                            div { class: "flex flex-col gap-2",
                                                                div { class: "grid grid-cols-2 gap-2",
                                                                    div { class: "col-span-2 sm:col-span-1",
                                                                        label { class: "text-[10px] text-muted-foreground block mb-0.5", "Föremålsnamn" }
                                                                        input {
                                                                            r#type: "text",
                                                                            placeholder: "t.ex. Piano, Byrå",
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
                            span { class: "font-bold text-foreground", "{swish_amount} kr" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-muted-foreground", "Referens:" }
                            span { class: "font-mono font-semibold text-foreground/80", "{current_inv_id}" }
                        }
                    }
                    div { class: "w-full space-y-2 pt-1",
                        div { class: "flex items-center justify-center gap-2 text-xs font-semibold text-primary",
                            if elapsed_sec < 4 {
                                div { class: "w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin" }
                                span { "Väntar på BankID-signering..." }
                            } else {
                                components::LucideIcon { name: "check-circle", class: "h-4 w-4 text-emerald-500 animate-bounce" }
                                span { class: "text-emerald-500", "Betalning Godkänd!" }
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
}

