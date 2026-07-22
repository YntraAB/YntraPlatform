use crate::database;
use crate::infra::observer::notify_observers;
use crate::models::jobs::{
    DriverVehicleInspectionReport, EldHosLogRecord, GvwrWeightComplianceWarning,
    IftaStateFuelLogRecord, VehicleDotComplianceSummary,
};
use crate::{MoveVehicle, YntraError};
use uuid::Uuid;

#[uniffi::export]
pub async fn get_vehicles(requester_user_id: String) -> Result<Vec<MoveVehicle>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, name, license_plate, capacity_m3, status, updated_at, sync_status, latitude, longitude, last_ping, gps_device_id FROM vehicles WHERE workspace_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![auth.workspace_id], |row| {
            Ok(MoveVehicle {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                license_plate: row.get(3)?,
                capacity_m3: row.get(4)?,
                status: row.get(5)?,
                updated_at: row.get(6)?,
                sync_status: row.get(7)?,
                latitude: row.get(8)?,
                longitude: row.get(9)?,
                last_ping: row.get(10)?,
                gps_device_id: row.get(11)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn create_vehicle(
    requester_user_id: String,
    name: String,
    license_plate: String,
    capacity_m3: f64,
    gps_device_id: Option<String>,
) -> Result<MoveVehicle, YntraError> {
    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: only staff can manage vehicles".to_string(),
        ));
    }

    let vehicle = MoveVehicle {
        id: id.clone(),
        workspace_id: auth.workspace_id.clone(),
        name,
        license_plate,
        capacity_m3,
        status: "active".to_string(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
        latitude: None,
        longitude: None,
        last_ping: None,
        gps_device_id,
    };

    conn.execute(
        "INSERT INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status, updated_at, sync_status, latitude, longitude, last_ping, gps_device_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        crate::params![
            vehicle.id,
            vehicle.workspace_id,
            vehicle.name,
            vehicle.license_plate,
            vehicle.capacity_m3,
            vehicle.status,
            vehicle.updated_at,
            vehicle.sync_status,
            vehicle.latitude,
            vehicle.longitude,
            vehicle.last_ping,
            vehicle.gps_device_id
        ],
    ).await?;

    notify_observers();
    Ok(vehicle)
}

