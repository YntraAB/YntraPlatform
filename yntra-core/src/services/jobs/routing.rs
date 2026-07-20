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

pub fn mock_geocode(address: &str) -> (f64, f64) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    address.hash(&mut hasher);
    let hash = hasher.finish();
    // Stockholm region coordinates mockup
    let lat = 59.3293 + ((hash & 0xFFFF) as f64 / 65535.0) * 0.2 - 0.1;
    let lng = 18.0686 + (((hash >> 16) & 0xFFFF) as f64 / 65535.0) * 0.2 - 0.1;
    (lat, lng)
}

pub async fn geocode(address: &str) -> (f64, f64) {
    if address.trim().is_empty() {
        return (0.0, 0.0);
    }
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencode(address)
    );

    #[cfg(not(target_arch = "wasm32"))]
    let client_res = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    #[cfg(target_arch = "wasm32")]
    let client_res = reqwest::Client::builder()
        .user_agent("YntraPlatform/1.0 (contact@yntra.se)")
        .build();

    if let Ok(client) = client_res {
        if let Ok(resp) = client.get(&url).send().await {
            #[derive(serde::Deserialize)]
            struct GeocodeResponse {
                lat: String,
                lon: String,
            }
            if let Ok(results) = resp.json::<Vec<GeocodeResponse>>().await {
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

    mock_geocode(address)
}

pub async fn optimize_route(
    origin: &str,
    _destination: &str,
    intermediate_stops: &[String],
) -> Vec<String> {
    if intermediate_stops.is_empty() {
        return Vec::new();
    }

    let origin_coords = geocode(origin).await;
    let mut unvisited = Vec::new();
    for s in intermediate_stops {
        let coords = geocode(s).await;
        unvisited.push((coords, s.clone()));
    }

    let mut current_pos = origin_coords;
    let mut optimized = Vec::new();

    while !unvisited.is_empty() {
        let mut nearest_idx = 0;
        let mut min_dist = f64::MAX;

        for (idx, (coords, _)) in unvisited.iter().enumerate() {
            let dist = {
                let dx = current_pos.0 - coords.0;
                let dy = current_pos.1 - coords.1;
                (dx * dx + dy * dy).sqrt()
            };
            if dist < min_dist {
                min_dist = dist;
                nearest_idx = idx;
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
            "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json FROM job_tickets WHERE id = ?1",
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
