use crate::database;
use crate::infra::observer::notify_observers;
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
}