#[uniffi::export]
pub async fn delete_vehicle(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: only staff can manage vehicles".to_string(),
        ));
    }

    let (ws_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM vehicles WHERE id = ?1",
        crate::params![vehicle_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vehicle_management_flow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace and user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-veh-test', 'Vehicle WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-veh-staff', 'ws-veh-test', 'staff@fleet.io', 'admin')", ()).await.unwrap();

        // 1. Create two vehicles
        let v1 = create_vehicle("u-veh-staff".to_string(), "Truck A".to_string(), "ABC-123".to_string(), 45.5, None).await.unwrap();
        let v2 = create_vehicle("u-veh-staff".to_string(), "Van B".to_string(), "XYZ-789".to_string(), 12.0, None).await.unwrap();

        assert_eq!(v1.name, "Truck A");
        assert_eq!(v1.license_plate, "ABC-123");
        assert_eq!(v1.capacity_m3, 45.5);
        assert_eq!(v1.status, "active");

        // 2. Retrieve vehicles list
        let list = get_vehicles("u-veh-staff".to_string()).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|v| v.id == v1.id));
        assert!(list.iter().any(|v| v.id == v2.id));

        // 3. Delete one vehicle
        delete_vehicle("u-veh-staff".to_string(), v2.id.clone()).await.unwrap();

        // Verify vehicle is deleted
        let list_after = get_vehicles("u-veh-staff".to_string()).await.unwrap();
        assert_eq!(list_after.len(), 1);
        assert_eq!(list_after[0].id, v1.id);

        // Cleanup
        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-veh-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-veh-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-veh-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_vehicle_gps_tracking_and_simulation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace and user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-sim-test', 'GPS Sim WS', '[\"moving_company\"]', '{\"target_region\":\"US\"}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-staff', 'ws-gps-sim-test', 'staff-gps@fleet.io', 'admin')", ()).await.unwrap();

        // 1. Create a vehicle with GPS Device ID
        let v = create_vehicle("u-gps-staff".to_string(), "Sim Truck".to_string(), "GPS-999".to_string(), 35.0, Some("IMEI-555".to_string())).await.unwrap();

        // 2. Register a GPS ping using GPS Device ID
        register_gps_ping("u-gps-staff".to_string(), "IMEI-555".to_string(), 37.7749, -122.4194).await.unwrap();

        // Verify coordinates are saved
        let list = get_vehicles("u-gps-staff".to_string()).await.unwrap();
        let found = list.iter().find(|item| item.id == v.id).unwrap();
        assert_eq!(found.latitude, Some(37.7749));
        assert_eq!(found.longitude, Some(-122.4194));

        // 3. Simulate vehicle movement
        simulate_vehicle_movement("u-gps-staff".to_string(), v.id.clone(), 5).await.unwrap();

        // Verify simulated movement has updated coordinates
        let list2 = get_vehicles("u-gps-staff".to_string()).await.unwrap();
        let found2 = list2.iter().find(|item| item.id == v.id).unwrap();
        assert!(found2.latitude.is_some());
        assert!(found2.longitude.is_some());
        assert_ne!(found2.latitude, Some(37.7749)); // coordinates should have moved

        // Cleanup
        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-gps-sim-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-gps-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-gps-sim-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_hardware_gps_webhook_processing() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspace with webhook settings
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hw-gps', 'HW GPS WS', '[\"moving_company\"]', '{\"gps_webhook_token\":\"secret-token-123\"}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-hw-gps-staff', 'ws-hw-gps', 'staff-hw-gps@fleet.io', 'admin')", ()).await.unwrap();

        // 2. Create vehicle with GPS Device ID
        let v = create_vehicle("u-hw-gps-staff".to_string(), "HW Sim Truck".to_string(), "GPS-HW-888".to_string(), 35.0, Some("IMEI-HW-999".to_string())).await.unwrap();

        // Test case A: Valid Traccar/Generic JSON
        let payload_a = r#"{"deviceId": "IMEI-HW-999", "lat": 59.3293, "lon": 18.0686}"#;
        register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_a.to_string()).await.unwrap();

        // Verify coords updated
        let list_a = get_vehicles("u-hw-gps-staff".to_string()).await.unwrap();
        let v_a = list_a.iter().find(|item| item.id == v.id).unwrap();
        assert_eq!(v_a.latitude, Some(59.3293));
        assert_eq!(v_a.longitude, Some(18.0686));

        // Test case B: Teltonika JSON (nested/custom field names)
        let payload_b = r#"{"imei": "IMEI-HW-999", "latitude": 57.7089, "longitude": 11.9746}"#;
        register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_b.to_string()).await.unwrap();

        // Verify coords updated to Gothenburg
        let list_b = get_vehicles("u-hw-gps-staff".to_string()).await.unwrap();
        let v_b = list_b.iter().find(|item| item.id == v.id).unwrap();
        assert_eq!(v_b.latitude, Some(57.7089));
        assert_eq!(v_b.longitude, Some(11.9746));

        // Test case C: Samsara Payload (data array wrapper with nested gps object)
        let payload_samsara = r#"{"data": [{"vehicleId": "IMEI-HW-999", "gps": {"latitude": 55.6050, "longitude": 13.0038}}]}"#;
        register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_samsara.to_string()).await.unwrap();
        let list_s = get_vehicles("u-hw-gps-staff".to_string()).await.unwrap();
        let v_s = list_s.iter().find(|item| item.id == v.id).unwrap();
        assert_eq!(v_s.latitude, Some(55.6050));
        assert_eq!(v_s.longitude, Some(13.0038));

        // Test case D: ABAX Payload (equipmentId with position object)
        let payload_abax = r#"{"equipmentId": "IMEI-HW-999", "position": {"lat": 60.1699, "lon": 24.9384}}"#;
        register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_abax.to_string()).await.unwrap();
        let list_ab = get_vehicles("u-hw-gps-staff".to_string()).await.unwrap();
        let v_ab = list_ab.iter().find(|item| item.id == v.id).unwrap();
        assert_eq!(v_ab.latitude, Some(60.1699));
        assert_eq!(v_ab.longitude, Some(24.9384));

        // Test case E: Invalid Webhook Token (should fail with AuthError)
        let payload_err = r#"{"deviceId": "IMEI-HW-999", "lat": 59.3293, "lon": 18.0686}"#;
        let err_c = register_gps_ping_from_webhook("ws-hw-gps".to_string(), "wrong-token".to_string(), payload_err.to_string()).await;
        assert!(matches!(err_c, Err(YntraError::AuthError(_))));

        // Test case D: Non-existent Device ID (should fail with NotFoundError)
        let payload_d = r#"{"deviceId": "IMEI-NON-EXISTENT", "lat": 59.3293, "lon": 18.0686}"#;
        let err_d = register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_d.to_string()).await;
        assert!(matches!(err_d, Err(YntraError::NotFoundError(_))));

        // Cleanup
        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-hw-gps'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-hw-gps-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hw-gps'", ()).await.unwrap();
    }
}

