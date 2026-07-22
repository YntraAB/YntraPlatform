use dioxus::prelude::*;
use crate::components;
use crate::locales::t;
use crate::utils::use_action_runner;
use yntra_core::HvacSystemDiagnostic;

#[derive(Props, Clone, PartialEq)]
pub struct HvacModalProps {
    pub show: Signal<bool>,
    pub job_id: String,
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn HvacDiagnosticModal(props: HvacModalProps) -> Element {
    let mut show = props.show;
    let runner = use_action_runner();

    let mut active_tab = use_signal(|| 0usize); // 0: Diagnostic, 1: Parts, 2: ROT Split, 3: History

    // Diagnostic form signals
    let mut refrigerant_type = use_signal(|| "R-410A".to_string());
    let mut charge_level = use_signal(|| "Optimal (95-100%)".to_string());
    let mut high_side_psi = use_signal(|| "320.0".to_string());
    let mut low_side_psi = use_signal(|| "115.0".to_string());
    let mut temp_diff_c = use_signal(|| "12.5".to_string());
    let mut voltage_v = use_signal(|| "230.0".to_string());
    let mut amp_draw_a = use_signal(|| "14.2".to_string());
    let mut diag_notes = use_signal(String::new);

    // ROT Split Calculator signals
    let mut labor_cost_input = use_signal(|| "6500.0".to_string());
    let mut parts_cost_input = use_signal(|| "2800.0".to_string());
    let mut travel_fee_input = use_signal(|| "450.0".to_string());

    let db_trig = props.db_trigger;
    let uid = props.active_user_id.clone();
    let jid = props.job_id.clone();

    // Resource for diagnostics history
    let jid_res = jid.clone();
    let uid_res = uid.clone();
    let diagnostics_res = use_resource(move || {
        let _ = db_trig;
        let u = uid_res.clone();
        let j = jid_res.clone();
        async move {
            yntra_core::get_hvac_job_diagnostics(u, j).await.unwrap_or_default()
        }
    });

    let diagnostics_history: Vec<HvacSystemDiagnostic> = diagnostics_res.read().clone().unwrap_or_default();

    if !*show.read() {
        return rsx! {};
    }

    let labor_val = labor_cost_input.read().parse::<f64>().unwrap_or(0.0);
    let parts_val = parts_cost_input.read().parse::<f64>().unwrap_or(0.0);
    let travel_val = travel_fee_input.read().parse::<f64>().unwrap_or(0.0);

    let rot_deduction = (labor_val * 0.30).min(50000.0);
    let gross_total = labor_val + parts_val + travel_val;
    let net_customer = gross_total - rot_deduction;

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-end bg-black/60 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "h-full w-full max-w-2xl bg-card border-l border-border p-6 flex flex-col gap-6 shadow-2xl overflow-y-auto animate-in slide-in-from-right duration-300",

                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex flex-col gap-1",
                        h3 { class: "text-lg font-bold text-foreground flex items-center gap-2",
                            span { class: "text-primary text-xl", "⚡" }
                            "HVAC & Plumbing System Diagnostic Drawer"
                        }
                        p { class: "text-xs text-muted-foreground",
                            "Work Order #{props.job_id} • Heat Pump & Hydronic On-Site Diagnostic Log"
                        }
                    }
                    button {
                        class: "yntra-btn-ghost text-muted-foreground hover:text-foreground text-sm font-semibold px-2 py-1",
                        onclick: move |_| show.set(false),
                        "✕ Close"
                    }
                }

                // Tabs Navigation
                div { class: "flex border-b border-border gap-2 text-xs font-semibold",
                    button {
                        class: if *active_tab.read() == 0 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(0),
                        "🔧 System Diagnostic"
                    }
                    button {
                        class: if *active_tab.read() == 1 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(1),
                        "🔩 Parts & Fittings Tracker"
                    }
                    button {
                        class: if *active_tab.read() == 2 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(2),
                        "🇸🇪 ROT 30% Tax Credit Split"
                    }
                    button {
                        class: if *active_tab.read() == 3 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2 text-xs" },
                        onclick: move |_| active_tab.set(3),
                        "📜 Inspection History ({diagnostics_history.len()})"
                    }
                }

                // Tab 0: System Diagnostic Form
                if *active_tab.read() == 0 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "grid grid-cols-2 gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Refrigerant / System Medium" }
                                select {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{refrigerant_type}",
                                    onchange: move |e: Event<FormData>| refrigerant_type.set(e.value()),
                                    option { value: "R-410A", "R-410A (Eco Heat Pump)" }
                                    option { value: "R-32", "R-32 (Next-Gen Inverter)" }
                                    option { value: "R-22", "R-22 (Legacy HVAC)" }
                                    option { value: "R-134a", "R-134a (Chiller Medium)" }
                                    option { value: "Hydronic Water", "Hydronic Water (Radiator/Underfloor)" }
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Refrigerant Charge Status" }
                                select {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{charge_level}",
                                    onchange: move |e: Event<FormData>| charge_level.set(e.value()),
                                    option { value: "Optimal (95-100%)", "Optimal (95-100%)" }
                                    option { value: "Undercharged (<80%)", "Undercharged (<80% - Leak Risk)" }
                                    option { value: "Overcharged (>110%)", "Overcharged (>110% - High Pressure)" }
                                }
                            }
                        }

                        div { class: "grid grid-cols-3 gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "High Side Pressure (PSI)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{high_side_psi}",
                                    oninput: move |e: Event<FormData>| high_side_psi.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Low Side Pressure (PSI)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{low_side_psi}",
                                    oninput: move |e: Event<FormData>| low_side_psi.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Delta T Differential (ΔT °C)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{temp_diff_c}",
                                    oninput: move |e: Event<FormData>| temp_diff_c.set(e.value()),
                                }
                            }
                        }

                        div { class: "grid grid-cols-2 gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Electrical Voltage (V)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{voltage_v}",
                                    oninput: move |e: Event<FormData>| voltage_v.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Compressor Amp Draw (A)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{amp_draw_a}",
                                    oninput: move |e: Event<FormData>| amp_draw_a.set(e.value()),
                                }
                            }
                        }

                        div { class: "flex flex-col gap-1",
                            label { class: "font-semibold text-foreground", "Technician Notes & Diagnostics" }
                            textarea {
                                class: "yntra-input p-3 bg-background border border-border text-foreground rounded text-xs h-20 resize-none",
                                placeholder: "Enter technical observations, leak detector results, valve status...",
                                value: "{diag_notes}",
                                oninput: move |e: Event<FormData>| diag_notes.set(e.value()),
                            }
                        }

                        button {
                            class: "yntra-btn py-2 text-xs font-semibold mt-2",
                            onclick: {
                                let runner_c = runner.clone();
                                let requester_uid = uid.clone();
                                let job_ticket_id = jid.clone();
                                let r_type = refrigerant_type.read().clone();
                                let c_level = charge_level.read().clone();
                                let h_psi = high_side_psi.read().parse::<f64>().unwrap_or(320.0);
                                let l_psi = low_side_psi.read().parse::<f64>().unwrap_or(115.0);
                                let t_diff = temp_diff_c.read().parse::<f64>().unwrap_or(12.5);
                                let volt = voltage_v.read().parse::<f64>().unwrap_or(230.0);
                                let amps = amp_draw_a.read().parse::<f64>().unwrap_or(14.2);
                                let notes_opt = if diag_notes.read().trim().is_empty() { None } else { Some(diag_notes.read().clone()) };
                                let mut db_trig_c = db_trig;

                                move |_| {
                                    let runner_inner = runner_c.clone();
                                    let uid_inner = requester_uid.clone();
                                    let jid_inner = job_ticket_id.clone();
                                    let r_type_inner = r_type.clone();
                                    let c_level_inner = c_level.clone();
                                    let notes_inner = notes_opt.clone();

                                    runner_inner.run(async move {
                                        yntra_core::log_hvac_system_diagnostic(
                                            uid_inner,
                                            jid_inner,
                                            r_type_inner,
                                            c_level_inner,
                                            h_psi,
                                            l_psi,
                                            t_diff,
                                            volt,
                                            amps,
                                            notes_inner,
                                        ).await?;
                                        let cur = *db_trig_c.read();
                                        db_trig_c.set(cur + 1);
                                        Ok(())
                                    });
                                }
                            },
                            "💾 Log HVAC System Diagnostic"
                        }
                    }
                }

                // Tab 1: Parts & Materials Inventory Tracker
                if *active_tab.read() == 1 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "p-3 bg-muted/20 border border-border rounded-lg flex flex-col gap-2",
                            h4 { class: "font-bold text-foreground", "On-Site Parts & Fittings Log" }
                            p { class: "text-muted-foreground", "Record materials used on-site for customer job billing and ROT labor separation." }
                        }
                        div { class: "border border-border rounded-lg overflow-hidden",
                            table { class: "w-full text-left border-collapse text-xs",
                                class: "divide-y divide-border",
                                thead { class: "bg-muted/40 font-semibold text-muted-foreground",
                                    tr {
                                        th { class: "p-2.5", "Part Name / Fitting" }
                                        th { class: "p-2.5", "Qty" }
                                        th { class: "p-2.5", "Unit Cost (SEK)" }
                                        th { class: "p-2.5", "ROT Eligible" }
                                    }
                                }
                                tbody { class: "divide-y divide-border text-foreground",
                                    tr {
                                        td { class: "p-2.5 font-medium", "Copper Pipe 3/4\" (2m)" }
                                        td { class: "p-2.5", "2" }
                                        td { class: "p-2.5", "450 SEK" }
                                        td { class: "p-2.5 text-destructive font-bold", "No (Material)" }
                                    }
                                    tr {
                                        td { class: "p-2.5 font-medium", "2-Port Motorized Zone Valve" }
                                        td { class: "p-2.5", "1" }
                                        td { class: "p-2.5", "1,200 SEK" }
                                        td { class: "p-2.5 text-destructive font-bold", "No (Part)" }
                                    }
                                    tr {
                                        td { class: "p-2.5 font-medium", "System Diagnostic & Valve Fitting Labor" }
                                        td { class: "p-2.5", "5 hrs" }
                                        td { class: "p-2.5", "1,300 SEK/hr" }
                                        td { class: "p-2.5 text-emerald-500 font-bold", "Yes (30% ROT)" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Tab 2: Swedish ROT 30% Tax Credit Calculator
                if *active_tab.read() == 2 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "grid grid-cols-3 gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Labor Cost (SEK)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{labor_cost_input}",
                                    oninput: move |e: Event<FormData>| labor_cost_input.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Parts & Materials (SEK)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{parts_cost_input}",
                                    oninput: move |e: Event<FormData>| parts_cost_input.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Travel Fee (SEK)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{travel_fee_input}",
                                    oninput: move |e: Event<FormData>| travel_fee_input.set(e.value()),
                                }
                            }
                        }

                        div { class: "p-4 bg-primary/10 border border-primary/20 rounded-xl flex flex-col gap-3",
                            h4 { class: "font-bold text-foreground text-sm flex items-center justify-between",
                                span { "🇸🇪 Skatteverket ROT 30% Tax Deduction Breakdown" }
                                span { class: "px-2 py-0.5 bg-emerald-500/20 text-emerald-400 rounded text-xs font-semibold", "Skatteverket ROT Valid" }
                            }
                            div { class: "grid grid-cols-2 gap-2 text-xs",
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "Total Gross Service Invoice:" }
                                    span { class: "font-bold text-foreground", "{gross_total:.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "ROT Eligible Labor Amount:" }
                                    span { class: "font-bold text-foreground", "{labor_val:.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "Non-Deductible Parts & Travel:" }
                                    span { class: "font-bold text-foreground", "{(parts_val + travel_val):.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-emerald-400 font-semibold", "Customer ROT 30% Credit:" }
                                    span { class: "font-bold text-emerald-400", "-{rot_deduction:.2} SEK" }
                                }
                            }
                            div { class: "flex items-center justify-between pt-2 border-t border-border/60",
                                span { class: "font-bold text-foreground text-sm", "Net Customer Amount Due On-Site:" }
                                span { class: "font-extrabold text-primary text-base", "{net_customer:.2} SEK" }
                            }
                        }
                    }
                }

                // Tab 3: Inspection History Timeline
                if *active_tab.read() == 3 {
                    div { class: "flex flex-col gap-3 text-xs animate-in fade-in duration-200",
                        if diagnostics_history.is_empty() {
                            div { class: "p-6 border border-dashed border-border rounded-lg text-center text-muted-foreground",
                                "No previous diagnostic inspections recorded for this work order."
                            }
                        } else {
                            for diag in diagnostics_history.iter() {
                                div { key: "{diag.id}", class: "p-3 bg-muted/20 border border-border rounded-lg flex flex-col gap-2",
                                    div { class: "flex items-center justify-between",
                                        span { class: "font-bold text-foreground", "Status: {diag.diagnostic_status}" }
                                        span { class: "text-xs text-muted-foreground", "Refrigerant: {diag.refrigerant_type}" }
                                    }
                                    div { class: "grid grid-cols-3 gap-2 text-muted-foreground text-[11px]",
                                        span { "High Side: {diag.high_side_psi} PSI" }
                                        span { "Low Side: {diag.low_side_psi} PSI" }
                                        span { "ΔT: {diag.temp_differential_c}°C" }
                                    }
                                    if let Some(ref note) = diag.notes {
                                        p { class: "text-foreground text-xs italic bg-background/50 p-2 rounded border border-border/40 mt-1", "{note}" }
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
