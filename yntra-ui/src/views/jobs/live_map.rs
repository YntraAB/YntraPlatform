use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use yntra_core::{JobTicket, MoveVehicle, WorkspaceUser};

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
    fn eq(&self, other: &Self) -> bool {
        self.active_user_id == other.active_user_id
            && self.auth_region == other.auth_region
            && self.db_trigger == other.db_trigger
    }
}

#[component]
pub fn LiveMapView(props: LiveMapViewProps) -> Element {
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let workspace_id = if let Some(ref ws) = workspace_opt {
        ws.id.clone()
    } else {
        "workspace-1".to_string()
    };

    // Fine-grained state signals
    let mut tracking_mode = use_signal(|| "both".to_string());
    let mut status_filter = use_signal(|| "all".to_string());
    let mut tile_style = use_signal(|| "dark".to_string());
    let mut map_search = use_signal(String::new);
    let mut sidebar_collapsed = use_signal(|| false);
    let mut selected_unit_id = use_signal(|| Option::<String>::None);

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
            let mut unique_addrs = std::collections::HashSet::new();
            for job in &jobs_list {
                if !job.location_address.trim().is_empty() {
                    unique_addrs.insert(job.location_address.clone());
                }
                if let Some(ref origin) = job.origin_address {
                    if !origin.trim().is_empty() {
                        unique_addrs.insert(origin.clone());
                    }
                }
                if let Some(ref dest) = job.destination_address {
                    if !dest.trim().is_empty() {
                        unique_addrs.insert(dest.clone());
                    }
                }
            }

            let futures: Vec<_> = unique_addrs
                .into_iter()
                .map(|addr| {
                    let ws = ws_id.clone();
                    async move {
                        let (lat, lon) = yntra_core::geocode(&ws, &addr).await;
                        (addr, [lat, lon])
                    }
                })
                .collect();

            let results = join_all(futures).await;
            let mut coords_map = std::collections::HashMap::new();
            for (addr, coords) in results {
                coords_map.insert(addr, coords);
            }
            coords_map
        }
    });

    let vehicles_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move { yntra_core::get_vehicles(uid).await.unwrap_or_default() }
    });
    let vehicles: Vec<MoveVehicle> = vehicles_res.read().clone().unwrap_or_default();

    let users_res = use_resource(move || {
        let _ = db_trig;
        let uid = props.active_user_id.read().clone();
        async move { yntra_core::get_users(uid).await.unwrap_or_default() }
    });
    let raw_users: Vec<WorkspaceUser> = users_res.read().clone().unwrap_or_default();

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
                    format!(
                        "{}{}",
                        parts[0].chars().next().unwrap_or('P'),
                        parts[1].chars().next().unwrap_or('U')
                    )
                } else {
                    name.chars().take(2).collect::<String>().to_uppercase()
                };
                let offset_lat = (idx as f64 * 0.0035) - 0.005;
                let offset_lon = (idx as f64 * 0.0045) - 0.003;

                TrackedPersonnel {
                    id: u.id,
                    name: name,
                    role: u.role,
                    status: if idx % 3 == 0 {
                        "idle".to_string()
                    } else {
                        "active".to_string()
                    },
                    latitude: base_lat + offset_lat,
                    longitude: base_lon + offset_lon,
                    phone: u.phone.unwrap_or_else(|| "+46 70 000 00 00".to_string()),
                    initials: initials,
                }
            })
            .collect()
    };

    let i18n_map = serde_json::json!({
        "cargo_capacity": t("live-map-popup-cargo-capacity", &region),
        "status": t("live-map-popup-status", &region),
        "last_tracked": t("live-map-popup-last-tracked", &region),
        "role": t("live-map-popup-role", &region),
        "phone": t("live-map-popup-phone", &region),
        "gps_status": t("live-map-popup-gps-status", &region),
        "start_point": t("live-map-popup-start-point", &region),
        "destination": t("live-map-popup-destination", &region),
        "planned_route": t("live-map-popup-planned-route", &region),
        "job": t("live-map-popup-job", &region),
    });
    let i18n_json = serde_json::to_string(&i18n_map).unwrap_or_else(|_| "{}".to_string());

    // Fine-grained memoized derived states
    let query_str = use_memo(move || map_search.read().to_lowercase());
    let mode = use_memo(move || tracking_mode.read().clone());
    let sf = use_memo(move || status_filter.read().clone());
    let ts = use_memo(move || tile_style.read().clone());

    let vehicles_clone = vehicles.clone();
    let drawer_vehicles = use_memo(move || {
        let q = query_str.read();
        let filter_s = sf.read();
        vehicles_clone
            .iter()
            .cloned()
            .filter(|v| {
                let matches_q = q.is_empty()
                    || v.name.to_lowercase().contains(q.as_str())
                    || v.license_plate.to_lowercase().contains(q.as_str());
                let matches_s = match filter_s.as_str() {
                    "active" => v.status == "active",
                    "idle" => v.status != "active",
                    _ => true,
                };
                matches_q && matches_s
            })
            .collect::<Vec<MoveVehicle>>()
    });

    let personnel_clone = personnel.clone();
    let drawer_personnel = use_memo(move || {
        let q = query_str.read();
        let filter_s = sf.read();
        personnel_clone
            .iter()
            .cloned()
            .filter(|p| {
                let matches_q = q.is_empty()
                    || p.name.to_lowercase().contains(q.as_str())
                    || p.role.to_lowercase().contains(q.as_str());
                let matches_s = match filter_s.as_str() {
                    "active" => p.status == "active",
                    "idle" => p.status != "active",
                    _ => true,
                };
                matches_q && matches_s
            })
            .collect::<Vec<TrackedPersonnel>>()
    });

    let total_active_units = use_memo(move || {
        let m = mode.read();
        let dv = drawer_vehicles.read();
        let dp = drawer_personnel.read();
        if *m == "vehicles" {
            dv.len()
        } else if *m == "personnel" {
            dp.len()
        } else {
            dv.len() + dp.len()
        }
    });

    let region_for_title = region.clone();
    let drawer_title = use_memo(move || {
        let m = mode.read();
        if *m == "vehicles" {
            t("live-map-vehicle-tracking", &region_for_title)
        } else if *m == "personnel" {
            t("live-map-personnel-tracking", &region_for_title)
        } else {
            t("live-map-tracked-units", &region_for_title)
        }
    });

    // Reactive effect to update map markers smoothly without reloading iframe
    let personnel_for_effect = personnel.clone();
    let i18n_json_c = i18n_json.clone();
    use_effect(move || {
        let v_data = vehicles_res.read().clone().unwrap_or_default();
        let j_data = jobs_resource.read().clone().unwrap_or_default();
        let c_data = coords_res.read().clone().unwrap_or_default();
        let p_data = personnel_for_effect.clone();
        let current_mode = mode.read().clone();
        let current_sf = sf.read().clone();
        let current_ts = ts.read().clone();
        let q = query_str.read().clone();

        let v_filtered: Vec<_> = v_data
            .into_iter()
            .filter(|v| {
                let matches_q = q.is_empty()
                    || v.name.to_lowercase().contains(&q)
                    || v.license_plate.to_lowercase().contains(&q);
                let matches_s = match current_sf.as_str() {
                    "active" => v.status == "active",
                    "idle" => v.status != "active",
                    _ => true,
                };
                matches_q && matches_s
            })
            .collect();

        let p_filtered: Vec<_> = p_data
            .into_iter()
            .filter(|p| {
                let matches_q = q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.role.to_lowercase().contains(&q);
                let matches_s = match current_sf.as_str() {
                    "active" => p.status == "active",
                    "idle" => p.status != "active",
                    _ => true,
                };
                matches_q && matches_s
            })
            .collect();

        let v_json = serde_json::to_string(&v_filtered).unwrap_or_else(|_| "[]".to_string());
        let p_json = serde_json::to_string(&p_filtered).unwrap_or_else(|_| "[]".to_string());
        let j_json = serde_json::to_string(&j_data).unwrap_or_else(|_| "[]".to_string());
        let c_json = serde_json::to_string(&c_data).unwrap_or_else(|_| "{}".to_string());
        let mode_json =
            serde_json::to_string(&current_mode).unwrap_or_else(|_| "\"both\"".to_string());
        let ts_json = serde_json::to_string(&current_ts).unwrap_or_else(|_| "\"dark\"".to_string());

        let script = format!(
            r#"
            var iframe = document.getElementById('live-map-iframe');
            if (iframe && iframe.contentWindow && typeof iframe.contentWindow.updateMapData === 'function') {{
                iframe.contentWindow.updateMapData({}, {}, {}, {}, {}, {}, {});
            }}
            "#,
            v_json, p_json, j_json, c_json, mode_json, i18n_json_c, ts_json
        );
        let _ = dioxus::document::eval(&script);
    });

    // Static iframe template - Leaflet map initialized once
    let map_html = format!(
        r#"
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
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 10px;
            font-family: system-ui, -apple-system, sans-serif;
            box-shadow: 0 12px 30px -5px rgba(0,0,0,0.6);
        }}
        .leaflet-popup-tip {{
            background: rgba(15, 23, 42, 0.95);
        }}
        .vehicle-popup {{
            font-size: 11px;
            line-height: 1.5;
            padding: 6px 4px;
        }}
        .vehicle-popup h4 {{
            margin: 0 0 6px 0;
            font-size: 13px;
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
            box-shadow: 0 4px 8px rgba(0,0,0,0.4);
            white-space: nowrap;
            box-sizing: border-box;
            transition: all 0.2s ease;
        }}
        .enterprise-vehicle-pin:hover {{
            transform: scale(1.08);
            border-color: #38bdf8;
            box-shadow: 0 0 12px rgba(56, 189, 248, 0.4);
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
            background: var(--accent-color-soft, rgba(59, 130, 246, 0.25));
            border: 1.5px solid var(--accent-color, #3b82f6);
            border-radius: 20px;
            padding: 3px 9px 3px 4px;
            box-shadow: 0 4px 12px var(--accent-color-soft, rgba(59, 130, 246, 0.4));
            white-space: nowrap;
            box-sizing: border-box;
            transition: all 0.2s ease;
        }}
        .enterprise-personnel-pin:hover {{
            transform: scale(1.08);
            border-color: var(--accent-color-hover, #2563eb);
            box-shadow: 0 0 14px var(--accent-color-soft, rgba(59, 130, 246, 0.5));
        }}
        .avatar-badge {{
            width: 18px;
            height: 18px;
            border-radius: 50%;
            background: var(--accent-color, #3b82f6);
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
            width: 22px;
            height: 22px;
            border-radius: 50%;
            font-size: 11px;
            font-weight: 800;
            font-family: system-ui, sans-serif;
            color: #ffffff;
            box-shadow: 0 2px 6px rgba(0,0,0,0.4);
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
        function escapeHtml(str) {{
            if (str === null || str === undefined) return '';
            return String(str)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#039;');
        }}

        var map = L.map('map', {{
            zoomControl: false
        }}).setView([{center_lat}, {center_lon}], 12);

        L.control.zoom({{
            position: 'bottomright'
        }}).addTo(map);

        var currentTileLayer = null;
        var currentTileStyle = 'dark';
        var tileUrls = {{
            dark: 'https://{{s}}.basemaps.cartocdn.com/dark_all/{{z}}/{{x}}/{{y}}{{r}}.png',
            light: 'https://{{s}}.basemaps.cartocdn.com/rastertiles/voyager/{{z}}/{{x}}/{{y}}{{r}}.png',
            satellite: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{{z}}/{{y}}/{{x}}'
        }};
        var tileAttributions = {{
            dark: '&copy; OpenStreetMap contributors &copy; CARTO',
            light: '&copy; OpenStreetMap contributors &copy; CARTO',
            satellite: '&copy; Esri &mdash; Source: Esri, i-cubed, USDA, USGS, AEX, GeoEye, Getmapping, Aerogrid, IGN, IGP, UPR-EGP'
        }};

        function setTileStyle(styleName) {{
            var style = styleName || 'dark';
            if (currentTileLayer && currentTileStyle === style) return;
            if (currentTileLayer) {{
                map.removeLayer(currentTileLayer);
            }}
            var url = tileUrls[style] || tileUrls.dark;
            var attr = tileAttributions[style] || tileAttributions.dark;
            currentTileLayer = L.tileLayer(url, {{ attribution: attr }}).addTo(map);
            currentTileStyle = style;
        }}

        setTileStyle('dark');

        var coordsMap = {{}};
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

        window.recenterMap = function(lat, lng, zoom) {{
            map.flyTo([lat, lng], zoom || 12, {{ duration: 1.2 }});
        }};

        window.fitBoundsMap = function() {{
            var featureGroup = L.featureGroup();
            if (map.hasLayer(vehicleGroup)) vehicleGroup.eachLayer(function(l) {{ featureGroup.addLayer(l); }});
            if (map.hasLayer(personnelGroup)) personnelGroup.eachLayer(function(l) {{ featureGroup.addLayer(l); }});
            if (map.hasLayer(jobGroup)) jobGroup.eachLayer(function(l) {{ featureGroup.addLayer(l); }});
            
            var bounds = featureGroup.getBounds();
            if (bounds.isValid()) {{
                map.fitBounds(bounds, {{ padding: [40, 40], maxZoom: 15 }});
            }}
        }};

        window.focusUnit = function(id, type, lat, lng) {{
            if (lat && lng) {{
                map.flyTo([lat, lng], 15, {{ duration: 1.0 }});
                if (type === 'vehicle' && vehicleMarkers[id]) {{
                    vehicleMarkers[id].openPopup();
                }} else if (type === 'personnel' && personnelMarkers[id]) {{
                    personnelMarkers[id].openPopup();
                }}
            }}
        }};

        window.updateMapData = function(vehicles, personnel, jobs, newCoords, trackingMode, i18n, tileStyle) {{
            if (tileStyle) {{
                setTileStyle(tileStyle);
            }}
            if (newCoords) {{
                Object.assign(coordsMap, newCoords);
            }}
            var mode = trackingMode || 'both';
            var labels = Object.assign({{
                cargo_capacity: 'Cargo Capacity',
                status: 'Status',
                last_tracked: 'Last Tracked',
                role: 'Role',
                phone: 'Phone',
                gps_status: 'GPS Status',
                start_point: 'Start Point (A)',
                destination: 'Destination (B)',
                planned_route: 'Planned Route',
                job: 'Job'
            }}, i18n || {{}});

            // Toggle Vehicle Group Visibility
            if (mode === 'both' || mode === 'vehicles') {{
                if (!map.hasLayer(vehicleGroup)) map.addLayer(vehicleGroup);
            }} else {{
                if (map.hasLayer(vehicleGroup)) map.removeLayer(vehicleGroup);
            }}

            // Toggle Personnel Group Visibility
            if (mode === 'both' || mode === 'personnel') {{
                if (!map.hasLayer(personnelGroup)) map.addLayer(personnelGroup);
            }} else {{
                if (map.hasLayer(personnelGroup)) map.removeLayer(personnelGroup);
            }}

            // Update Vehicle Markers
            var currentVehicleIds = {{}};
            vehicles.forEach(function(v) {{
                if (v.latitude !== null && v.longitude !== null) {{
                    currentVehicleIds[v.id] = true;
                    var popupContent = '<div class="vehicle-popup">' +
                        '<h4>' + escapeHtml(v.license_plate) + ' (' + escapeHtml(v.name) + ')</h4>' +
                        '<b>' + escapeHtml(labels.cargo_capacity) + ':</b> ' + escapeHtml(v.capacity_m3) + ' m³<br/>' +
                        '<b>' + escapeHtml(labels.status) + ':</b> ' + escapeHtml(v.status) + '<br/>' +
                        (v.last_ping ? '<b>' + escapeHtml(labels.last_tracked) + ':</b> ' + escapeHtml(new Date(v.last_ping).toLocaleTimeString()) : '') +
                        '</div>';

                    if (vehicleMarkers[v.id]) {{
                        var marker = vehicleMarkers[v.id];
                        var oldLatLng = marker.getLatLng();
                        if (Math.abs(oldLatLng.lat - v.latitude) > 0.00001 || Math.abs(oldLatLng.lng - v.longitude) > 0.00001) {{
                            animateMarker(marker, oldLatLng.lat, oldLatLng.lng, v.latitude, v.longitude, 1200);
                        }}
                        marker.setPopupContent(popupContent);
                    }} else {{
                        var icon = L.divIcon({{
                            html: '<div class="enterprise-vehicle-pin ' + (v.status === 'active' ? 'active' : '') + '">' +
                                  '<div class="status-dot"></div>' +
                                  '<span class="pin-plate">' + escapeHtml(v.license_plate) + '</span>' +
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
            for (var vid in vehicleMarkers) {{
                if (!currentVehicleIds[vid]) {{
                    vehicleGroup.removeLayer(vehicleMarkers[vid]);
                    delete vehicleMarkers[vid];
                }}
            }}

            // Update Personnel Markers
            var currentPersonnelIds = {{}};
            personnel.forEach(function(p) {{
                if (p.latitude !== null && p.longitude !== null) {{
                    currentPersonnelIds[p.id] = true;
                    var popupContent = '<div class="vehicle-popup">' +
                        '<h4>' + escapeHtml(p.name) + ' (' + escapeHtml(p.initials) + ')</h4>' +
                        '<b>' + escapeHtml(labels.role) + ':</b> ' + escapeHtml(p.role) + '<br/>' +
                        '<b>' + escapeHtml(labels.phone) + ':</b> ' + escapeHtml(p.phone) + '<br/>' +
                        '<b>' + escapeHtml(labels.gps_status) + ':</b> ' + escapeHtml(p.status) + '<br/>' +
                        '</div>';

                    if (personnelMarkers[p.id]) {{
                        var marker = personnelMarkers[p.id];
                        var oldLatLng = marker.getLatLng();
                        if (Math.abs(oldLatLng.lat - p.latitude) > 0.00001 || Math.abs(oldLatLng.lng - p.longitude) > 0.00001) {{
                            animateMarker(marker, oldLatLng.lat, oldLatLng.lng, p.latitude, p.longitude, 1200);
                        }}
                        marker.setPopupContent(popupContent);
                    }} else {{
                        var icon = L.divIcon({{
                            html: '<div class="enterprise-personnel-pin">' +
                                  '<div class="avatar-badge">' + escapeHtml(p.initials) + '</div>' +
                                  '<span class="personnel-name">' + escapeHtml(p.name) + '</span>' +
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
            for (var pid in personnelMarkers) {{
                if (!currentPersonnelIds[pid]) {{
                    personnelGroup.removeLayer(personnelMarkers[pid]);
                    delete personnelMarkers[pid];
                }}
            }}

            // Refresh Job Route Polylines and Stop Markers
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
                        iconSize: [22, 22],
                        iconAnchor: [11, 11]
                    }});
                    var m = L.marker(origin, {{ icon: originIcon }}).bindPopup('<b>' + escapeHtml(labels.start_point) + ':</b> ' + escapeHtml(j.origin_address || j.location_address) + '<br/><b>' + escapeHtml(labels.job) + ':</b> ' + escapeHtml(j.title));
                    m.addTo(jobGroup);
                    jobMarkers.push(m);
                }}

                if (dest) {{
                    var destIcon = L.divIcon({{
                        html: '<div class="job-stop-pin destination">B</div>',
                        className: 'job-pin-container',
                        iconSize: [22, 22],
                        iconAnchor: [11, 11]
                    }});
                    var m = L.marker(dest, {{ icon: destIcon }}).bindPopup('<b>' + escapeHtml(labels.destination) + ':</b> ' + escapeHtml(j.destination_address) + '<br/><b>' + escapeHtml(labels.job) + ':</b> ' + escapeHtml(j.title));
                    m.addTo(jobGroup);
                    jobMarkers.push(m);
                }}

                if (origin && dest) {{
                    var lineColor = getComputedStyle(document.documentElement).getPropertyValue('--accent-color').trim() || '#3b82f6';
                    var line = L.polyline([origin, dest], {{
                        color: lineColor,
                        weight: 3.5,
                        opacity: 0.85,
                        dashArray: '6, 6'
                    }}).bindPopup('<b>' + escapeHtml(labels.planned_route) + ':</b> ' + escapeHtml(j.title));
                    line.addTo(jobGroup);
                    routeLines.push(line);
                }}
            }});
        }};
    </script>
</body>
</html>
"#,
        center_lat = base_lat,
        center_lon = base_lon
    );

    rsx! {
        div { class: "w-full h-full min-h-[calc(100vh-3.5rem)] relative overflow-hidden bg-slate-950 flex flex-col animate-in fade-in duration-300",

            // Static Full-bleed map iframe
            iframe {
                id: "live-map-iframe",
                srcdoc: "{map_html}",
                style: "width: 100%; height: 100%; position: absolute; inset: 0; border: none; display: block; z-index: 0;",
                class: "bg-slate-900"
            }

            // Floating Clean Top Control Bar
            div {
                style: "position: absolute; top: 16px; left: 16px; right: 16px; z-index: 10; pointer-events: auto; background: rgba(15, 23, 42, 0.88); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 20px; padding: 0.55rem 0.9rem; box-shadow: 0 15px 35px -10px rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;",

                // Real-Time Search Bar
                div { class: "flex-1 max-w-xs relative min-w-[180px]",
                    input {
                        class: "w-full bg-white/10 border border-white/15 rounded-xl px-3 py-1.5 pl-8 text-xs text-white placeholder-slate-400 focus:outline-none focus:border-primary focus:bg-white/15 transition-all shadow-inner",
                        placeholder: "{t(\"live-map-search-placeholder\", &region)}",
                        value: "{map_search}",
                        oninput: move |e| map_search.set(e.value())
                    }
                    div { class: "absolute left-2.5 top-2 text-slate-400 pointer-events-none",
                        components::LucideIcon { name: "search", size: "13" }
                    }
                }

                // Tracking Mode & Status Filters & Tile Switcher
                div { class: "flex items-center gap-2 flex-wrap",

                    // Unit Type Selector Pills
                    div { class: "flex items-center gap-1 bg-white/5 border border-white/10 p-1 rounded-xl shadow-xs",
                        button {
                            onclick: move |_| tracking_mode.set("both".to_string()),
                            class: format!("px-2.5 py-1 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "both" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "layers", size: "12" }
                            "{t(\"live-map-filter-all\", &region)}"
                        }
                        button {
                            onclick: move |_| tracking_mode.set("vehicles".to_string()),
                            class: format!("px-2.5 py-1 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "vehicles" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "truck", size: "12" }
                            "{t(\"live-map-filter-fleet\", &region)}"
                        }
                        button {
                            onclick: move |_| tracking_mode.set("personnel".to_string()),
                            class: format!("px-2.5 py-1 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center gap-1.5 {}", if *tracking_mode.read() == "personnel" { "bg-primary text-primary-foreground shadow-md" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "users", size: "12" }
                            "{t(\"live-map-filter-field\", &region)}"
                        }
                    }

                    // Status Filter Pills
                    div { class: "flex items-center gap-1 bg-white/5 border border-white/10 p-1 rounded-xl shadow-xs",
                        button {
                            onclick: move |_| status_filter.set("all".to_string()),
                            class: format!("px-2 py-1 rounded-lg text-[11px] font-bold transition-all border-0 cursor-pointer {}", if *status_filter.read() == "all" { "bg-slate-700 text-white shadow-xs" } else { "bg-transparent text-slate-400 hover:text-white" }),
                            "Alla status"
                        }
                        button {
                            onclick: move |_| status_filter.set("active".to_string()),
                            class: format!("px-2 py-1 rounded-lg text-[11px] font-bold transition-all border-0 cursor-pointer flex items-center gap-1 {}", if *status_filter.read() == "active" { "bg-emerald-600 text-white shadow-xs" } else { "bg-transparent text-slate-400 hover:text-emerald-400" }),
                            span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 shadow-[0_0_6px_#34d399]" }
                            "Aktiv"
                        }
                        button {
                            onclick: move |_| status_filter.set("idle".to_string()),
                            class: format!("px-2 py-1 rounded-lg text-[11px] font-bold transition-all border-0 cursor-pointer flex items-center gap-1 {}", if *status_filter.read() == "idle" { "bg-amber-600 text-white shadow-xs" } else { "bg-transparent text-slate-400 hover:text-amber-400" }),
                            span { class: "w-1.5 h-1.5 rounded-full bg-amber-400" }
                            "Inaktiv"
                        }
                    }

                    // Tile Theme Selector
                    div { class: "flex items-center gap-1 bg-white/5 border border-white/10 p-1 rounded-xl shadow-xs",
                        button {
                            onclick: move |_| tile_style.set("dark".to_string()),
                            title: "Mörkt karttema",
                            class: format!("p-1.5 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center justify-center {}", if *tile_style.read() == "dark" { "bg-primary text-white shadow-xs" } else { "text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "moon", size: "13" }
                        }
                        button {
                            onclick: move |_| tile_style.set("light".to_string()),
                            title: "Ljust karttema",
                            class: format!("p-1.5 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center justify-center {}", if *tile_style.read() == "light" { "bg-primary text-white shadow-xs" } else { "text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "sun", size: "13" }
                        }
                        button {
                            onclick: move |_| tile_style.set("satellite".to_string()),
                            title: "Satellitvy",
                            class: format!("p-1.5 rounded-lg text-xs font-bold transition-all border-0 cursor-pointer flex items-center justify-center {}", if *tile_style.read() == "satellite" { "bg-primary text-white shadow-xs" } else { "text-slate-400 hover:text-white" }),
                            components::LucideIcon { name: "globe", size: "13" }
                        }
                    }

                    // Auto-Fit View Action Button
                    button {
                        class: "p-2 rounded-xl bg-white/10 border border-white/15 text-slate-300 hover:text-white hover:bg-white/20 cursor-pointer transition-all flex items-center justify-center shadow-xs",
                        title: "{t(\"live-map-fit-bounds\", &region)}",
                        onclick: move |_| {
                            let script = r#"
                                var iframe = document.getElementById('live-map-iframe');
                                if (iframe && iframe.contentWindow && typeof iframe.contentWindow.fitBoundsMap === 'function') {
                                    iframe.contentWindow.fitBoundsMap();
                                }
                            "#;
                            let _ = dioxus::document::eval(script);
                        },
                        components::LucideIcon { name: "maximize", size: "15" }
                    }

                    // Recenter Map Action Button
                    button {
                        class: "p-2 rounded-xl bg-white/10 border border-white/15 text-slate-300 hover:text-white hover:bg-white/20 cursor-pointer transition-all flex items-center justify-center shadow-xs",
                        title: "{t(\"live-map-recenter\", &region)}",
                        onclick: move |_| {
                            let script = format!(
                                r#"
                                var iframe = document.getElementById('live-map-iframe');
                                if (iframe && iframe.contentWindow && typeof iframe.contentWindow.recenterMap === 'function') {{
                                    iframe.contentWindow.recenterMap({}, {}, 12);
                                }}
                                "#,
                                base_lat, base_lon
                            );
                            let _ = dioxus::document::eval(&script);
                        },
                        components::LucideIcon { name: "compass", size: "15" }
                    }
                }
            }

            // Collapsable Left Telemetry Drawer Panel
            if *sidebar_collapsed.read() {
                button {
                    onclick: move |_| sidebar_collapsed.set(false),
                    style: "position: absolute; top: 76px; left: 16px; z-index: 1000; background: rgba(15, 23, 42, 0.88); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 16px; padding: 0.6rem 1rem; box-shadow: 0 15px 35px -10px rgba(0,0,0,0.7); cursor: pointer; color: white; display: flex; align-items: center; gap: 8px; transition: all 0.2s;",
                    class: "hover:bg-white/15 hover:scale-105 active:scale-95 border-0",
                    components::LucideIcon { name: "layers", size: "14" }
                    span { class: "text-xs font-extrabold tracking-wide", "{t(\"live-map-expand-panel\", &region)}" }
                    span { class: "bg-primary/30 text-primary-foreground text-[10px] font-bold px-1.5 py-0.5 rounded-full ml-1", "{total_active_units}" }
                }
            } else {
                div {
                    style: "position: absolute; top: 76px; left: 16px; bottom: 16px; z-index: 1000; display: flex; flex-direction: column; background: rgba(15, 23, 42, 0.88); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 20px; box-shadow: 0 20px 50px -10px rgba(0,0,0,0.8); box-sizing: border-box; overflow: hidden; padding: 1.1rem;",
                    class: "w-[320px] max-sm:w-[calc(100%-2rem)] max-sm:left-4 max-sm:right-4 max-sm:bottom-4 max-sm:top-auto max-sm:max-h-[50vh] animate-in slide-in-from-left-4 duration-200",
                    div { class: "flex items-center justify-between pb-2.5 border-b border-white/10",
                        div { class: "flex items-center gap-2.5",
                            div { class: "rounded-xl bg-primary/20 p-2 text-primary border border-primary/20",
                                components::LucideIcon { name: if *tracking_mode.read() == "personnel" { "users" } else { "truck" }, class: "h-4 w-4" }
                            }
                            div {
                                h4 { class: "text-xs font-bold text-white m-0", "{drawer_title}" }
                                p { class: "text-[9px] text-slate-400 m-0", "{t(\"live-map-telemetry-subtitle\", &region)}" }
                            }
                        }
                        button {
                            onclick: move |_| sidebar_collapsed.set(true),
                            class: "p-1.5 rounded-xl hover:bg-white/10 text-slate-400 hover:text-white border-0 bg-transparent cursor-pointer transition-all flex items-center justify-center",
                            title: "{t(\"live-map-collapse-panel\", &region)}",
                            components::LucideIcon { name: "chevron-left", size: "16" }
                        }
                    }

                    div { class: "flex-1 overflow-y-auto space-y-2 mt-3 pr-1 scrollbar-thin",

                        // Empty State if no units match current filter
                        if drawer_vehicles.read().is_empty() && drawer_personnel.read().is_empty() {
                            div { class: "py-8 text-center flex flex-col items-center justify-center px-3",
                                div { class: "w-10 h-10 rounded-full bg-slate-800 flex items-center justify-center text-slate-400 mb-2 border border-slate-700",
                                    components::LucideIcon { name: "search-x", size: "18" }
                                }
                                p { class: "text-xs font-bold text-slate-300 m-0", "Inga enheter hittades" }
                                p { class: "text-[10px] text-slate-500 m-0 mt-1", "Ändra sökord eller nollställ valda filter" }
                            }
                        }

                        // Vehicles telemetry section
                        if *tracking_mode.read() == "both" || *tracking_mode.read() == "vehicles" {
                            if !drawer_vehicles.read().is_empty() {
                                div { class: "text-[10px] font-extrabold uppercase text-slate-400 tracking-wider mb-1 flex items-center justify-between",
                                    span { "{t(\"live-map-fleet-section\", &region)}" }
                                    span { class: "bg-white/10 text-white px-1.5 py-0.5 rounded text-[9px]", "{drawer_vehicles.read().len()}" }
                                }
                                for v in drawer_vehicles.read().iter().cloned() {
                                    {
                                        let is_selected = selected_unit_id.read().as_ref() == Some(&v.id);
                                        let card_class = if is_selected {
                                            "p-2.5 rounded-xl bg-primary/20 border border-primary/40 flex items-center gap-3 transition-all cursor-pointer mb-1.5 shadow-md ring-1 ring-primary/40"
                                        } else {
                                            "p-2.5 rounded-xl bg-white/5 border border-white/5 flex items-center gap-3 transition-all hover:bg-white/15 hover:border-white/20 cursor-pointer mb-1.5 shadow-xs group"
                                        };

                                        rsx! {
                                            div {
                                                key: "{v.id}",
                                                onclick: {
                                                    let v_id = v.id.clone();
                                                    let v_id_json = serde_json::to_string(&v.id).unwrap_or_default();
                                                    let lat_opt = v.latitude;
                                                    let lng_opt = v.longitude;
                                                    move |_| {
                                                        selected_unit_id.set(Some(v_id.clone()));
                                                        if let (Some(lat), Some(lng)) = (lat_opt, lng_opt) {
                                                            let script = format!(
                                                                r#"
                                                                var iframe = document.getElementById('live-map-iframe');
                                                                if (iframe && iframe.contentWindow && typeof iframe.contentWindow.focusUnit === 'function') {{
                                                                    iframe.contentWindow.focusUnit({}, 'vehicle', {}, {});
                                                                }}
                                                                "#,
                                                                v_id_json, lat, lng
                                                            );
                                                            let _ = dioxus::document::eval(&script);
                                                        }
                                                    }
                                                },
                                                class: "{card_class}",
                                                div {
                                                    class: format!("w-2.5 h-2.5 rounded-full flex-shrink-0 {}", if v.status == "active" { "bg-emerald-400 shadow-[0_0_8px_#34d399]" } else { "bg-slate-500" })
                                                }
                                                div { class: "flex-1 min-w-0",
                                                    p { class: "text-xs font-bold text-white m-0 truncate group-hover:text-primary transition-colors", "{v.name}" }
                                                    p { class: "text-[9px] text-slate-400 m-0 flex items-center gap-1 mt-0.5",
                                                        components::LucideIcon { name: "credit-card", size: "9" }
                                                        "{v.license_plate} • {v.capacity_m3}m³"
                                                    }
                                                }
                                                if let (Some(lat), Some(lng)) = (v.latitude, v.longitude) {
                                                    div { class: "text-[8px] font-mono text-emerald-400 text-right flex-shrink-0 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/10 group-hover:bg-emerald-500/20 transition-all",
                                                        div { "{lat:.4}°N" }
                                                        div { "{lng:.4}°E" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Personnel telemetry section
                        if *tracking_mode.read() == "both" || *tracking_mode.read() == "personnel" {
                            if !drawer_personnel.read().is_empty() {
                                div { class: "text-[10px] font-extrabold uppercase text-slate-400 tracking-wider mb-1 mt-3 flex items-center justify-between",
                                    span { "{t(\"live-map-team-section\", &region)}" }
                                    span { class: "bg-primary/20 text-primary px-1.5 py-0.5 rounded text-[9px]", "{drawer_personnel.read().len()}" }
                                }
                                for p in drawer_personnel.read().iter().cloned() {
                                    {
                                        let is_selected = selected_unit_id.read().as_ref() == Some(&p.id);
                                        let card_class = if is_selected {
                                            "p-2.5 rounded-xl bg-primary/30 border border-primary/40 flex items-center gap-3 transition-all cursor-pointer mb-1.5 shadow-md ring-1 ring-primary/40"
                                        } else {
                                            "p-2.5 rounded-xl bg-primary/10 border border-primary/20 flex items-center gap-3 transition-all hover:bg-primary/20 hover:border-primary/40 cursor-pointer mb-1.5 shadow-xs group"
                                        };

                                        rsx! {
                                            div {
                                                key: "{p.id}",
                                                onclick: {
                                                    let p_id = p.id.clone();
                                                    let p_id_json = serde_json::to_string(&p.id).unwrap_or_default();
                                                    let lat = p.latitude;
                                                    let lng = p.longitude;
                                                    move |_| {
                                                        selected_unit_id.set(Some(p_id.clone()));
                                                        let script = format!(
                                                            r#"
                                                            var iframe = document.getElementById('live-map-iframe');
                                                            if (iframe && iframe.contentWindow && typeof iframe.contentWindow.focusUnit === 'function') {{
                                                                iframe.contentWindow.focusUnit({}, 'personnel', {}, {});
                                                            }}
                                                            "#,
                                                            p_id_json, lat, lng
                                                        );
                                                        let _ = dioxus::document::eval(&script);
                                                    }
                                                },
                                                class: "{card_class}",
                                                div {
                                                    class: "w-6 h-6 rounded-full bg-primary text-white text-[9px] font-extrabold flex items-center justify-center shrink-0 shadow",
                                                    "{p.initials}"
                                                }
                                                div { class: "flex-1 min-w-0",
                                                    p { class: "text-xs font-bold text-white m-0 truncate group-hover:text-primary-foreground transition-colors", "{p.name}" }
                                                    p { class: "text-[9px] text-primary m-0 flex items-center gap-1 mt-0.5",
                                                        components::LucideIcon { name: "user-check", size: "9" }
                                                        "{p.role}"
                                                    }
                                                }
                                                div { class: "text-[8px] font-mono text-primary text-right flex-shrink-0 bg-primary/20 px-1.5 py-0.5 rounded border border-primary/20 group-hover:bg-primary/20 transition-all",
                                                    div { "{p.latitude:.4}°N" }
                                                    div { "{p.longitude:.4}°E" }
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