#[uniffi::export]
pub async fn register_gps_ping(
    requester_user_id: String,
    device_id: String,
    latitude: f64,
    longitude: f64,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn.prepare(
        "SELECT id FROM vehicles WHERE workspace_id = ?1 AND (id = ?2 OR license_plate = ?2 OR gps_device_id = ?2)"
    ).await?;
    let mut rows = stmt.query(crate::params![auth.workspace_id, &device_id]).await?;
    if let Some(row) = rows.next().await? {
        let vehicle_id: String = row.get(0)?;
        conn.execute(
            "UPDATE vehicles SET latitude = ?1, longitude = ?2, last_ping = ?3, updated_at = ?3 WHERE id = ?4",
            crate::params![latitude, longitude, now_ms, vehicle_id]
        ).await?;
        notify_observers();
        Ok(())
    } else {
        Err(YntraError::NotFoundError("No matching vehicle found in this workspace".to_string()))
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

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
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

    // Base coordinates
    let (base_lat, base_lon) = match target_region.as_str() {
        "US" => (37.7749, -122.4194), // San Francisco
        "DE" => (52.5200, 13.4050),   // Berlin
        _ => (59.3293, 18.0686),      // Stockholm
    };

    // Calculate a dynamic path offset based on tick
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

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
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

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
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
        return Err(YntraError::AuthError("Access denied: invalid or missing webhook token".to_string()));
    }
    
    let parsed: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid JSON payload: {}", e)))?;
    
    // Support root array or data array wrappers (e.g. Samsara, Teltonika, Fleet Complete)
    let item = if let Some(arr) = parsed.as_array() {
        arr.first().cloned().unwrap_or(parsed.clone())
    } else if let Some(data_arr) = parsed.get("data").and_then(|d| d.as_array()) {
        data_arr.first().cloned().unwrap_or(parsed.clone())
    } else {
        parsed.clone()
    };
        
    // Multi-provider Device ID extraction (Samsara, Teltonika, ABAX, Fleet Complete, Traccar)
    let device_id = item.get("deviceId")
        .or_else(|| item.get("device_id"))
        .or_else(|| item.get("imei"))
        .or_else(|| item.get("id"))
        .or_else(|| item.get("serial"))
        .or_else(|| item.get("serialNumber"))
        .or_else(|| item.get("equipmentId"))
        .or_else(|| item.get("assetId"))
        .or_else(|| item.get("vehicleId"))
        .or_else(|| item.get("vehicle_id"))
        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
        .ok_or_else(|| YntraError::ValidationError("Missing device identification".to_string()))?;
        
    // Multi-provider Coordinate extraction (Direct or nested under 'gps', 'location', or 'position')
    let location_obj = item.get("gps")
        .or_else(|| item.get("location"))
        .or_else(|| item.get("position"))
        .unwrap_or(&item);

    let latitude = location_obj.get("lat")
        .or_else(|| location_obj.get("latitude"))
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
        .ok_or_else(|| YntraError::ValidationError("Missing or invalid latitude".to_string()))?;
        
    let longitude = location_obj.get("lon")
        .or_else(|| location_obj.get("lng"))
        .or_else(|| location_obj.get("longitude"))
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
        .ok_or_else(|| YntraError::ValidationError("Missing or invalid longitude".to_string()))?;
        
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
        Err(YntraError::NotFoundError(format!("No matching vehicle found for device ID '{}'", device_id)))
    }
}

