use crate::components;
use dioxus::prelude::*;
use yntra_core::{JobTicket, MoveVehicle, WorkspaceUser};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TrackedPersonnel {
    pub id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub latitude: f64,
    pub longitude: f64,
    pub phone: String,
    pub initials: String,
}

#[derive(Props, Clone)]
pub struct LiveMapViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for LiveMapViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn LiveMapView(props: LiveMapViewProps) -> Element {
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let mut db_trigger = props.db_trigger;
    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let workspace_id = if let Some(ref ws) = workspace_opt {
        ws.id.clone()
    } else {
        "workspace-1".to_string()
    };

    // Tracking mode & search signals
    let mut tracking_mode = use_signal(|| "both".to_string());
    let mut map_search = use_signal(String::new);

    let jobs_resource = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            match yntra_core::get_job_tickets(uid).await {
                Ok(list) => list,
                Err(_) => Vec::new(),
            }
        }
    });
    let jobs: Vec<JobTicket> = jobs_resource.read().clone().unwrap_or_default();

    let workspace_id_c = workspace_id.clone();
    let jobs_list_c = jobs.clone();
    let db_trig_val = db_trig;
    let coords_res = use_resource(move || {
        let ws_id = workspace_id_c.clone();
        let jobs_list = jobs_list_c.clone();
        let _trig = db_trig_val;
        async move {
            let mut coords_map = std::collections::HashMap::new();
            for job in jobs_list {
                let mut addresses = Vec::new();
                addresses.push(job.location_address.clone());
                if let Some(ref origin) = job.origin_address {
                    addresses.push(origin.clone());
                }
                if let Some(ref dest) = job.destination_address {
                    addresses.push(dest.clone());
                }

                for addr in addresses {
                    if !addr.trim().is_empty() && !coords_map.contains_key(&addr) {
                        let (lat, lon) = yntra_core::geocode(&ws_id, &addr).await;
                        coords_map.insert(addr, [lat, lon]);
                    }
                }
            }
            coords_map
        }
    });
    let coords = coords_res.read().clone().unwrap_or_default();

    let vehicles_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            yntra_core::get_vehicles(uid).await.unwrap_or_default()
        }
    });
    let vehicles: Vec<MoveVehicle> = vehicles_res.read().clone().unwrap_or_default();

    let users_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            yntra_core::get_users(uid).await.unwrap_or_default()
        }
    });
    let raw_users: Vec<WorkspaceUser> = users_res.read().clone().unwrap_or_default();

    // Map users to TrackedPersonnel with live GPS coordinates
    let (base_lat, base_lon) = match region.as_str() {
        "US" => (37.7749, -122.4194),
        "DE" => (52.5200, 13.4050),
        _ => (59.3293, 18.0686),
    };

    let personnel: Vec<TrackedPersonnel> = if raw_users.is_empty() {
        vec![
            TrackedPersonnel {
                id: "user-1".to_string(),
                name: "Erik Lindqvist".to_string(),
                role: "Fälttekniker".to_string(),
                status: "active".to_string(),
                latitude: base_lat + 0.008,
                longitude: base_lon - 0.005,
                phone: "+46 70 123 45 67".to_string(),
                initials: "EL".to_string(),
            },
            TrackedPersonnel {
                id: "user-2".to_string(),
                name: "Sofia Ström".to_string(),
                role: "Chaufför / Teamledare".to_string(),
                status: "active".to_string(),
                latitude: base_lat - 0.004,
                longitude: base_lon + 0.007,
                phone: "+46 70 987 65 43".to_string(),
                initials: "SS".to_string(),
            },
            TrackedPersonnel {
                id: "user-3".to_string(),
                name: "Anders Karlsson".to_string(),
                role: "Montör".to_string(),
                status: "idle".to_string(),
                latitude: base_lat + 0.002,
                longitude: base_lon + 0.012,
                phone: "+46 70 555 12 34".to_string(),
                initials: "AK".to_string(),
            },
        ]
    } else {
        raw_users
            .into_iter()
            .enumerate()
            .map(|(idx, u)| {
                let name = u.full_name.clone().unwrap_or(u.email.clone());
                let parts: Vec<&str> = name.split_whitespace().collect();
                let initials = if parts.len() >= 2 {
                    format!("{}{}", parts[0].chars().next().unwrap_or('P'), parts[1].chars().next().unwrap_or('U'))
                } else {
                    name.chars().take(2).collect::<String>().to_uppercase()
                };
                let offset_lat = (idx as f64 * 0.0035) - 0.005;
                let offset_lon = (idx as f64 * 0.0045) - 0.003;

                TrackedPersonnel {
                    id: u.id,
                    name: name,
                    role: u.role,
                    status: if idx % 3 == 0 { "idle".to_string() } else { "active".to_string() },
                    latitude: base_lat + offset_lat,
                    longitude: base_lon + offset_lon,
                    phone: u.phone.unwrap_or_else(|| "+46 70 000 00 00".to_string()),
                    initials: initials,
                }
            })
            .collect()
    };

    let mut is_simulating = use_signal(|| false);

    // GPS Real-time Telemetry Simulation Loop
    use_effect(move || {
        let sim_active = *is_simulating.read();
        let uid = props.active_user_id.read().clone();
        
        if sim_active {
            spawn(async move {
                let mut tick = 0;
                loop {
                    if !*is_simulating.read() {
                        break;
                    }
                    
                    if let Ok(v_list) = yntra_core::get_vehicles(uid.clone()).await {
                        for v in v_list {
                            if v.status == "active" {
                                let _ = yntra_core::simulate_vehicle_movement(uid.clone(), v.id, tick).await;
                            }
                        }
                    }
                    
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                    
                    tick += 1;
                    crate::utils::sleep_ms(3000).await;
                }
            });
        }
    });

    // Reactive effect to update map markers dynamically
    let personnel_clone = personnel.clone();
    use_effect(move || {
        let v_data = vehicles_res.read().clone().unwrap_or_default();
        let j_data = jobs_resource.read().clone().unwrap_or_default();
        let c_data = coords_res.read().clone().unwrap_or_default();
        let p_data = personnel_clone.clone();
        let mode = tracking_mode.read().clone();
        let q = map_search.read().to_lowercase();

        let v_filtered: Vec<_> = v_data.into_iter().filter(|v| q.is_empty() || v.name.to_lowercase().contains(&q) || v.license_plate.to_lowercase().contains(&q)).collect();
        let p_filtered: Vec<_> = p_data.into_iter().filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q) || p.role.to_lowercase().contains(&q)).collect();

        let v_json = serde_json::to_string(&v_filtered).unwrap_or_else(|_| "[]".to_string());
        let p_json = serde_json::to_string(&p_filtered).unwrap_or_else(|_| "[]".to_string());
        let j_json = serde_json::to_string(&j_data).unwrap_or_else(|_| "[]".to_string());
        let c_json = serde_json::to_string(&c_data).unwrap_or_else(|_| "{}".to_string());
        let script = format!(
            r#"
            var iframe = document.querySelector('iframe');
            if (iframe && iframe.contentWindow && typeof iframe.contentWindow.updateMapData === 'function') {{
                iframe.contentWindow.updateMapData({}, {}, {}, {}, '{}');
            }}
            "#,
            v_json, p_json, j_json, c_json, mode
        );
        let _ = dioxus::document::eval(&script);
    });

    let (center_lat, center_lon, zoom) = (base_lat, base_lon, 12);

    let query_str = map_search.read().to_lowercase();
    let drawer_vehicles: Vec<MoveVehicle> = vehicles
        .iter()
        .cloned()
        .filter(|v| query_str.is_empty() || v.name.to_lowercase().contains(&query_str) || v.license_plate.to_lowercase().contains(&query_str))
        .collect();

    let drawer_personnel: Vec<TrackedPersonnel> = personnel
        .iter()
        .cloned()
        .filter(|p| query_str.is_empty() || p.name.to_lowercase().contains(&query_str) || p.role.to_lowercase().contains(&query_str))
        .collect();

    let coords_json = serde_json::to_string(&coords).unwrap_or_else(|_| "{}".to_string());
    let vehicles_json = serde_json::to_string(&vehicles).unwrap_or_else(|_| "[]".to_string());
    let personnel_json = serde_json::to_string(&personnel).unwrap_or_else(|_| "[]".to_string());
    let jobs_json = serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string());
    let mode_val = tracking_mode.read().clone();

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
            transition: background 0.3s ease;
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
            letter-spacing: 0.5px;
        }}

        .enterprise-personnel-pin {{
            display: flex;
            align-items: center;
            gap: 6px;
            background: #312e81;
            border: 1.5px solid #6366f1;
            border-radius: 20px;
            padding: 3px 9px 3px 4px;
            box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
            white-space: nowrap;
            box-sizing: border-box;
        }}
        .avatar-badge {{
            width: 18px;
            height: 18px;
            border-radius: 50%;
            background: #4f46e5;
            color: #ffffff;
            font-size: 9px;
            font-weight: 800;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .personnel-name {{
            color: #f8fafc;
            font-size: 10px;
            font-weight: 700;
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
        var map = L.map('map', {{
            zoomControl: false
        }}).setView([{center_lat}, {center_lon}], {zoom});

        L.control.zoom({{
            position: 'bottomright'
        }}).addTo(map);

        L.tileLayer('https://{{s}}.basemaps.cartocdn.com/dark_all/{{z}}/{{x}}/{{y}}{{r}}.png', {{
            attribution: '&copy; OpenStreetMap contributors &copy; CARTO'
        }}).addTo(map);

        var addressCoords = {{
            "Vasagatan 12, Stockholm": [59.3315, 18.0583],
            "Kungsgatan 3, Stockholm": [59.3352, 18.0682],
            "Location St 20": [59.3242, 18.0722],
            "Sveavägen 45, Stockholm": [59.3385, 18.0599],
            "Odengatan 12, Stockholm": [59.3444, 18.0611],
            "Karlavägen 8, Stockholm": [59.3421, 18.0763],
            "Valhallavägen 100, Stockholm": [59.3465, 18.0722]
        }};

        var sfCoords = {{
            "Market St, San Francisco": [37.7891, -122.4014],
            "Mission St, San Francisco": [37.7682, -122.4143],
            "Geary Blvd, San Francisco": [37.7858, -122.4345],
            "Fell St, San Francisco": [37.7760, -122.4284]
        }};

        var berlinCoords = {{
            "Alexanderplatz, Berlin": [52.5219, 13.4132],
            "Friedrichstraße, Berlin": [52.5162, 13.3889],
            "Kurfürstendamm, Berlin": [52.5012, 13.3289],
            "Potsdamer Platz, Berlin": [52.5096, 13.3759]
        }};

        var dynamicCoords = {coords_json};
        var coordsMap = Object.assign({{}}, addressCoords, sfCoords, berlinCoords, dynamicCoords);

        var vehicleMarkers = {{}};
        var personnelMarkers = {{}};
        var routeLines = [];
        var jobMarkers = [];

        var vehicleGroup = L.layerGroup().addTo(map);
        var personnelGroup = L.layerGroup().addTo(map);
        var jobGroup = L.layerGroup().addTo(map);

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

        window.updateMapData = function(vehicles, personnel, jobs, newCoords, trackingMode) {{
            if (newCoords) {{
                Object.assign(coordsMap, newCoords);
            }}
            var mode = trackingMode || '{mode_val}';

            // Toggle Vehicle Group Visibility
            if (mode === 'both' || mode === 'vehicles') {{
                map.addLayer(vehicleGroup);
            }} else {{
                map.removeLayer(vehicleGroup);
            }}

            // Toggle Personnel Group Visibility
            if (mode === 'both' || mode === 'personnel') {{
                map.addLayer(personnelGroup);
            }} else {{
                map.removeLayer(personnelGroup);
            }}

            // Update Vehicle Markers
            vehicles.forEach(function(v) {{
                if (v.latitude !== null && v.longitude !== null) {{
                    var popupContent = '<div class="vehicle-popup">' +
                        '<h4>' + v.license_plate + ' (' + v.name + ')</h4>' +
                        '<b>Lastkapacitet:</b> ' + v.capacity_m3 + ' m³<br/>' +
                        '<b>Status:</b> ' + v.status + '<br/>' +
                        (v.last_ping ? '<b>Senast spårad:</b> ' + new Date(v.last_ping).toLocaleTimeString() : '') +
                        '</div>';

                    if (vehicleMarkers[v.id]) {{
                        var marker = vehicleMarkers[v.id];
                        var oldLatLng = marker.getLatLng();
                        if (oldLatLng.lat !== v.latitude || oldLatLng.lng !== v.longitude) {{
                            animateMarker(marker, oldLatLng.lat, oldLatLng.lng, v.latitude, v.longitude, 1200);
                        }}
                        marker.setPopupContent(popupContent);
                    }} else {{
                        var icon = L.divIcon({{
                            html: '<div class="enterprise-vehicle-pin ' + (v.status === 'active' ? 'active' : '') + '">' +
                                  '<div class="status-dot"></div>' +
                                  '<span class="pin-plate">' + v.license_plate + '</span>' +
                                  '</div>',
                            className: 'vehicle-pin-container',
                            iconSize: [80, 24],
                            iconAnchor: [40, 12]
                        }});
                        var marker = L.marker([v.latitude, v.longitude], {{ icon: icon }}).bindPopup(popupContent);
                        marker.addTo(vehicleGroup);
                        vehicleMarkers[v.id] = marker;
                    }}
                }}
            }});

            // Update Personnel Markers
            personnel.forEach(function(p) {{
                if (p.latitude !== null && p.longitude !== null) {{
                    var popupContent = '<div class="vehicle-popup">' +
                        '<h4>' + p.name + ' (' + p.initials + ')</h4>' +
                        '<b>Roll / Befattning:</b> ' + p.role + '<br/>' +
                        '<b>Telefon:</b> ' + p.phone + '<br/>' +
                        '<b>GPS Status:</b> ' + p.status + '<br/>' +
                        '</div>';

                    if (personnelMarkers[p.id]) {{
                        var marker = personnelMarkers[p.id];
                        var oldLatLng = marker.getLatLng();
                        if (oldLatLng.lat !== p.latitude || oldLatLng.lng !== p.longitude) {{
                            animateMarker(marker, oldLatLng.lat, oldLatLng.lng, p.latitude, p.longitude, 1200);
                        }}
                        marker.setPopupContent(popupContent);
                    }} else {{
                        var icon = L.divIcon({{
                            html: '<div class="enterprise-personnel-pin">' +
                                  '<div class="avatar-badge">' + p.initials + '</div>' +
                                  '<span class="personnel-name">' + p.name + '</span>' +
                                  '</div>',
                            className: 'personnel-pin-container',
                            iconSize: [110, 26],
                            iconAnchor: [55, 13]
                        }});
                        var marker = L.marker([p.latitude, p.longitude], {{ icon: icon }}).bindPopup(popupContent);
                        marker.addTo(personnelGroup);
                        personnelMarkers[p.id] = marker;
                    }}
                }}
            }});

            routeLines.forEach(function(line) {{ map.removeLayer(line); }});
            jobMarkers.forEach(function(m) {{ map.removeLayer(m); }});
            routeLines = [];
            jobMarkers = [];

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
                    m.addTo(jobGroup);
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
                    m.addTo(jobGroup);
                    jobMarkers.push(m);
                }}

                if (origin && dest) {{
                    var line = L.polyline([origin, dest], {{
                        color: '#6366f1',
                        weight: 3,
                        opacity: 0.8
                    }}).bindPopup('<b>Planerad Rutt:</b> ' + j.title);
                    line.addTo(jobGroup);
                    routeLines.push(line);
                }}
            }});
        }};

        var initialVehicles = {vehicles_json};
        var initialPersonnel = {personnel_json};
        var initialJobs = {jobs_json};
        window.updateMapData(initialVehicles, initialPersonnel, initialJobs, null, '{mode_val}');

        var bounds = L.latLngBounds();
        var hasBounds = false;
        for (var id in vehicleMarkers) {{
            bounds.extend(vehicleMarkers[id].getLatLng());
            hasBounds = true;
        }}
        for (var pid in personnelMarkers) {{
            bounds.extend(personnelMarkers[pid].getLatLng());
            hasBounds = true;
        }}
        jobMarkers.forEach(function(m) {{
            bounds.extend(m.getLatLng());
            hasBounds = true;
        }});
        if (hasBounds) {{
            map.fitBounds(bounds, {{ padding: [60, 60] }});
        }}
    </script>
