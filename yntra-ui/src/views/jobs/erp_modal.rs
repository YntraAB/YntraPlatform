use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct ErpSyncModalProps {
    pub job_id: String,
    pub invoice_id: Option<String>,
    pub active_user_id: String,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn ErpSyncModal(props: ErpSyncModalProps) -> Element {
    let job_id = props.job_id;
    let invoice_id = props.invoice_id;
    let active_user_id = props.active_user_id;
    let on_close = props.on_close;

    let mut selected_provider = use_signal(|| "quickbooks".to_string());
    let mut active_tab = use_signal(|| "invoices".to_string());
    let mut is_syncing = use_signal(|| false);
    let mut sync_message = use_signal(|| Option::<String>::None);
    let mut sync_error = use_signal(|| Option::<String>::None);

    let mut vehicle_id_input = use_signal(|| "TRUCK-01".to_string());
    let mut fuel_liters_input = use_signal(|| "65.5".to_string());
    let mut fuel_cost_input = use_signal(|| "1420.0".to_string());
    let mut fuel_type_input = use_signal(|| "Diesel".to_string());
    let mut odometer_input = use_signal(|| "142500".to_string());
    let mut station_input = use_signal(|| "Circle K E4 Nyköping".to_string());

    let u_gl = active_user_id.clone();
    let mut gl_res = use_resource(move || {
        let u = u_gl.clone();
        async move {
            yntra_core::get_accounting_general_ledger_summary(u).await.ok()
        }
    });

    let u_logs = active_user_id.clone();
    let mut logs_res = use_resource(move || {
        let u = u_logs.clone();
        async move {
            yntra_core::get_erp_sync_history(u).await.unwrap_or_default()
        }
    });

    let u_fuel = active_user_id.clone();
    let mut fuel_res = use_resource(move || {
        let u = u_fuel.clone();
        async move {
            yntra_core::get_fleet_fuel_receipts(u).await.unwrap_or_default()
        }
    });

    let inv_id_clone = invoice_id.clone();

    let ar_str = match gl_res.read().as_ref() {
        Some(Some(gl)) => format!("{:.2} SEK", gl.total_accounts_receivable),
        _ => "0.00 SEK".to_string(),
    };
    let rut_str = match gl_res.read().as_ref() {
        Some(Some(gl)) => format!("{:.2} SEK", gl.total_rut_tax_claims_pending),
        _ => "0.00 SEK".to_string(),
    };
    let pay_str = match gl_res.read().as_ref() {
        Some(Some(gl)) => format!("{:.2} SEK", gl.total_payroll_liabilities),
        _ => "0.00 SEK".to_string(),
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-slate-900/80 backdrop-blur-sm flex items-center justify-center p-4 overflow-y-auto animate-fade-in",
            onclick: move |_| on_close.call(()),

            div {
                class: "relative bg-slate-900 border border-slate-700/80 rounded-2xl shadow-2xl max-w-4xl w-full p-6 text-slate-100 overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between border-b border-slate-800 pb-4 mb-6",
                    div {
                        class: "flex items-center gap-3",
                        div {
                            class: "w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 font-bold text-xl",
                            "⇄"
                        }
                        div {
                            h2 { class: "text-xl font-bold tracking-tight text-white", "Bokföring & ERP Direktintegrering" }
                            p { class: "text-xs text-slate-400", "Automatisk synkronisering mot QuickBooks Online, Xero, Fortnox, Visma och Sage" }
                        }
                    }
                    button {
                        class: "text-slate-400 hover:text-white transition-colors p-2 rounded-lg hover:bg-slate-800",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Provider Selector & GL Summary Cards
                div {
                    class: "grid grid-cols-1 md:grid-cols-4 gap-4 mb-6",

                    // Provider Dropdown Card
                    div {
                        class: "md:col-span-1 bg-slate-800/60 border border-slate-700/60 rounded-xl p-4 flex flex-col justify-between",
                        label { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider block mb-2", "Aktivt ERP-System" }
                        select {
                            class: "w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-emerald-400 font-semibold focus:outline-none focus:border-emerald-500",
                            value: "{selected_provider}",
                            onchange: move |e| selected_provider.set(e.value()),
                            option { value: "quickbooks", "QuickBooks Online" }
                            option { value: "xero", "Xero Accounting" }
                            option { value: "fortnox", "Fortnox Sweden" }
                            option { value: "visma", "Visma eEkonomi" }
                            option { value: "sage", "Sage Business Cloud" }
                        }
                    }

                    // GL Summary Metrics
                    if gl_res.read().as_ref().and_then(|opt| opt.as_ref()).is_some() {
                        div {
                            class: "bg-slate-800/60 border border-slate-700/60 rounded-xl p-4",
                            span { class: "text-xs text-slate-400 block", "Kundfordringar (A/R)" }
                            span { class: "text-lg font-extrabold text-white mt-1 block", "{ar_str}" }
                            span { class: "text-[10px] text-emerald-400", "Fakturerade uppdrag" }
                        }
                        div {
                            class: "bg-slate-800/60 border border-slate-700/60 rounded-xl p-4",
                            span { class: "text-xs text-slate-400 block", "RUT-Skattefordringar" }
                            span { class: "text-lg font-extrabold text-emerald-400 mt-1 block", "{rut_str}" }
                            span { class: "text-[10px] text-slate-400", "Skatteverket krav" }
                        }
                        div {
                            class: "bg-slate-800/60 border border-slate-700/60 rounded-xl p-4",
                            span { class: "text-xs text-slate-400 block", "Beräknade Löneskulder" }
                            span { class: "text-lg font-extrabold text-indigo-400 mt-1 block", "{pay_str}" }
                            span { class: "text-[10px] text-slate-400", "Konto 5000 / 2710" }
                        }
                    } else {
                        div { class: "md:col-span-3 bg-slate-800/40 rounded-xl p-4 text-center text-slate-400 text-sm", "Laddar Huvudbok..." }
                    }
                }

                // Main Content Tabs Header
                div {
                    class: "flex items-center gap-2 border-b border-slate-800 mb-6 pb-2",
                    button {
                        class: if *active_tab.read() == "invoices" { "px-4 py-2 text-xs font-bold text-emerald-400 border-b-2 border-emerald-400 bg-emerald-500/10 rounded-t-lg" } else { "px-4 py-2 text-xs font-semibold text-slate-400 hover:text-white transition-colors" },
                        onclick: move |_| active_tab.set("invoices".to_string()),
                        "Fakturasynk"
                    }
                    button {
                        class: if *active_tab.read() == "fuel" { "px-4 py-2 text-xs font-bold text-emerald-400 border-b-2 border-emerald-400 bg-emerald-500/10 rounded-t-lg" } else { "px-4 py-2 text-xs font-semibold text-slate-400 hover:text-white transition-colors" },
                        onclick: move |_| active_tab.set("fuel".to_string()),
                        "Drivmedel & Utlägg"
                    }
                    button {
                        class: if *active_tab.read() == "payroll" { "px-4 py-2 text-xs font-bold text-emerald-400 border-b-2 border-emerald-400 bg-emerald-500/10 rounded-t-lg" } else { "px-4 py-2 text-xs font-semibold text-slate-400 hover:text-white transition-colors" },
                        onclick: move |_| active_tab.set("payroll".to_string()),
                        "Lönejournal (Huvudbok)"
                    }
                    button {
                        class: if *active_tab.read() == "audit" { "px-4 py-2 text-xs font-bold text-emerald-400 border-b-2 border-emerald-400 bg-emerald-500/10 rounded-t-lg" } else { "px-4 py-2 text-xs font-semibold text-slate-400 hover:text-white transition-colors" },
                        onclick: move |_| active_tab.set("audit".to_string()),
                        "Synkhistorik & Loggar"
                    }
                }

                // Error / Success Notifications
                if let Some(ref err) = *sync_error.read() {
                    div {
                        class: "p-3 bg-red-500/10 border border-red-500/30 text-red-400 rounded-xl text-xs font-semibold mb-4 flex items-center justify-between",
                        "{err}"
                        button { class: "text-red-400 hover:text-white ml-2", onclick: move |_| sync_error.set(None), "✕" }
                    }
                }
                if let Some(ref msg) = *sync_message.read() {
                    div {
                        class: "p-3 bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 rounded-xl text-xs font-semibold mb-4 flex items-center justify-between",
                        "{msg}"
                        button { class: "text-emerald-400 hover:text-white ml-2", onclick: move |_| sync_message.set(None), "✕" }
                    }
                }

                // Tab Content Body
                match active_tab.read().as_str() {
                    "invoices" => rsx! {
                        div {
                            class: "p-4 bg-slate-800/50 border border-slate-700/60 rounded-xl space-y-4",
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    h4 { class: "font-semibold text-white text-base", "Faktura & RUT-Skatteavdrag Synk" }
                                    p { class: "text-xs text-slate-400 mt-1", "Exportera bokförda flyttfakturor, trapp/packtillägg och RUT-skattefordringar till ERP Intäktskonto (Konto 3050)." }
                                }
                                if let Some(ref inv_id) = inv_id_clone {
                                    button {
                                        class: "px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-xl shadow-lg transition-colors flex items-center gap-2",
                                        disabled: *is_syncing.read(),
                                        onclick: {
                                            let active_uid = active_user_id.clone();
                                            let inv_opt = inv_id_clone.clone();
                                            move |_| {
                                                let uid = active_uid.clone();
                                                let provider = selected_provider.read().clone();
                                                let inv_o = inv_opt.clone();
                                                is_syncing.set(true);
                                                spawn(async move {
                                                    if let Some(inv_id) = inv_o {
                                                        let _ = yntra_core::sync_invoice_to_erp(uid, inv_id, provider).await;
                                                    }
                                                    is_syncing.set(false);
                                                    gl_res.restart();
                                                    logs_res.restart();
                                                });
                                            }
                                        },
                                        "⇄ Synka Faktura ({inv_id})"
                                    }
                                } else {
                                    span { class: "text-xs text-amber-400 font-semibold bg-amber-500/10 border border-amber-500/30 px-3 py-1.5 rounded-lg", "Ingen skapad faktura hittades" }
                                }
                            }
                        }
                    },
                    "fuel" => rsx! {
                        div {
                            class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                            // Fuel Form
                            div {
                                class: "md:col-span-1 bg-slate-800/50 border border-slate-700/60 rounded-xl p-4 space-y-3",
                                h4 { class: "font-semibold text-white text-sm border-b border-slate-700 pb-2", "Registrera Tankningskvitto" }
                                div {
                                    label { class: "text-[11px] text-slate-400 block mb-1", "Fordon ID" }
                                    input {
                                        class: "w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white",
                                        value: "{vehicle_id_input}",
                                        oninput: move |e| vehicle_id_input.set(e.value()),
                                    }
                                }
                                div {
                                    class: "grid grid-cols-2 gap-2",
                                    div {
                                        label { class: "text-[11px] text-slate-400 block mb-1", "Volym (Liter)" }
                                        input {
                                            class: "w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white",
                                            value: "{fuel_liters_input}",
                                            oninput: move |e| fuel_liters_input.set(e.value()),
                                        }
                                    }
                                    div {
                                        label { class: "text-[11px] text-slate-400 block mb-1", "Kostnad (SEK)" }
                                        input {
                                            class: "w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white",
                                            value: "{fuel_cost_input}",
                                            oninput: move |e| fuel_cost_input.set(e.value()),
                                        }
                                    }
                                }
                                div {
                                    label { class: "text-[11px] text-slate-400 block mb-1", "Bensinmack / Station" }
                                    input {
                                        class: "w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white",
                                        value: "{station_input}",
                                        oninput: move |e| station_input.set(e.value()),
                                    }
                                }
                                button {
                                    class: "w-full py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-lg transition-colors mt-2",
                                    onclick: {
                                        let active_uid = active_user_id.clone();
                                        move |_| {
                                            let uid = active_uid.clone();
                                            let v_id = vehicle_id_input.read().clone();
                                            let f_type = fuel_type_input.read().clone();
                                            let odo = odometer_input.read().parse::<i64>().unwrap_or(142500);
                                            let st_name = station_input.read().clone();
                                            let liters = fuel_liters_input.read().parse::<f64>().unwrap_or(65.0);
                                            let cost = fuel_cost_input.read().parse::<f64>().unwrap_or(1430.0);
                                            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                                            is_syncing.set(true);
                                            spawn(async move {
                                                let _ = yntra_core::record_fleet_fuel_receipt(
                                                    uid, v_id, liters, cost, f_type, odo, None, Some(st_name), today
                                                ).await;
                                                is_syncing.set(false);
                                                fuel_res.restart();
                                                gl_res.restart();
                                            });
                                        }
                                    },
                                    "+ Spara Kvitto"
                                }
                            }
                            // Fuel Receipts List & Batch Sync
                            div {
                                class: "md:col-span-2 bg-slate-800/50 border border-slate-700/60 rounded-xl p-4 flex flex-col justify-between space-y-4",
                                div {
                                    div {
                                        class: "flex items-center justify-between mb-3",
                                        h4 { class: "font-semibold text-white text-sm", "Registrerade Tankningskvitton" }
                                        button {
                                            class: "px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow transition-colors",
                                            disabled: *is_syncing.read(),
                                            onclick: {
                                                let active_uid = active_user_id.clone();
                                                move |_| {
                                                    let uid = active_uid.clone();
                                                    let provider = selected_provider.read().clone();
                                                    is_syncing.set(true);
                                                    spawn(async move {
                                                        let _ = yntra_core::sync_fuel_receipts_to_erp(uid, provider).await;
                                                        is_syncing.set(false);
                                                        fuel_res.restart();
                                                        logs_res.restart();
                                                        gl_res.restart();
                                                    });
                                                }
                                            },
                                            "Batch-synka Oskickade till ERP"
                                        }
                                    }
                                    if let Some(receipts) = fuel_res.read().as_ref() {
                                        if receipts.is_empty() {
                                            p { class: "text-xs text-slate-400 py-4 text-center", "Inga registrerade drivmedelskvitton hittades." }
                                        } else {
                                             div {
                                                class: "space-y-2 max-h-48 overflow-y-auto pr-1",
                                                for rec in receipts.iter() {
                                                    {
                                                        let rec_label = format!("{} - {:.2} SEK", rec.vehicle_id, rec.cost_sek);
                                                        let rec_sub = format!("{}L {} • {} • {}", rec.liters, rec.fuel_type, rec.station_name.as_deref().unwrap_or("Station"), rec.purchase_date);
                                                        rsx! {
                                                            div {
                                                                key: "{rec.id}",
                                                                class: "p-2.5 bg-slate-900/60 border border-slate-700/40 rounded-lg flex items-center justify-between text-xs",
                                                                div {
                                                                    span { class: "font-semibold text-white block", "{rec_label}" }
                                                                    span { class: "text-[10px] text-slate-400 block", "{rec_sub}" }
                                                                }
                                                                span {
                                                                    class: if rec.erp_sync_status == "synced" { "px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/20 text-emerald-400" } else { "px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/20 text-amber-400" },
                                                                    "{rec.erp_sync_status}"
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
                    "payroll" => rsx! {
                        div {
                            class: "p-4 bg-slate-800/50 border border-slate-700/60 rounded-xl space-y-4",
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    h4 { class: "font-semibold text-white text-base", "Lönejournal & Traktamentsjournal" }
                                    p { class: "text-xs text-slate-400 mt-1", "Exportera månadens arbetade timmar, övertid, förarlön, traktamenten och dricks till ERP Huvudbok (Konto 5000 & 2710)." }
                                }
                                button {
                                    class: "px-5 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white text-sm font-semibold rounded-xl shadow-lg shadow-indigo-600/20 transition-all flex items-center gap-2",
                                    disabled: *is_syncing.read(),
                                    onclick: {
                                        let active_uid = active_user_id.clone();
                                        move |_| {
                                            let uid = active_uid.clone();
                                            let provider = selected_provider.read().clone();
                                            let p_start = chrono::Utc::now().format("%Y-%m-01").to_string();
                                            let p_end = chrono::Utc::now().format("%Y-%m-28").to_string();
                                            is_syncing.set(true);
                                            sync_error.set(None);
                                            sync_message.set(None);
                                            spawn(async move {
                                                match yntra_core::sync_payroll_journal_to_erp(uid, p_start, p_end, provider.clone()).await {
                                                    Ok(res) => {
                                                        sync_message.set(Some(format!("Lönejournal synkad (Konto {}): {}", res.ledger_account, res.message)));
                                                        logs_res.restart();
                                                        gl_res.restart();
                                                    }
                                                    Err(e) => {
                                                        sync_error.set(Some(format!("Lönesynk misslyckades: {}", e)));
                                                    }
                                                }
                                                is_syncing.set(false);
                                            });
                                        }
                                    },
                                    if *is_syncing.read() { "Synkar Löner..." } else { "Synka Löner till {selected_provider.read().to_uppercase()}" }
                                }
                            }
                        }
                    },
                    "logs" => rsx! {
                        div {
                            class: "space-y-3",
                            h4 { class: "font-semibold text-white text-sm mb-2", "Historisk Synkroniseringslogg" }
                            if let Some(logs) = logs_res.read().as_ref() {
                                if logs.is_empty() {
                                    p { class: "text-xs text-slate-400 py-6 text-center bg-slate-800/40 rounded-xl", "Inga tidigare ERP-synkroniseringar har registrerats." }
                                } else {
                                    div {
                                        class: "space-y-2 max-h-60 overflow-y-auto pr-1",
                                        for log in logs.iter() {
                                            div {
                                                key: "{log.id}",
                                                class: "p-3 bg-slate-800/40 border border-slate-700/50 rounded-xl flex items-center justify-between text-xs",
                                                div {
                                                    div { class: "flex items-center gap-2",
                                                        span { class: "font-bold text-white", "{log.erp_provider.to_uppercase()}" }
                                                        span { class: "text-slate-400", "Ref: {log.erp_invoice_number}" }
                                                    }
                                                    span { class: "text-[11px] text-slate-400 block mt-0.5", "Konto: {log.ledger_account} • {log.synced_at}" }
                                                }
                                                span {
                                                    class: if log.status == "synced" { "px-2.5 py-1 rounded-full text-[11px] font-semibold bg-emerald-500/20 text-emerald-400" } else { "px-2.5 py-1 rounded-full text-[11px] font-semibold bg-rose-500/20 text-rose-400" },
                                                    "{log.status}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => rsx! {}
                }

                // Modal Footer
                div {
                    class: "mt-6 pt-4 border-t border-slate-800 flex justify-end",
                    button {
                        class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-xl transition-colors",
                        onclick: move |_| on_close.call(()),
                        "Stäng"
                    }
                }
            }
        }
    }
}