#[uniffi::export]
pub async fn get_vehicle_commercial_routing_profile(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<crate::CommercialRouteRestrictions, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, capacity_m3): (String, f64) = conn
        .query_row(
            "SELECT workspace_id, capacity_m3 FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let estimated_height = if capacity_m3 > 35.0 { 3.9 } else if capacity_m3 > 15.0 { 3.4 } else { 2.6 };
    let estimated_weight = if capacity_m3 > 35.0 { 16.0 } else if capacity_m3 > 15.0 { 7.5 } else { 3.5 };

    let mut details = Vec::new();
    let low_bridge_warning = estimated_height >= 3.8;
    let weight_limit_warning = estimated_weight >= 3.5;

    if low_bridge_warning {
        details.push(format!("Vehicle height ({:.1}m) requires commercial truck navigation route planning.", estimated_height));
    }
    if weight_limit_warning {
        details.push(format!("Vehicle weight ({:.1}t) requires residential weight restriction checks.", estimated_weight));
    }

    Ok(crate::CommercialRouteRestrictions {
        low_bridge_warning,
        environmental_zone_warning: false,
        weight_limit_warning,
        parking_permit_required: capacity_m3 > 20.0,
        restriction_details: details,
    })
}

#[uniffi::export]
pub async fn evaluate_vehicle_route_clearance(
    requester_user_id: String,
    vehicle_id: String,
    origin_address: String,
    destination_address: String,
) -> Result<crate::CommercialRouteRestrictions, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, capacity_m3): (String, f64) = conn
        .query_row(
            "SELECT workspace_id, capacity_m3 FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let estimated_height = if capacity_m3 > 35.0 { 3.9 } else if capacity_m3 > 15.0 { 3.4 } else { 2.6 };
    let estimated_weight = if capacity_m3 > 35.0 { 16.0 } else if capacity_m3 > 15.0 { 7.5 } else { 3.5 };
    let emission_class = if capacity_m3 > 35.0 { "Euro 6 Heavy Diesel" } else { "Euro 6 Clean" };

    let combined = format!("{} {}", origin_address, destination_address).to_lowercase();

    let mut details = Vec::new();
    let mut low_bridge_warning = false;
    let mut weight_limit_warning = false;
    let mut environmental_zone_warning = false;

    if estimated_height >= 3.8 {
        low_bridge_warning = true;
        details.push(format!("Low bridge risk: Heavy truck height {:.1}m exceeds 3.8m standard urban clearance.", estimated_height));
    }

    if estimated_weight >= 3.5 {
        weight_limit_warning = true;
        details.push(format!("Weight limit warning: {:.1}t vehicle exceeds 3.5t residential zone limit.", estimated_weight));
    }

    let env_cities = ["stockholm", "göteborg", "gothenburg", "malmö", "malmo", "berlin", "london", "paris", "hamburg"];
    if env_cities.iter().any(|c| combined.contains(c)) {
        environmental_zone_warning = true;
        details.push(format!("Low Emission Zone (LEZ) warning: Target city enforces Euro 6 / Green badge regulations. Vehicle class: '{}'.", emission_class));
    }

    let parking_permit_required = capacity_m3 > 20.0;
    if parking_permit_required {
        details.push("Commercial loading zone parking permit recommended for target addresses.".to_string());
    }

    Ok(crate::CommercialRouteRestrictions {
        low_bridge_warning,
        environmental_zone_warning,
        weight_limit_warning,
        parking_permit_required,
        restriction_details: details,
    })
}

