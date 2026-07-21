use crate::database;
use crate::infra::errors::YntraError;
use crate::JobTicket;

fn urlencode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[allow(dead_code)]
pub fn mock_geocode(address: &str) -> (f64, f64) {
    mock_geocode_with_center(address, 59.3293, 18.0686)
}

pub fn mock_geocode_with_center(address: &str, center_lat: f64, center_lng: f64) -> (f64, f64) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    address.hash(&mut hasher);
    let hash = hasher.finish();
    let lat = center_lat + ((hash & 0xFFFF) as f64 / 65535.0) * 0.2 - 0.1;
    let lng = center_lng + (((hash >> 16) & 0xFFFF) as f64 / 65535.0) * 0.2 - 0.1;
    (lat, lng)
}

pub async fn geocode(workspace_id: &str, address: &str) -> (f64, f64) {
    if address.trim().is_empty() {
        return (0.0, 0.0);
    }

    // 1. Fetch workspace settings
    let settings_str: String = if let Ok(conn) = database::acquire_connection().await {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let provider = settings_json
        .get("geocoder_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("nominatim");

    let api_key = settings_json
        .get("geocoder_api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let custom_url = settings_json
        .get("geocoder_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Resolve fallback mock center coordinates based on company country/default settings
    let center_lat = settings_json
        .get("default_geocoding_latitude")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            let country = settings_json.get("company_country").and_then(|v| v.as_str()).unwrap_or("");
            match country.to_lowercase().as_str() {
                "us" | "usa" | "united states" => Some(37.7749),
                "de" | "germany" | "deutschland" => Some(52.5200),
                "gb" | "uk" | "united kingdom" => Some(51.5074),
                "fi" | "finland" => Some(60.1699),
                "no" | "norway" => Some(59.9139),
                "dk" | "denmark" => Some(55.6761),
                _ => None,
            }
        })
        .unwrap_or(59.3293);

    let center_lng = settings_json
        .get("default_geocoding_longitude")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            let country = settings_json.get("company_country").and_then(|v| v.as_str()).unwrap_or("");
            match country.to_lowercase().as_str() {
                "us" | "usa" | "united states" => Some(-122.4194),
                "de" | "germany" | "deutschland" => Some(13.4050),
                "gb" | "uk" | "united kingdom" => Some(-0.1278),
                "fi" | "finland" => Some(24.9384),
                "no" | "norway" => Some(10.7522),
                "dk" | "denmark" => Some(12.5683),
                _ => None,
            }
        })
        .unwrap_or(18.0686);

    #[cfg(not(target_arch = "wasm32"))]
    let client_res = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    #[cfg(target_arch = "wasm32")]
    let client_res = reqwest::Client::builder()
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    let client = match client_res {
        Ok(c) => c,
        Err(_) => {
            tracing::warn!(
                "Geocoder HTTP client initialization failed. Falling back to mock geocoding centered at ({:.4}, {:.4}) for address: {}",
                center_lat, center_lng, address
            );
            return mock_geocode_with_center(address, center_lat, center_lng);
        }
    };

    match provider {
        "google" => {
            let key = if api_key.is_empty() {
                #[cfg(not(target_arch = "wasm32"))]
                { std::env::var("GOOGLE_MAPS_API_KEY").unwrap_or_default() }
                #[cfg(target_arch = "wasm32")]
                { "".to_string() }
            } else {
                api_key.to_string()
            };

            if key.is_empty() {
                tracing::warn!(
                    "Google Maps API key missing. Falling back to mock geocoding centered at ({:.4}, {:.4}) for address: {}",
                    center_lat, center_lng, address
                );
                return mock_geocode_with_center(address, center_lat, center_lng);
            }

            let url = format!(
                "https://maps.googleapis.com/maps/api/geocode/json?address={}&key={}",
                urlencode(address),
                key
            );

            if let Ok(resp) = client.get(&url).send().await {
                #[derive(serde::Deserialize)]
                struct GoogleLocation {
                    lat: f64,
                    lng: f64,
                }
                #[derive(serde::Deserialize)]
                struct GoogleGeometry {
                    location: GoogleLocation,
                }
                #[derive(serde::Deserialize)]
                struct GoogleResult {
                    geometry: GoogleGeometry,
                }
                #[derive(serde::Deserialize)]
                struct GoogleResponse {
                    results: Vec<GoogleResult>,
                }

                if let Ok(results_res) = resp.json::<GoogleResponse>().await {
                    if let Some(first) = results_res.results.first() {
                        return (first.geometry.location.lat, first.geometry.location.lng);
                    }
                }
            }
        }
        "mapbox" => {
            let token = if api_key.is_empty() {
                #[cfg(not(target_arch = "wasm32"))]
                { std::env::var("MAPBOX_ACCESS_TOKEN").unwrap_or_default() }
                #[cfg(target_arch = "wasm32")]
                { "".to_string() }
            } else {
                api_key.to_string()
            };

            if token.is_empty() {
                tracing::warn!(
                    "Mapbox access token missing. Falling back to mock geocoding centered at ({:.4}, {:.4}) for address: {}",
                    center_lat, center_lng, address
                );
                return mock_geocode_with_center(address, center_lat, center_lng);
            }

            let url = format!(
                "https://api.mapbox.com/geocoding/v5/mapbox.places/{}.json?access_token={}&limit=1",
                urlencode(address),
                token
            );

            if let Ok(resp) = client.get(&url).send().await {
                #[derive(serde::Deserialize)]
                struct GeoJsonGeometry {
                    coordinates: Vec<f64>,
                }
                #[derive(serde::Deserialize)]
                struct GeoJsonFeature {
                    geometry: GeoJsonGeometry,
                }
                #[derive(serde::Deserialize)]
                struct GeoJsonFeatureCollection {
                    features: Vec<GeoJsonFeature>,
                }

                if let Ok(collection) = resp.json::<GeoJsonFeatureCollection>().await {
                    if let Some(first) = collection.features.first() {
                        if first.geometry.coordinates.len() >= 2 {
                            return (first.geometry.coordinates[1], first.geometry.coordinates[0]);
                        }
                    }
                }
            }
        }
        "photon" => {
            let base_url = if custom_url.is_empty() {
                "http://localhost:2322"
            } else {
                custom_url
            };

            let url = format!(
                "{}/api?q={}&limit=1",
                base_url.trim_end_matches('/'),
                urlencode(address)
            );

            if let Ok(resp) = client.get(&url).send().await {
                #[derive(serde::Deserialize)]
                struct GeoJsonGeometry {
                    coordinates: Vec<f64>,
                }
                #[derive(serde::Deserialize)]
                struct GeoJsonFeature {
                    geometry: GeoJsonGeometry,
                }
                #[derive(serde::Deserialize)]
                struct GeoJsonFeatureCollection {
                    features: Vec<GeoJsonFeature>,
                }

                if let Ok(collection) = resp.json::<GeoJsonFeatureCollection>().await {
                    if let Some(first) = collection.features.first() {
                        if first.geometry.coordinates.len() >= 2 {
                            return (first.geometry.coordinates[1], first.geometry.coordinates[0]);
                        }
                    }
                }
            }
        }
        _ => {
            let base_url = if custom_url.is_empty() {
                "https://nominatim.openstreetmap.org"
            } else {
                custom_url
            };

            let url = format!(
                "{}/search?q={}&format=json&limit=1",
                base_url.trim_end_matches('/'),
                urlencode(address)
            );

            if let Ok(resp) = client.get(&url).send().await {
                #[derive(serde::Deserialize)]
                struct NominatimResponse {
                    lat: String,
                    lon: String,
                }
                if let Ok(results) = resp.json::<Vec<NominatimResponse>>().await {
                    if let Some(first) = results.first() {
                        let lat_parsed = first.lat.parse::<f64>();
                        let lon_parsed = first.lon.parse::<f64>();
                        if let (Ok(lat), Ok(lon)) = (lat_parsed, lon_parsed) {
                            return (lat, lon);
                        }
                    }
                }
            }
        }
    }

    tracing::warn!(
        "Geocoding API request failed or returned empty results. Falling back to mock geocoding centered at ({:.4}, {:.4}) for address: {}",
        center_lat, center_lng, address
    );
    mock_geocode_with_center(address, center_lat, center_lng)
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

async fn get_road_distances_osrm(
    client: &reqwest::Client,
    base_url: &str,
    current_pos: (f64, f64),
    targets: &[(f64, f64)],
) -> Option<Vec<f64>> {
    let mut coords_str = format!("{},{}", current_pos.1, current_pos.0);
    for t in targets {
        coords_str.push_str(&format!(";{},{}", t.1, t.0));
    }

    let url = format!(
        "{}/table/v1/driving/{}?sources=0&annotations=distance",
        base_url.trim_end_matches('/'),
        coords_str
    );

    if let Ok(resp) = client.get(&url).send().await {
        #[derive(serde::Deserialize)]
        struct OSRMTableResponse {
            distances: Option<Vec<Vec<Option<f64>>>>,
        }
        if let Ok(table) = resp.json::<OSRMTableResponse>().await {
            if let Some(distances_rows) = table.distances {
                if let Some(first_row) = distances_rows.first() {
                    let dists: Vec<f64> = first_row.iter().skip(1).map(|opt| opt.unwrap_or(f64::MAX)).collect();
                    if dists.len() == targets.len() {
                        return Some(dists);
                    }
                }
            }
        }
    }
    None
}

pub async fn optimize_route(
    workspace_id: &str,
    origin: &str,
    _destination: &str,
    intermediate_stops: &[String],
) -> Vec<String> {
    if intermediate_stops.is_empty() {
        return Vec::new();
    }

    // 1. Fetch workspace settings
    let settings_str: String = if let Ok(conn) = database::acquire_connection().await {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let routing_provider = settings_json
        .get("routing_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("osrm");

    let routing_url = settings_json
        .get("routing_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://router.project-osrm.org");

    #[cfg(not(target_arch = "wasm32"))]
    let client_res = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    #[cfg(target_arch = "wasm32")]
    let client_res = reqwest::Client::builder()
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    let client = client_res.ok();

    let origin_coords = geocode(workspace_id, origin).await;
    let mut unvisited = Vec::new();
    for s in intermediate_stops {
        let coords = geocode(workspace_id, s).await;
        unvisited.push((coords, s.clone()));
    }

    let mut current_pos = origin_coords;
    let mut optimized = Vec::new();

    while !unvisited.is_empty() {
        let mut nearest_idx = 0;
        let mut min_dist = f64::MAX;

        // Try querying the road network matrix first if configured
        let mut queried_distances = None;
        if routing_provider == "osrm" && !routing_url.is_empty() {
            if let Some(ref cl) = client {
                let targets: Vec<(f64, f64)> = unvisited.iter().map(|(coords, _)| *coords).collect();
                queried_distances = get_road_distances_osrm(cl, routing_url, current_pos, &targets).await;
            }
        }

        if let Some(dists) = queried_distances {
            for (idx, dist) in dists.iter().enumerate() {
                if *dist < min_dist {
                    min_dist = *dist;
                    nearest_idx = idx;
                }
            }
        } else {
            // Fall back to compliant spherical Haversine distance
            for (idx, (coords, _)) in unvisited.iter().enumerate() {
                let dist = haversine_distance(current_pos.0, current_pos.1, coords.0, coords.1);
                if dist < min_dist {
                    min_dist = dist;
                    nearest_idx = idx;
                }
            }
        }

        let (coords, stop) = unvisited.remove(nearest_idx);
        current_pos = coords;
        optimized.push(stop);
    }

    optimized
}

#[uniffi::export]
pub async fn get_directions_url(
    requester_user_id: String,
    job_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let job: JobTicket = conn
        .query_row(
            "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |row| {
                Ok(JobTicket {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    location_address: row.get(4)?,
                    priority: row.get(5)?,
                    status: row.get(6)?,
                    assigned_user_id: row.get(7)?,
                    scheduled_date: row.get(8)?,
                    checklist_json: row.get(9)?,
                    completion_report: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    sync_status: row.get(13)?,
                    origin_address: row.get(14)?,
                    destination_address: row.get(15)?,
                    origin_floor: row.get(16)?,
                    destination_floor: row.get(17)?,
                    origin_has_elevator: row.get::<bool>(18)?,
                    destination_has_elevator: row.get::<bool>(19)?,
                    origin_parking_permit_needed: row.get::<bool>(20)?,
                    destination_parking_permit_needed: row.get::<bool>(21)?,
                    assigned_vehicle_id: row.get::<Option<String>>(22)?,
                    route_stops_json: row.get::<Option<String>>(23)?,
                    long_carry_meters: row.get::<i64>(24)? as i32,
                    toll_fees: row.get::<f64>(25)?,
                })
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let origin = job.origin_address.filter(|s| !s.trim().is_empty());
    let dest = job.destination_address.filter(|s| !s.trim().is_empty())
        .unwrap_or(job.location_address);

    let stops_str = job.route_stops_json.unwrap_or_else(|| "[]".to_string());
    let stops: Vec<String> = serde_json::from_str(&stops_str).unwrap_or_default();

    let url = match origin {
        Some(org) => {
            if stops.is_empty() {
                format!(
                    "https://www.google.com/maps/dir/?api=1&origin={}&destination={}&travelmode=truck&dirflg=t",
                    urlencode(&org),
                    urlencode(&dest)
                )
            } else {
                let waypoints_str = stops
                    .iter()
                    .map(|s| urlencode(s))
                    .collect::<Vec<String>>()
                    .join("%7C");
                format!(
                    "https://www.google.com/maps/dir/?api=1&origin={}&destination={}&waypoints={}&travelmode=truck&dirflg=t",
                    urlencode(&org),
                    urlencode(&dest),
                    waypoints_str
                )
            }
        }
        None => {
            if stops.is_empty() {
                format!(
                    "https://www.google.com/maps/dir/?api=1&destination={}&travelmode=truck&dirflg=t",
                    urlencode(&dest)
                )
            } else {
                let waypoints_str = stops
                    .iter()
                    .map(|s| urlencode(s))
                    .collect::<Vec<String>>()
                    .join("%7C");
                format!(
                    "https://www.google.com/maps/dir/?api=1&destination={}&waypoints={}&travelmode=truck&dirflg=t",
                    urlencode(&dest),
                    waypoints_str
                )
            }
        }
    };

    Ok(url)
}

#[uniffi::export]
pub async fn get_commercial_truck_directions_url(
    requester_user_id: String,
    job_id: String,
    vehicle_height_m: Option<f64>,
    vehicle_weight_tons: Option<f64>,
    navigation_provider: Option<String>,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (workspace_id, origin_address, destination_address, location_address, route_stops_json): (String, Option<String>, Option<String>, String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, origin_address, destination_address, location_address, route_stops_json FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let origin = origin_address.filter(|s| !s.trim().is_empty());
    let dest = destination_address.filter(|s| !s.trim().is_empty()).unwrap_or(location_address);

    let stops_str = route_stops_json.unwrap_or_else(|| "[]".to_string());
    let stops: Vec<String> = serde_json::from_str(&stops_str).unwrap_or_default();

    let provider = navigation_provider.unwrap_or_else(|| "google_truck".to_string());

    let height = vehicle_height_m.unwrap_or(3.8);
    let weight = vehicle_weight_tons.unwrap_or(12.0);

    let url = match provider.as_str() {
        "here_truck" => {
            format!(
                "https://wego.here.com/directions/truck/{}/{}?height={}&weight={}",
                origin.as_deref().map(urlencode).unwrap_or_default(),
                urlencode(&dest),
                height,
                weight
            )
        }
        "tomtom_truck" => {
            format!(
                "https://mydrive.tomtom.com/goroute?origin={}&destination={}&vehicleType=truck&height={}",
                origin.as_deref().map(urlencode).unwrap_or_default(),
                urlencode(&dest),
                height
            )
        }
        _ => {
            let waypoints_part = if stops.is_empty() {
                "".to_string()
            } else {
                format!("&waypoints={}", stops.iter().map(|s| urlencode(s)).collect::<Vec<_>>().join("%7C"))
            };

            let origin_part = origin.as_deref().map(|org| format!("&origin={}", urlencode(org))).unwrap_or_default();

            format!(
                "https://www.google.com/maps/dir/?api=1{}&destination={}{}&travelmode=truck&dirflg=t",
                origin_part,
                urlencode(&dest),
                waypoints_part
            )
        }
    };

    Ok(url)
}

#[uniffi::export]
pub async fn verify_commercial_route_restrictions(
    requester_user_id: String,
    job_id: String,
    vehicle_height_m: f64,
    vehicle_weight_tons: f64,
    emission_class: String,
) -> Result<crate::CommercialRouteRestrictions, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (workspace_id, origin_address, destination_address, location_address, origin_permit, dest_permit): (String, Option<String>, Option<String>, String, bool, bool) = conn
        .query_row(
            "SELECT workspace_id, origin_address, destination_address, location_address, origin_parking_permit_needed, destination_parking_permit_needed FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<bool>(4)?, r.get::<bool>(5)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let combined_addresses = format!(
        "{} {} {}",
        origin_address.unwrap_or_default(),
        destination_address.unwrap_or_default(),
        location_address
    ).to_lowercase();

    let mut details = Vec::new();
    let mut low_bridge_warning = false;
    let mut environmental_zone_warning = false;
    let mut weight_limit_warning = false;
    let parking_permit_required = origin_permit || dest_permit;

    // 1. Height clearance check (< 3.8m standard clearance)
    if vehicle_height_m >= 3.8 {
        low_bridge_warning = true;
        details.push(format!("Low bridge height clearance risk: vehicle height {:.1}m exceeds 3.8m limit.", vehicle_height_m));
    }

    // 2. Weight limit check (> 3.5 tons residential roads)
    if vehicle_weight_tons >= 3.5 {
        weight_limit_warning = true;
        details.push(format!("Heavy vehicle weight limit risk: {:.1}t vehicle exceeds 3.5t residential zone limit.", vehicle_weight_tons));
    }

    // 3. Environmental Zone (Miljözon / LEZ) check for major cities
    let env_zone_cities = ["stockholm", "göteborg", "gothenburg", "malmö", "malmo", "berlin", "london", "paris", "munich", "hamburg"];
    let is_env_zone_city = env_zone_cities.iter().any(|city| combined_addresses.contains(city));

    if is_env_zone_city && (emission_class.to_lowercase().contains("euro 4") || emission_class.to_lowercase().contains("euro 5") || emission_class.to_lowercase().contains("diesel")) {
        environmental_zone_warning = true;
        details.push(format!("Environmental Zone (Miljözon) warning: Emission class '{}' requires Class 1/2 permit in target city zone.", emission_class));
    }

    if parking_permit_required {
        details.push("Commercial truck parking permit required for loading/unloading at target address.".to_string());
    }

    Ok(crate::CommercialRouteRestrictions {
        low_bridge_warning,
        environmental_zone_warning,
        weight_limit_warning,
        parking_permit_required,
        restriction_details: details,
    })
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn calculate_multi_segment_move_route(
    workspace_id: String,
    origin_address: String,
    waypoints: Vec<String>,
    destination_address: String,
) -> Result<crate::MultiSegmentRouteSummary, YntraError> {
    let mut stops_seq = Vec::new();
    let org_clean = origin_address.trim().to_string();
    if !org_clean.is_empty() {
        stops_seq.push(org_clean);
    }
    for wp in waypoints {
        let wp_clean = wp.trim().to_string();
        if !wp_clean.is_empty() {
            stops_seq.push(wp_clean);
        }
    }
    let dest_clean = destination_address.trim().to_string();
    if !dest_clean.is_empty() {
        stops_seq.push(dest_clean);
    }

    if stops_seq.len() < 2 {
        return Ok(crate::MultiSegmentRouteSummary {
            total_distance_km: 0.0,
            total_duration_minutes: 0.0,
            total_segments: 0,
            segments: Vec::new(),
            storage_in_transit_stops: 0,
        });
    }

    let mut segments = Vec::new();
    let mut total_dist = 0.0;
    let mut total_duration = 0.0;
    let mut sit_count = 0;

    for i in 0..(stops_seq.len() - 1) {
        let start = &stops_seq[i];
        let end = &stops_seq[i + 1];

        let c1 = geocode(&workspace_id, start).await;
        let c2 = geocode(&workspace_id, end).await;
        let dist = haversine_distance(c1.0, c1.1, c2.0, c2.1);
        let road_dist = (dist * 1.28).max(0.5);
        let duration_mins = (road_dist / 45.0 * 60.0).round();

        let seg_lower = end.to_lowercase();
        let is_sit = seg_lower.contains("lager") || seg_lower.contains("storage") || seg_lower.contains("depå") || seg_lower.contains("förvaring") || seg_lower.contains("magasin") || seg_lower.contains("sit");
        if is_sit {
            sit_count += 1;
        }

        let seg_type = if is_sit {
            "storage_in_transit".to_string()
        } else if i == 0 {
            "pickup".to_string()
        } else if i == stops_seq.len() - 2 {
            "dropoff".to_string()
        } else {
            "intermediate_waypoint".to_string()
        };

        total_dist += road_dist;
        total_duration += duration_mins;

        segments.push(crate::RouteSegment {
            segment_index: (i + 1) as i32,
            start_address: start.clone(),
            end_address: end.clone(),
            distance_km: (road_dist * 10.0).round() / 10.0,
            estimated_duration_minutes: duration_mins,
            segment_type: seg_type,
        });
    }

    Ok(crate::MultiSegmentRouteSummary {
        total_distance_km: (total_dist * 10.0).round() / 10.0,
        total_duration_minutes: total_duration,
        total_segments: segments.len() as i32,
        segments,
        storage_in_transit_stops: sit_count,
    })
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn get_job_multi_segment_route(
    requester_user_id: String,
    job_id: String,
) -> Result<crate::MultiSegmentRouteSummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (workspace_id, origin_address, destination_address, location_address, route_stops_json): (String, Option<String>, Option<String>, String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, origin_address, destination_address, location_address, route_stops_json FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let origin = origin_address.unwrap_or_default();
    let dest = destination_address.unwrap_or(location_address);
    let stops_str = route_stops_json.unwrap_or_else(|| "[]".to_string());
    let stops: Vec<String> = serde_json::from_str(&stops_str).unwrap_or_default();

    calculate_multi_segment_move_route(workspace_id, origin, stops, dest).await
}

#[cfg(test)]
mod multi_segment_tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_segment_move_routing_workflow() {
        let waypoints = vec![
            "Shurgard Self Storage, Stockholm".to_string(),
            "Centralgatan 15, Uppsala".to_string(),
        ];
        let res = calculate_multi_segment_move_route(
            "ws_test".to_string(),
            "Kungsgatan 1, Stockholm".to_string(),
            waypoints,
            "Stora Torget 5, Uppsala".to_string(),
        ).await.unwrap();

        assert_eq!(res.total_segments, 3);
        assert_eq!(res.storage_in_transit_stops, 1);
        assert!(res.total_distance_km > 0.0);
        assert_eq!(res.segments[0].segment_type, "storage_in_transit");
        assert_eq!(res.segments[1].segment_type, "intermediate_waypoint");
        assert_eq!(res.segments[2].segment_type, "dropoff");
    }
}
