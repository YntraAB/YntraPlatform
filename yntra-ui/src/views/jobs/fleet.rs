use crate::components;
use crate::locales::t;
use crate::utils::use_action_runner;
use dioxus::prelude::*;
use yntra_core::{CommercialRouteRestrictions, DriverVehicleInspectionReport, MoveVehicle};

#[derive(Props, Clone)]
pub struct FleetViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for FleetViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn FleetView(props: FleetViewProps) -> Element {
    let runner = use_action_runner();
    let db_trig = *props.db_trigger.read();
    let mut db_trigger = props.db_trigger;
    let active_uid = props.active_user_id.read().clone();
    let region = props.auth_region.read().clone();

    // Registration Modal State
    let mut show_register_modal = use_signal(|| false);
    let mut form_validation_error = use_signal(|| None::<String>);

    // Deletion Confirmation Modal State
    let mut vehicle_to_delete = use_signal(|| None::<(String, String, String)>);

    // SOTA Interactive Drawer State: stores Option<MoveVehicle>
    let mut active_drawer_vehicle = use_signal(|| None::<MoveVehicle>);
    let mut drawer_tab = use_signal(|| "dvir".to_string()); // "dvir", "routing", "telemetry"

    // Digital DVIR Inspection Form Signals
    let mut dvir_type = use_signal(|| "PRE_TRIP".to_string());
    let mut dvir_brakes = use_signal(|| true);
    let mut dvir_tires = use_signal(|| true);
    let mut dvir_lights = use_signal(|| true);
    let mut dvir_steering = use_signal(|| true);
    let mut dvir_coupling = use_signal(|| true);
    let mut dvir_defects = use_signal(String::new);

    // Route Clearance Evaluator Signals
    let mut eval_origin = use_signal(|| "Stockholm Central, Stockholm".to_string());
    let mut eval_dest = use_signal(|| "Sveavägen 100, Stockholm".to_string());
    let mut route_eval_result = use_signal(|| None::<CommercialRouteRestrictions>);

    // Filter & Search Signals
    let mut search_query = use_signal(String::new);
    let mut status_filter = use_signal(|| "all".to_string());

    // Registration Form Inputs
    let mut new_vehicle_name = use_signal(String::new);
    let mut new_vehicle_plate = use_signal(String::new);
    let mut new_vehicle_capacity = use_signal(|| 15.0);
    let mut new_vehicle_gps_id = use_signal(String::new);

