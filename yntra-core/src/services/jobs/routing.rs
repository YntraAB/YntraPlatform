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
                    "https://www.google.com/maps/dir/?api=1&origin={}&destination={}",
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
                    "https://www.google.com/maps/dir/?api=1&origin={}&destination={}&waypoints={}",
                    urlencode(&org),
                    urlencode(&dest),
                    waypoints_str
                )
            }
        }
        None => {
            if stops.is_empty() {
                format!(
                    "https://www.google.com/maps/dir/?api=1&destination={}",
                    urlencode(&dest)
                )
            } else {
                let waypoints_str = stops
                    .iter()
                    .map(|s| urlencode(s))
                    .collect::<Vec<String>>()
                    .join("%7C");
                format!(
                    "https://www.google.com/maps/dir/?api=1&destination={}&waypoints={}",
                    urlencode(&dest),
                    waypoints_str
                )
            }
        }
    };

    Ok(url)
}
