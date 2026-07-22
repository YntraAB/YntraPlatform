use crate::components;
use dioxus::prelude::*;
use yntra_core::MoveVehicle;

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
    let db_trig = *props.db_trigger.read();
    let mut db_trigger = props.db_trigger;
    let active_uid_c = props.active_user_id.read().clone();

    // Registration Modal State
    let mut show_register_modal = use_signal(|| false);

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
        async move {
            yntra_core::get_vehicles(uid).await.unwrap_or_default()
        }
    });
    let vehicles: Vec<MoveVehicle> = vehicles_res.read().clone().unwrap_or_default();

    // Compute Metrics
    let total_vehicles = vehicles.len();
    let total_capacity: f64 = vehicles.iter().map(|v| v.capacity_m3).sum();
    let active_count = vehicles.iter().filter(|v| v.status == "active").count();
    let avg_capacity = if total_vehicles > 0 { total_capacity / total_vehicles as f64 } else { 0.0 };

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
                    h1 { class: "text-2xl font-bold text-foreground", "Fordonsflotta" }
                    p { class: "text-sm text-muted-foreground", "Realtidsöversikt, mätarställning och kapacitet för er transportflotta." }
                }

                button {
                    onclick: move |_| show_register_modal.set(true),
                    class: "px-4 py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all border-0 cursor-pointer shadow flex items-center gap-2",
                    components::LucideIcon { name: "plus", size: "16" }
                    "Registrera nytt fordon"
                }
            }

            // Top Metrics Summary Bar
            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 w-full",
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Totalt Antal Fordon" }
                    div { class: "text-2xl font-black text-foreground", "{total_vehicles}" }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Sammanlagd Kapacitet" }
                    div { class: "text-2xl font-black text-primary", "{total_capacity:.1} m³" }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Aktiva i Trafik" }
                    div { class: "text-2xl font-black text-emerald-500 flex items-center gap-2",
                        "{active_count}"
                        span { class: "w-2 h-2 rounded-full bg-emerald-500 animate-pulse" }
                    }
                }
                div { class: "p-4 rounded-2xl border border-border bg-sidebar shadow-xs flex flex-col gap-1",
                    span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Snittkapacitet" }
                    div { class: "text-2xl font-black text-foreground", "{avg_capacity:.1} m³" }
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
                            components::CardTitle { class: "text-lg font-bold", "Registrerade Transportfordon" }
                            components::CardDescription { class: "text-xs", "Aktuell status och teknisk specifikation per fordon." }
                        }
                    }

                    // Search & Filter Controls
                    div { class: "flex items-center gap-2 flex-wrap",
                        div { class: "relative flex-1 md:w-64",
                            input {
                                class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 pl-8 text-xs text-foreground focus:outline-none focus:border-primary transition-all",
                                placeholder: "Sök fordon eller regnr...",
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
                                "Alla"
                            }
                            button {
                                onclick: move |_| status_filter.set("active".to_string()),
                                class: format!("px-2.5 py-1 rounded-lg text-[10px] font-bold border-0 cursor-pointer transition-all {}", if *status_filter.read() == "active" { "bg-primary text-primary-foreground" } else { "bg-transparent text-muted-foreground hover:text-foreground" }),
                                "Aktiva"
                            }
                        }
                    }
                }

                components::CardContent {
                    class: "pt-4 flex flex-1 flex-col",
                    if filtered_vehicles.is_empty() {
                        div { class: "flex flex-col items-center justify-center py-16 text-center text-muted-foreground border border-dashed rounded-xl border-border bg-secondary/10 my-4",
                            components::LucideIcon { name: "truck", class: "h-10 w-10 opacity-20 mb-3" }
                            p { class: "text-sm font-bold text-foreground m-0", "Inga fordon hittades." }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-xs m-0", 
                                if vehicles.is_empty() { "Klicka på \"Registrera nytt fordon\" ovan för att lägga till er första flyttbil." } else { "Pröva att ändra er sökning eller filter inställning." }
                            }
                        }
                    } else {
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-3.5",
                            for vehicle in filtered_vehicles.iter() {
                                {
                                    let v_id = vehicle.id.clone();
                                    let v_name = vehicle.name.clone();
                                    let v_plate = vehicle.license_plate.clone();
                                    let v_capacity = vehicle.capacity_m3;
                                    let v_status = vehicle.status.clone();
                                    let v_device = vehicle.gps_device_id.clone();
                                    let uid_del = active_uid_c.clone();

                                    rsx! {
                                        div {
                                            key: "{v_id}",
                                            class: "flex items-center justify-between p-4 rounded-xl border border-border/40 bg-background/60 hover:bg-background hover:border-primary/30 transition-all shadow-xs group",
                                            
                                            div { class: "flex items-center gap-3.5 min-w-0",
                                                div { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary border border-primary/20",
                                                    components::LucideIcon { name: "truck", class: "h-5 w-5" }
                                                }
                                                div { class: "min-w-0 flex-1",
                                                    div { class: "flex items-center gap-2",
                                                        span { class: "text-sm font-bold text-foreground truncate", "{v_name}" }
                                                        span { class: "font-mono font-bold bg-muted text-foreground border border-border/60 px-1.5 py-0.5 rounded text-[10px] tracking-wide shrink-0", 
                                                            "{v_plate}" 
                                                        }
                                                    }
                                                    div { class: "text-xs text-muted-foreground flex items-center gap-2 mt-1 flex-wrap",
                                                        span { class: "font-semibold text-primary", "Kapacitet: {v_capacity} m³" }
                                                        if let Some(ref device) = v_device {
                                                            span { class: "text-[10px] text-slate-400 font-mono flex items-center gap-1",
                                                                components::LucideIcon { name: "rss", size: "10" }
                                                                "GPS: {device}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            div { class: "flex items-center gap-2 shrink-0 pl-2",
                                                span {
                                                    class: format!("px-2.5 py-1 rounded-full text-[10px] font-extrabold tracking-wide flex items-center gap-1.5 {}",
                                                        if v_status == "active" { "bg-emerald-500/10 text-emerald-500 border border-emerald-500/20" } else { "bg-slate-500/10 text-slate-400 border border-slate-500/20" }
                                                    ),
                                                    span { class: format!("w-1.5 h-1.5 rounded-full {}", if v_status == "active" { "bg-emerald-500 animate-pulse" } else { "bg-slate-400" }) }
                                                    if v_status == "active" { "Aktiv" } else { "Inaktiv" }
                                                }

                                                button {
                                                    onclick: move |_| {
                                                        let vehicle_id = v_id.clone();
                                                        let uid = uid_del.clone();
                                                        let mut db_trigger = db_trigger;
                                                        spawn(async move {
                                                            if yntra_core::delete_vehicle(uid, vehicle_id).await.is_ok() {
                                                                let current = *db_trigger.read();
                                                                db_trigger.set(current + 1);
                                                            }
                                                        });
                                                    },
                                                    class: "p-2 rounded-lg text-slate-400 hover:text-rose-500 hover:bg-rose-500/10 border-0 bg-transparent cursor-pointer transition-all flex items-center justify-center opacity-70 group-hover:opacity-100",
                                                    title: "Ta bort fordon",
                                                    components::LucideIcon { name: "trash-2", size: "16" }
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

            // Registration Modal Overlay
            if *show_register_modal.read() {
                div { 
                    style: "position: fixed; inset: 0; z-index: 9999; display: flex; items-center: center; justify-content: center; background: rgba(0, 0, 0, 0.65); backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px); padding: 1rem;",
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
                                    h3 { class: "text-base font-bold text-foreground m-0", "Registrera Nytt Fordon" }
                                    p { class: "text-xs text-muted-foreground m-0 mt-0.5", "Fyll i fordonsuppgifter och GPS-enhet." }
                                }
                            }
                            button {
                                class: "p-1.5 rounded-lg border-0 bg-transparent text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                                onclick: move |_| show_register_modal.set(false),
                                components::LucideIcon { name: "x", size: "18" }
                            }
                        }

                        div { class: "space-y-4",
                            div {
                                label { class: "text-xs font-bold text-foreground block mb-1", "Fordonsnamn / Modell *" }
                                input {
                                    r#type: "text",
                                    placeholder: "t.ex. Volvo FL6, Ford Transit",
                                    value: "{new_vehicle_name}",
                                    oninput: move |e| new_vehicle_name.set(e.value()),
                                    class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                }
                            }
                            div {
                                label { class: "text-xs font-bold text-foreground block mb-1", "Registreringsnummer *" }
                                input {
                                    r#type: "text",
                                    placeholder: "t.ex. ABC-123",
                                    value: "{new_vehicle_plate}",
                                    oninput: move |e| new_vehicle_plate.set(e.value()),
                                    class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all",
                                }
                            }
                            div { class: "grid grid-cols-2 gap-3",
                                div {
                                    label { class: "text-xs font-bold text-foreground block mb-1", "Kapacitet (m³) *" }
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
                                    label { class: "text-xs font-bold text-foreground block mb-1", "GPS Tracker ID" }
                                    input {
                                        r#type: "text",
                                        placeholder: "Valfri IMEI / ID",
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
                                "Avbryt"
                            }
                            button {
                                onclick: {
                                    let uid = active_uid_c.clone();
                                    let mut db_trigger = db_trigger;
                                    move |_| {
                                        let name = new_vehicle_name.read().trim().to_string();
                                        let plate = new_vehicle_plate.read().trim().to_string();
                                        let cap = *new_vehicle_capacity.read();
                                        let gps_id = new_vehicle_gps_id.read().trim().to_string();
                                        if name.is_empty() || plate.is_empty() {
                                            return;
                                        }
                                        let uid = uid.clone();
                                        new_vehicle_name.set(String::new());
                                        new_vehicle_plate.set(String::new());
                                        new_vehicle_capacity.set(15.0);
                                        new_vehicle_gps_id.set(String::new());
                                        show_register_modal.set(false);
                                        
                                        let gps_opt = if gps_id.is_empty() { None } else { Some(gps_id) };
                                        
                                        spawn(async move {
                                            if yntra_core::create_vehicle(uid, name, plate, cap, gps_opt).await.is_ok() {
                                                let current = *db_trigger.read();
                                                db_trigger.set(current + 1);
                                            }
                                        });
                                    }
                                },
                                class: "px-5 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-extrabold shadow hover:opacity-90 transition-all border-0 cursor-pointer flex items-center gap-1.5",
                                components::LucideIcon { name: "check", size: "14" }
                                "Spara & Registrera"
                            }
                        }
                    }
                }
            }
        }
    }
}
