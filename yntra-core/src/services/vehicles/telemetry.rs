use crate::YntraError;
use crate::database;
use crate::infra::observer::notify_observers;

#[uniffi::export]
pub async fn register_gps_ping(
    requester_user_id: String,
    device_id: String,
    latitude: f64,
    longitude: f64,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest"
        || auth.role == "anonymous"
        || auth.role == "deleted"
        || auth.role == "client"
    {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn.prepare(
        "SELECT id FROM vehicles WHERE workspace_id = ?1 AND (id = ?2 OR license_plate = ?2 OR gps_device_id = ?2)"
    ).await?;
    let mut rows = stmt
        .query(crate::params![auth.workspace_id, &device_id])
        .await?;
    if let Some(row) = rows.next().await? {
        let vehicle_id: String = row.get(0)?;
        conn.execute(
            "UPDATE vehicles SET latitude = ?1, longitude = ?2, last_ping = ?3, updated_at = ?3 WHERE id = ?4",
            crate::params![latitude, longitude, now_ms, vehicle_id]
        ).await?;
        notify_observers();
        Ok(())
    } else {
        Err(YntraError::NotFoundError(
            "No matching vehicle found in this workspace".to_string(),
        ))
    }
}

#[uniffi::export]
pub async fn simulate_vehicle_movement(
    requester_user_id: String,
    vehicle_id: String,
    tick: i32,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest"
        || auth.role == "anonymous"
        || auth.role == "deleted"
        || auth.role == "client"
    {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    // Fetch workspace settings to determine target_region
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    #[cfg(not(debug_assertions))]
    {
        let allow_sim = settings_json
            .get("allow_vehicle_simulation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !allow_sim {
            return Err(YntraError::AuthError(
                "Vehicle GPS simulation is disabled in release builds unless explicitly enabled in workspace settings".to_string(),
            ));
        }
    }

    // Base coordinates
    let (base_lat, base_lon) = match target_region.as_str() {
        "US" => (37.7749, -122.4194), // San Francisco
        "DE" => (52.5200, 13.4050),   // Berlin
        _ => (59.3293, 18.0686),      // Stockholm
    };

    // Calculate dynamic path offset based on tick
    let angle = (tick as f64 * 0.1) % (2.0 * std::f64::consts::PI);
    let radius = 0.015; // roughly 1.5km radius
    let lat_offset = angle.sin() * radius * 0.6;
    let lon_offset = angle.cos() * radius;

    let latitude = base_lat + lat_offset;
    let longitude = base_lon + lon_offset;
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE vehicles SET latitude = ?1, longitude = ?2, last_ping = ?3, updated_at = ?3 WHERE id = ?4 AND workspace_id = ?5",
        crate::params![latitude, longitude, now_ms, vehicle_id, auth.workspace_id]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn stream_vehicle_gps_location(
    requester_user_id: String,
    vehicle_id: String,
    latitude: f64,
    longitude: f64,
    _speed_kmh: f64,
    _heading_deg: f64,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest"
        || auth.role == "anonymous"
        || auth.role == "deleted"
        || auth.role == "client"
    {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE vehicles SET latitude = ?1, longitude = ?2, last_ping = ?3, updated_at = ?3 WHERE id = ?4 AND workspace_id = ?5",
        crate::params![latitude, longitude, now_ms, vehicle_id, auth.workspace_id]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_active_vehicles_telemetry(
    requester_user_id: String,
) -> Result<Vec<crate::VehicleGpsTelemetry>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest"
        || auth.role == "anonymous"
        || auth.role == "deleted"
        || auth.role == "client"
    {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT v.id, v.name, v.license_plate, COALESCE(v.latitude, 59.3293), COALESCE(v.longitude, 18.0686), COALESCE(v.last_ping, 0), j.id, j.title, u.full_name
         FROM vehicles v
         LEFT JOIN job_tickets j ON j.assigned_vehicle_id = v.id AND j.status IN ('in_progress', 'scheduled', 'en_route')
         LEFT JOIN users u ON j.assigned_user_id = u.id
         WHERE v.workspace_id = ?1 AND v.status = 'active'"
    ).await?;

    let now_ms = chrono::Utc::now().timestamp_millis();

    let list = stmt
        .query_map(crate::params![auth.workspace_id], |row| {
            let v_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let plate: String = row.get(2)?;
            let lat: f64 = row.get(3)?;
            let lon: f64 = row.get(4)?;
            let last_ping: i64 = row.get(5)?;
            let job_id: Option<String> = row.get(6)?;
            let job_title: Option<String> = row.get(7)?;
            let driver: Option<String> = row.get(8)?;

            let is_recent = (now_ms - last_ping).abs() < 300_000; // pinged in last 5 mins
            let speed = if is_recent { 45.0 } else { 0.0 };
            let is_moving = is_recent && speed > 5.0;

            Ok(crate::VehicleGpsTelemetry {
                vehicle_id: v_id,
                name,
                license_plate: plate,
                latitude: lat,
                longitude: lon,
                speed_kmh: speed,
                heading_deg: 45.0,
                last_ping,
                assigned_job_id: job_id,
                assigned_job_title: job_title,
                driver_name: driver,
                is_moving,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn register_gps_ping_from_webhook(
    workspace_id: String,
    webhook_token: String,
    payload_json: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| Ok(r.get::<String>(0)?),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Workspace not found".to_string()))?;

    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    let expected_token = settings_json
        .get("gps_webhook_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if expected_token.is_empty() || expected_token != webhook_token {
        return Err(YntraError::AuthError(
            "Access denied: invalid or missing webhook token".to_string(),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid JSON payload: {}", e)))?;

    // Decoupled Multi-Provider Payload Extraction (Samsara, Teltonika, ABAX, Fleet Complete, Traccar)
    let (device_id, latitude, longitude) = extract_telemetry_payload(&parsed)?;
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn.prepare(
        "SELECT id FROM vehicles WHERE workspace_id = ?1 AND (id = ?2 OR license_plate = ?2 OR gps_device_id = ?2)"
    ).await?;
    let mut rows = stmt.query(crate::params![workspace_id, &device_id]).await?;
    if let Some(row) = rows.next().await? {
        let vehicle_id: String = row.get(0)?;
        conn.execute(
            "UPDATE vehicles SET latitude = ?1, longitude = ?2, last_ping = ?3, updated_at = ?3 WHERE id = ?4",
            crate::params![latitude, longitude, now_ms, vehicle_id]
        ).await?;
        notify_observers();
        Ok(())
    } else {
        Err(YntraError::NotFoundError(format!(
            "No matching vehicle found for device ID '{}'",
            device_id
        )))
    }
}

/// Helper payload parser supporting Samsara, Teltonika, ABAX, Fleet Complete, and Traccar
fn extract_telemetry_payload(parsed: &serde_json::Value) -> Result<(String, f64, f64), YntraError> {
    let item = if let Some(arr) = parsed.as_array() {
        arr.first().cloned().unwrap_or_else(|| parsed.clone())
    } else if let Some(data_arr) = parsed.get("data").and_then(|d| d.as_array()) {
        data_arr.first().cloned().unwrap_or_else(|| parsed.clone())
    } else {
        parsed.clone()
    };

    let device_id = item
        .get("deviceId")
        .or_else(|| item.get("device_id"))
        .or_else(|| item.get("imei"))
        .or_else(|| item.get("id"))
        .or_else(|| item.get("serial"))
        .or_else(|| item.get("serialNumber"))
        .or_else(|| item.get("equipmentId"))
        .or_else(|| item.get("assetId"))
        .or_else(|| item.get("vehicleId"))
        .or_else(|| item.get("vehicle_id"))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
        })
        .ok_or_else(|| YntraError::ValidationError("Missing device identification".to_string()))?;

    let location_obj = item
        .get("gps")
        .or_else(|| item.get("location"))
        .or_else(|| item.get("position"))
        .unwrap_or(&item);

    let latitude = location_obj
        .get("lat")
        .or_else(|| location_obj.get("latitude"))
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .ok_or_else(|| YntraError::ValidationError("Missing or invalid latitude".to_string()))?;

    let longitude = location_obj
        .get("lon")
        .or_else(|| location_obj.get("lng"))
        .or_else(|| location_obj.get("longitude"))
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .ok_or_else(|| YntraError::ValidationError("Missing or invalid longitude".to_string()))?;

    Ok((device_id, latitude, longitude))
}