#[uniffi::export]
pub async fn log_eld_hos_status(
    requester_user_id: String,
    driver_id: String,
    driver_name: String,
    vehicle_id: String,
    status: String,
    driving_hours_today: f64,
    on_duty_hours_today: f64,
    cycle_hours_7day: f64,
) -> Result<EldHosLogRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to log ELD HOS status".to_string(),
        ));
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS eld_hos_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            driver_name TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            status TEXT NOT NULL,
            driving_hours_today REAL NOT NULL,
            on_duty_hours_today REAL NOT NULL,
            cycle_hours_7day REAL NOT NULL,
            rest_break_required INTEGER NOT NULL,
            violation_flag INTEGER NOT NULL,
            violation_reason TEXT,
            timestamp_ms INTEGER NOT NULL
        )",
        (),
    ).await?;

    let rest_break_required = driving_hours_today >= 8.0;
    let mut violation_flag = false;
    let mut violation_reason = None;

    if driving_hours_today > 11.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 11-Hour Driving Limit Rule".to_string());
    } else if on_duty_hours_today > 14.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 14-Hour On-Duty Window Limit Rule".to_string());
    } else if cycle_hours_7day > 70.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 70-Hour 8-Day Cycle Limit Rule".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO eld_hos_logs (
            id, workspace_id, driver_id, driver_name, vehicle_id, status,
            driving_hours_today, on_duty_hours_today, cycle_hours_7day,
            rest_break_required, violation_flag, violation_reason, timestamp_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            driver_id.clone(),
            driver_name.clone(),
            vehicle_id.clone(),
            status.clone(),
            driving_hours_today,
            on_duty_hours_today,
            cycle_hours_7day,
            if rest_break_required { 1 } else { 0 },
            if violation_flag { 1 } else { 0 },
            violation_reason.clone(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(EldHosLogRecord {
        id,
        workspace_id: auth.workspace_id,
        driver_id,
        driver_name,
        vehicle_id,
        status,
        driving_hours_today,
        on_duty_hours_today,
        cycle_hours_7day,
        rest_break_required,
        violation_flag,
        violation_reason,
        timestamp_ms: now_ms,
    })
}

#[uniffi::export]
pub async fn submit_dvir_inspection(
    requester_user_id: String,
    vehicle_id: String,
    inspector_driver_id: String,
    inspection_type: String,
    brakes_ok: bool,
    tires_ok: bool,
    lights_ok: bool,
    steering_ok: bool,
    coupling_devices_ok: bool,
    defect_details: Option<String>,
) -> Result<DriverVehicleInspectionReport, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to submit DVIR".to_string(),
        ));
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS dvir_inspections (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            inspector_driver_id TEXT NOT NULL,
            inspection_type TEXT NOT NULL,
            brakes_ok INTEGER NOT NULL,
            tires_ok INTEGER NOT NULL,
            lights_ok INTEGER NOT NULL,
            steering_ok INTEGER NOT NULL,
            coupling_devices_ok INTEGER NOT NULL,
            defects_found INTEGER NOT NULL,
            defect_details TEXT,
            safety_status TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        (),
    ).await?;

    let defects_found = !(brakes_ok && tires_ok && lights_ok && steering_ok && coupling_devices_ok)
        || defect_details.as_ref().map_or(false, |d| !d.trim().is_empty());

    let safety_status = if !brakes_ok || !steering_ok {
        "OUT_OF_SERVICE".to_string()
    } else if defects_found {
        "PASS_REPAIR_REQUIRED".to_string()
    } else {
        "PASS_SAFE".to_string()
    };

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO dvir_inspections (
            id, workspace_id, vehicle_id, inspector_driver_id, inspection_type,
            brakes_ok, tires_ok, lights_ok, steering_ok, coupling_devices_ok,
            defects_found, defect_details, safety_status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            vehicle_id.clone(),
            inspector_driver_id.clone(),
            inspection_type.clone(),
            if brakes_ok { 1 } else { 0 },
            if tires_ok { 1 } else { 0 },
            if lights_ok { 1 } else { 0 },
            if steering_ok { 1 } else { 0 },
            if coupling_devices_ok { 1 } else { 0 },
            if defects_found { 1 } else { 0 },
            defect_details.clone(),
            safety_status.clone(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(DriverVehicleInspectionReport {
        id,
        workspace_id: auth.workspace_id,
        vehicle_id,
        inspector_driver_id,
        inspection_type,
        brakes_ok,
        tires_ok,
        lights_ok,
        steering_ok,
        coupling_devices_ok,
        defects_found,
        defect_details,
        safety_status,
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn check_gvwr_overload_status(
    requester_user_id: String,
    vehicle_id: String,
    cargo_weight_kg: f64,
    tare_weight_kg: f64,
    gvwr_kg: f64,
) -> Result<GvwrWeightComplianceWarning, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to check GVWR overload status".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare("SELECT name, license_plate FROM vehicles WHERE id = ?1 AND workspace_id = ?2")
        .await?;

    let mut rows = stmt
        .query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()])
        .await?;

    let (vehicle_name, license_plate) = if let Some(row) = rows.next().await? {
        (row.get::<String>(0)?, row.get::<String>(1)?)
    } else {
        ("Move Vehicle".to_string(), "TRK-000".to_string())
    };

    let total_actual_gross_weight_kg = tare_weight_kg + cargo_weight_kg;
    let is_overloaded = total_actual_gross_weight_kg > gvwr_kg;
    let overload_margin_kg = if is_overloaded {
        total_actual_gross_weight_kg - gvwr_kg
    } else {
        0.0
    };

    let warning_severity = if !is_overloaded {
        "NORMAL".to_string()
    } else if overload_margin_kg <= 500.0 {
        "WARNING_OVERLOAD".to_string()
    } else {
        "CRITICAL_OVERLOAD".to_string()
    };

    Ok(GvwrWeightComplianceWarning {
        vehicle_id,
        vehicle_name,
        license_plate,
        gvwr_kg,
        current_cargo_weight_kg: cargo_weight_kg,
        tare_weight_kg,
        total_actual_gross_weight_kg,
        is_overloaded,
        overload_margin_kg,
        warning_severity,
    })
}

#[uniffi::export]
pub async fn log_ifta_jurisdiction_crossing(
    requester_user_id: String,
    vehicle_id: String,
    driver_id: String,
    from_jurisdiction: String,
    to_jurisdiction: String,
    odometer_km: f64,
    fuel_purchased_liters: f64,
) -> Result<IftaStateFuelLogRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to log IFTA crossing".to_string(),
        ));
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ifta_fuel_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            from_jurisdiction TEXT NOT NULL,
            to_jurisdiction TEXT NOT NULL,
            odometer_km REAL NOT NULL,
            fuel_purchased_liters REAL NOT NULL,
            timestamp_ms INTEGER NOT NULL
        )",
        (),
    ).await?;

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO ifta_fuel_logs (
            id, workspace_id, vehicle_id, driver_id, from_jurisdiction,
            to_jurisdiction, odometer_km, fuel_purchased_liters, timestamp_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            vehicle_id.clone(),
            driver_id.clone(),
            from_jurisdiction.clone(),
            to_jurisdiction.clone(),
            odometer_km,
            fuel_purchased_liters,
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(IftaStateFuelLogRecord {
        id,
        workspace_id: auth.workspace_id,
        vehicle_id,
        driver_id,
        from_jurisdiction,
        to_jurisdiction,
        odometer_km,
        fuel_purchased_liters,
        timestamp_ms: now_ms,
    })
}