    let vehicles_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move { yntra_core::get_vehicles(uid).await.unwrap_or_default() }
    });

    let is_loading = vehicles_res.read().is_none();
    let vehicles: Vec<MoveVehicle> = vehicles_res.read().clone().unwrap_or_default();

    // Resource for active vehicle DVIR reports
    let uid_for_dvir_res = active_uid.clone();
    let active_vid_dvir = active_drawer_vehicle.read().as_ref().map(|v| v.id.clone());
    let dvir_history_res = use_resource(move || {
        let _ = db_trig;
        let uid = uid_for_dvir_res.clone();
        let vid_opt = active_vid_dvir.clone();
        async move {
            if let Some(vid) = vid_opt {
                yntra_core::get_vehicle_dvir_reports(uid, vid)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });

    let dvir_reports: Vec<DriverVehicleInspectionReport> =
        dvir_history_res.read().clone().unwrap_or_default();

    // Compute Metrics
    let total_vehicles = vehicles.len();
    let total_capacity: f64 = vehicles.iter().map(|v| v.capacity_m3).sum();
    let active_count = vehicles.iter().filter(|v| v.status == "active").count();
    let heavy_fleet_count = vehicles.iter().filter(|v| v.capacity_m3 > 20.0).count();
    let avg_capacity = if total_vehicles > 0 {
        total_capacity / total_vehicles as f64
    } else {
        0.0
    };

    // Filter vehicles
    let filtered_vehicles: Vec<MoveVehicle> = vehicles
        .iter()
        .cloned()
        .filter(|v| {
            let q = search_query.read().to_lowercase();
            let matches_query = q.is_empty()
                || v.name.to_lowercase().contains(&q)
                || v.license_plate.to_lowercase().contains(&q);

            let st = status_filter.read().clone();
            let matches_status = match st.as_str() {
                "active" => v.status == "active",
                "inactive" => v.status != "active",
                _ => true,
            };

            matches_query && matches_status
        })
        .collect();

    rsx! {
        div { class: "mx-auto w-full max-w-5xl p-6 flex flex-col gap-6 animate-in fade-in duration-300",

            // Header with action button
            div { class: "flex items-center justify-between flex-wrap gap-4",
                div { class: "flex flex-col gap-1",
                    h1 { class: "text-2xl font-bold text-foreground", "{t(\"fleet-title\", &region)}" }
                    p { class: "text-sm text-muted-foreground", "{t(\"fleet-subtitle\", &region)}" }
                }

                button {
                    onclick: move |_| {
                        form_validation_error.set(None);
                        show_register_modal.set(true);
                    },
                    class: "px-4 py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all border-0 cursor-pointer shadow flex items-center gap-2",
                    components::LucideIcon { name: "plus", size: "16" }
                    "{t(\"fleet-register-btn\", &region)}"
                }
            }

            // Top Metrics Summary Bar
            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 w-full",
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "{t(\"fleet-kpi-total-vehicles\", &region)}" }
                    div { class: "text-2xl font-black text-foreground", "{total_vehicles}" }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "{t(\"fleet-kpi-total-capacity\", &region)}" }
                    div { class: "text-2xl font-black text-primary", "{total_capacity:.1} m³" }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "{t(\"fleet-kpi-active\", &region)}" }
                    div { class: "text-2xl font-black text-emerald-500 flex items-center gap-2",
                        "{active_count}"
                        span { class: "w-2 h-2 rounded-full bg-emerald-500 animate-pulse" }
                    }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "{t(\"fleet-kpi-avg-capacity\", &region)}" }
                    div { class: "text-2xl font-black text-foreground flex items-center justify-between",
                        "{avg_capacity:.1} m³"
                        if heavy_fleet_count > 0 {
                            span {
                                class: "text-[9px] font-bold px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-500 border border-amber-500/20",
                                title: "Heavy Transport Rig Fleet",
                                "Heavy ({heavy_fleet_count})"
                            }
                        }
                    }
                }
            }

            // Main Fleet Overview Card
            components::Card {
                class: "border border-border bg-sidebar p-1 shadow-md flex flex-col w-full",
                components::CardHeader {
                    class: "pb-4 border-b border-border/40 flex flex-col gap-3 md:flex-row md:items-center md:justify-between",
                    div { class: "flex items-center gap-3",
                        div { class: "rounded-xl bg-primary/10 p-2.5 text-primary",
                            components::LucideIcon { name: "truck", class: "h-5 w-5" }
                        }
                        div {
                            components::CardTitle { class: "text-lg font-bold", "{t(\"fleet-registered-title\", &region)}" }
                            components::CardDescription { class: "text-xs", "{t(\"fleet-registered-desc\", &region)}" }
                        }
                    }

                    // Search & Filter Controls
                    div { class: "flex items-center gap-2 flex-wrap",
                        div { class: "relative flex-1 md:w-64",
                            input {
                                class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 pl-8 text-xs text-foreground focus:outline-none focus:border-primary transition-all",
                                placeholder: "{t(\"fleet-search-placeholder\", &region)}",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value())
                            }
                            div { class: "absolute left-2.5 top-2 text-muted-foreground pointer-events-none",
                                components::LucideIcon { name: "search", size: "12" }
                            }
                        }

                        div { class: "flex items-center gap-1 bg-background border border-border p-1 rounded-xl shadow-xs",
                            button {
                                onclick: move |_| status_filter.set("all".to_string()),
                                class: format!("px-2.5 py-1 rounded-lg text-[10px] font-bold border-0 cursor-pointer transition-all {}", if *status_filter.read() == "all" { "bg-primary text-primary-foreground" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                "{t(\"fleet-filter-all\", &region)}"
                            }
                            button {
                                onclick: move |_| status_filter.set("active".to_string()),
                                class: format!("px-2.5 py-1 rounded-lg text-[10px] font-bold border-0 cursor-pointer transition-all {}", if *status_filter.read() == "active" { "bg-primary text-primary-foreground" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                "{t(\"fleet-filter-active\", &region)}"
                            }
                        }
                    }
                }

                components::CardContent {
                    class: "pt-4 flex flex-1 flex-col",

                    if is_loading {
                        // Skeleton Loader State
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-3.5 animate-pulse",
                            for i in 0..4 {
                                div { key: "{i}", class: "p-4 rounded-xl border border-border/40 bg-background/30 flex items-center justify-between h-24",
                                    div { class: "flex items-center gap-3.5 w-full",
                                        div { class: "h-11 w-11 rounded-xl bg-muted/60 shrink-0" }
                                        div { class: "flex-1 space-y-2",
                                            div { class: "h-3.5 bg-muted/60 rounded w-1/2" }
                                            div { class: "h-2.5 bg-muted/40 rounded w-3/4" }
                                        }
                                    }
                                }
                            }
                        }
                    } else if filtered_vehicles.is_empty() {
                        {
                            let empty_desc = if vehicles.is_empty() { t("fleet-empty-desc-first", &region) } else { t("fleet-empty-desc-filter", &region) };
                            rsx! {
                                div { class: "flex flex-col items-center justify-center py-16 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10 my-4",
                                    components::LucideIcon { name: "truck", class: "h-10 w-10 opacity-20 mb-3" }
                                    p { class: "text-sm font-bold text-foreground m-0", "{t(\"fleet-empty-title\", &region)}" }
                                    p { class: "text-xs text-muted-foreground mt-1 max-w-xs m-0", "{empty_desc}" }
                                }
                            }
                        }
                    } else {
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-3.5",
                            for vehicle in filtered_vehicles.iter() {
                                {
                                    let v = vehicle.clone();
                                    let v_id = vehicle.id.clone();
                                    let v_name = vehicle.name.clone();
                                    let v_plate = vehicle.license_plate.clone();
                                    let v_capacity = vehicle.capacity_m3;
                                    let v_status = vehicle.status.clone();
                                    let v_device = vehicle.gps_device_id.clone();
                                    let is_active = v_status == "active";

                                    // Compute Commercial Routing & Specs Risk Badges
                                    let is_heavy_rig = v_capacity > 35.0;
                                    let is_medium_truck = v_capacity > 15.0 && v_capacity <= 35.0;
                                    let requires_permit = v_capacity > 20.0;

                                    // Compute ping status
                                    let has_gps = v_device.is_some() || vehicle.latitude.is_some();
                                    let last_ping_ms = vehicle.last_ping.unwrap_or(0);
                                    let now_ms = chrono::Utc::now().timestamp_millis();
                                    let is_ping_recent = has_gps && (now_ms - last_ping_ms).abs() < 600_000;

                                    let v_name_del = v_name.clone();
                                    let v_plate_del = v_plate.clone();
                                    let v_id_del = v_id.clone();
                                    let uid_sim = active_uid.clone();
                                    let v_id_sim = v_id.clone();
                                    let runner_sim = runner.clone();
                                    let v_drawer = v.clone();

                                    rsx! {
                                        div {
                                            key: "{v_id}",
                                            class: "flex flex-col p-4 rounded-xl border border-border/40 bg-background/60 hover:bg-background hover:border-primary/30 transition-all shadow-xs group gap-3 cursor-pointer",
                                            onclick: move |_| {
                                                route_eval_result.set(None);
                                                active_drawer_vehicle.set(Some(v_drawer.clone()));
                                            },

                                            // Top Row: Info & Status
                                            div { class: "flex items-start justify-between gap-3 min-w-0",
                                                div { class: "flex items-start gap-3.5 min-w-0",
                                                    div { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary border border-primary/20 mt-0.5",
                                                        components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                                    }
                                                    div { class: "min-w-0 flex-1",
                                                        div { class: "flex items-center gap-2 flex-wrap",
                                                            span { class: "text-sm font-bold text-foreground truncate group-hover:text-primary transition-all", "{v_name}" }
                                                            span { class: "font-mono font-bold bg-muted text-foreground border border-border/60 px-1.5 py-0.5 rounded text-[10px] tracking-wide shrink-0",
                                                                "{v_plate}"
                                                            }
                                                        }
                                                        div { class: "text-xs text-muted-foreground flex items-center gap-2 mt-1 flex-wrap",
                                                            span { class: "font-semibold text-primary", "{t(\"fleet-capacity-label\", &region)} {v_capacity} m³" }
                                                            if let Some(ref device) = v_device {
                                                                span { class: "text-[10px] text-slate-400 font-mono flex items-center gap-1",
                                                                    components::LucideIcon { name: "rss", size: "10" }
                                                                    "GPS: {device}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                div { class: "flex items-center gap-1.5 shrink-0",
                                                    span {
                                                        class: format!("px-2.5 py-1 rounded-full text-[10px] font-extrabold tracking-wide flex items-center gap-1.5 {}",
                                                            if is_active { "bg-emerald-500/10 text-emerald-500 border border-emerald-500/20" } else { "bg-slate-500/10 text-slate-400 border border-slate-500/20" }
                                                        ),
                                                        span { class: format!("w-1.5 h-1.5 rounded-full {}", if is_active { "bg-emerald-500 animate-pulse" } else { "bg-slate-400" }) }
                                                        if is_active { "{t(\"fleet-status-active\", &region)}" } else { "{t(\"fleet-status-inactive\", &region)}" }
                                                    }

                                                    button {
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            vehicle_to_delete.set(Some((v_id_del.clone(), v_name_del.clone(), v_plate_del.clone())));
                                                        },
                                                        class: "p-2 rounded-lg text-slate-400 hover:text-rose-500 hover:bg-rose-500/10 border-0 bg-transparent cursor-pointer transition-all flex items-center justify-center opacity-70 group-hover:opacity-100",
                                                        title: "{t(\"fleet-delete-tooltip\", &region)}",
                                                        components::LucideIcon { name: "trash-2", size: "16" }
                                                    }
                                                }
                                            }

                                            // Commercial Specs & Compliance Badges
                                            div { class: "flex items-center gap-1.5 flex-wrap pt-2 border-t border-border/30 text-[10px]",
                                                // DOT Safety Badge
                                                span { class: "px-2 py-0.5 rounded-md font-bold bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 flex items-center gap-1",
                                                    components::LucideIcon { name: "shield-check", size: "10" }
                                                    "{t(\"fleet-dot-compliant\", &region)}"
                                                }

                                                if is_heavy_rig {
                                                    span { class: "px-2 py-0.5 rounded-md font-medium bg-amber-500/10 text-amber-600 border border-amber-500/20 flex items-center gap-1",
                                                        components::LucideIcon { name: "alert-triangle", size: "10" }
                                                        "{t(\"fleet-badge-low-bridge\", &region)}"
                                                    }
                                                }

                                                if is_heavy_rig || is_medium_truck {
                                                    span { class: "px-2 py-0.5 rounded-md font-medium bg-blue-500/10 text-blue-600 border border-blue-500/20 flex items-center gap-1",
                                                        components::LucideIcon { name: "scale", size: "10" }
                                                        "{t(\"fleet-badge-weight-limit\", &region)}"
                                                    }
                                                }

                                                if requires_permit {
                                                    span { class: "px-2 py-0.5 rounded-md font-medium bg-purple-500/10 text-purple-600 border border-purple-500/20 flex items-center gap-1",
                                                        components::LucideIcon { name: "file-text", size: "10" }
                                                        "{t(\"fleet-badge-permit-required\", &region)}"
                                                    }
                                                }

                                                // GPS Ping Status Badge & Dev Simulation Trigger
                                                div { class: "ml-auto flex items-center gap-2",
                                                    if is_ping_recent {
                                                        span { class: "text-[9px] font-semibold text-emerald-500 flex items-center gap-1",
                                                            components::LucideIcon { name: "navigation", size: "9" }
                                                            "{t(\"fleet-ping-just-now\", &region)}"
                                                        }
                                                    } else {
                                                        span { class: "text-[9px] font-medium text-slate-400",
                                                            "{t(\"fleet-ping-offline\", &region)}"
                                                        }
                                                    }

                                                    button {
                                                        onclick: {
                                                            let uid = uid_sim.clone();
                                                            let vid = v_id_sim.clone();
                                                            let runner_c = runner_sim.clone();
                                                            move |e| {
                                                                e.stop_propagation();
                                                                let uid = uid.clone();
                                                                let vid = vid.clone();
                                                                let runner_c = runner_c.clone();
                                                                runner_c.run(async move {
                                                                    yntra_core::simulate_vehicle_movement(uid, vid, 1).await?;
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                    Ok(())
                                                                });
                                                            }
                                                        },
                                                        class: "px-1.5 py-0.5 rounded text-[9px] font-bold bg-muted hover:bg-primary/20 text-foreground border border-border cursor-pointer transition-all",
                                                        title: "Simulate GPS Movement (Testing)",
                                                        "Simulate Ping"
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

            // SOTA Interactive Vehicle Specs & Safety Drawer Overlay
            if let Some(active_v) = active_drawer_vehicle.read().clone() {
                {
                    let drawer_vid = active_v.id.clone();
                    let drawer_vname = active_v.name.clone();
                    let drawer_vplate = active_v.license_plate.clone();
                    let drawer_vcap = active_v.capacity_m3;
                    let drawer_vdevice = active_v.gps_device_id.clone();
                    let drawer_lat = active_v.latitude;
                    let drawer_lon = active_v.longitude;
                    let drawer_ping = active_v.last_ping;

                    let runner_dvir = runner.clone();
                    let runner_eval = runner.clone();
                    let runner_sim_drawer = runner.clone();
                    let uid_dvir = active_uid.clone();
                    let uid_eval = active_uid.clone();
                    let uid_sim_drawer = active_uid.clone();

                    rsx! {
                        div {
                            style: "position: fixed; inset: 0; z-index: 9999; display: flex; justify-content: flex-end; background: rgba(0, 0, 0, 0.65); backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);",
                            onclick: move |_| active_drawer_vehicle.set(None),

                            div {
                                class: "w-full max-w-xl bg-sidebar border-l border-border h-full flex flex-col shadow-2xl animate-in slide-in-from-right duration-300 overflow-y-auto p-6 space-y-5",
                                onclick: move |e| e.stop_propagation(),

                                // Header
                                div { class: "flex items-start justify-between border-b border-border/40 pb-4",
                                    div { class: "flex items-center gap-3.5",
                                        div { class: "rounded-xl bg-primary/10 p-3 text-primary border border-primary/20",
                                            components::LucideIcon { name: "truck", class: "h-6 w-6" }
                                        }
                                        div {
                                            div { class: "flex items-center gap-2",
                                                h2 { class: "text-lg font-bold text-foreground m-0", "{drawer_vname}" }
                                                span { class: "font-mono font-bold bg-muted text-foreground border border-border/60 px-2 py-0.5 rounded text-xs", "{drawer_vplate}" }
                                            }
                                            p { class: "text-xs text-muted-foreground m-0 mt-1", "{t(\"fleet-drawer-title\", &region)} • Capacity {drawer_vcap} m³" }
                                        }
                                    }

                                    button {
                                        class: "p-2 rounded-xl border-0 bg-transparent text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer transition-all",
                                        onclick: move |_| active_drawer_vehicle.set(None),
                                        components::LucideIcon { name: "x", size: "20" }
                                    }
                                }

                                // Tab Navigation Bar
                                div { class: "flex items-center gap-1 p-1 bg-background border border-border rounded-xl",
                                    button {
                                        onclick: move |_| drawer_tab.set("dvir".to_string()),
                                        class: format!("flex-1 py-2 text-xs font-bold rounded-lg border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5 {}", if *drawer_tab.read() == "dvir" { "bg-primary text-primary-foreground shadow" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                        components::LucideIcon { name: "clipboard-check", size: "14" }
                                        "{t(\"fleet-drawer-tab-dvir\", &region)}"
                                    }
                                    button {
                                        onclick: move |_| drawer_tab.set("routing".to_string()),
                                        class: format!("flex-1 py-2 text-xs font-bold rounded-lg border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5 {}", if *drawer_tab.read() == "routing" { "bg-primary text-primary-foreground shadow" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                        components::LucideIcon { name: "map-pin", size: "14" }
                                        "{t(\"fleet-drawer-tab-routing\", &region)}"
                                    }
                                    button {
                                        onclick: move |_| drawer_tab.set("telemetry".to_string()),
                                        class: format!("flex-1 py-2 text-xs font-bold rounded-lg border-0 cursor-pointer transition-all flex items-center justify-center gap-1.5 {}", if *drawer_tab.read() == "telemetry" { "bg-primary text-primary-foreground shadow" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                        components::LucideIcon { name: "activity", size: "14" }
                                        "{t(\"fleet-drawer-tab-telemetry\", &region)}"
                                    }
                                }

                                // Tab Content 1: Digital DVIR & DOT Safety
                                if *drawer_tab.read() == "dvir" {
                                    div { class: "space-y-5 animate-in fade-in duration-200",
                                        // Form Card
                                        div { class: "p-4 rounded-2xl border border-border bg-background/60 space-y-4 shadow-xs",
                                            h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                                components::LucideIcon { name: "file-plus", size: "16", class: "text-primary" }
                                                "{t(\"fleet-dvir-submit-btn\", &region)}"
                                            }

                                            div { class: "grid grid-cols-2 gap-3",
                                                div {
                                                    label { class: "text-[11px] font-bold text-foreground block mb-1", "{t(\"fleet-dvir-inspection-type\", &region)}" }
                                                    select {
                                                        value: "{dvir_type}",
                                                        onchange: move |e| dvir_type.set(e.value()),
                                                        class: "w-full text-xs p-2 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:border-primary",
                                                        option { value: "PRE_TRIP", "{t(\"fleet-dvir-pre-trip\", &region)}" }
                                                        option { value: "POST_TRIP", "{t(\"fleet-dvir-post-trip\", &region)}" }
                                                    }
                                                }
                                            }

                                            // Inspection Item Checkboxes
                                            div { class: "grid grid-cols-2 gap-2 text-xs font-semibold text-foreground pt-1",
                                                label { class: "flex items-center gap-2 p-2 rounded-lg border border-border/40 bg-muted/30 cursor-pointer hover:bg-muted/60 transition-all",
                                                    input { r#type: "checkbox", checked: "{dvir_brakes}", onchange: move |e| dvir_brakes.set(e.value() == "true") }
                                                    "{t(\"fleet-dvir-check-brakes\", &region)}"
                                                }
                                                label { class: "flex items-center gap-2 p-2 rounded-lg border border-border/40 bg-muted/30 cursor-pointer hover:bg-muted/60 transition-all",
                                                    input { r#type: "checkbox", checked: "{dvir_tires}", onchange: move |e| dvir_tires.set(e.value() == "true") }
                                                    "{t(\"fleet-dvir-check-tires\", &region)}"
                                                }
                                                label { class: "flex items-center gap-2 p-2 rounded-lg border border-border/40 bg-muted/30 cursor-pointer hover:bg-muted/60 transition-all",
                                                    input { r#type: "checkbox", checked: "{dvir_lights}", onchange: move |e| dvir_lights.set(e.value() == "true") }
                                                    "{t(\"fleet-dvir-check-lights\", &region)}"
                                                }
                                                label { class: "flex items-center gap-2 p-2 rounded-lg border border-border/40 bg-muted/30 cursor-pointer hover:bg-muted/60 transition-all",
                                                    input { r#type: "checkbox", checked: "{dvir_steering}", onchange: move |e| dvir_steering.set(e.value() == "true") }
                                                    "{t(\"fleet-dvir-check-steering\", &region)}"
                                                }
                                                label { class: "col-span-2 flex items-center gap-2 p-2 rounded-lg border border-border/40 bg-muted/30 cursor-pointer hover:bg-muted/60 transition-all",
                                                    input { r#type: "checkbox", checked: "{dvir_coupling}", onchange: move |e| dvir_coupling.set(e.value() == "true") }
                                                    "{t(\"fleet-dvir-check-coupling\", &region)}"
                                                }
                                            }

                                            div {
                                                label { class: "text-[11px] font-bold text-foreground block mb-1", "{t(\"fleet-dvir-defects-label\", &region)}" }
                                                textarea {
                                                    value: "{dvir_defects}",
                                                    oninput: move |e| dvir_defects.set(e.value()),
                                                    placeholder: "{t(\"fleet-dvir-defects-placeholder\", &region)}",
                                                    class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all h-16 resize-none",
                                                }
                                            }

                                            button {
                                                onclick: {
                                                    let uid = uid_dvir.clone();
                                                    let vid = drawer_vid.clone();
                                                    let runner_c = runner_dvir.clone();
                                                    move |_| {
                                                        let uid = uid.clone();
                                                        let vid = vid.clone();
                                                        let runner_c = runner_c.clone();
                                                        let itype = dvir_type.read().clone();
                                                        let b_ok = *dvir_brakes.read();
                                                        let t_ok = *dvir_tires.read();
                                                        let l_ok = *dvir_lights.read();
                                                        let s_ok = *dvir_steering.read();
                                                        let c_ok = *dvir_coupling.read();
                                                        let notes_str = dvir_defects.read().trim().to_string();
                                                        let notes_opt = if notes_str.is_empty() { None } else { Some(notes_str) };

                                                        runner_c.run(async move {
                                                            yntra_core::submit_dvir_inspection(
                                                                uid, vid, "inspector-staff".to_string(), itype,
                                                                b_ok, t_ok, l_ok, s_ok, c_ok, notes_opt
                                                            ).await?;
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                            Ok(())
                                                        });
                                                    }
                                                },
                                                class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center justify-center gap-1.5",
                                                components::LucideIcon { name: "check-circle-2", size: "14" }
                                                "{t(\"fleet-dvir-submit-btn\", &region)}"
                                            }
                                        }

                                        // History Audit Log Timeline
                                        div { class: "space-y-3",
                                            h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                                components::LucideIcon { name: "history", size: "16", class: "text-primary" }
                                                "{t(\"fleet-dvir-history-title\", &region)}"
                                            }

                                            if dvir_reports.is_empty() {
                                                p { class: "text-xs text-muted-foreground py-4 text-center border border-dashed border-border rounded-xl m-0",
                                                    "{t(\"fleet-dvir-empty-history\", &region)}"
                                                }
                                            } else {
                                                div { class: "space-y-2 max-h-60 overflow-y-auto pr-1",
                                                    for rep in dvir_reports.iter() {
                                                        {
                                                            let r_id = rep.id.clone();
                                                            let r_type = rep.inspection_type.clone();
                                                            let r_status = rep.safety_status.clone();
                                                            let r_defects = rep.defects_found;
                                                            let r_notes = rep.defect_details.clone();
                                                            let is_out_of_service = r_status == "OUT_OF_SERVICE";

                                                            rsx! {
                                                                div { key: "{r_id}", class: "p-3 rounded-xl border border-border/40 bg-background/40 flex items-start justify-between gap-3 text-xs",
                                                                    div { class: "space-y-1 min-w-0 flex-1",
                                                                        div { class: "flex items-center gap-2",
                                                                            span { class: "font-bold text-foreground", "{r_type}" }
                                                                            span { class: format!("px-2 py-0.5 rounded text-[9px] font-black uppercase tracking-wider {}", if is_out_of_service { "bg-rose-500/10 text-rose-500 border border-rose-500/20" } else if r_defects { "bg-amber-500/10 text-amber-500 border border-amber-500/20" } else { "bg-emerald-500/10 text-emerald-500 border border-emerald-500/20" }),
                                                                                "{r_status}"
                                                                            }
                                                                        }
                                                                        if let Some(ref notes) = r_notes {
                                                                            p { class: "text-muted-foreground text-[11px] m-0 italic", "\"{notes}\"" }
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

                                // Tab Content 2: Commercial Heavy Route Clearance Evaluator
                                if *drawer_tab.read() == "routing" {
                                    div { class: "space-y-5 animate-in fade-in duration-200",
                                        div { class: "p-4 rounded-2xl border border-border bg-background/60 space-y-4 shadow-xs",
                                            h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                                components::LucideIcon { name: "navigation-2", size: "16", class: "text-primary" }
                                                "{t(\"fleet-drawer-tab-routing\", &region)}"
                                            }

                                            div { class: "space-y-3",
                                                div {
                                                    label { class: "text-[11px] font-bold text-foreground block mb-1", "{t(\"fleet-route-origin-label\", &region)}" }
                                                    input {
                                                        r#type: "text",
                                                        value: "{eval_origin}",
                                                        oninput: move |e| eval_origin.set(e.value()),
                                                        class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                    }
                                                }
                                                div {
                                                    label { class: "text-[11px] font-bold text-foreground block mb-1", "{t(\"fleet-route-dest-label\", &region)}" }
                                                    input {
                                                        r#type: "text",
                                                        value: "{eval_dest}",
                                                        oninput: move |e| eval_dest.set(e.value()),
                                                        class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                                    }
                                                }
                                            }

                                            button {
                                                onclick: {
                                                    let uid = uid_eval.clone();
                                                    let vid = drawer_vid.clone();
                                                    let runner_c = runner_eval.clone();
                                                    move |_| {
                                                        let uid = uid.clone();
                                                        let vid = vid.clone();
                                                        let runner_c = runner_c.clone();
                                                        let orig = eval_origin.read().trim().to_string();
                                                        let dest = eval_dest.read().trim().to_string();

                                                        runner_c.run(async move {
                                                            let res = yntra_core::evaluate_vehicle_route_clearance(uid, vid, orig, dest).await?;
                                                            route_eval_result.set(Some(res));
                                                            Ok(())
                                                        });
                                                    }
                                                },
                                                class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center justify-center gap-1.5",
                                                components::LucideIcon { name: "search", size: "14" }
                                                "{t(\"fleet-route-eval-btn\", &region)}"
                                            }
                                        }

                                        // Route Restriction Results Box
                                        if let Some(ref eval) = *route_eval_result.read() {
                                            div { class: "p-4 rounded-2xl border border-border bg-sidebar space-y-3 animate-in zoom-in-95 duration-200",
                                                h4 { class: "text-xs font-bold text-foreground flex items-center gap-2 m-0",
                                                    components::LucideIcon { name: "shield-alert", size: "16", class: "text-amber-500" }
                                                    "{t(\"fleet-route-results-title\", &region)}"
                                                }

                                                div { class: "space-y-2 text-xs",
                                                    for detail in eval.restriction_details.iter() {
                                                        div { key: "{detail}", class: "p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-600 font-semibold flex items-start gap-2",
                                                            components::LucideIcon { name: "alert-triangle", size: "14", class: "shrink-0 mt-0.5" }
                                                            span { "{detail}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Tab Content 3: Live Telemetry & Dev Controls
                                if *drawer_tab.read() == "telemetry" {
                                    div { class: "space-y-4 animate-in fade-in duration-200",
                                        div { class: "p-4 rounded-2xl border border-border bg-background/60 space-y-3 shadow-xs text-xs",
                                            h3 { class: "text-sm font-bold text-foreground flex items-center gap-2 m-0",
                                                components::LucideIcon { name: "rss", size: "16", class: "text-emerald-500" }
                                                "{t(\"fleet-drawer-tab-telemetry\", &region)}"
                                            }

                                            div { class: "grid grid-cols-2 gap-3 pt-2",
                                                div { class: "p-3 rounded-xl border border-border/40 bg-muted/20 flex flex-col gap-1",
                                                    span { class: "text-[10px] font-bold text-muted-foreground uppercase", "GPS Tracker ID" }
                                                    span { class: "font-mono font-bold text-foreground", "{drawer_vdevice.clone().unwrap_or_else(|| \"None\".to_string())}" }
                                                }
                                                div { class: "p-3 rounded-xl border border-border/40 bg-muted/20 flex flex-col gap-1",
                                                    span { class: "text-[10px] font-bold text-muted-foreground uppercase", "Last Ping" }
                                                    span { class: "font-mono font-bold text-foreground", "{drawer_ping.unwrap_or(0)} ms" }
                                                }
                                                div { class: "p-3 rounded-xl border border-border/40 bg-muted/20 flex flex-col gap-1",
                                                    span { class: "text-[10px] font-bold text-muted-foreground uppercase", "Latitude" }
                                                    span { class: "font-mono font-bold text-emerald-500", "{drawer_lat.unwrap_or(59.3293):.4}" }
                                                }
                                                div { class: "p-3 rounded-xl border border-border/40 bg-muted/20 flex flex-col gap-1",
                                                    span { class: "text-[10px] font-bold text-muted-foreground uppercase", "Longitude" }
                                                    span { class: "font-mono font-bold text-emerald-500", "{drawer_lon.unwrap_or(18.0686):.4}" }
                                                }
                                            }

                                            button {
                                                onclick: {
                                                    let uid = uid_sim_drawer.clone();
                                                    let vid = drawer_vid.clone();
                                                    let runner_c = runner_sim_drawer.clone();
                                                    move |_| {
                                                        let uid = uid.clone();
                                                        let vid = vid.clone();
                                                        let runner_c = runner_c.clone();
                                                        runner_c.run(async move {
                                                            yntra_core::simulate_vehicle_movement(uid, vid, 5).await?;
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                            Ok(())
                                                        });
                                                    }
                                                },
                                                class: "w-full py-2.5 rounded-xl bg-emerald-600 text-white text-xs font-bold shadow hover:bg-emerald-700 transition-all border-0 cursor-pointer flex items-center justify-center gap-1.5 mt-2",
                                                components::LucideIcon { name: "play", size: "14" }
                                                "Simulate Movement Vector (+5 Ticks)"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Deletion Confirmation Modal
            if let Some((del_id, del_name, del_plate)) = vehicle_to_delete.read().clone() {
                {
                    let uid_del = active_uid.clone();
                    let vid_del = del_id.clone();
                    let runner_del = runner.clone();

                    rsx! {
                        div {
                            style: "position: fixed; inset: 0; z-index: 9999; display: flex; align-items: center; justify-content: center; background: rgba(0, 0, 0, 0.65); backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px); padding: 1rem;",
                            onclick: move |_| vehicle_to_delete.set(None),

                            div {
                                class: "w-full max-w-md bg-sidebar border border-border rounded-2xl shadow-2xl p-6 space-y-4 animate-in zoom-in-95 duration-200",
                                onclick: move |e| e.stop_propagation(),

                                div { class: "flex items-center gap-3 border-b border-border/40 pb-3",
                                    div { class: "rounded-xl bg-rose-500/10 p-2 text-rose-500 border border-rose-500/20",
                                        components::LucideIcon { name: "alert-triangle", class: "h-5 w-5" }
                                    }
                                    div {
                                        h3 { class: "text-base font-bold text-foreground m-0", "{t(\"fleet-delete-confirm-title\", &region)}" }
                                        p { class: "text-xs text-muted-foreground m-0 mt-0.5", "{del_name} ({del_plate})" }
                                    }
                                }

                                p { class: "text-xs text-muted-foreground leading-relaxed m-0",
                                    "{t(\"fleet-delete-confirm-desc\", &region)}"
                                }

                                div { class: "flex items-center justify-end gap-3 pt-3 border-t border-border/40",
                                    button {
                                        class: "px-4 py-2 rounded-xl border border-border bg-background text-xs font-bold text-foreground hover:bg-muted cursor-pointer transition-all",
                                        onclick: move |_| vehicle_to_delete.set(None),
                                        "{t(\"fleet-delete-cancel-btn\", &region)}"
                                    }
                                    button {
                                        onclick: {
                                            let uid = uid_del.clone();
                                            let vid = vid_del.clone();
                                            let runner_c = runner_del.clone();
                                            move |_| {
                                                let uid = uid.clone();
                                                let vid = vid.clone();
                                                let runner_c = runner_c.clone();
                                                vehicle_to_delete.set(None);
                                                runner_c.run(async move {
                                                    yntra_core::delete_vehicle(uid, vid).await?;
                                                    let current = *db_trigger.read();
                                                    db_trigger.set(current + 1);
                                                    Ok(())
                                                });
                                            }
                                        },
                                        class: "px-5 py-2 rounded-xl bg-rose-600 text-white text-xs font-extrabold shadow hover:bg-rose-700 transition-all border-0 cursor-pointer flex items-center gap-1.5",
                                        components::LucideIcon { name: "trash-2", size: "14" }
                                        "{t(\"fleet-delete-confirm-btn\", &region)}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Registration Modal Overlay
            if *show_register_modal.read() {
                {
                    let uid_reg = active_uid.clone();
                    let runner_reg = runner.clone();

                    rsx! {
                        div {
                            style: "position: fixed; inset: 0; z-index: 9999; display: flex; align-items: center; justify-content: center; background: rgba(0, 0, 0, 0.65); backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px); padding: 1rem;",
                            onclick: move |_| show_register_modal.set(false),

                            div {
                                class: "w-full max-w-md bg-sidebar border border-border rounded-2xl shadow-2xl p-6 space-y-5 animate-in zoom-in-95 duration-200",
                                onclick: move |e| e.stop_propagation(),

                                div { class: "flex items-center justify-between border-b border-border/40 pb-3",
                                    div { class: "flex items-center gap-3",
                                        div { class: "rounded-xl bg-primary/10 p-2 text-primary",
                                            components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                        }
                                        div {
                                            h3 { class: "text-base font-bold text-foreground m-0", "{t(\"fleet-modal-title\", &region)}" }
                                            p { class: "text-xs text-muted-foreground m-0 mt-0.5", "{t(\"fleet-modal-subtitle\", &region)}" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 rounded-lg border-0 bg-transparent text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                                        onclick: move |_| show_register_modal.set(false),
                                        components::LucideIcon { name: "x", size: "18" }
                                    }
                                }

                                if let Some(ref err_msg) = *form_validation_error.read() {
                                    div { class: "p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-500 text-xs font-semibold flex items-center gap-2",
                                        components::LucideIcon { name: "alert-circle", size: "14" }
                                        "{err_msg}"
                                    }
                                }

                                div { class: "space-y-4",
                                    div {
                                        label { class: "text-xs font-bold text-foreground block mb-1", "{t(\"fleet-label-name\", &region)}" }
                                        input {
                                            r#type: "text",
                                            placeholder: "{t(\"fleet-placeholder-name\", &region)}",
                                            value: "{new_vehicle_name}",
                                            oninput: move |e| new_vehicle_name.set(e.value()),
                                            class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                        }
                                    }
                                    div {
                                        label { class: "text-xs font-bold text-foreground block mb-1", "{t(\"fleet-label-plate\", &region)}" }
                                        input {
                                            r#type: "text",
                                            placeholder: "{t(\"fleet-placeholder-plate\", &region)}",
                                            value: "{new_vehicle_plate}",
                                            oninput: move |e| new_vehicle_plate.set(e.value()),
                                            class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                        }
                                    }
                                    div { class: "grid grid-cols-2 gap-3",
                                        div {
                                            label { class: "text-xs font-bold text-foreground block mb-1", "{t(\"fleet-label-capacity\", &region)}" }
                                            input {
                                                r#type: "number",
                                                step: "0.5",
                                                min: "0.1",
                                                value: "{new_vehicle_capacity}",
                                                oninput: move |e| new_vehicle_capacity.set(e.value().parse().unwrap_or(15.0)),
                                                class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                            }
                                        }
                                        div {
                                            label { class: "text-xs font-bold text-foreground block mb-1", "{t(\"fleet-label-gps\", &region)}" }
                                            input {
                                                r#type: "text",
                                                placeholder: "{t(\"fleet-placeholder-gps\", &region)}",
                                                value: "{new_vehicle_gps_id}",
                                                oninput: move |e| new_vehicle_gps_id.set(e.value()),
                                                class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                            }
                                        }
                                    }
                                }

                                div { class: "flex items-center justify-end gap-3 pt-3 border-t border-border/40",
                                    button {
                                        class: "px-4 py-2 rounded-xl border border-border bg-background text-xs font-bold text-foreground hover:bg-muted cursor-pointer transition-all",
                                        onclick: move |_| show_register_modal.set(false),
                                        "{t(\"fleet-cancel\", &region)}"
                                    }
                                    button {
                                        onclick: {
                                            let uid = uid_reg.clone();
                                            let runner_c = runner_reg.clone();
                                            move |_| {
                                                let name = new_vehicle_name.read().trim().to_string();
                                                let plate = new_vehicle_plate.read().trim().to_string();
                                                let cap = *new_vehicle_capacity.read();
                                                let gps_id = new_vehicle_gps_id.read().trim().to_string();

                                                if name.is_empty() || plate.is_empty() {
                                                    form_validation_error.set(Some("Please fill in all required fields (Name and License Plate).".to_string()));
                                                    return;
                                                }

                                                let uid = uid.clone();
                                                let runner_c = runner_c.clone();
                                                new_vehicle_name.set(String::new());
                                                new_vehicle_plate.set(String::new());
                                                new_vehicle_capacity.set(15.0);
                                                new_vehicle_gps_id.set(String::new());
                                                form_validation_error.set(None);
                                                show_register_modal.set(false);

                                                let gps_opt = if gps_id.is_empty() { None } else { Some(gps_id) };

                                                runner_c.run(async move {
                                                    yntra_core::create_vehicle(uid, name, plate, cap, gps_opt).await?;
                                                    let current = *db_trigger.read();
                                                    db_trigger.set(current + 1);
                                                    Ok(())
                                                });
                                            }
                                        },
                                        class: "px-5 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-extrabold shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center gap-1.5",
                                        components::LucideIcon { name: "check", size: "14" }
                                        "{t(\"fleet-save\", &region)}"
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
