use dioxus::prelude::*;
use yntra_core::{JobTicket, MoveVehicle};
use crate::components;
use crate::views::scheduling::{add_days_to_date, parse_date};
use std::collections::HashMap;

#[derive(Props, Clone, PartialEq)]
pub struct DispatchBoardProps {
    pub active_user_id: String,
    pub region: String,
    pub db_trigger: Signal<u32>,
    pub vehicles: Vec<MoveVehicle>,
    pub jobs: Vec<JobTicket>,
}

fn format_day_header(date_str: &str) -> String {
    let (y, m, d) = parse_date(date_str);
    let mut year = y;
    let mut month = m;
    let day = d as i32;
    if month < 3 {
        month += 12;
        year -= 1;
    }
    let k = year % 100;
    let j = year / 100;
    let h = (day + (13 * (month as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    let day_name = match h {
        0 => "Lör",
        1 => "Sön",
        2 => "Mån",
        3 => "Tis",
        4 => "Ons",
        5 => "Tor",
        6 => "Fre",
        _ => "",
    };
    format!("{} {}/{}", day_name, d, m)
}

#[component]
pub fn DispatchBoard(props: DispatchBoardProps) -> Element {
    let active_user_id = props.active_user_id.clone();
    let region = props.region.clone();
    let mut db_trigger = props.db_trigger;
    let vehicles = props.vehicles.clone();
    let jobs = props.jobs.clone();

    // Visual states
    let mut dragged_job_id = use_signal(|| Option::<String>::None);
    let mut selected_start_date = use_signal(|| "2026-06-30".to_string());
    let mut dragged_over_cell = use_signal(|| Option::<(String, String)>::None); // (vehicle_id, date)

    // GPS Telemetry Tracking states
    let mut selected_tracking_vehicle = use_signal(|| Option::<MoveVehicle>::None);

    let tracking_vehicle_resolved = selected_tracking_vehicle.read().clone().and_then(|sel_v| {
        vehicles.iter().find(|v| v.id == sel_v.id).cloned()
    });

    // Retrieve workspace GPS Webhook settings for production instructions
    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };
    let gps_webhook_token = settings_json
        .get("gps_webhook_token")
        .and_then(|v| v.as_str())
        .unwrap_or("saknas");

    let tracking_vehicle_resolved_c = tracking_vehicle_resolved.clone();
    // Reactive effect to update map markers dynamically without iframe reload
    use_effect(move || {
        if let Some(ref v) = tracking_vehicle_resolved_c {
            let v_json = serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string());
            let script = format!(
                r#"
                var iframe = document.getElementById('dispatch-tracking-map');
                if (iframe && iframe.contentWindow && typeof iframe.contentWindow.updateVehiclePosition === 'function') {{
                    iframe.contentWindow.updateVehiclePosition({});
                }}
                "#,
                v_json
            );
            let _ = dioxus::document::eval(&script);
        }
    });

    // Load inventory volumes for all jobs dynamically
    let active_uid_c = active_user_id.clone();
    let jobs_list = jobs.clone();
    let db_trig_val = *db_trigger.read();
    let volumes_res = use_resource(move || {
        let uid = active_uid_c.clone();
        let jobs = jobs_list.clone();
        let _trig = db_trig_val;
        async move {
            let mut volumes_map = HashMap::new();
            for job in jobs {
                let inv_res = yntra_core::get_move_inventory(uid.clone(), job.id.clone()).await;
                if let Ok(inv) = inv_res {
                    let total_vol: f64 = inv.iter().map(|item| item.estimated_volume_m3 * item.quantity as f64).sum();
                    volumes_map.insert(job.id.clone(), total_vol);
                }
            }
            volumes_map
        }
    });

    let volumes = volumes_res.read().clone().unwrap_or_default();

    // Column Dates (next 7 days starting from selected_start_date)
    let start_date = selected_start_date.read().clone();
    let dates: Vec<String> = (0..7).map(|i| add_days_to_date(&start_date, i)).collect();

    // Filter unscheduled or unassigned jobs for sidebar
    let unscheduled_jobs: Vec<JobTicket> = jobs
        .iter()
        .filter(|j| {
            j.assigned_vehicle_id.is_none()
                || j.scheduled_date.is_empty()
                || j.scheduled_date == "unscheduled"
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "flex gap-6 w-full animate-in fade-in zoom-in duration-300 items-stretch select-none",
            style: "min-height: 500px;",

            // Left Sidebar: Unscheduled Jobs bucket
            div { class: "w-80 flex-shrink-0 flex flex-col gap-4 border border-border bg-sidebar rounded-xl p-4 shadow-sm",
                div {
                    h3 { class: "text-sm font-extrabold m-0 text-foreground", "Oplanerade / Ej tilldelade uppdrag" }
                    p { class: "text-[10px] text-muted-foreground mt-1 mb-0", "Dra uppdrag härifrån till fordonsschemat för att boka dem." }
                }

                // Unschedule Drop target
                div {
                    class: "border-2 border-dashed border-border hover:border-red-500/50 hover:bg-red-500/5 p-3 rounded-lg text-center cursor-pointer transition-all flex flex-col items-center justify-center gap-1",
                    ondragover: |e| e.prevent_default(),
                    ondrop: {
                        let active_uid = active_user_id.clone();
                        move |_| {
                            if let Some(job_id) = dragged_job_id.read().clone() {
                                let active_uid = active_uid.clone();
                                spawn(async move {
                                    let _ = yntra_core::schedule_job_ticket(active_uid.clone(), job_id.clone(), "unscheduled".to_string(), None).await;
                                    let _ = yntra_core::assign_vehicle_to_job(active_uid, job_id, None).await;
                                    let current = *db_trigger.read();
                                    db_trigger.set(current + 1);
                                });
                            }
                        }
                    },
                    components::LucideIcon { name: "trash-2", class: "h-4 w-4 text-muted-foreground/60" }
                    span { class: "text-[10px] font-bold text-muted-foreground", "Släpp här för att avboka" }
                }

                // Unscheduled Jobs List
                div { class: "flex-1 overflow-y-auto space-y-3 pr-1",
                    if unscheduled_jobs.is_empty() {
                        div { class: "flex flex-col items-center justify-center py-12 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-background/30",
                            components::LucideIcon { name: "check-circle", class: "h-6 w-6 text-emerald-500/60 mb-1" }
                            p { class: "text-xs font-bold m-0", "Alla uppdrag tilldelade!" }
                        }
                    } else {
                        for job in unscheduled_jobs.iter() {
                            div {
                                key: "{job.id}",
                                draggable: true,
                                ondragstart: {
                                    let job_id = job.id.clone();
                                    move |_| {
                                        dragged_job_id.set(Some(job_id.clone()));
                                    }
                                },
                                ondragend: move |_| {
                                    dragged_job_id.set(None);
                                },
                                class: "p-3 rounded-lg border border-border bg-background cursor-grab active:cursor-grabbing transition-all hover:border-primary/50 hover:shadow-md hover:bg-secondary/10 flex flex-col gap-1.5",
                                style: if dragged_job_id.read().as_ref() == Some(&job.id) { "opacity: 0.4;" } else { "" },
                                
                                div { class: "flex justify-between items-start gap-2",
                                    span { class: "text-xs font-extrabold text-foreground leading-tight", "{job.title}" }
                                    span {
                                        class: match job.priority.as_str() {
                                            "critical" => "px-1.5 py-0.5 rounded text-[8px] font-bold uppercase tracking-wider flex-shrink-0 border border-red-500 bg-red-500/10 text-red-400",
                                            "high" => "px-1.5 py-0.5 rounded text-[8px] font-bold uppercase tracking-wider flex-shrink-0 border border-amber-500 bg-amber-500/10 text-amber-400",
                                            "medium" => "px-1.5 py-0.5 rounded text-[8px] font-bold uppercase tracking-wider flex-shrink-0 border border-blue-500 bg-blue-500/10 text-blue-400",
                                            _ => "px-1.5 py-0.5 rounded text-[8px] font-bold uppercase tracking-wider flex-shrink-0 border border-slate-500 bg-slate-500/10 text-slate-400",
                                        },
                                        "{job.priority}"
                                    }
                                }
                                p { class: "text-[10px] text-muted-foreground m-0 line-clamp-2 leading-relaxed", "{job.description}" }
                                div { class: "flex items-center justify-between text-[9px] text-muted-foreground font-semibold mt-1",
                                    span { class: "flex items-center gap-1",
                                        components::LucideIcon { name: "box", size: "10" }
                                        "{*volumes.get(&job.id).unwrap_or(&0.0):.1} m³"
                                    }
                                    span { class: "flex items-center gap-1",
                                        components::LucideIcon { name: "map-pin", size: "10" }
                                        "{job.location_address}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right Panel: 7-Day Timeline / Gantt grid
            div { class: "flex-1 flex flex-col gap-4 border border-border bg-sidebar rounded-xl p-4 shadow-sm",
                
                // Gantt Header (Week Navigation)
                div { class: "flex justify-between items-center pb-2 border-b border-border/40",
                    div {
                        h3 { class: "text-sm font-extrabold m-0 text-foreground", "Fordonsschema & Beläggning" }
                        p { class: "text-[10px] text-muted-foreground mt-1 mb-0", "Flytta och fördela uppdrag över fordon och dagar." }
                    }
                    div { class: "flex items-center gap-1.5",
                        button {
                            class: "rounded p-1.5 hover:bg-muted border border-border bg-background cursor-pointer text-foreground flex items-center justify-center transition-all",
                            onclick: move |_| {
                                let date = selected_start_date.read().clone();
                                selected_start_date.set(add_days_to_date(&date, -7));
                            },
                            components::LucideIcon { name: "chevron-left", size: "14" }
                        }
                        button {
                            class: "rounded px-2.5 py-1 text-xs font-semibold hover:bg-muted border border-border bg-background cursor-pointer text-foreground transition-all",
                            onclick: move |_| {
                                selected_start_date.set("2026-06-30".to_string());
                            },
                            "Idag"
                        }
                        button {
                            class: "rounded p-1.5 hover:bg-muted border border-border bg-background cursor-pointer text-foreground flex items-center justify-center transition-all",
                            onclick: move |_| {
                                let date = selected_start_date.read().clone();
                                selected_start_date.set(add_days_to_date(&date, 7));
                            },
                            components::LucideIcon { name: "chevron-right", size: "14" }
                        }
                    }
                }

                // Grid Container
                div { class: "flex-1 overflow-x-auto select-none",
                    div { class: "min-w-[800px] flex flex-col",
                        
                        // Column Headers (Dates)
                        div { class: "grid border-b border-border/60 pb-2",
                            style: "grid-template-columns: 180px repeat(7, minmax(0, 1fr));",
                            div { class: "text-xs font-extrabold text-muted-foreground/80 self-end px-2", "Fordon" }
                            for date in dates.iter() {
                                div { key: "{date}", class: "text-center px-1",
                                    div { class: "text-xs font-bold text-foreground", "{format_day_header(date)}" }
                                    div { class: "text-[9px] text-muted-foreground font-semibold mt-0.5", "{date}" }
                                }
                            }
                        }

                        // Grid Rows (Vehicles)
                        div { class: "divide-y divide-border/20 flex-1 flex flex-col justify-stretch",
                            if vehicles.is_empty() {
                                div { class: "text-center py-20 text-muted-foreground",
                                    components::LucideIcon { name: "truck", class: "h-8 w-8 opacity-25 mx-auto mb-2" }
                                    p { class: "text-sm font-bold m-0", "Inga fordon registrerade." }
                                    p { class: "text-xs text-muted-foreground/60 mt-1 m-0", "Registrera fordon under tabben 'Fordonsflotta' först." }
                                }
                            } else {
                                for vehicle in vehicles.iter() {
                                    div {
                                        key: "{vehicle.id}",
                                        class: "grid py-3 hover:bg-background/20 transition-all items-stretch",
                                        style: "grid-template-columns: 180px repeat(7, minmax(0, 1fr));",

                                        // Vehicle Info Panel
                                        div { class: "flex flex-col justify-center px-2 pr-4 border-r border-border/20 gap-0.5",
                                            span { class: "text-xs font-extrabold text-foreground", "{vehicle.name}" }
                                            span { class: "text-[9px] bg-muted w-max px-1.5 py-0.5 rounded font-mono font-bold text-muted-foreground mt-0.5", "{vehicle.license_plate}" }
                                            span { class: "text-[9px] text-muted-foreground font-semibold mt-1", "Kapacitet: {vehicle.capacity_m3:.1} m³" }
                                            button {
                                                onclick: {
                                                    let v = vehicle.clone();
                                                    move |_| {
                                                        selected_tracking_vehicle.set(Some(v.clone()));
                                                    }
                                                },
                                                class: "mt-1.5 px-2 py-0.75 w-max rounded bg-primary/10 hover:bg-primary/20 text-primary border-0 cursor-pointer text-[9px] font-extrabold flex items-center gap-1 transition-all",
                                                components::LucideIcon { name: "map-pin", size: "10" }
                                                "Spåra Live"
                                            }
                                        }

                                        // 7 Cells
                                        for date in dates.iter() {
                                            {
                                                let cell_jobs: Vec<JobTicket> = jobs
                                                    .iter()
                                                    .filter(|j| j.assigned_vehicle_id.as_ref() == Some(&vehicle.id) && j.scheduled_date == *date)
                                                    .cloned()
                                                    .collect();
                                                
                                                let cell_vol: f64 = cell_jobs.iter().map(|j| *volumes.get(&j.id).unwrap_or(&0.0)).sum();
                                                let is_overloaded = cell_vol > vehicle.capacity_m3;
                                                let is_dragged_over = dragged_over_cell.read().as_ref() == Some(&(vehicle.id.clone(), date.clone()));

                                                let cell_bg = if is_dragged_over {
                                                    "bg-primary/10 border-primary/40"
                                                } else if is_overloaded {
                                                    "bg-red-500/5 border-red-500/20"
                                                } else if !cell_jobs.is_empty() {
                                                    "bg-background/40 border-border/20"
                                                } else {
                                                    "border-border/10"
                                                };

                                                let active_uid_drop = active_user_id.clone();
                                                let vehicle_id_drop = vehicle.id.clone();
                                                let target_date = date.clone();

                                                rsx! {
                                                    div {
                                                        key: "{vehicle.id}-{date}",
                                                        class: "border-r border-border/10 px-2 py-1.5 flex flex-col gap-2 transition-all relative border-dashed border {cell_bg}",
                                                        style: "min-height: 120px;",
                                                        
                                                        ondragover: |e| e.prevent_default(),
                                                        ondragenter: {
                                                            let v_id_c = vehicle.id.clone();
                                                            let date_c = date.clone();
                                                            move |e| {
                                                                e.prevent_default();
                                                                dragged_over_cell.set(Some((v_id_c.clone(), date_c.clone())));
                                                            }
                                                        },
                                                        ondragleave: move |_| {
                                                            dragged_over_cell.set(None);
                                                        },
                                                        ondrop: {
                                                            let mut db_trigger = db_trigger;
                                                            move |_| {
                                                                dragged_over_cell.set(None);
                                                                if let Some(job_id) = dragged_job_id.read().clone() {
                                                                    let active_uid = active_uid_drop.clone();
                                                                    let v_id = vehicle_id_drop.clone();
                                                                    let date = target_date.clone();
                                                                    spawn(async move {
                                                                        let _ = yntra_core::schedule_job_ticket(active_uid.clone(), job_id.clone(), date, None).await;
                                                                        let _ = yntra_core::assign_vehicle_to_job(active_uid, job_id, Some(v_id)).await;
                                                                        let val = *db_trigger.read();
                                                                        db_trigger.set(val + 1);
                                                                    });
                                                                }
                                                            }
                                                        },

                                                        // Overload / Capacity alert pill
                                                        if !cell_jobs.is_empty() {
                                                            div {
                                                                class: format!("text-[8px] font-bold px-1.5 py-0.5 rounded-full w-max mx-auto border {}",
                                                                    if is_overloaded { "bg-red-500/20 text-red-400 border-red-500/30 shadow-sm animate-pulse" } else { "bg-emerald-500/10 text-emerald-400 border-emerald-500/20" }
                                                                ),
                                                                if is_overloaded {
                                                                    span { "⚠️ Överlast: {cell_vol:.1}/{vehicle.capacity_m3:.0} m³" }
                                                                } else {
                                                                    span { "Fyllnad: {cell_vol:.1}/{vehicle.capacity_m3:.0} m³" }
                                                                }
                                                            }
                                                        }

                                                        // Job Cards in cell
                                                        div { class: "flex-1 flex flex-col gap-1.5",
                                                            for job in cell_jobs.iter() {
                                                                div {
                                                                    key: "{job.id}",
                                                                    draggable: true,
                                                                    ondragstart: {
                                                                        let job_id_drg = job.id.clone();
                                                                        move |_| {
                                                                            dragged_job_id.set(Some(job_id_drg.clone()));
                                                                        }
                                                                    },
                                                                    ondragend: move |_| {
                                                                        dragged_job_id.set(None);
                                                                    },
                                                                    class: match job.priority.as_str() {
                                                                        "critical" => "p-2 rounded border border-l-2 bg-background/90 shadow-sm cursor-grab active:cursor-grabbing hover:border-primary/40 transition-all flex flex-col gap-1 pr-6 relative group border-red-500 bg-red-500/10",
                                                                        "high" => "p-2 rounded border border-l-2 bg-background/90 shadow-sm cursor-grab active:cursor-grabbing hover:border-primary/40 transition-all flex flex-col gap-1 pr-6 relative group border-amber-500 bg-amber-500/10",
                                                                        "medium" => "p-2 rounded border border-l-2 bg-background/90 shadow-sm cursor-grab active:cursor-grabbing hover:border-primary/40 transition-all flex flex-col gap-1 pr-6 relative group border-blue-500 bg-blue-500/10",
                                                                        _ => "p-2 rounded border border-l-2 bg-background/90 shadow-sm cursor-grab active:cursor-grabbing hover:border-primary/40 transition-all flex flex-col gap-1 pr-6 relative group border-slate-500 bg-slate-500/10",
                                                                    },
                                                                    
                                                                    span { class: "text-[10px] font-extrabold text-foreground leading-tight truncate", "{job.title}" }
                                                                    span { class: "text-[8px] font-semibold text-muted-foreground flex items-center gap-0.5",
                                                                        components::LucideIcon { name: "box", size: "8" }
                                                                        "{*volumes.get(&job.id).unwrap_or(&0.0):.1} m³"
                                                                    }

                                                                    // Remove / Unassign button
                                                                    button {
                                                                        class: "absolute top-1 right-1 opacity-0 group-hover:opacity-100 p-0.5 rounded-full hover:bg-muted text-muted-foreground hover:text-red-500 border-0 bg-transparent cursor-pointer flex items-center justify-center transition-all",
                                                                        onclick: {
                                                                            let job_id_del = job.id.clone();
                                                                            let active_uid_c = active_user_id.clone();
                                                                            let mut db_trigger = db_trigger;
                                                                            move |e| {
                                                                                e.stop_propagation();
                                                                                let job_id = job_id_del.clone();
                                                                                let active_uid = active_uid_c.clone();
                                                                                spawn(async move {
                                                                                    let _ = yntra_core::schedule_job_ticket(active_uid.clone(), job_id.clone(), "unscheduled".to_string(), None).await;
                                                                                    let _ = yntra_core::assign_vehicle_to_job(active_uid, job_id, None).await;
                                                                                    let current = *db_trigger.read();
                                                                                    db_trigger.set(current + 1);
                                                                                });
                                                                            }
                                                                        },
                                                                        components::LucideIcon { name: "x", size: "10" }
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
                    }
                }
            }

            // GPS Live Tracking Modal
            if let Some(ref tracking_v) = *selected_tracking_vehicle.read() {
                {
                    let resolved_v = vehicles.iter().find(|v| v.id == tracking_v.id).cloned().unwrap_or_else(|| tracking_v.clone());
                    let has_coords = resolved_v.latitude.is_some() && resolved_v.longitude.is_some();
                    let tracking_jobs: Vec<JobTicket> = jobs
                        .iter()
                        .filter(|j| j.assigned_vehicle_id.as_ref() == Some(&resolved_v.id) && !j.scheduled_date.is_empty() && j.scheduled_date != "unscheduled")
                        .cloned()
                        .collect();
                    
                    let (center_lat, center_lon) = if let (Some(lat), Some(lng)) = (resolved_v.latitude, resolved_v.longitude) {
                        (lat, lng)
                    } else {
                        match region.as_str() {
                            "US" => (37.7749, -122.4194),
                            "DE" => (52.5200, 13.4050),
                            _ => (59.3293, 18.0686),
                        }
                    };

                    let formatted_ping_time = resolved_v.last_ping.map(|t_ms| {
                        let dt = chrono::DateTime::from_timestamp(t_ms / 1000, 0).unwrap_or_default();
                        dt.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
                    });

                    let v_json = serde_json::to_string(&resolved_v).unwrap_or_else(|_| "null".to_string());
                    let jobs_json = serde_json::to_string(&tracking_jobs).unwrap_or_else(|_| "[]".to_string());

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
        }}
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
        var centerLat = {center_lat};
        var centerLon = {center_lon};
        var map = L.map('map', {{
            zoomControl: false
        }}).setView([centerLat, centerLon], 12);

        L.control.zoom({{
            position: 'bottomright'
        }}).addTo(map);

        L.tileLayer('https://{{s}}.basemaps.cartocdn.com/dark_all/{{z}}/{{x}}/{{y}}{{r}}.png', {{
            attribution: '&copy; OpenStreetMap contributors &copy; CARTO'
        }}).addTo(map);

        var coordsMap = {{
            "Vasagatan 12, Stockholm": [59.3315, 18.0583],
            "Kungsgatan 3, Stockholm": [59.3352, 18.0682],
            "Location St 20": [59.3242, 18.0722],
            "Sveavägen 45, Stockholm": [59.3385, 18.0599],
            "Odengatan 12, Stockholm": [59.3444, 18.0611],
            "Karlavägen 8, Stockholm": [59.3421, 18.0763],
            "Valhallavägen 100, Stockholm": [59.3465, 18.0722],
            "Market St, San Francisco": [37.7891, -122.4014],
            "Mission St, San Francisco": [37.7682, -122.4143],
            "Geary Blvd, San Francisco": [37.7858, -122.4345],
            "Fell St, San Francisco": [37.7760, -122.4284],
            "Alexanderplatz, Berlin": [52.5219, 13.4132],
            "Friedrichstraße, Berlin": [52.5162, 13.3889],
            "Kurfürstendamm, Berlin": [52.5012, 13.3289],
            "Potsdamer Platz, Berlin": [52.5096, 13.3759]
        }};

        var vehicleMarker = null;
        var jobMarkers = [];
        var routeLines = [];

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

        window.updateVehiclePosition = function(v) {{
            if (v.latitude !== null && v.longitude !== null) {{
                var popupContent = '<div class="vehicle-popup">' +
                    '<h4>' + v.license_plate + ' (' + v.name + ')</h4>' +
                    '<b>Status:</b> ' + v.status + '<br/>' +
                    (v.last_ping ? '<b>Senast spårad:</b> ' + new Date(v.last_ping).toLocaleTimeString() : '') +
                    '</div>';

                if (vehicleMarker) {{
                    var oldLatLng = vehicleMarker.getLatLng();
                    if (oldLatLng.lat !== v.latitude || oldLatLng.lng !== v.longitude) {{
                        animateMarker(vehicleMarker, oldLatLng.lat, oldLatLng.lng, v.latitude, v.longitude, 1200);
                    }}
                    vehicleMarker.setPopupContent(popupContent);
                }} else {{
                    var iconHtml = '<div class="enterprise-vehicle-pin active">' +
                        '<div class="status-dot"></div>' +
                        '<span class="pin-plate">' + v.license_plate + '</span>' +
                        '</div>';
                    var customIcon = L.divIcon({{
                        html: iconHtml,
                        className: 'custom-div-icon',
                        iconSize: [80, 24],
                        iconAnchor: [40, 12]
                    }});
                    vehicleMarker = L.marker([v.latitude, v.longitude], {{ icon: customIcon }}).addTo(map);
                    vehicleMarker.bindPopup(popupContent).openPopup();
                }}
            }}
        }};

        var initialV = {v_json};
        if (initialV) {{
            window.updateVehiclePosition(initialV);
        }}

        var jobs = {jobs_json};
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
                m.addTo(map);
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
                m.addTo(map);
                jobMarkers.push(m);
            }}
            if (origin && dest) {{
                var line = L.polyline([origin, dest], {{
                    color: '#6366f1',
                    weight: 3,
                    opacity: 0.8
                }}).addTo(map);
                routeLines.push(line);
            }}
        }});

        var bounds = L.latLngBounds();
        var hasBounds = false;
        if (vehicleMarker) {{
            bounds.extend(vehicleMarker.getLatLng());
            hasBounds = true;
        }}
        jobMarkers.forEach(function(m) {{
            bounds.extend(m.getLatLng());
            hasBounds = true;
        }});
        if (hasBounds) {{
            map.fitBounds(bounds, {{ padding: [50, 50] }});
        }}
    </script>
</body>
</html>
"#, center_lat=center_lat, center_lon=center_lon, v_json=v_json, jobs_json=jobs_json);

                    rsx! {
                        div { class: "fixed inset-0 bg-black/60 backdrop-blur-sm z-[100] flex items-center justify-center p-4 animate-in fade-in duration-200",
                            div { class: "bg-background border border-border rounded-2xl max-w-3xl w-full h-[550px] shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200",
                                // Header
                                div { class: "p-4 border-b border-border/40 flex items-center justify-between bg-secondary/10",
                                    div { class: "flex items-center gap-2",
                                        components::LucideIcon { name: "truck", class: "h-5 w-5 accent-text animate-pulse" }
                                        div {
                                            h4 { class: "text-sm font-bold text-foreground m-0", "Fordonspårning: {resolved_v.name}" }
                                            p { class: "text-[10px] text-muted-foreground m-0", "{resolved_v.license_plate} • Kapacitet: {resolved_v.capacity_m3} m³" }
                                        }
                                    }
                                    button {
                                        onclick: move |_| {
                                            selected_tracking_vehicle.set(None);
                                        },
                                        class: "p-1.5 rounded-full hover:bg-secondary text-muted-foreground border-0 bg-transparent cursor-pointer flex items-center justify-center transition-all",
                                        components::LucideIcon { name: "x", size: "16" }
                                    }
                                }
                                
                                // Content grid
                                div { class: "flex-1 flex min-h-0",
                                    // Left Sidebar: Status & Controls
                                    div { class: "w-64 border-r border-border/40 p-4 flex flex-col justify-between bg-secondary/5",
                                        div { class: "flex flex-col gap-4",
                                            div { class: "flex flex-col gap-1",
                                                span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Status" }
                                                div { class: "flex items-center gap-1.5 mt-0.5",
                                                    span { class: format!("w-2 h-2 rounded-full {}", if has_coords { "bg-emerald-500 animate-pulse shadow-[0_0_6px_#10b981]" } else { "bg-slate-400" }) }
                                                    span { class: "text-xs font-bold text-foreground", if has_coords { "Spårar live" } else { "Inga koordinater" } }
                                                }
                                            }
                                            
                                            if let (Some(lat), Some(lng)) = (resolved_v.latitude, resolved_v.longitude) {
                                                div { class: "flex flex-col gap-1.5 p-3 rounded-lg border border-border/20 bg-background",
                                                    div { class: "text-[10px] font-extrabold text-muted-foreground uppercase", "Senaste position" }
                                                    div { class: "text-xs font-mono font-bold text-foreground mt-0.5", "{lat:.5}°N" }
                                                    div { class: "text-xs font-mono font-bold text-foreground", "{lng:.5}°E" }
                                                    if let Some(ref time_str) = formatted_ping_time {
                                                        div { class: "text-[9px] text-muted-foreground mt-1.5 font-semibold",
                                                            "Mottagen: {time_str}"
                                                        }
                                                    }
                                                }
                                            } else {
                                                div { class: "p-3 rounded-lg border border-dashed border-border text-center text-muted-foreground",
                                                    components::LucideIcon { name: "help-circle", class: "h-5 w-5 mx-auto opacity-40 mb-1" }
                                                    p { class: "text-[10px] italic m-0", "Inga aktiva GPS-signaler har tagits emot från denna lastbil ännu." }
                                                }
                                            }

                                            div { class: "flex flex-col gap-1",
                                                span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Schemalagda rutter" }
                                                if tracking_jobs.is_empty() {
                                                    span { class: "text-xs text-muted-foreground italic mt-0.5", "Inga jobb schemalagda" }
                                                } else {
                                                    div { class: "flex flex-col gap-1.5 mt-1 max-h-40 overflow-y-auto pr-1",
                                                        for job in tracking_jobs.iter() {
                                                            div { key: "{job.id}", class: "p-2 rounded bg-background border border-border/20 text-left",
                                                                div { class: "text-xs font-bold text-foreground truncate", "{job.title}" }
                                                                div { class: "text-[9px] text-muted-foreground truncate mt-0.5", "Från: {job.origin_address.as_deref().unwrap_or(\"\")}" }
                                                                div { class: "text-[9px] text-muted-foreground truncate", "Till: {job.destination_address.as_deref().unwrap_or(\"\")}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Production GPS Telemetry Hook Info
                                        div { class: "border-t border-border/40 pt-4 flex flex-col gap-2.5 text-left",
                                            div { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "IoT GPS Gateway (Produktion)" }
                                            
                                            div { class: "p-2.5 rounded-lg bg-background/50 border border-border/20 space-y-2 text-[10px]",
                                                div { class: "flex flex-col gap-1",
                                                    span { class: "text-muted-foreground", "Mottagare (Webhook URL):" }
                                                    span { class: "font-mono font-bold text-foreground break-all", "https://api.yntra.se/v1/vehicles/gps-webhook" }
                                                }
                                                div { class: "flex justify-between items-center gap-2",
                                                    span { class: "text-muted-foreground", "Auktoriserings-token:" }
                                                    span { class: "font-mono font-bold text-emerald-500 bg-emerald-500/10 px-1.5 py-0.5 rounded", "{gps_webhook_token}" }
                                                }
                                                if let Some(ref v) = tracking_vehicle_resolved {
                                                    div { class: "flex justify-between items-center gap-2 border-t border-border/10 pt-1.5 mt-1.5",
                                                        span { class: "text-muted-foreground", "Enhet GPS ID (IMEI):" }
                                                        span { class: "font-mono font-bold text-blue-500 bg-blue-500/10 px-1.5 py-0.5 rounded", 
                                                            "{v.gps_device_id.as_deref().unwrap_or(\"Saknas - ställ in i inställningar\")}"
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            div { class: "rounded-lg border border-primary/20 bg-primary/5 p-2 text-[9px] text-muted-foreground flex gap-1.5 items-start",
                                                components::LucideIcon { name: "info", class: "h-3.5 w-3.5 shrink-0 mt-0.5 text-primary" }
                                                span {
                                                    "Instruktion: Konfigurera din fysiska GPS-spårare (eller trådlösa mobilapp) att skicka HTTP POST pings med JSON payload (t.ex. "
                                                    code { class: "font-mono text-foreground", "{{ \"deviceId\": \"IMEI\", \"lat\": 59.32, \"lon\": 18.06 }}" }
                                                    ") till ovanstående webhook-URL."
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Right Side: The Map
                                    div { class: "flex-1 relative bg-slate-900",
                                        iframe {
                                            id: "dispatch-tracking-map",
                                            srcdoc: "{map_html}",
                                            style: "width: 100%; height: 100%; border: none; display: block;"
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