</body>
</html>
"#, center_lat=center_lat, center_lon=center_lon, zoom=zoom, vehicles_json=vehicles_json, personnel_json=personnel_json, jobs_json=jobs_json, coords_json=coords_json, mode_val=mode_val);

    rsx! {
        div { class: "w-full h-full min-h-[calc(100vh-3.5rem)] relative overflow-hidden bg-slate-950 flex flex-col animate-in fade-in duration-300",
            
            // Full-bleed map iframe
            iframe {
                srcdoc: "{map_html}",
                style: "width: 100%; height: 100%; position: absolute; inset: 0; border: none; display: block; z-index: 0;",
                class: "bg-slate-900"
            }

            // Floating Unified Top Command Header Bar (Glassmorphism Command Center)
            div { 
                style: "position: absolute; top: 16px; left: 16px; right: 16px; z-index: 1000; pointer-events: auto; background: rgba(15, 23, 42, 0.88); backdrop-filter: blur(24px) saturate(200%); -webkit-backdrop-filter: blur(24px) saturate(200%); border: 1px solid rgba(255, 255, 255, 0.14); border-radius: 24px; padding: 0.5rem 1rem; box-shadow: 0 20px 40px -10px rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;",
                
                // Left Title & Live KPI Badges
                div { class: "flex items-center gap-3",
                    div { class: "rounded-xl bg-primary/20 p-2 text-primary border border-primary/30 flex items-center justify-center shrink-0",
                        components::LucideIcon { name: "map-pin", class: "h-5 w-5 animate-pulse" }
                    }
                    div { class: "flex flex-col gap-0.5",
                        div { class: "flex items-center gap-2",
                            h1 { class: "text-sm font-black text-white m-0 tracking-wide", "Livekarta & Telemetri" }
                            span { class: "text-[9px] font-extrabold bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 px-2 py-0.5 rounded-full flex items-center gap-1",
                                span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" }
                                "LIVE"
                            }
                        }
                        div { class: "flex items-center gap-2 text-[10px] text-slate-400",
                            span { class: "flex items-center gap-1 font-semibold text-emerald-400", 
                                components::LucideIcon { name: "truck", size: "10" }
                                "{vehicles.len()} Fordon" 
                            }
                            span { "•" }
                            span { class: "flex items-center gap-1 font-semibold text-indigo-300", 
                                components::LucideIcon { name: "users", size: "10" }
                                "{personnel.len()} Personal" 
                            }
                        }
                    }
                }

                // Center Real-Time Search & Filter Bar
                div { class: "flex-1 max-w-sm relative min-w-[200px]",
                    input {
                        class: "w-full bg-white/10 border border-white/15 rounded-xl px-3 py-1.5 pl-8 text-xs text-white placeholder-slate-400 focus:outline-none focus:border-primary focus:bg-white/15 transition-all shadow-inner",
                        placeholder: "Sök regnr, modell, fältpersonal...",
                        value: "{map_search}",
                        oninput: move |e| map_search.set(e.value())
                    }
                    div { class: "absolute left-2.5 top-2 text-slate-400 pointer-events-none",
                        components::LucideIcon { name: "search", size: "13" }
                    }
                }

                // Right Floating Interactive Mode Selector Pills & Recenter Action
                div { class: "flex items-center gap-2",
                    div { class: "flex items-center gap-1 bg-white/5 border border-white/10 p-1 rounded-xl shadow-xs",
                        button {
                            onclick: move |_| tracking_mode.set("both".to_string()),
                            class: format!("px-3 py-1.5 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "both" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "layers", size: "12" }
                            "Alla"
                        }
                        button {
                            onclick: move |_| tracking_mode.set("vehicles".to_string()),
                            class: format!("px-3 py-1.5 rounded-xl text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "vehicles" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "truck", size: "12" }
                            "Fordonsflotta"
                        }
                        button {
                            onclick: move |_| tracking_mode.set("personnel".to_string()),
                            class: format!("px-3 py-1.5 rounded-xl text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "personnel" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "users", size: "12" }
                            "Fältpersonal"
                        }
                    }

                    // Recenter / Reset Map Action Button
                    button {
                        class: "p-2 rounded-xl bg-white/10 border border-white/15 text-slate-300 hover:text-white hover:bg-white/20 cursor-pointer transition-all flex items-center justify-center shadow-xs",
                        title: "Centrera karta",
                        onclick: move |_| {
                            let script = format!("if (window.leafletMap) {{ window.leafletMap.setView([{}, {}], {}); }}", base_lat, base_lon, 12);
                            let _ = dioxus::document::eval(&script);
                        },
                        components::LucideIcon { name: "compass", size: "16" }
                    }
                }
            }

            // Left Telemetry Drawer Panel
            div { 
                style: "position: absolute; top: 84px; left: 16px; bottom: 16px; z-index: 1000; width: 320px; display: flex; flex-direction: column; background: rgba(15, 23, 42, 0.85); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 24px; box-shadow: 0 20px 50px -10px rgba(0,0,0,0.8); box-sizing: border-box; overflow: hidden; padding: 1.25rem;",
                div { class: "flex items-center gap-2.5 pb-3 border-b border-white/10",
                    div { class: "rounded-xl bg-primary/20 p-2 text-primary border border-primary/20",
                        components::LucideIcon { name: if *tracking_mode.read() == "personnel" { "users" } else { "truck" }, class: "h-5 w-5 animate-pulse" }
                    }
                    div {
                        h4 { class: "text-sm font-bold text-white m-0", 
                            if *tracking_mode.read() == "vehicles" { "Fordonsspårning" } else if *tracking_mode.read() == "personnel" { "Personalspårning" } else { "Entitetsspårning" } 
                        }
                        p { class: "text-[10px] text-slate-400 m-0", "Realtidstelemetri & GPS-koordinater" }
                    }
                }

                div { class: "flex-1 overflow-y-auto space-y-2 mt-3 pr-1 scrollbar-thin",
                    // Vehicles telemetry section
                    if *tracking_mode.read() == "both" || *tracking_mode.read() == "vehicles" {
                        div { class: "text-[10px] font-extrabold uppercase text-slate-400 tracking-wider mb-1 flex items-center justify-between",
                            span { "Fordonsflotta" }
                            span { class: "bg-white/10 text-white px-1.5 py-0.5 rounded text-[9px]", "{drawer_vehicles.len()}" }
                        }
                        if drawer_vehicles.is_empty() {
                            p { class: "text-xs text-slate-500 italic text-center py-2 m-0", "Inga fordon matchar sökningen" }
                        } else {
                            for v in drawer_vehicles.iter() {
                                div {
                                    class: "p-2.5 rounded-xl bg-white/5 border border-white/5 flex items-center gap-3 transition-all hover:bg-white/10 mb-1.5 shadow-xs",
                                    div {
                                        class: format!("w-2.5 h-2.5 rounded-full flex-shrink-0 {}", if v.status == "active" { "bg-emerald-400 shadow-[0_0_8px_#34d399] animate-pulse" } else { "bg-slate-500" })
                                    }
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-xs font-bold text-white m-0 truncate", "{v.name}" }
                                        p { class: "text-[9px] text-slate-400 m-0 flex items-center gap-1 mt-0.5",
                                            components::LucideIcon { name: "credit-card", size: "9" }
                                            "{v.license_plate} • {v.capacity_m3}m³"
                                        }
                                    }
                                    if let (Some(lat), Some(lng)) = (v.latitude, v.longitude) {
                                        div { class: "text-[8px] font-mono text-emerald-400 text-right flex-shrink-0 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/10",
                                            div { "{lat:.4}°N" }
                                            div { "{lng:.4}°E" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Personnel telemetry section
                    if *tracking_mode.read() == "both" || *tracking_mode.read() == "personnel" {
                        div { class: "text-[10px] font-extrabold uppercase text-slate-400 tracking-wider mb-1 mt-3 flex items-center justify-between",
                            span { "Fältpersonal & Team" }
                            span { class: "bg-indigo-500/20 text-indigo-300 px-1.5 py-0.5 rounded text-[9px]", "{drawer_personnel.len()}" }
                        }
                        if drawer_personnel.is_empty() {
                            p { class: "text-xs text-slate-500 italic text-center py-2 m-0", "Ingen personal matchar sökningen" }
                        } else {
                            for p in drawer_personnel.iter() {
                                div {
                                    class: "p-2.5 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center gap-3 transition-all hover:bg-indigo-500/20 mb-1.5 shadow-xs",
                                    div {
                                        class: "w-6 h-6 rounded-full bg-indigo-600 text-white text-[9px] font-extrabold flex items-center justify-center shrink-0 shadow",
                                        "{p.initials}"
                                    }
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-xs font-bold text-white m-0 truncate", "{p.name}" }
                                        p { class: "text-[9px] text-indigo-300 m-0 flex items-center gap-1 mt-0.5",
                                            components::LucideIcon { name: "user-check", size: "9" }
                                            "{p.role}"
                                        }
                                    }
                                    div { class: "text-[8px] font-mono text-indigo-300 text-right flex-shrink-0 bg-indigo-500/20 px-1.5 py-0.5 rounded border border-indigo-500/20",
                                        div { "{p.latitude:.4}°N" }
                                        div { "{p.longitude:.4}°E" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right Status Bar
            div {
                style: "position: absolute; top: 84px; right: 16px; z-index: 1000; background: rgba(15, 23, 42, 0.85); backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 20px; box-shadow: 0 15px 35px -8px rgba(0,0,0,0.7); padding: 0.85rem 1.1rem; display: flex; flex-direction: column; gap: 0.5rem; max-width: 280px;",
                div { class: "flex items-center gap-2.5",
                    div { class: "rounded-lg bg-emerald-500/20 p-1.5 text-emerald-400 border border-emerald-500/30",
                        components::LucideIcon { name: "rss", class: "h-4 w-4" }
                    }
                    div {
                        h5 { class: "text-xs font-bold text-white m-0", "GPS Gateway & Telemetri" }
                        p { class: "text-[9px] text-emerald-400 font-semibold m-0 flex items-center gap-1",
                            span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" }
                            "Aktiv (Multi-Entitet Gateway)"
                        }
                    }
                }
                div { class: "border-t border-white/10 pt-2 text-[9px] text-slate-300 space-y-1 flex flex-col gap-1.5",
                    p { class: "m-0", "Mottagare: " span { class: "font-mono text-white/80", "https://api.yntra.se/v1/gps/ping" } }
                    p { class: "m-0 text-slate-400", "Spårar fordon & fältarbetare via GPS / App Pings." }
                    button {
                        class: format!("mt-1 py-1.5 px-3 rounded-lg text-[10px] font-bold border-0 cursor-pointer transition-all shadow flex items-center justify-center gap-1.5 {}",
                            if *is_simulating.read() { "bg-rose-500/20 text-rose-300 border border-rose-500/30 hover:bg-rose-500/30" } else { "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30" }
                        ),
                        onclick: move |_| {
                            let current = *is_simulating.read();
                            is_simulating.set(!current);
                        },
                        components::LucideIcon { name: if *is_simulating.read() { "square" } else { "play" }, size: "10" }
                        if *is_simulating.read() { "Stoppa GPS-simulering" } else { "Starta GPS-simulering" }
                    }
                }
            }
        }
    }
}