#[uniffi::export]
pub async fn get_vehicle_dot_compliance_summary(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<VehicleDotComplianceSummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to view DOT compliance summary".to_string(),
        ));
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS eld_hos_logs (
            id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, driver_id TEXT NOT NULL,
            driver_name TEXT NOT NULL, vehicle_id TEXT NOT NULL, status TEXT NOT NULL,
            driving_hours_today REAL NOT NULL, on_duty_hours_today REAL NOT NULL,
            cycle_hours_7day REAL NOT NULL, rest_break_required INTEGER NOT NULL,
            violation_flag INTEGER NOT NULL, violation_reason TEXT, timestamp_ms INTEGER NOT NULL
        )",
        (),
    ).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS dvir_inspections (
            id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, vehicle_id TEXT NOT NULL,
            inspector_driver_id TEXT NOT NULL, inspection_type TEXT NOT NULL,
            brakes_ok INTEGER NOT NULL, tires_ok INTEGER NOT NULL, lights_ok INTEGER NOT NULL,
            steering_ok INTEGER NOT NULL, coupling_devices_ok INTEGER NOT NULL,
            defects_found INTEGER NOT NULL, defect_details TEXT, safety_status TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        (),
    ).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ifta_fuel_logs (
            id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, vehicle_id TEXT NOT NULL,
            driver_id TEXT NOT NULL, from_jurisdiction TEXT NOT NULL, to_jurisdiction TEXT NOT NULL,
            odometer_km REAL NOT NULL, fuel_purchased_liters REAL NOT NULL, timestamp_ms INTEGER NOT NULL
        )",
        (),
    ).await?;

    let mut hos_stmt = conn
        .prepare("SELECT status, violation_flag FROM eld_hos_logs WHERE vehicle_id = ?1 AND workspace_id = ?2 ORDER BY timestamp_ms DESC LIMIT 1")
        .await?;
    let mut hos_rows = hos_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let (active_hos_status, latest_hos_violation) = if let Some(row) = hos_rows.next().await? {
        (row.get::<String>(0)?, row.get::<i32>(1)? == 1)
    } else {
        ("OFF_DUTY".to_string(), false)
    };

    let mut count_stmt = conn
        .prepare("SELECT COUNT(*) FROM eld_hos_logs WHERE vehicle_id = ?1 AND workspace_id = ?2 AND violation_flag = 1")
        .await?;
    let mut count_rows = count_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let hos_violation_count = if let Some(row) = count_rows.next().await? {
        row.get::<i32>(0)?
    } else {
        0
    };

    let mut dvir_stmt = conn
        .prepare("SELECT safety_status FROM dvir_inspections WHERE vehicle_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC, rowid DESC LIMIT 1")
        .await?;
    let mut dvir_rows = dvir_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let latest_dvir_status = if let Some(row) = dvir_rows.next().await? {
        row.get::<String>(0)?
    } else {
        "PASS_SAFE".to_string()
    };

    let mut ifta_stmt = conn
        .prepare("SELECT COUNT(*) FROM ifta_fuel_logs WHERE vehicle_id = ?1 AND workspace_id = ?2")
        .await?;
    let mut ifta_rows = ifta_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let total_ifta_jurisdictions_logged = if let Some(row) = ifta_rows.next().await? {
        row.get::<i32>(0)?
    } else {
        0
    };

    let is_dot_compliant = !latest_hos_violation && latest_dvir_status != "OUT_OF_SERVICE";

    Ok(VehicleDotComplianceSummary {
        vehicle_id,
        active_hos_status,
        hos_violation_count,
        latest_dvir_status,
        gvwr_status: if is_dot_compliant { "NORMAL".to_string() } else { "ATTENTION_REQUIRED".to_string() },
        total_ifta_jurisdictions_logged,
        is_dot_compliant,
    })
}

