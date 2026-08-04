use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct CustomerLiveTrackingModalProps {
    pub job_id: String,
    pub active_user_id: String,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn CustomerLiveTrackingModal(props: CustomerLiveTrackingModalProps) -> Element {
    let job_ticket_id = props.job_id.clone();
    let uid = props.active_user_id.clone();

    let mut is_simulating = use_signal(|| false);
    let mut sim_tick = use_signal(|| 0i32);

    let j_id = job_ticket_id.clone();
    let tracking_res = use_resource(move || {
        let job_id_val = j_id.clone();
        let _ = *sim_tick.read(); // reactive trigger for simulation updates
        async move {
            yntra_core::get_customer_live_tracking_portal(job_id_val)
                .await
                .ok()
        }
    });

    let j_id_sim = job_ticket_id.clone();
    let uid_sim = uid.clone();

    // Background streaming simulation effect
    use_effect(move || {
        let active = *is_simulating.read();
        let u = uid_sim.clone();
        let j = j_id_sim.clone();
        if active {
            spawn(async move {
                let current_tick = *sim_tick.read();
                // Get job details to find assigned vehicle
                if let Ok(portal) = yntra_core::get_customer_live_tracking_portal(j.clone()).await {
                    if let Ok(vehicles) = yntra_core::get_vehicles(u.clone()).await {
                        if let Some(veh) = vehicles.iter().find(|v| {
                            v.license_plate
                                == portal.vehicle_license_plate.clone().unwrap_or_default()
                        }) {
                            let _ = yntra_core::simulate_vehicle_movement(
                                u,
                                veh.id.clone(),
                                current_tick + 1,
                            )
                            .await;
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                sim_tick.set(current_tick + 1);
            });
        }
    });

    let portal_opt = tracking_res.read().clone().flatten();
    let on_close_handler = props.on_close;

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4 overflow-y-auto animate-in fade-in duration-200",
            div {
                class: "bg-slate-900 border border-slate-700/70 rounded-2xl max-w-3xl w-full p-6 shadow-2xl space-y-5 text-slate-100",

                // Header Banner
                div {
                    class: "flex items-center justify-between border-b border-slate-800 pb-4",
                    div {
                        class: "flex items-center gap-3",
                        div {
                            class: "w-10 h-10 rounded-xl bg-primary/20 border border-primary/30 flex items-center justify-center text-primary font-bold text-lg",
                            "GPS"
                        }
                        div {
                            h3 { class: "font-bold text-white text-lg tracking-tight", "Live Lastbilspårning & Rutt" }
                            p { class: "text-xs text-slate-400 flex items-center gap-2 mt-0.5",
                                span { class: "font-medium text-slate-300", "Uppdrag ID:" }
                                span { class: "font-mono text-primary bg-primary/10 px-1.5 py-0.5 rounded text-[11px]", "{props.job_id}" }
                            }
                        }
                    }
                    button {
                        class: "w-8 h-8 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white flex items-center justify-center transition-colors text-sm font-bold",
                        onclick: move |_| on_close_handler.call(()),
                        "✕"
                    }
                }

                if let Some(portal) = portal_opt {
                    div {
                        class: "space-y-4",

                        // ETA & Status Header Card
                        div {
                            class: "p-4 bg-gradient-to-r from-primary/20 via-slate-800/80 to-slate-900 border border-primary/30 rounded-xl flex items-center justify-between shadow-lg",
                            div {
                                class: "space-y-1",
                                div { class: "text-[11px] font-bold uppercase tracking-wider text-primary", "Beräknad Ankomst (ETA)" }
                                div { class: "text-3xl font-extrabold text-white flex items-baseline gap-2",
                                    if portal.estimated_arrival_mins > 0 {
                                        "{portal.estimated_arrival_mins} min"
                                    } else {
                                        "Framme vid destination"
                                    }
                                    span { class: "text-xs font-semibold text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20",
                                        if portal.route_status == "completed" { "Slutförd" } else { "Aktiv Transport" }
                                    }
                                }
                            }
                            div {
                                class: "text-right space-y-1",
                                div { class: "text-[11px] font-semibold text-slate-400", "Hastighet" }
                                div { class: "text-xl font-bold text-white font-mono", "{portal.speed_kmh:.1} km/h" }
                            }
                        }

                        // Telemetry & Driver Card Grid
                        div {
                            class: "grid grid-cols-1 md:grid-cols-3 gap-3 text-xs",

                            // Driver Info Card
                            div {
                                class: "p-3 bg-slate-800/50 border border-slate-700/60 rounded-xl space-y-2",
                                div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-wider", "Tilldelad Förare" }
                                div { class: "font-semibold text-white text-sm flex items-center gap-2",
                                    span { class: "w-6 h-6 rounded-full bg-slate-700 flex items-center justify-center text-[10px] font-bold text-primary", "F" }
                                    "{portal.driver_name}"
                                }
                                if let Some(ref phone) = portal.driver_phone {
                                    a {
                                        class: "inline-flex items-center gap-1.5 text-xs text-primary hover:text-primary font-semibold bg-primary/10 px-2.5 py-1 rounded-lg transition-colors border border-primary/20 mt-1",
                                        href: "tel:{phone}",
                                        "Ring Förare ({phone})"
                                    }
                                }
                            }

                            // Vehicle Info Card
                            div {
                                class: "p-3 bg-slate-800/50 border border-slate-700/60 rounded-xl space-y-2",
                                div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-wider", "Fordon & Registrering" }
                                div { class: "font-bold text-white text-sm font-mono flex items-center gap-1.5",
                                    span { class: "w-2 h-2 rounded-full bg-emerald-400 animate-pulse" }
                                    "{portal.vehicle_license_plate.as_deref().unwrap_or(\"T-100\")}"
                                }
                                div { class: "text-[11px] text-slate-400 font-mono",
                                    "Lat: {portal.current_lat:.4} • Lon: {portal.current_lon:.4}"
                                }
                            }

                            // Address Route Card
                            div {
                                class: "p-3 bg-slate-800/50 border border-slate-700/60 rounded-xl space-y-1.5",
                                div { class: "text-[10px] font-bold text-slate-400 uppercase tracking-wider", "Rutt" }
                                div { class: "text-slate-300 font-medium truncate", "Från: {portal.origin_address}" }
                                div { class: "text-primary font-semibold truncate", "Till: {portal.destination_address}" }
                            }
                        }

                        // Interactive Map Rendering Viewport
                        div {
                            class: "relative w-full h-72 rounded-xl border border-slate-700/80 overflow-hidden bg-slate-950 flex items-center justify-center shadow-inner",
                            iframe {
                                id: "live-customer-tracking-map",
                                class: "w-full h-full border-0",
                                srcdoc: format!(
                                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
  <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
  <style>
    body {{ margin:0; padding:0; background:#020617; color:#f8fafc; font-family: sans-serif; }}
    #map {{ width:100%; height:100vh; }}
    .truck-marker {{ background:#4f46e5; border:2px solid #ffffff; border-radius:50%; width:16px; height:16px; box-shadow:0 0 10px #4f46e5; }}
  </style>
</head>
<body>
  <div id="map"></div>
  <script>
    var map = L.map('map').setView([{}, {}], 12);
    L.tileLayer('https://{{s}}.basemaps.cartocdn.com/dark_all/{{z}}/{{x}}/{{y}}{{r}}.png', {{
      attribution: '&copy; OpenStreetMap &copy; CARTO',
      maxZoom: 19
    }}).addTo(map);

    var originMarker = L.marker([{}, {}]).addTo(map).bindPopup('<b>Ursprung</b><br>{}');
    var destMarker = L.marker([{}, {}]).addTo(map).bindPopup('<b>Destination</b><br>{}');

    var truckIcon = L.divIcon({{ className: 'truck-marker' }});
    var truckMarker = L.marker([{}, {}], {{ icon: truckIcon }}).addTo(map).bindPopup('<b>Flyttlastbil</b><br>Status: {}');

    var latlngs = [
      [{}, {}],
      [{}, {}],
      [{}, {}]
    ];
    var polyline = L.polyline(latlngs, {{ color: '#6366f1', weight: 4, opacity: 0.8, dashArray: '8, 8' }}).addTo(map);
    map.fitBounds(polyline.getBounds(), {{ padding: [30, 30] }});
  </script>
</body>
</html>"#,
                                    portal.current_lat, portal.current_lon,
                                    portal.origin_lat, portal.origin_lon, portal.origin_address,
                                    portal.destination_lat, portal.destination_lon, portal.destination_address,
                                    portal.current_lat, portal.current_lon, portal.route_status,
                                    portal.origin_lat, portal.origin_lon,
                                    portal.current_lat, portal.current_lon,
                                    portal.destination_lat, portal.destination_lon
                                )
                            }
                        }

                        // Simulation / Auto-Stream Control Toolbar
                        div {
                            class: "p-3 bg-slate-800/40 border border-slate-700/50 rounded-xl flex items-center justify-between text-xs",
                            div {
                                class: "flex items-center gap-2",
                                button {
                                    class: if *is_simulating.read() {
                                        "px-3 py-1.5 bg-rose-600 hover:bg-rose-500 text-white font-semibold rounded-lg transition-colors flex items-center gap-1.5"
                                    } else {
                                        "px-3 py-1.5 bg-primary hover:bg-primary/90 text-white font-semibold rounded-lg transition-colors flex items-center gap-1.5"
                                    },
                                    onclick: move |_| {
                                        let curr = *is_simulating.read();
                                        is_simulating.set(!curr);
                                    },
                                    if *is_simulating.read() {
                                        "Stoppa Bakgrundsströmmande Telemetri"
                                    } else {
                                        "Starta Bakgrundsströmmande Telemetri"
                                    }
                                }
                                if *is_simulating.read() {
                                    span { class: "text-emerald-400 font-mono text-[11px] animate-pulse", "● GPS Ström Aktiv (3s frekvens)" }
                                }
                            }
                            span { class: "text-slate-400 text-[11px]", "Senast uppdaterad: LIVE" }
                        }
                    }
                } else {
                    div {
                        class: "py-12 text-center text-slate-400 space-y-2",
                        div { class: "w-8 h-8 rounded-full border-2 border-primary border-t-transparent animate-spin mx-auto" }
                        p { class: "text-sm", "Hämtar live GPS telemetridata för flyttuppdrag..." }
                    }
                }

                // Modal Footer
                div {
                    class: "pt-4 border-t border-slate-800 flex justify-end",
                    button {
                        class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-xl transition-colors",
                        onclick: move |_| on_close_handler.call(()),
                        "Stäng"
                    }
                }
            }
        }
    }
}
