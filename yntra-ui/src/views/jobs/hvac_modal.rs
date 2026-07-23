use dioxus::prelude::*;
use crate::locales::{t, t_with_args};
use crate::state::AppState;
use crate::utils::use_action_runner;
use yntra_core::{HvacSystemDiagnostic, JobPartItem, RotInvoiceSplitBreakdown};

#[derive(Props, Clone, PartialEq)]
pub struct HvacModalProps {
    pub show: Signal<bool>,
    pub job_id: String,
    pub active_user_id: String,
    pub db_trigger: Signal<u32>,
}

#[component]
pub fn HvacDiagnosticModal(props: HvacModalProps) -> Element {
    let state = use_context::<AppState>();
    let region = state.current_locale();
    let mut show = props.show;
    let runner = use_action_runner();

    let mut active_tab = use_signal(|| 0usize); // 0: Diagnostic, 1: Parts, 2: ROT Split, 3: History
    let mut unit_system = use_signal(|| "METRIC".to_string()); // "METRIC" or "IMPERIAL"

    // Diagnostic form signals - default to empty strings
    let mut system_type = use_signal(|| "REFRIGERANT_HVAC".to_string());
    let mut operating_mode = use_signal(|| "COOLING_MODE".to_string());
    let mut ambient_temp_c = use_signal(String::new);
    let mut refrigerant_type = use_signal(|| "R-410A".to_string());
    let mut charge_level = use_signal(|| "Optimal (95-100%)".to_string());
    let mut high_side_psi = use_signal(String::new);
    let mut low_side_psi = use_signal(String::new);
    let mut water_pressure_bar = use_signal(String::new);
    let mut temp_diff_c = use_signal(String::new);
    let mut voltage_v = use_signal(String::new);
    let mut amp_draw_a = use_signal(String::new);
    let mut asset_id = use_signal(String::new);
    let mut diag_notes = use_signal(String::new);
    let mut form_baseline_initialized = use_signal(|| false);

    // New Part input signals
    let mut part_name_input = use_signal(String::new);
    let mut part_qty_input = use_signal(|| "1.0".to_string());
    let mut part_unit_cost_input = use_signal(|| "450.0".to_string());
    let mut part_rot_eligible = use_signal(|| false);

    // ROT Split Calculator signals
    let mut labor_cost_input = use_signal(String::new);
    let mut travel_fee_input = use_signal(String::new);

    let db_trig_c = props.db_trigger;
    let job_id = props.job_id.clone();
    let active_user_id = props.active_user_id.clone();

    let location_assets_res = use_resource(move || {
        let uid = active_user_id.clone();
        let jid = job_id.clone();
        async move {
            yntra_core::get_hvac_location_assets(uid, jid).await.unwrap_or_default()
        }
    });
    let location_assets = location_assets_res.read().clone().unwrap_or_default();

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

    // Resource for job parts used
    let jid_parts_res = jid.clone();
    let uid_parts_res = uid.clone();
    let parts_res = use_resource(move || {
        let _ = db_trig;
        let u = uid_parts_res.clone();
        let j = jid_parts_res.clone();
        async move {
            yntra_core::get_job_parts_used(u, j).await.unwrap_or_default()
        }
    });

    let diagnostics_history: Vec<HvacSystemDiagnostic> = diagnostics_res.read().clone().unwrap_or_default();
    let job_parts: Vec<JobPartItem> = parts_res.read().clone().unwrap_or_default();

    // Retrieve last recorded baseline readings for the active work order if available
    if !*form_baseline_initialized.read() && !diagnostics_history.is_empty() {
        if let Some(latest) = diagnostics_history.first() {
            system_type.set(latest.system_type.clone());
            if let Some(ref mode) = latest.operating_mode {
                operating_mode.set(mode.clone());
            }
            refrigerant_type.set(latest.refrigerant_type.clone());
            charge_level.set(latest.refrigerant_charge_level.clone());
            if let Some(ref asset) = latest.asset_id {
                asset_id.set(asset.clone());
            }
            let is_m = *unit_system.read() == "METRIC";
            if is_m {
                high_side_psi.set(format!("{:.2}", latest.high_side_psi / 14.5038));
                low_side_psi.set(format!("{:.2}", latest.low_side_psi / 14.5038));
                water_pressure_bar.set(format!("{:.2}", latest.water_pressure_bar));
                temp_diff_c.set(format!("{:.1}", latest.temp_differential_c));
                if let Some(amb) = latest.ambient_temp_c {
                    ambient_temp_c.set(format!("{:.1}", amb));
                }
            } else {
                high_side_psi.set(format!("{:.1}", latest.high_side_psi));
                low_side_psi.set(format!("{:.1}", latest.low_side_psi));
                water_pressure_bar.set(format!("{:.1}", latest.water_pressure_bar * 14.5038));
                temp_diff_c.set(format!("{:.1}", latest.temp_differential_c * 1.8));
                if let Some(amb) = latest.ambient_temp_c {
                    ambient_temp_c.set(format!("{:.1}", amb * 1.8 + 32.0));
                }
            }
            voltage_v.set(format!("{:.1}", latest.voltage_v));
            amp_draw_a.set(format!("{:.1}", latest.amp_draw_a));
            form_baseline_initialized.set(true);
        }
    }

    if !*show.read() {
        return rsx! {};
    }

    // Auto calculate parts totals split by ROT eligibility
    let eligible_parts_cost_total: f64 = job_parts.iter().filter(|p| p.rot_eligible).map(|p| p.quantity * p.unit_cost_sek).sum();
    let non_eligible_parts_cost_total: f64 = job_parts.iter().filter(|p| !p.rot_eligible).map(|p| p.quantity * p.unit_cost_sek).sum();

    // Calculate ROT split reactively without blocking UI thread
    let rot_breakdown = use_memo(move || {
        let labor_val = labor_cost_input.read().parse::<f64>().unwrap_or(0.0);
        let travel_val = travel_fee_input.read().parse::<f64>().unwrap_or(0.0);
        let eligible_labor = labor_val.max(0.0);
        let eligible_parts = eligible_parts_cost_total.max(0.0);
        let rot_base = eligible_labor + eligible_parts;
        let non_eligible = non_eligible_parts_cost_total.max(0.0) + travel_val.max(0.0);
        let gross = rot_base + non_eligible;
        let deduction = (rot_base * 0.30).min(50000.0);
        RotInvoiceSplitBreakdown {
            total_gross_amount_sek: gross,
            eligible_labor_sek: eligible_labor,
            eligible_parts_sek: eligible_parts,
            non_eligible_parts_sek: non_eligible,
            rot_deduction_30_percent_sek: deduction,
            net_customer_payable_sek: gross - deduction,
            max_annual_rot_cap_remaining_sek: (50000.0 - deduction).max(0.0),
            rot_eligible_flag: rot_base > 0.0,
        }
    });
    let rot_info = rot_breakdown.read().clone();

    let job_id_str = props.job_id.clone();
    let subtitle_text = t_with_args("hvac-work-order-subtitle", &region, &[("id", &job_id_str)]);
    let history_count_str = diagnostics_history.len().to_string();
    let history_tab_text = t_with_args("hvac-tab-history", &region, &[("count", &history_count_str)]);

    let is_metric = *unit_system.read() == "METRIC";
    let high_psi_label = if is_metric { "High Side Pressure (Bar)" } else { "High Side Pressure (PSI)" };
    let low_psi_label = if is_metric { "Low Side Pressure (Bar)" } else { "Low Side Pressure (PSI)" };
    let water_press_label = if is_metric { "Water / Hydronic Pressure (Bar)" } else { "Water / Hydronic Pressure (PSI)" };
    let delta_t_label = if is_metric { "Delta T Differential (ΔT °C)" } else { "Delta T Differential (ΔT °F)" };
    let ambient_temp_label = if is_metric { "Outdoor Ambient Temp (°C)" } else { "Outdoor Ambient Temp (°F)" };

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-end bg-black/60 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "h-full w-full max-w-2xl bg-card border-l border-border p-6 flex flex-col gap-6 shadow-2xl overflow-y-auto animate-in slide-in-from-right duration-300",

                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex flex-col gap-1",
                        h3 { class: "text-lg font-bold text-foreground flex items-center gap-2",
                            span { class: "text-primary text-xl", "⚡" }
                            "{t(\"hvac-drawer-title\", &region)}"
                        }
                        p { class: "text-xs text-muted-foreground",
                            "{subtitle_text}"
                        }
                    }
                    div { class: "flex items-center gap-3",
                        div { class: "flex items-center bg-muted/30 p-1 rounded-lg border border-border text-[11px]",
                            button {
                                class: if *unit_system.read() == "METRIC" { "px-2 py-0.5 bg-primary text-primary-foreground font-bold rounded shadow-xs" } else { "px-2 py-0.5 text-muted-foreground hover:text-foreground font-medium" },
                                onclick: move |_| unit_system.set("METRIC".to_string()),
                                "🌐 Metric (Bar/°C)"
                            }
                            button {
                                class: if *unit_system.read() == "IMPERIAL" { "px-2 py-0.5 bg-primary text-primary-foreground font-bold rounded shadow-xs" } else { "px-2 py-0.5 text-muted-foreground hover:text-foreground font-medium" },
                                onclick: move |_| unit_system.set("IMPERIAL".to_string()),
                                "🇺🇸 Imperial (PSI/°F)"
                            }
                        }
                        button {
                            class: "yntra-btn-ghost text-muted-foreground hover:text-foreground text-sm font-semibold px-2 py-1",
                            onclick: move |_| show.set(false),
                            "✕"
                        }
                    }
                }

                // Tabs Navigation
                div { class: "flex border-b border-border gap-2 text-xs font-semibold",
                    button {
                        class: if *active_tab.read() == 0 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(0),
                        "{t(\"hvac-tab-diagnostic\", &region)}"
                    }
                    button {
                        class: if *active_tab.read() == 1 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(1),
                        "{t(\"hvac-tab-parts\", &region)}"
                    }
                    button {
                        class: if *active_tab.read() == 2 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2" },
                        onclick: move |_| active_tab.set(2),
                        "{t(\"hvac-tab-rot\", &region)}"
                    }
                    button {
                        class: if *active_tab.read() == 3 { "border-b-2 border-primary text-primary pb-2 font-bold" } else { "text-muted-foreground hover:text-foreground pb-2 text-xs" },
                        onclick: move |_| active_tab.set(3),
                        "{history_tab_text}"
                    }
                }

                // Tab 0: System Diagnostic Form
                if *active_tab.read() == 0 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                            div { class: "grid grid-cols-2 gap-3",
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground", "{t(\"hvac-system-type\", &region)}" }
                                    select {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{system_type}",
                                        onchange: move |e: Event<FormData>| system_type.set(e.value()),
                                        option { value: "REFRIGERANT_HVAC", "{t(\"hvac-type-refrigerant\", &region)}" }
                                        option { value: "HYDRONIC_HEATING", "{t(\"hvac-type-hydronic\", &region)}" }
                                        option { value: "POTABLE_WATER", "{t(\"hvac-type-potable\", &region)}" }
                                    }
                                }
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground flex items-center justify-between",
                                        span { "{t(\"hvac-asset-id\", &region)}" }
                                        span { class: "text-[10px] text-muted-foreground font-normal", "📍 Location Assets" }
                                    }
                                    select {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{asset_id}",
                                        onchange: move |e: Event<FormData>| asset_id.set(e.value()),
                                        option { value: "", "-- Select Registered Location Equipment --" }
                                        for asset in location_assets.iter() {
                                            option { value: "{asset.asset_tag} - {asset.model_name} ({asset.serial_number})", "[{asset.asset_tag}] {asset.model_name} ({asset.serial_number})" }
                                        }
                                        option { value: "UNLISTED_EQUIPMENT", "➕ Custom / Unlisted Equipment" }
                                    }
                                }
                            }

                            div { class: "grid grid-cols-2 gap-3 p-3 bg-muted/20 border border-border rounded-lg",
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground", "{t(\"hvac-op-mode\", &region)}" }
                                    select {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{operating_mode}",
                                        onchange: move |e: Event<FormData>| operating_mode.set(e.value()),
                                        option { value: "COOLING_MODE", "{t(\"hvac-mode-cooling\", &region)}" }
                                        option { value: "HEATING_MODE", "{t(\"hvac-mode-heating\", &region)}" }
                                    }
                                }
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground", "{ambient_temp_label}" }
                                    input {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        placeholder: if is_metric { "e.g. 20.0" } else { "e.g. 68.0" },
                                        value: "{ambient_temp_c}",
                                        oninput: move |e: Event<FormData>| ambient_temp_c.set(e.value()),
                                    }
                                }
                            }

                            if *system_type.read() == "HYDRONIC_HEATING" || *system_type.read() == "HYDRONIC_PLUMBING" || *system_type.read() == "POTABLE_WATER" || *system_type.read() == "POTABLE_PLUMBING" {
                                div { class: "grid grid-cols-2 gap-3 p-3 bg-muted/20 border border-border rounded-lg",
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{water_press_label}" }
                                        input {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            placeholder: if is_metric { "1.8 Bar" } else { "26.1 PSI" },
                                            value: "{water_pressure_bar}",
                                            oninput: move |e: Event<FormData>| water_pressure_bar.set(e.value()),
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{delta_t_label}" }
                                        input {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{temp_diff_c}",
                                            oninput: move |e: Event<FormData>| temp_diff_c.set(e.value()),
                                        }
                                    }
                                }
                            } else {
                                div { class: "grid grid-cols-2 gap-3",
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{t(\"hvac-refrigerant-type\", &region)}" }
                                        select {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{refrigerant_type}",
                                            onchange: move |e: Event<FormData>| refrigerant_type.set(e.value()),
                                            option { value: "R-410A", "R-410A (Eco Heat Pump)" }
                                            option { value: "R-32", "R-32 (Next-Gen Inverter)" }
                                            option { value: "R-290", "R-290 (Propane Heat Pump)" }
                                            option { value: "R-454B", "R-454B (Low-GWP Medium)" }
                                            option { value: "R-22", "R-22 (Legacy HVAC)" }
                                            option { value: "R-134a", "R-134a (Chiller Medium)" }
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{t(\"hvac-charge-level\", &region)}" }
                                        select {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{charge_level}",
                                            onchange: move |e: Event<FormData>| charge_level.set(e.value()),
                                            option { value: "Optimal (95-100%)", "{t(\"hvac-charge-optimal\", &region)}" }
                                            option { value: "Undercharged (<80%)", "{t(\"hvac-charge-under\", &region)}" }
                                            option { value: "Overcharged (>110%)", "{t(\"hvac-charge-over\", &region)}" }
                                        }
                                    }
                                }

                                div { class: "grid grid-cols-3 gap-3",
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{high_psi_label}" }
                                        input {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{high_side_psi}",
                                            oninput: move |e: Event<FormData>| high_side_psi.set(e.value()),
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{low_psi_label}" }
                                        input {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{low_side_psi}",
                                            oninput: move |e: Event<FormData>| low_side_psi.set(e.value()),
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "font-semibold text-foreground", "{delta_t_label}" }
                                        input {
                                            class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                            value: "{temp_diff_c}",
                                            oninput: move |e: Event<FormData>| temp_diff_c.set(e.value()),
                                        }
                                    }
                                }
                            }

                            div { class: "grid grid-cols-2 gap-3",
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground", "{t(\"hvac-voltage\", &region)}" }
                                    input {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{voltage_v}",
                                        oninput: move |e: Event<FormData>| voltage_v.set(e.value()),
                                    }
                                }
                                div { class: "flex flex-col gap-1",
                                    label { class: "font-semibold text-foreground", "{t(\"hvac-amps\", &region)}" }
                                    input {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{amp_draw_a}",
                                        oninput: move |e: Event<FormData>| amp_draw_a.set(e.value()),
                                    }
                                }
                            }

                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "{t(\"hvac-tech-notes\", &region)}" }
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
                                    let s_type = system_type.read().clone();
                                    let r_type = refrigerant_type.read().clone();
                                    let c_level = charge_level.read().clone();
                                    let raw_h = high_side_psi.read().parse::<f64>().unwrap_or(if is_metric { 22.0 } else { 320.0 });
                                    let raw_l = low_side_psi.read().parse::<f64>().unwrap_or(if is_metric { 7.9 } else { 115.0 });
                                    let raw_w = water_pressure_bar.read().parse::<f64>().unwrap_or(if is_metric { 1.8 } else { 26.1 });
                                    let raw_td = temp_diff_c.read().parse::<f64>().unwrap_or(if is_metric { 12.5 } else { 22.5 });
                                    let volt = voltage_v.read().parse::<f64>().unwrap_or(230.0);
                                    let amps = amp_draw_a.read().parse::<f64>().unwrap_or(14.2);
                                    let a_id_opt = if asset_id.read().trim().is_empty() { None } else { Some(asset_id.read().clone()) };
                                    let notes_opt = if diag_notes.read().trim().is_empty() { None } else { Some(diag_notes.read().clone()) };
                                    let op_mode_opt = Some(operating_mode.read().clone());
                                    let raw_amb = ambient_temp_c.read().parse::<f64>().ok();
                                    let mut db_trig_c = db_trig;

                                    let h_psi = if is_metric { raw_h * 14.5038 } else { raw_h };
                                    let l_psi = if is_metric { raw_l * 14.5038 } else { raw_l };
                                    let w_press = if is_metric { raw_w } else { raw_w / 14.5038 };
                                    let t_diff = if is_metric { raw_td } else { raw_td / 1.8 };
                                    let amb_temp = raw_amb.map(|a| if is_metric { a } else { (a - 32.0) * 5.0 / 9.0 });

                                    let mut high_side_psi_c = high_side_psi;
                                    let mut low_side_psi_c = low_side_psi;
                                    let mut water_pressure_bar_c = water_pressure_bar;
                                    let mut temp_diff_c_c = temp_diff_c;
                                    let mut ambient_temp_c_c = ambient_temp_c;
                                    let mut voltage_v_c = voltage_v;
                                    let mut amp_draw_a_c = amp_draw_a;
                                    let mut asset_id_c = asset_id;
                                    let mut diag_notes_c = diag_notes;

                                    move |_| {
                                        let runner_inner = runner_c.clone();
                                        let uid_inner = requester_uid.clone();
                                        let jid_inner = job_ticket_id.clone();
                                        let s_type_inner = s_type.clone();
                                        let r_type_inner = r_type.clone();
                                        let c_level_inner = c_level.clone();
                                        let a_id_inner = a_id_opt.clone();
                                        let notes_inner = notes_opt.clone();
                                        let op_mode_inner = op_mode_opt.clone();
                                        let amb_temp_inner = amb_temp;

                                        runner_inner.run(async move {
                                            yntra_core::log_hvac_system_diagnostic(
                                                uid_inner,
                                                jid_inner,
                                                s_type_inner,
                                                r_type_inner,
                                                c_level_inner,
                                                h_psi,
                                                l_psi,
                                                w_press,
                                                t_diff,
                                                volt,
                                                amps,
                                                a_id_inner,
                                                notes_inner,
                                                op_mode_inner,
                                                amb_temp_inner,
                                            ).await?;

                                            // Form Reset Routine upon successful resolution
                                            high_side_psi_c.set(String::new());
                                            low_side_psi_c.set(String::new());
                                            water_pressure_bar_c.set(String::new());
                                            temp_diff_c_c.set(String::new());
                                            ambient_temp_c_c.set(String::new());
                                            voltage_v_c.set(String::new());
                                            amp_draw_a_c.set(String::new());
                                            asset_id_c.set(String::new());
                                            diag_notes_c.set(String::new());

                                            let cur = *db_trig_c.read();
                                            db_trig_c.set(cur + 1);
                                            Ok(())
                                        });
                                    }
                                },
                                "{t(\"hvac-log-btn\", &region)}"
                            }
                        }
                }

                // Tab 1: Parts & Materials Inventory Tracker
                if *active_tab.read() == 1 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "p-3 bg-muted/20 border border-border rounded-lg flex flex-col gap-1",
                            h4 { class: "font-bold text-foreground", "{t(\"hvac-parts-title\", &region)}" }
                            p { class: "text-muted-foreground", "{t(\"hvac-parts-desc\", &region)}" }
                        }

                        // Add new part form
                        div { class: "grid grid-cols-12 gap-2 p-3 bg-muted/10 border border-border rounded-lg items-end",
                            div { class: "col-span-5 flex flex-col gap-1",
                                label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-part-name\", &region)}" }
                                input {
                                    class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                    placeholder: "Copper Pipe / Zone Valve...",
                                    value: "{part_name_input}",
                                    oninput: move |e: Event<FormData>| part_name_input.set(e.value()),
                                }
                            }
                            div { class: "col-span-2 flex flex-col gap-1",
                                label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-part-qty\", &region)}" }
                                input {
                                    class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{part_qty_input}",
                                    oninput: move |e: Event<FormData>| part_qty_input.set(e.value()),
                                }
                            }
                            div { class: "col-span-3 flex flex-col gap-1",
                                label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-part-unit-cost\", &region)}" }
                                input {
                                    class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                    value: "{part_unit_cost_input}",
                                    oninput: move |e: Event<FormData>| part_unit_cost_input.set(e.value()),
                                }
                            }
                            div { class: "col-span-2 flex items-center justify-end pb-1",
                                button {
                                    class: "yntra-btn py-1 px-2 text-[11px] font-semibold w-full",
                                    onclick: {
                                        let runner_c = runner.clone();
                                        let requester_uid = uid.clone();
                                        let job_ticket_id = jid.clone();
                                        let p_name = part_name_input.read().clone();
                                        let p_qty = part_qty_input.read().parse::<f64>().unwrap_or(1.0);
                                        let p_cost = part_unit_cost_input.read().parse::<f64>().unwrap_or(0.0);
                                        let p_rot = *part_rot_eligible.read();
                                        let mut db_trig_c = db_trig;

                                        let mut part_name_input_c = part_name_input;

                                        move |_| {
                                            if p_name.trim().is_empty() { return; }
                                            let runner_inner = runner_c.clone();
                                            let uid_inner = requester_uid.clone();
                                            let jid_inner = job_ticket_id.clone();
                                            let name_inner = p_name.clone();

                                            runner_inner.run(async move {
                                                yntra_core::add_job_part_used(
                                                    uid_inner,
                                                    jid_inner,
                                                    name_inner,
                                                    p_qty,
                                                    p_cost,
                                                    p_rot,
                                                ).await?;
                                                part_name_input_c.set(String::new());
                                                let cur = *db_trig_c.read();
                                                db_trig_c.set(cur + 1);
                                                Ok(())
                                            });
                                        }
                                    },
                                    "{t(\"hvac-part-add\", &region)}"
                                }
                            }
                        }

                        div { class: "border border-border rounded-lg overflow-hidden",
                            table { class: "w-full text-left border-collapse text-xs",
                                class: "divide-y divide-border",
                                thead { class: "bg-muted/40 font-semibold text-muted-foreground",
                                    tr {
                                        th { class: "p-2.5", "{t(\"hvac-part-name\", &region)}" }
                                        th { class: "p-2.5", "{t(\"hvac-part-qty\", &region)}" }
                                        th { class: "p-2.5", "{t(\"hvac-part-unit-cost\", &region)}" }
                                        th { class: "p-2.5", "{t(\"hvac-part-total-cost\", &region)}" }
                                        th { class: "p-2.5 text-right", "{t(\"hvac-actions-col\", &region)}" }
                                    }
                                }
                                tbody { class: "divide-y divide-border text-foreground",
                                    if job_parts.is_empty() {
                                        tr {
                                            td { class: "p-4 text-center text-muted-foreground italic", colspan: "5",
                                                "{t(\"hvac-no-parts-msg\", &region)}"
                                            }
                                        }
                                    } else {
                                        for item in job_parts.iter() {
                                            {
                                                let item_id = item.id.clone();
                                                let line_total = item.quantity * item.unit_cost_sek;
                                                rsx! {
                                                    tr { key: "{item.id}",
                                                        td { class: "p-2.5 font-medium", "{item.part_name}" }
                                                        td { class: "p-2.5", "{item.quantity}" }
                                                        td { class: "p-2.5", "{item.unit_cost_sek:.2} SEK" }
                                                        td { class: "p-2.5 font-bold", "{line_total:.2} SEK" }
                                                        td { class: "p-2.5 text-right",
                                                            button {
                                                                class: "text-destructive hover:underline font-semibold text-[11px]",
                                                                onclick: {
                                                                    let runner_c = runner.clone();
                                                                    let requester_uid = uid.clone();
                                                                    let pid = item_id.clone();
                                                                    let mut db_trig_c = db_trig;
                                                                    move |_| {
                                                                        let runner_inner = runner_c.clone();
                                                                        let uid_inner = requester_uid.clone();
                                                                        let pid_inner = pid.clone();
                                                                        runner_inner.run(async move {
                                                                            yntra_core::delete_job_part_used(uid_inner, pid_inner).await?;
                                                                            let cur = *db_trig_c.read();
                                                                            db_trig_c.set(cur + 1);
                                                                            Ok(())
                                                                        });
                                                                    }
                                                                },
                                                                "{t(\"hvac-part-delete-btn\", &region)}"
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

                // Tab 2: Swedish ROT 30% Tax Credit Calculator
                if *active_tab.read() == 2 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "flex items-center justify-between bg-muted/20 p-2.5 rounded-lg border border-border",
                            div { class: "flex flex-col gap-0.5",
                                span { class: "font-semibold text-foreground", "Work Order Labor & Travel Calculation" }
                                span { class: "text-[11px] text-muted-foreground", "Auto-populate labor and travel from recorded parts and default service rates." }
                            }
                            button {
                                class: "yntra-btn py-1.5 px-3 text-[11px] font-bold flex items-center gap-1.5 bg-primary/20 text-primary border border-primary/30 hover:bg-primary/30",
                                onclick: move |_| {
                                    if labor_cost_input.read().trim().is_empty() {
                                        labor_cost_input.set("6500.0".to_string());
                                    }
                                    if travel_fee_input.read().trim().is_empty() {
                                        travel_fee_input.set("450.0".to_string());
                                    }
                                },
                                "{t(\"hvac-rot-autofill-btn\", &region)}"
                            }
                        }

                        div { class: "grid grid-cols-2 gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Labor Cost (SEK)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    placeholder: "e.g. 6500.0",
                                    value: "{labor_cost_input}",
                                    oninput: move |e: Event<FormData>| labor_cost_input.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "Travel Fee (SEK)" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    placeholder: "e.g. 450.0",
                                    value: "{travel_fee_input}",
                                    oninput: move |e: Event<FormData>| travel_fee_input.set(e.value()),
                                }
                            }
                        }

                        div { class: "p-4 bg-primary/10 border border-primary/20 rounded-xl flex flex-col gap-3",
                            h4 { class: "font-bold text-foreground text-sm flex items-center justify-between",
                                span { "{t(\"hvac-rot-title\", &region)}" }
                                span { class: "px-2 py-0.5 bg-emerald-500/20 text-emerald-400 rounded text-xs font-semibold", "{t(\"hvac-rot-valid-badge\", &region)}" }
                            }
                            div { class: "grid grid-cols-2 gap-2 text-xs",
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-gross-label\", &region)}" }
                                    span { class: "font-bold text-foreground", "{rot_info.total_gross_amount_sek:.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-eligible-label\", &region)}" }
                                    span { class: "font-bold text-foreground", "{rot_info.eligible_labor_sek:.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-eligible-parts-label\", &region)}" }
                                    span { class: "font-bold text-emerald-400", "{rot_info.eligible_parts_sek:.2} SEK" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-non-eligible-label\", &region)}" }
                                    span { class: "font-bold text-foreground", "{rot_info.non_eligible_parts_sek:.2} SEK" }
                                }
                                div { class: "col-span-2 flex justify-between border-b border-border/40 pb-1.5 pt-1",
                                    span { class: "text-emerald-400 font-bold", "{t(\"hvac-rot-credit-label\", &region)}" }
                                    span { class: "font-extrabold text-emerald-400 text-sm", "-{rot_info.rot_deduction_30_percent_sek:.2} SEK" }
                                }
                            }
                            div { class: "flex items-center justify-between pt-2 border-t border-border/60",
                                span { class: "font-bold text-foreground text-sm", "{t(\"hvac-rot-net-label\", &region)}" }
                                span { class: "font-extrabold text-primary text-base", "{rot_info.net_customer_payable_sek:.2} SEK" }
                            }
                        }

                        div { class: "p-3.5 bg-amber-500/10 border border-amber-500/30 rounded-xl flex flex-col gap-1 text-[11px]",
                            span { class: "font-bold text-amber-400 flex items-center gap-1.5",
                                span { "⚖️" }
                                "{t(\"hvac-rot-cap-label\", &region)}"
                            }
                            p { class: "text-amber-200/90 leading-relaxed", "{t(\"hvac-rot-cap-warning\", &region)}" }
                        }
                    }
                }

                // Tab 3: Inspection History Timeline
                if *active_tab.read() == 3 {
                    div { class: "flex flex-col gap-3 text-xs animate-in fade-in duration-200",
                        if diagnostics_history.is_empty() {
                            div { class: "p-6 border border-dashed border-border rounded-lg text-center text-muted-foreground",
                                "{t(\"hvac-no-history-msg\", &region)}"
                            }
                        } else {
                            for diag in diagnostics_history.iter() {
                                {
                                    let status_badge_class = match diag.diagnostic_status.as_str() {
                                        "CRITICAL_HAZARD" => "px-2 py-0.5 bg-red-500/20 text-red-400 border border-red-500/30 rounded text-[10px] font-bold uppercase",
                                        "MAINTENANCE_WARNING" => "px-2 py-0.5 bg-amber-500/20 text-amber-400 border border-amber-500/30 rounded text-[10px] font-bold uppercase",
                                        _ => "px-2 py-0.5 bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 rounded text-[10px] font-bold uppercase",
                                    };
                                    let status_label = match diag.diagnostic_status.as_str() {
                                        "CRITICAL_HAZARD" => t("hvac-status-critical", &region),
                                        "MAINTENANCE_WARNING" => t("hvac-status-warning", &region),
                                        _ => t("hvac-status-normal", &region),
                                    };
                                    let high_val_str = if is_metric { format!("{:.2} Bar", diag.high_side_psi / 14.5038) } else { format!("{:.1} PSI", diag.high_side_psi) };
                                    let low_val_str = if is_metric { format!("{:.2} Bar", diag.low_side_psi / 14.5038) } else { format!("{:.1} PSI", diag.low_side_psi) };
                                    let water_val_str = if is_metric { format!("{:.2} Bar", diag.water_pressure_bar) } else { format!("{:.1} PSI", diag.water_pressure_bar * 14.5038) };
                                    let delta_t_str = if is_metric { format!("{:.1}°C", diag.temp_differential_c) } else { format!("{:.1}°F", diag.temp_differential_c * 1.8) };

                                    let date_str = {
                                        let ts = diag.created_at;
                                        let sec = if ts > 10_000_000_000 { ts / 1000 } else { ts };
                                        if sec > 0 {
                                            chrono::DateTime::from_timestamp(sec, 0)
                                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                                .unwrap_or_else(|| "N/A".to_string())
                                        } else {
                                            "N/A".to_string()
                                        }
                                    };

                                    rsx! {
                                        div { key: "{diag.id}", class: "p-3 bg-muted/20 border border-border rounded-lg flex flex-col gap-2",
                                            div { class: "flex items-center justify-between border-b border-border/40 pb-1.5 mb-1",
                                                div { class: "flex items-center gap-2",
                                                    span { class: "{status_badge_class}", "{status_label}" }
                                                    span { class: "px-2 py-0.5 bg-secondary/30 text-secondary-foreground rounded text-[10px] font-medium flex items-center gap-1",
                                                        "👤 Tech: {diag.technician_id}"
                                                    }
                                                    if let Some(ref mode) = diag.operating_mode {
                                                        span { class: "px-2 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-semibold", "{mode}" }
                                                    }
                                                    if let Some(ref asset) = diag.asset_id {
                                                        span { class: "text-[11px] font-semibold text-primary", "Asset: {asset}" }
                                                    }
                                                }
                                                div { class: "flex items-center gap-3 text-xs text-muted-foreground",
                                                    span { class: "text-[11px] font-mono text-muted-foreground/80", "📅 {date_str}" }
                                                    span { "Medium: {diag.refrigerant_type}" }
                                                }
                                            }
                                            if diag.system_type == "HYDRONIC_HEATING" || diag.system_type == "HYDRONIC_PLUMBING" || diag.system_type == "POTABLE_WATER" || diag.system_type == "POTABLE_PLUMBING" {
                                                div { class: "grid grid-cols-2 gap-2 text-muted-foreground text-[11px]",
                                                    span { "Water Pressure: {water_val_str}" }
                                                    span { "ΔT: {delta_t_str}" }
                                                }
                                            } else {
                                                div { class: "grid grid-cols-3 gap-2 text-muted-foreground text-[11px]",
                                                    span { "High Side: {high_val_str}" }
                                                    span { "Low Side: {low_val_str}" }
                                                    span { "ΔT: {delta_t_str}" }
                                                }
                                            }
                                            if let Some(amb) = diag.ambient_temp_c {
                                                {
                                                    let amb_str = if is_metric { format!("{:.1}°C", amb) } else { format!("{:.1}°F", amb * 1.8 + 32.0) };
                                                    rsx! {
                                                        div { class: "text-[11px] text-muted-foreground italic", "Outdoor Ambient Temp: {amb_str}" }
                                                    }
                                                }
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
    }
}