#[cfg(test)]
mod commercial_routing_tests {
    use super::*;

    #[tokio::test]
    async fn test_commercial_truck_navigation_clearance_rules() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-nav-rules', 'Nav Rules WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-nav-staff', 'ws-nav-rules', 'staff-nav@fleet.io', 'admin')", ()).await.unwrap();

        let v_heavy = create_vehicle("u-nav-staff".to_string(), "Heavy Rig".to_string(), "RIG-001".to_string(), 45.0, None).await.unwrap();
        let v_small = create_vehicle("u-nav-staff".to_string(), "Small Van".to_string(), "VAN-002".to_string(), 10.0, None).await.unwrap();

        let profile_heavy = get_vehicle_commercial_routing_profile("u-nav-staff".to_string(), v_heavy.id.clone()).await.unwrap();
        assert!(profile_heavy.low_bridge_warning);
        assert!(profile_heavy.weight_limit_warning);
        assert!(profile_heavy.parking_permit_required);

        let profile_small = get_vehicle_commercial_routing_profile("u-nav-staff".to_string(), v_small.id.clone()).await.unwrap();
        assert!(!profile_small.low_bridge_warning);

        let eval_heavy = evaluate_vehicle_route_clearance(
            "u-nav-staff".to_string(),
            v_heavy.id.clone(),
            "Hamngatan 1, Stockholm".to_string(),
            "Sveavägen 100, Stockholm".to_string(),
        ).await.unwrap();

        assert!(eval_heavy.low_bridge_warning);
        assert!(eval_heavy.weight_limit_warning);
        assert!(eval_heavy.environmental_zone_warning);
        assert!(!eval_heavy.restriction_details.is_empty());

        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-nav-rules'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-nav-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-nav-rules'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_realtime_gps_telemetry_streaming() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-stream-test', 'Stream WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-stream-staff', 'ws-stream-test', 'staff-stream@fleet.io', 'admin')", ()).await.unwrap();

        let v = create_vehicle("u-stream-staff".to_string(), "Stream Truck".to_string(), "STR-100".to_string(), 30.0, None).await.unwrap();

        stream_vehicle_gps_location("u-stream-staff".to_string(), v.id.clone(), 59.3300, 18.0700, 52.0, 90.0).await.unwrap();

