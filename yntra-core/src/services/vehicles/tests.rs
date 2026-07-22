use super::compliance::*;
use super::routing::*;
use super::telemetry::*;
use super::*;
use crate::database;

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

    // Test case F: Non-existent Device ID (should fail with NotFoundError)
    let payload_d = r#"{"deviceId": "IMEI-NON-EXISTENT", "lat": 59.3293, "lon": 18.0686}"#;
    let err_d = register_gps_ping_from_webhook("ws-hw-gps".to_string(), "secret-token-123".to_string(), payload_d.to_string()).await;
    assert!(matches!(err_d, Err(YntraError::NotFoundError(_))));

    // Cleanup
    conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-hw-gps'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-hw-gps-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-hw-gps'", ()).await.unwrap();
}

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

    // Query DVIR Reports list
    let dvir_list = get_vehicle_dvir_reports("u-eld-staff".to_string(), v.id.clone()).await.unwrap();
    assert_eq!(dvir_list.len(), 2);
    assert_eq!(dvir_list[0].safety_status, "OUT_OF_SERVICE");
    assert_eq!(dvir_list[1].safety_status, "PASS_SAFE");

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
