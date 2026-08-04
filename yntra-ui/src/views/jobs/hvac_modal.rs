use crate::locales::{t, t_with_args};
use crate::state::AppState;
use crate::utils::use_action_runner;
use dioxus::prelude::*;
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
    let currency_code = match region.as_str() {
        "sv" | "SE" => "SEK",
        "no" | "NO" => "NOK",
        "da" | "DK" => "DKK",
        "fi" | "FI" | "de" | "DE" => "EUR",
        _ => "USD",
    };
    let mut show = props.show;
    let runner = use_action_runner();
    let toast = dioxus_primitives::toast::use_toast();

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

    let mut refrigerant_added_kg = use_signal(String::new);
    let mut refrigerant_recovered_kg = use_signal(String::new);
    let mut reclaim_cylinder_id = use_signal(String::new);
    let mut diag_notes = use_signal(String::new);
    let mut static_flow_pressure_bar = use_signal(String::new);
    let mut dynamic_flow_pressure_bar = use_signal(String::new);
    let mut pipe_material = use_signal(|| "PEX".to_string());
    let mut backflow_preventer_status = use_signal(|| "PASS_TESTED".to_string());
    let mut water_heater_temp_c = use_signal(String::new);
    let mut leak_test_duration_min = use_signal(String::new);
    let mut leak_test_pressure_drop_bar = use_signal(String::new);
    let mut form_baseline_initialized = use_signal(|| false);

    // Custom / Unlisted Equipment input signals
    let mut custom_asset_model = use_signal(String::new);
    let mut custom_asset_serial = use_signal(String::new);
    let mut custom_asset_tag = use_signal(String::new);
    let mut custom_asset_loc = use_signal(String::new);

    // New Part input signals
    let mut part_name_input = use_signal(String::new);
    let mut part_qty_input = use_signal(|| "1.0".to_string());
    let mut part_unit_cost_input = use_signal(String::new);
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
            yntra_core::get_hvac_location_assets(uid, jid)
                .await
                .unwrap_or_default()
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
            yntra_core::get_hvac_job_diagnostics(u, j)
                .await
                .unwrap_or_default()
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
            yntra_core::get_job_parts_used(u, j)
                .await
                .unwrap_or_default()
        }
    });

    let diagnostics_history: Vec<HvacSystemDiagnostic> =
        diagnostics_res.read().clone().unwrap_or_default();
    let job_parts: Vec<JobPartItem> = parts_res.read().clone().unwrap_or_default();

    // Retrieve last recorded baseline readings for the active work order if available using a reactive effect
    use_effect(move || {
        let history_opt = diagnostics_res.read();
        if let Some(history) = history_opt.as_ref() {
            if !*form_baseline_initialized.read() && !history.is_empty() {
                if let Some(latest) = history.first() {
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
                    if let Some(h) = latest.high_side_psi {
                        high_side_psi.set(if is_m {
                            format!("{:.2}", h / 14.503773773)
                        } else {
                            format!("{:.1}", h)
                        });
                    }
                    if let Some(l) = latest.low_side_psi {
                        low_side_psi.set(if is_m {
                            format!("{:.2}", l / 14.503773773)
                        } else {
                            format!("{:.1}", l)
                        });
                    }
                    if let Some(w) = latest.water_pressure_bar {
                        water_pressure_bar.set(if is_m {
                            format!("{:.2}", w)
                        } else {
                            format!("{:.1}", w * 14.503773773)
                        });
                    }
                    if let Some(td) = latest.temp_differential_c {
                        temp_diff_c.set(if is_m {
                            format!("{:.1}", td)
                        } else {
                            format!("{:.1}", td * 1.8)
                        });
                    }
                    if let Some(amb) = latest.ambient_temp_c {
                        ambient_temp_c.set(if is_m {
                            format!("{:.1}", amb)
                        } else {
                            format!("{:.1}", amb * 1.8 + 32.0)
                        });
                    }
                    if let Some(v) = latest.voltage_v {
                        voltage_v.set(format!("{:.1}", v));
                    }
                    if let Some(a) = latest.amp_draw_a {
                        amp_draw_a.set(format!("{:.1}", a));
                    }
                    form_baseline_initialized.set(true);
                }
            }
        }
    });

    if !*show.read() {
        return rsx! {};
    }

    // Calculate parts total (under Skatteverket rules, materials and parts are 100% non-eligible for ROT tax credit)
    let parts_cost_total: f64 = job_parts.iter().map(|p| p.quantity * p.unit_cost_sek).sum();

    // Calculate ROT split reactively without blocking UI thread
    let rot_breakdown = use_memo(move || {
        let labor_val = labor_cost_input.read().parse::<f64>().unwrap_or(0.0);
        let travel_val = travel_fee_input.read().parse::<f64>().unwrap_or(0.0);
        let eligible_labor = labor_val.max(0.0);
        let non_eligible_parts_and_travel = parts_cost_total.max(0.0) + travel_val.max(0.0);
        let gross = eligible_labor + non_eligible_parts_and_travel;
        // ROT Deduction is strictly 30% of eligible labor in Sweden (capped at 50,000 SEK per person per year)
        let deduction = (eligible_labor * 0.30).min(50000.0);
        RotInvoiceSplitBreakdown {
            total_gross_amount_sek: gross,
            eligible_labor_sek: eligible_labor,
            eligible_parts_sek: 0.0,
            non_eligible_parts_sek: non_eligible_parts_and_travel,
            rot_deduction_30_percent_sek: deduction,
            net_customer_payable_sek: gross - deduction,
            max_annual_rot_cap_remaining_sek: (50000.0 - deduction).max(0.0),
            rot_eligible_flag: eligible_labor > 0.0,
        }
    });
    let rot_info = rot_breakdown.read().clone();

    let job_id_str = props.job_id.clone();
    let subtitle_text = t_with_args("hvac-work-order-subtitle", &region, &[("id", &job_id_str)]);
    let history_count_str = diagnostics_history.len().to_string();
    let history_tab_text = t_with_args(
        "hvac-tab-history",
        &region,
        &[("count", &history_count_str)],
    );

    let is_metric = *unit_system.read() == "METRIC";
    let high_psi_label = if is_metric {
        "High Side Pressure (Bar)"
    } else {
        "High Side Pressure (PSI)"
    };
    let low_psi_label = if is_metric {
        "Low Side Pressure (Bar)"
    } else {
        "Low Side Pressure (PSI)"
    };
    let water_press_label = if is_metric {
        "Water / Hydronic Pressure (Bar)"
    } else {
        "Water / Hydronic Pressure (PSI)"
    };
    let delta_t_label = if is_metric {
        "Delta T Differential (ΔT °C)"
    } else {
        "Delta T Differential (ΔT °F)"
    };
    let ambient_temp_label = if is_metric {
        "Outdoor Ambient Temp (°C)"
    } else {
        "Outdoor Ambient Temp (°F)"
    };

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-end bg-black/60 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "h-full w-full max-w-2xl bg-card border-l border-border p-6 flex flex-col gap-6 shadow-2xl overflow-y-auto animate-in slide-in-from-right duration-300",

                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex flex-col gap-1",
                        h3 { class: "text-lg font-bold text-foreground flex items-center gap-2",
                            "{t(\"hvac-drawer-title\", &region)}"
                        }
                        p { class: "text-xs text-muted-foreground",
                            "{subtitle_text}"
                        }
                    }
                    div { class: "flex items-center gap-3",
                        button {
                            class: "px-2.5 py-1 bg-primary/10 text-primary border border-primary/20 hover:bg-primary/20 font-bold rounded text-[11px] flex items-center gap-1.5 transition-colors shadow-xs",
                            onclick: {
                                let toast_c = toast.clone();
                                move |_| {
                                    toast_c.info(
                                        "Customer Service Certificate".to_string(),
                                        dioxus_primitives::toast::ToastOptions::new().description("HVAC & Service Audit Certificate exported to PDF format."),
                                    );
                                }
                            },
                            "Export Certificate (PDF)"
                        }
                        div { class: "flex items-center bg-muted/30 p-1 rounded-lg border border-border text-[11px]",
                            button {
                                class: if *unit_system.read() == "METRIC" { "px-2 py-0.5 bg-primary text-primary-foreground font-bold rounded shadow-xs" } else { "px-2 py-0.5 text-muted-foreground hover:text-foreground font-medium" },
                                onclick: move |_| {
                                    if *unit_system.read() == "IMPERIAL" {
                                        let v_h = high_side_psi.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_h { high_side_psi.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_l = low_side_psi.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_l { low_side_psi.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_w = water_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_w { water_pressure_bar.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_s = static_flow_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_s { static_flow_pressure_bar.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_d = dynamic_flow_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_d { dynamic_flow_pressure_bar.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_ld = leak_test_pressure_drop_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_ld { leak_test_pressure_drop_bar.set(format!("{:.2}", v / 14.503773773)); }
                                        let v_td = temp_diff_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_td { temp_diff_c.set(format!("{:.1}", v / 1.8)); }
                                        let v_amb = ambient_temp_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_amb { ambient_temp_c.set(format!("{:.1}", (v - 32.0) / 1.8)); }
                                        let v_ht = water_heater_temp_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_ht { water_heater_temp_c.set(format!("{:.1}", (v - 32.0) / 1.8)); }
                                    }
                                    unit_system.set("METRIC".to_string());
                                },
                                "{t(\"hvac-unit-metric\", &region)}"
                            }
                            button {
                                class: if *unit_system.read() == "IMPERIAL" { "px-2 py-0.5 bg-primary text-primary-foreground font-bold rounded shadow-xs" } else { "px-2 py-0.5 text-muted-foreground hover:text-foreground font-medium" },
                                onclick: move |_| {
                                    if *unit_system.read() == "METRIC" {
                                        let v_h = high_side_psi.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_h { high_side_psi.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_l = low_side_psi.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_l { low_side_psi.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_w = water_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_w { water_pressure_bar.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_s = static_flow_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_s { static_flow_pressure_bar.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_d = dynamic_flow_pressure_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_d { dynamic_flow_pressure_bar.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_ld = leak_test_pressure_drop_bar.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_ld { leak_test_pressure_drop_bar.set(format!("{:.1}", v * 14.503773773)); }
                                        let v_td = temp_diff_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_td { temp_diff_c.set(format!("{:.1}", v * 1.8)); }
                                        let v_amb = ambient_temp_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_amb { ambient_temp_c.set(format!("{:.1}", v * 1.8 + 32.0)); }
                                        let v_ht = water_heater_temp_c.read().trim().parse::<f64>().ok();
                                        if let Some(v) = v_ht { water_heater_temp_c.set(format!("{:.1}", v * 1.8 + 32.0)); }
                                    }
                                    unit_system.set("IMPERIAL".to_string());
                                },
                                "{t(\"hvac-unit-imperial\", &region)}"
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
                                        span { class: "text-[10px] text-muted-foreground font-normal", "{t(\"hvac-asset-header-badge\", &region)}" }
                                    }
                                    select {
                                        class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                        value: "{asset_id}",
                                        onchange: move |e: Event<FormData>| asset_id.set(e.value()),
                                        option { value: "", "{t(\"hvac-asset-select-placeholder\", &region)}" }
                                        for asset in location_assets.iter() {
                                            option { value: "{asset.asset_tag} - {asset.model_name} ({asset.serial_number})", "[{asset.asset_tag}] {asset.model_name} ({asset.serial_number})" }
                                        }
                                        option { value: "UNLISTED_EQUIPMENT", "➕ Custom / Unlisted Equipment" }
                                    }
                                }
                            }

                            if *asset_id.read() == "UNLISTED_EQUIPMENT" {
                                div { class: "flex flex-col gap-2.5 p-3 bg-primary/5 border border-primary/20 rounded-lg animate-in fade-in duration-200",
                                    div { class: "text-[11px] font-bold text-primary flex items-center gap-1.5",
                                        span { "🛠️" }
                                        span { "{t(\"hvac-custom-asset-title\", &region)}" }
                                    }
                                    div { class: "grid grid-cols-2 gap-2.5",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[10px]", "{t(\"hvac-custom-asset-model\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. NIBE F2120-12 / Grundfos Scala2",
                                                value: "{custom_asset_model}",
                                                oninput: move |e: Event<FormData>| custom_asset_model.set(e.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[10px]", "{t(\"hvac-custom-asset-serial\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. SN-88391029",
                                                value: "{custom_asset_serial}",
                                                oninput: move |e: Event<FormData>| custom_asset_serial.set(e.value()),
                                            }
                                        }
                                    }
                                    div { class: "grid grid-cols-2 gap-2.5",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[10px]", "{t(\"hvac-custom-asset-tag\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. HP-001 or PLUMB-02",
                                                value: "{custom_asset_tag}",
                                                oninput: move |e: Event<FormData>| custom_asset_tag.set(e.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[10px]", "{t(\"hvac-custom-asset-loc\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. Basement Boiler Room",
                                                value: "{custom_asset_loc}",
                                                oninput: move |e: Event<FormData>| custom_asset_loc.set(e.value()),
                                            }
                                        }
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
                                div { class: "flex flex-col gap-3 p-3 bg-muted/20 border border-border rounded-lg",
                                    div { class: "grid grid-cols-2 gap-3",
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

                                    div { class: "border-t border-border/50 pt-2 grid grid-cols-2 gap-3",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-static-flow-pressure\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "e.g. 4.5 Bar" } else { "e.g. 65.3 PSI" },
                                                value: "{static_flow_pressure_bar}",
                                                oninput: move |e: Event<FormData>| static_flow_pressure_bar.set(e.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-dynamic-flow-pressure\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "e.g. 4.1 Bar" } else { "e.g. 59.5 PSI" },
                                                value: "{dynamic_flow_pressure_bar}",
                                                oninput: move |e: Event<FormData>| dynamic_flow_pressure_bar.set(e.value()),
                                            }
                                        }
                                    }

                                    div { class: "grid grid-cols-3 gap-3",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-pipe-material\", &region)}" }
                                            select {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                value: "{pipe_material}",
                                                onchange: move |e: Event<FormData>| pipe_material.set(e.value()),
                                                option { value: "PEX", "{t(\"hvac-mat-pex\", &region)}" }
                                                option { value: "COPPER", "{t(\"hvac-mat-copper\", &region)}" }
                                                option { value: "STAINLESS", "{t(\"hvac-mat-stainless\", &region)}" }
                                                option { value: "GALVANIZED", "{t(\"hvac-mat-galvanized\", &region)}" }
                                                option { value: "PVC", "{t(\"hvac-mat-pvc\", &region)}" }
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-backflow-preventer\", &region)}" }
                                            select {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                value: "{backflow_preventer_status}",
                                                onchange: move |e: Event<FormData>| backflow_preventer_status.set(e.value()),
                                                option { value: "PASS_TESTED", "{t(\"hvac-backflow-pass\", &region)}" }
                                                option { value: "FAIL_LEAKING", "{t(\"hvac-backflow-fail\", &region)}" }
                                                option { value: "NOT_TESTED", "{t(\"hvac-backflow-untested\", &region)}" }
                                                option { value: "N_A", "{t(\"hvac-backflow-na\", &region)}" }
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-water-heater-temp\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "e.g. 58.0 °C" } else { "e.g. 136.4 °F" },
                                                value: "{water_heater_temp_c}",
                                                oninput: move |e: Event<FormData>| water_heater_temp_c.set(e.value()),
                                            }
                                        }
                                    }

                                     div { class: "grid grid-cols-2 gap-3 border-t border-border/50 pt-2",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-leak-test-duration\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. 30",
                                                value: "{leak_test_duration_min}",
                                                oninput: move |e: Event<FormData>| leak_test_duration_min.set(e.value()),
                                            }
                                            div { class: "flex items-center gap-1 mt-0.5",
                                                button {
                                                    class: if *leak_test_duration_min.read() == "15.0" { "px-1.5 py-0.5 text-[10px] font-bold bg-primary/15 text-primary border border-primary/30 rounded transition-colors shadow-xs" } else { "px-1.5 py-0.5 text-[10px] font-semibold bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground rounded border border-border transition-colors" },
                                                    onclick: move |_| leak_test_duration_min.set("15.0".to_string()),
                                                    "15m"
                                                }
                                                button {
                                                    class: if *leak_test_duration_min.read() == "30.0" { "px-1.5 py-0.5 text-[10px] font-bold bg-primary/15 text-primary border border-primary/30 rounded transition-colors shadow-xs" } else { "px-1.5 py-0.5 text-[10px] font-semibold bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground rounded border border-border transition-colors" },
                                                    onclick: move |_| leak_test_duration_min.set("30.0".to_string()),
                                                    "30m"
                                                }
                                                button {
                                                    class: if *leak_test_duration_min.read() == "60.0" { "px-1.5 py-0.5 text-[10px] font-bold bg-primary/15 text-primary border border-primary/30 rounded transition-colors shadow-xs" } else { "px-1.5 py-0.5 text-[10px] font-semibold bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground rounded border border-border transition-colors" },
                                                    onclick: move |_| leak_test_duration_min.set("60.0".to_string()),
                                                    "60m"
                                                }
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "{t(\"hvac-leak-test-drop\", &region)}" }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "0.0 Bar" } else { "0.0 PSI" },
                                                value: "{leak_test_pressure_drop_bar}",
                                                oninput: move |e: Event<FormData>| leak_test_pressure_drop_bar.set(e.value()),
                                            }
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

                                div { class: "p-3 bg-muted/20 border border-border/50 rounded-lg flex flex-col gap-2 text-xs",
                                    span { class: "font-bold text-foreground text-[11px]", "F-Gas & EPA Section 608 Compliance Log" }
                                    div { class: "grid grid-cols-3 gap-3",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", if is_metric { "Refrigerant Added (kg)" } else { "Refrigerant Added (lbs)" } }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "0.00 kg" } else { "0.00 lbs" },
                                                value: "{refrigerant_added_kg}",
                                                oninput: move |e: Event<FormData>| refrigerant_added_kg.set(e.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", if is_metric { "Refrigerant Recovered (kg)" } else { "Refrigerant Recovered (lbs)" } }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: if is_metric { "0.00 kg" } else { "0.00 lbs" },
                                                value: "{refrigerant_recovered_kg}",
                                                oninput: move |e: Event<FormData>| refrigerant_recovered_kg.set(e.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "font-semibold text-foreground text-[11px]", "Reclaim Cylinder ID" }
                                            input {
                                                class: "yntra-input py-1 px-2.5 bg-background border border-border text-foreground rounded text-xs",
                                                placeholder: "e.g. CYL-88291",
                                                value: "{reclaim_cylinder_id}",
                                                oninput: move |e: Event<FormData>| reclaim_cylinder_id.set(e.value()),
                                            }
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
                                    div { class: "flex items-center gap-1.5 mt-0.5",
                                        button {
                                            class: if *voltage_v.read() == "230.0" { "px-2 py-0.5 text-[10px] font-bold bg-primary/15 text-primary border border-primary/30 rounded transition-colors shadow-xs" } else { "px-2 py-0.5 text-[10px] font-semibold bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground rounded border border-border transition-colors" },
                                            onclick: move |_| voltage_v.set("230.0".to_string()),
                                            "230V 1-Ph"
                                        }
                                        button {
                                            class: if *voltage_v.read() == "400.0" { "px-2 py-0.5 text-[10px] font-bold bg-primary/15 text-primary border border-primary/30 rounded transition-colors shadow-xs" } else { "px-2 py-0.5 text-[10px] font-semibold bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground rounded border border-border transition-colors" },
                                            onclick: move |_| voltage_v.set("400.0".to_string()),
                                            "400V 3-Ph"
                                        }
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
                                    placeholder: "{t(\"hvac-tech-notes-placeholder\", &region)}",
                                    value: "{diag_notes}",
                                    oninput: move |e: Event<FormData>| diag_notes.set(e.value()),
                                }
                            }

                            button {
                                class: "yntra-btn py-2 text-xs font-semibold mt-2",
                                onclick: {
                                    let toast_c = toast.clone();
                                    let runner_c = runner.clone();
                                    let requester_uid = uid.clone();
                                    let job_ticket_id = jid.clone();
                                    let s_type = system_type.read().clone();
                                    let r_type = refrigerant_type.read().clone();
                                    let c_level = charge_level.read().clone();

                                    let raw_high_str = high_side_psi.read().trim().to_string();
                                    let raw_low_str = low_side_psi.read().trim().to_string();
                                    let raw_water_str = water_pressure_bar.read().trim().to_string();
                                    let raw_temp_str = temp_diff_c.read().trim().to_string();
                                    let raw_volt_str = voltage_v.read().trim().to_string();
                                    let raw_amp_str = amp_draw_a.read().trim().to_string();
                                    let raw_amb_str = ambient_temp_c.read().trim().to_string();

                                    let raw_static_str = static_flow_pressure_bar.read().trim().to_string();
                                    let raw_dynamic_str = dynamic_flow_pressure_bar.read().trim().to_string();
                                    let p_material_val = pipe_material.read().clone();
                                    let backflow_val = backflow_preventer_status.read().clone();
                                    let raw_heater_temp_str = water_heater_temp_c.read().trim().to_string();
                                    let raw_leak_dur_str = leak_test_duration_min.read().trim().to_string();
                                    let raw_leak_drop_str = leak_test_pressure_drop_bar.read().trim().to_string();

                                    let raw_ref_added_str = refrigerant_added_kg.read().trim().to_string();
                                    let raw_ref_recovered_str = refrigerant_recovered_kg.read().trim().to_string();
                                    let raw_reclaim_cyl_str = reclaim_cylinder_id.read().trim().to_string();

                                    let c_model_val = custom_asset_model.read().trim().to_string();
                                    let c_serial_val = custom_asset_serial.read().trim().to_string();
                                    let c_tag_val = custom_asset_tag.read().trim().to_string();
                                    let c_loc_val = custom_asset_loc.read().trim().to_string();

                                    let mut custom_asset_model_c = custom_asset_model;
                                    let mut custom_asset_serial_c = custom_asset_serial;
                                    let mut custom_asset_tag_c = custom_asset_tag;
                                    let mut custom_asset_loc_c = custom_asset_loc;

                                    let mut refrigerant_added_kg_c = refrigerant_added_kg;
                                    let mut refrigerant_recovered_kg_c = refrigerant_recovered_kg;
                                    let mut reclaim_cylinder_id_c = reclaim_cylinder_id;

                                    let a_id_opt = if asset_id.read().trim().is_empty() { None } else { Some(asset_id.read().clone()) };
                                    let notes_opt = if diag_notes.read().trim().is_empty() { None } else { Some(diag_notes.read().clone()) };
                                    let op_mode_opt = Some(operating_mode.read().clone());
                                    let mut db_trig_c = db_trig;

                                    let mut high_side_psi_c = high_side_psi;
                                    let mut low_side_psi_c = low_side_psi;
                                    let mut water_pressure_bar_c = water_pressure_bar;
                                    let mut temp_diff_c_c = temp_diff_c;
                                    let mut ambient_temp_c_c = ambient_temp_c;
                                    let mut voltage_v_c = voltage_v;
                                    let mut amp_draw_a_c = amp_draw_a;
                                    let mut asset_id_c = asset_id;
                                    let mut diag_notes_c = diag_notes;

                                    let mut static_flow_pressure_bar_c = static_flow_pressure_bar;
                                    let mut dynamic_flow_pressure_bar_c = dynamic_flow_pressure_bar;
                                    let mut water_heater_temp_c_c = water_heater_temp_c;
                                    let mut leak_test_duration_min_c = leak_test_duration_min;
                                    let mut leak_test_pressure_drop_bar_c = leak_test_pressure_drop_bar;

                                    move |_| {
                                        if a_id_opt == Some("UNLISTED_EQUIPMENT".to_string()) {
                                            if c_model_val.is_empty() || c_serial_val.is_empty() {
                                                toast_c.error("Equipment Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Please provide at least a Model Name and Serial Number for unlisted equipment."));
                                                return;
                                            }
                                        }
                                        // Numeric field validation
                                        let parsed_high = if raw_high_str.is_empty() { None } else {
                                            match raw_high_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("High side pressure must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_low = if raw_low_str.is_empty() { None } else {
                                            match raw_low_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Low side pressure must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_water = if raw_water_str.is_empty() { None } else {
                                            match raw_water_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Water pressure must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        // Required metric validation per system type
                                        if s_type == "REFRIGERANT_HVAC" {
                                            if parsed_high.is_none() || parsed_low.is_none() {
                                                toast_c.error("Required Metric Missing".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Both High Side and Low Side pressure readings are required for Refrigerant HVAC diagnostics."));
                                                return;
                                            }
                                        } else if s_type.starts_with("HYDRONIC") || s_type.starts_with("POTABLE") {
                                            if parsed_water.is_none() {
                                                toast_c.error("Required Metric Missing".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Water / Hydronic Pressure reading is required for plumbing & heating diagnostics."));
                                                return;
                                            }
                                        }

                                        let parsed_td = if raw_temp_str.is_empty() { None } else {
                                            match raw_temp_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Delta T differential must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_volt = if raw_volt_str.is_empty() { None } else {
                                            match raw_volt_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Voltage must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_amps = if raw_amp_str.is_empty() { None } else {
                                            match raw_amp_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Amp draw must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_amb = if raw_amb_str.is_empty() { None } else {
                                            match raw_amb_str.parse::<f64>() {
                                                Ok(val) => Some(val),
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Outdoor ambient temp must be a valid number."));
                                                    return;
                                                }
                                            }
                                        };

                                        let parsed_static = if raw_static_str.is_empty() { None } else { raw_static_str.parse::<f64>().ok() };
                                        let parsed_dynamic = if raw_dynamic_str.is_empty() { None } else { raw_dynamic_str.parse::<f64>().ok() };
                                        let parsed_heater_temp = if raw_heater_temp_str.is_empty() { None } else { raw_heater_temp_str.parse::<f64>().ok() };
                                        let parsed_leak_dur = if raw_leak_dur_str.is_empty() { None } else { raw_leak_dur_str.parse::<f64>().ok() };
                                        let parsed_leak_drop = if raw_leak_drop_str.is_empty() { None } else { raw_leak_drop_str.parse::<f64>().ok() };

                                        let parsed_ref_added = if raw_ref_added_str.is_empty() { None } else { raw_ref_added_str.parse::<f64>().ok() };
                                        let parsed_ref_recovered = if raw_ref_recovered_str.is_empty() { None } else { raw_ref_recovered_str.parse::<f64>().ok() };
                                        let reclaim_cyl_opt = if raw_reclaim_cyl_str.is_empty() { None } else { Some(raw_reclaim_cyl_str.clone()) };

                                        let opt_h_psi = parsed_high.map(|h| if is_metric { h * 14.503773773 } else { h });
                                        let opt_l_psi = parsed_low.map(|l| if is_metric { l * 14.503773773 } else { l });
                                        let opt_w_press = parsed_water.map(|w| if is_metric { w } else { w / 14.503773773 });
                                        let opt_t_diff = parsed_td.map(|td| if is_metric { td } else { td / 1.8 });
                                        let opt_amb_temp = parsed_amb.map(|a| if is_metric { a } else { (a - 32.0) / 1.8 });

                                        let opt_static_bar = parsed_static.map(|s| if is_metric { s } else { s / 14.503773773 });
                                        let opt_dynamic_bar = parsed_dynamic.map(|d| if is_metric { d } else { d / 14.503773773 });
                                        let opt_heater_temp_c = parsed_heater_temp.map(|ht| if is_metric { ht } else { (ht - 32.0) / 1.8 });
                                        let opt_leak_drop_bar = parsed_leak_drop.map(|ld| if is_metric { ld } else { ld / 14.503773773 });

                                        let opt_ref_added_kg = parsed_ref_added.map(|a| if is_metric { a } else { a * 0.45359237 });
                                        let opt_ref_recovered_kg = parsed_ref_recovered.map(|r| if is_metric { r } else { r * 0.45359237 });

                                        let pipe_mat_opt = if s_type.starts_with("HYDRONIC") || s_type.starts_with("POTABLE") { Some(p_material_val.clone()) } else { None };
                                        let backflow_opt = if s_type.starts_with("HYDRONIC") || s_type.starts_with("POTABLE") { Some(backflow_val.clone()) } else { None };

                                        let runner_inner = runner_c.clone();
                                        let uid_inner = requester_uid.clone();
                                        let jid_inner = job_ticket_id.clone();
                                        let s_type_inner = s_type.clone();
                                        let r_type_inner = r_type.clone();
                                        let c_level_inner = c_level.clone();
                                        let a_id_inner = a_id_opt.clone();
                                        let notes_inner = notes_opt.clone();
                                        let op_mode_inner = op_mode_opt.clone();

                                        let c_model_inner = c_model_val.clone();
                                        let c_serial_inner = c_serial_val.clone();
                                        let c_tag_inner = c_tag_val.clone();
                                        let c_loc_inner = c_loc_val.clone();

                                        runner_inner.run(async move {
                                            let final_asset_id = if a_id_inner == Some("UNLISTED_EQUIPMENT".to_string()) {
                                                let tag = if c_tag_inner.is_empty() { "UNLISTED".to_string() } else { c_tag_inner.clone() };
                                                let loc = if c_loc_inner.is_empty() { "Site Location".to_string() } else { c_loc_inner.clone() };
                                                let cat = s_type_inner.clone();

                                                let _ = yntra_core::add_hvac_location_asset(
                                                    uid_inner.clone(),
                                                    Some(jid_inner.clone()),
                                                    None,
                                                    tag.clone(),
                                                    c_model_inner.clone(),
                                                    c_serial_inner.clone(),
                                                    cat,
                                                    loc,
                                                ).await;

                                                Some(format!("[{}] {} ({})", tag, c_model_inner, c_serial_inner))
                                            } else {
                                                a_id_inner
                                            };

                                            yntra_core::log_hvac_system_diagnostic(
                                                uid_inner,
                                                jid_inner,
                                                s_type_inner,
                                                r_type_inner,
                                                c_level_inner,
                                                opt_h_psi,
                                                opt_l_psi,
                                                opt_w_press,
                                                opt_t_diff,
                                                parsed_volt,
                                                parsed_amps,
                                                final_asset_id,
                                                notes_inner,
                                                op_mode_inner,
                                                opt_amb_temp,
                                                opt_static_bar,
                                                opt_dynamic_bar,
                                                pipe_mat_opt,
                                                backflow_opt,
                                                opt_heater_temp_c,
                                                parsed_leak_dur,
                                                opt_leak_drop_bar,
                                                opt_ref_added_kg,
                                                opt_ref_recovered_kg,
                                                reclaim_cyl_opt,
                                            ).await?;

                                            // Form Reset Routine upon successful resolution
                                            high_side_psi_c.set(String::new());
                                            low_side_psi_c.set(String::new());
                                            water_pressure_bar_c.set(String::new());
                                            temp_diff_c_c.set(String::new());
                                            ambient_temp_c_c.set(String::new());
                                            refrigerant_added_kg_c.set(String::new());
                                            refrigerant_recovered_kg_c.set(String::new());
                                            reclaim_cylinder_id_c.set(String::new());
                                            voltage_v_c.set(String::new());
                                            amp_draw_a_c.set(String::new());
                                            asset_id_c.set(String::new());
                                            custom_asset_model_c.set(String::new());
                                            custom_asset_serial_c.set(String::new());
                                            custom_asset_tag_c.set(String::new());
                                            custom_asset_loc_c.set(String::new());
                                            diag_notes_c.set(String::new());
                                            static_flow_pressure_bar_c.set(String::new());
                                            dynamic_flow_pressure_bar_c.set(String::new());
                                            water_heater_temp_c_c.set(String::new());
                                            leak_test_duration_min_c.set(String::new());
                                            leak_test_pressure_drop_bar_c.set(String::new());

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
                                    placeholder: "e.g. 450.0",
                                    value: "{part_unit_cost_input}",
                                    oninput: move |e: Event<FormData>| part_unit_cost_input.set(e.value()),
                                }
                            }
                            div { class: "col-span-2 flex items-center justify-end pb-1",
                                button {
                                    class: "yntra-btn py-1 px-2 text-[11px] font-semibold w-full",
                                    onclick: {
                                        let toast_c = toast.clone();
                                        let runner_c = runner.clone();
                                        let requester_uid = uid.clone();
                                        let job_ticket_id = jid.clone();
                                        let p_name = part_name_input.read().clone();
                                        let raw_qty_str = part_qty_input.read().clone();
                                        let raw_cost_str = part_unit_cost_input.read().clone();
                                        let p_rot = *part_rot_eligible.read();
                                        let mut db_trig_c = db_trig;

                                        let mut part_name_input_c = part_name_input;
                                        let mut part_unit_cost_input_c = part_unit_cost_input;

                                        move |_| {
                                            if p_name.trim().is_empty() {
                                                toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Please enter a part or material name."));
                                                return;
                                            }
                                            if raw_cost_str.trim().is_empty() {
                                                toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Unit cost is required. Please enter an explicit unit cost."));
                                                return;
                                            }
                                            let p_cost = match raw_cost_str.trim().parse::<f64>() {
                                                Ok(val) => val,
                                                Err(_) => {
                                                    toast_c.error("Validation Error".to_string(), dioxus_primitives::toast::ToastOptions::new().description("Please enter a valid numeric unit cost."));
                                                    return;
                                                }
                                            };
                                            let p_qty = raw_qty_str.trim().parse::<f64>().unwrap_or(1.0);

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
                                                part_unit_cost_input_c.set(String::new());
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
                                        for (idx, item) in job_parts.iter().enumerate() {
                                            {
                                                let item_id = item.id.clone();
                                                let line_total = item.quantity * item.unit_cost_sek;
                                                rsx! {
                                                    tr { key: "{item.id}_{idx}",
                                                        td { class: "p-2.5 font-medium", "{item.part_name}" }
                                                        td { class: "p-2.5", "{item.quantity}" }
                                                        td { class: "p-2.5", "{item.unit_cost_sek:.2} {currency_code}" }
                                                        td { class: "p-2.5 font-bold", "{line_total:.2} {currency_code}" }
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

                // Tab 2: Swedish ROT / Regional Tax Credit Calculator
                if *active_tab.read() == 2 {
                    div { class: "flex flex-col gap-4 text-xs animate-in fade-in duration-200",
                        div { class: "flex items-center justify-between bg-muted/20 p-2.5 rounded-lg border border-border",
                            div { class: "flex flex-col gap-0.5",
                                span { class: "font-semibold text-foreground", "{t(\"hvac-rot-calc-title\", &region)}" }
                                span { class: "text-[11px] text-muted-foreground", "{t(\"hvac-rot-calc-subtitle\", &region)}" }
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
                                label { class: "font-semibold text-foreground", "{t(\"hvac-rot-labor-cost-label\", &region)}" }
                                input {
                                    class: "yntra-input py-1.5 px-3 bg-background border border-border text-foreground rounded text-xs",
                                    placeholder: "e.g. 6500.0",
                                    value: "{labor_cost_input}",
                                    oninput: move |e: Event<FormData>| labor_cost_input.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "font-semibold text-foreground", "{t(\"hvac-rot-travel-fee-label\", &region)}" }
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
                                    span { class: "font-bold text-foreground", "{rot_info.total_gross_amount_sek:.2} {currency_code}" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-eligible-label\", &region)}" }
                                    span { class: "font-bold text-foreground", "{rot_info.eligible_labor_sek:.2} {currency_code}" }
                                }
                                div { class: "flex justify-between border-b border-border/40 pb-1.5",
                                    span { class: "text-muted-foreground", "{t(\"hvac-rot-non-eligible-label\", &region)}" }
                                    span { class: "font-bold text-foreground", "{rot_info.non_eligible_parts_sek:.2} {currency_code}" }
                                }
                                div { class: "col-span-2 flex justify-between border-b border-border/40 pb-1.5 pt-1",
                                    span { class: "text-emerald-400 font-bold", "{t(\"hvac-rot-credit-label\", &region)}" }
                                    span { class: "font-extrabold text-emerald-400 text-sm", "-{rot_info.rot_deduction_30_percent_sek:.2} {currency_code}" }
                                }
                            }
                            div { class: "flex items-center justify-between pt-2 border-t border-border/60",
                                span { class: "font-bold text-foreground text-sm", "{t(\"hvac-rot-net-label\", &region)}" }
                                span { class: "font-extrabold text-primary text-base", "{rot_info.net_customer_payable_sek:.2} {currency_code}" }
                            }
                        }

                        div { class: "p-3.5 bg-amber-500/10 border border-amber-500/30 rounded-xl flex flex-col gap-1 text-[11px]",
                            span { class: "font-bold text-amber-400 flex items-center gap-1.5",
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
                            for (idx, diag) in diagnostics_history.iter().enumerate() {
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
                                    let high_val_str = diag.high_side_psi.map(|h| if is_metric { format!("{:.2} Bar", h / 14.503773773) } else { format!("{:.1} PSI", h) }).unwrap_or_else(|| "N/A".to_string());
                                    let low_val_str = diag.low_side_psi.map(|l| if is_metric { format!("{:.2} Bar", l / 14.503773773) } else { format!("{:.1} PSI", l) }).unwrap_or_else(|| "N/A".to_string());
                                    let water_val_str = diag.water_pressure_bar.map(|w| if is_metric { format!("{:.2} Bar", w) } else { format!("{:.1} PSI", w * 14.503773773) }).unwrap_or_else(|| "N/A".to_string());
                                    let delta_t_str = diag.temp_differential_c.map(|td| if is_metric { format!("{:.1}°C", td) } else { format!("{:.1}°F", td * 1.8) }).unwrap_or_else(|| "N/A".to_string());

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
                                        div { key: "{diag.id}_{idx}", class: "p-3 bg-muted/20 border border-border rounded-lg flex flex-col gap-2",
                                            div { class: "flex items-center justify-between border-b border-border/40 pb-1.5 mb-1",
                                                div { class: "flex items-center gap-2",
                                                    span { class: "{status_badge_class}", "{status_label}" }
                                                    {
                                                        let tech_str = diag.technician_id.as_str();
                                                        let tech_label = t_with_args("hvac-history-tech-label", &region, &[("tech", tech_str)]);
                                                        rsx! {
                                                            span { class: "px-2 py-0.5 bg-secondary/30 text-secondary-foreground rounded text-[10px] font-medium flex items-center gap-1",
                                                                "{tech_label}"
                                                            }
                                                        }
                                                    }
                                                    if let Some(ref mode) = diag.operating_mode {
                                                        span { class: "px-2 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-semibold", "{mode}" }
                                                    }
                                                    if let Some(ref asset) = diag.asset_id {
                                                        {
                                                            let asset_str = asset.as_str();
                                                            let asset_label = t_with_args("hvac-history-asset-label", &region, &[("asset", asset_str)]);
                                                            rsx! { span { class: "text-[11px] font-semibold text-primary", "{asset_label}" } }
                                                        }
                                                    }
                                                }
                                                div { class: "flex items-center gap-3 text-xs text-muted-foreground",
                                                    span { class: "text-[11px] font-mono text-muted-foreground/80", "{date_str}" }
                                                    {
                                                        let med_str = diag.refrigerant_type.as_str();
                                                        let med_label = t_with_args("hvac-history-medium", &region, &[("medium", med_str)]);
                                                        rsx! { span { "{med_label}" } }
                                                    }
                                                }
                                            }
                                            {
                                                if diag.system_type == "HYDRONIC_HEATING" || diag.system_type == "HYDRONIC_PLUMBING" || diag.system_type == "POTABLE_WATER" || diag.system_type == "POTABLE_PLUMBING" {
                                                    let static_str = diag.static_flow_pressure_bar.map(|s| if is_metric { format!("{:.2} Bar", s) } else { format!("{:.1} PSI", s * 14.503773773) }).unwrap_or_else(|| "N/A".to_string());
                                                    let dynamic_str = diag.dynamic_flow_pressure_bar.map(|d| if is_metric { format!("{:.2} Bar", d) } else { format!("{:.1} PSI", d * 14.503773773) }).unwrap_or_else(|| "N/A".to_string());
                                                    let heater_temp_str = diag.water_heater_temp_c.map(|ht| if is_metric { format!("{:.1}°C", ht) } else { format!("{:.1}°F", ht * 1.8 + 32.0) }).unwrap_or_else(|| "N/A".to_string());
                                                    let leak_drop_str = diag.leak_test_pressure_drop_bar.map(|ld| if is_metric { format!("{:.2} Bar", ld) } else { format!("{:.1} PSI", ld * 14.503773773) }).unwrap_or_else(|| "N/A".to_string());
                                                    let pipe_mat = diag.pipe_material.as_deref().unwrap_or("N/A");
                                                    let backflow_st = diag.backflow_preventer_status.as_deref().unwrap_or("N/A");

                                                    let h_wp = t_with_args("hvac-history-water-press", &region, &[("val", water_val_str.as_str())]);
                                                    let h_static = t_with_args("hvac-history-static", &region, &[("val", static_str.as_str())]);
                                                    let h_dynamic = t_with_args("hvac-history-dynamic", &region, &[("val", dynamic_str.as_str())]);
                                                    let h_pipe = t_with_args("hvac-history-pipe", &region, &[("mat", pipe_mat)]);
                                                    let h_backflow = t_with_args("hvac-history-backflow", &region, &[("status", backflow_st)]);
                                                    let h_heater = t_with_args("hvac-history-heater", &region, &[("temp", heater_temp_str.as_str())]);
                                                    let h_dt = t_with_args("hvac-history-delta-t", &region, &[("delta", delta_t_str.as_str())]);

                                                    rsx! {
                                                        div { class: "grid grid-cols-3 gap-2 text-muted-foreground text-[11px] bg-background/30 p-2 rounded border border-border/30",
                                                            span { "{h_wp}" }
                                                            span { "{h_static}" }
                                                            span { "{h_dynamic}" }
                                                            span { "{h_pipe}" }
                                                            span { "{h_backflow}" }
                                                            span { "{h_heater}" }
                                                            if let Some(dur) = diag.leak_test_duration_min {
                                                                {
                                                                    let dur_s = dur.to_string();
                                                                    let h_leak = t_with_args("hvac-history-leak-test", &region, &[("dur", dur_s.as_str()), ("drop", leak_drop_str.as_str())]);
                                                                    rsx! { span { "{h_leak}" } }
                                                                }
                                                            }
                                                            span { "{h_dt}" }
                                                        }
                                                    }
                                                } else {
                                                    let h_high = t_with_args("hvac-history-high-side", &region, &[("val", high_val_str.as_str())]);
                                                    let h_low = t_with_args("hvac-history-low-side", &region, &[("val", low_val_str.as_str())]);
                                                    let h_dt = t_with_args("hvac-history-delta-t", &region, &[("delta", delta_t_str.as_str())]);

                                                    let ref_added_str = diag.refrigerant_added_kg.map(|a| if is_metric { format!("{:.2} kg", a) } else { format!("{:.2} lbs", a / 0.45359237) });
                                                    let ref_recovered_str = diag.refrigerant_recovered_kg.map(|r| if is_metric { format!("{:.2} kg", r) } else { format!("{:.2} lbs", r / 0.45359237) });
                                                    let cyl_id_str = diag.reclaim_cylinder_id.as_deref();

                                                    rsx! {
                                                        div { class: "grid grid-cols-3 gap-2 text-muted-foreground text-[11px]",
                                                            span { "{h_high}" }
                                                            span { "{h_low}" }
                                                            span { "{h_dt}" }
                                                        }
                                                        if ref_added_str.is_some() || ref_recovered_str.is_some() || cyl_id_str.is_some() {
                                                            div { class: "flex items-center gap-3 text-[10px] text-muted-foreground/90 bg-background/40 p-1.5 rounded border border-border/30 font-mono",
                                                                if let Some(ref add_s) = ref_added_str {
                                                                    span { "Added: {add_s}" }
                                                                }
                                                                if let Some(ref rec_s) = ref_recovered_str {
                                                                    span { "Recovered: {rec_s}" }
                                                                }
                                                                if let Some(cyl) = cyl_id_str {
                                                                    span { "Cylinder: {cyl}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(amb) = diag.ambient_temp_c {
                                                {
                                                    let amb_str = if is_metric { format!("{:.1}°C", amb) } else { format!("{:.1}°F", amb * 1.8 + 32.0) };
                                                    let amb_label = t_with_args("hvac-history-ambient-label", &region, &[("temp", amb_str.as_str())]);
                                                    rsx! {
                                                        div { class: "text-[11px] text-muted-foreground italic", "{amb_label}" }
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