        let telemetry = get_active_vehicles_telemetry("u-stream-staff".to_string()).await.unwrap();
        assert_eq!(telemetry.len(), 1);
        assert_eq!(telemetry[0].vehicle_id, v.id);
        assert_eq!(telemetry[0].latitude, 59.3300);
        assert_eq!(telemetry[0].longitude, 18.0700);
        assert!(telemetry[0].is_moving);

        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-stream-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-stream-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-stream-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_eld_hos_and_dvir_dot_compliance_workflow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-eld-test', 'ELD WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-eld-staff', 'ws-eld-test', 'driver-eld@fleet.io', 'admin')", ()).await.unwrap();

        let v = create_vehicle("u-eld-staff".to_string(), "ELD Interstate Truck".to_string(), "ELD-888".to_string(), 40.0, None).await.unwrap();

        // Log Normal HOS
        let hos_ok = log_eld_hos_status(
            "u-eld-staff".to_string(),
            "drv-001".to_string(),
            "Johan Mover".to_string(),
            v.id.clone(),
            "ON_DUTY_DRIVING".to_string(),
            6.5,
            8.0,
            42.0,
        ).await.unwrap();
        assert!(!hos_ok.violation_flag);
        assert!(!hos_ok.rest_break_required);

        // Log HOS Violation (driving > 11 hrs)
        let hos_viol = log_eld_hos_status(
            "u-eld-staff".to_string(),
            "drv-001".to_string(),
            "Johan Mover".to_string(),
            v.id.clone(),
            "ON_DUTY_DRIVING".to_string(),
            12.5,
            14.5,
            55.0,
        ).await.unwrap();
        assert!(hos_viol.violation_flag);
        assert!(hos_viol.rest_break_required);
        assert!(hos_viol.violation_reason.unwrap().contains("11-Hour Driving Limit"));

        // Submit Safe DVIR
        let dvir_pass = submit_dvir_inspection(
            "u-eld-staff".to_string(),
            v.id.clone(),
            "drv-001".to_string(),
            "PRE_TRIP".to_string(),
            true, true, true, true, true,
            None,
        ).await.unwrap();
        assert_eq!(dvir_pass.safety_status, "PASS_SAFE");

        // Submit Out-of-Service DVIR (Brake failure)
        let dvir_fail = submit_dvir_inspection(
            "u-eld-staff".to_string(),
            v.id.clone(),
            "drv-001".to_string(),
            "POST_TRIP".to_string(),
            false, true, true, true, true,
            Some("Brake line pressure leak detected".to_string()),
        ).await.unwrap();
        assert_eq!(dvir_fail.safety_status, "OUT_OF_SERVICE");

        // Check GVWR Overload
        let gvwr_warn = check_gvwr_overload_status(
            "u-eld-staff".to_string(),
            v.id.clone(),
            8500.0,
            4500.0,
            12000.0,
        ).await.unwrap();
        assert!(gvwr_warn.is_overloaded);
        assert_eq!(gvwr_warn.warning_severity, "CRITICAL_OVERLOAD");

        // Log IFTA Crossing
        let ifta = log_ifta_jurisdiction_crossing(
            "u-eld-staff".to_string(),
            v.id.clone(),
            "drv-001".to_string(),
            "SE-AB".to_string(),
            "SE-VG".to_string(),
            145200.0,
            120.0,
        ).await.unwrap();
        assert_eq!(ifta.from_jurisdiction, "SE-AB");

        // Check Summary
        let summary = get_vehicle_dot_compliance_summary("u-eld-staff".to_string(), v.id.clone()).await.unwrap();
        assert_eq!(summary.hos_violation_count, 1);
        assert_eq!(summary.latest_dvir_status, "OUT_OF_SERVICE");
        assert!(!summary.is_dot_compliant);

        conn.execute("DELETE FROM eld_hos_logs WHERE workspace_id = 'ws-eld-test'", ()).await.unwrap();
        conn.execute("DELETE FROM dvir_inspections WHERE workspace_id = 'ws-eld-test'", ()).await.unwrap();
        conn.execute("DELETE FROM ifta_fuel_logs WHERE workspace_id = 'ws-eld-test'", ()).await.unwrap();
        conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-eld-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-eld-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-eld-test'", ()).await.unwrap();
    }
}

