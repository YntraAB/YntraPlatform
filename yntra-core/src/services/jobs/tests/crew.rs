use crate::database;
use crate::services::jobs::{
    add_crew_member, assign_vehicle_to_job, create_job_ticket, get_job_crew, remove_crew_member,
};

#[tokio::test]
async fn test_multi_mover_crew_assignment() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    crate::infra::crypto::set_session_key(
        "test-session-key-for-crew-tests".to_string().into_bytes(),
        "ws-crew-test".to_string(),
    );
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-crew-test', 'Crew Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff1', 'ws-crew-test', 'lead@crew.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff2', 'ws-crew-test', 'mover1@crew.io', 'mover')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff3', 'ws-crew-test', 'mover2@crew.io', 'mover')", ()).await.unwrap();

    // Create job ticket
    let job = create_job_ticket(
        "u-crew-staff1".to_string(),
        "ws-crew-test".to_string(),
        "Relocate Piano".to_string(),
        "Heavy lift move".to_string(),
        "Piano St 1".to_string(),
        "high".to_string(),
        None,
        "2026-08-12".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // 1. Assign crew members
    add_crew_member(
        "u-crew-staff1".to_string(),
        job.id.clone(),
        "u-crew-staff2".to_string(),
        "driver".to_string(),
    )
    .await
    .unwrap();
    add_crew_member(
        "u-crew-staff1".to_string(),
        job.id.clone(),
        "u-crew-staff3".to_string(),
        "helper".to_string(),
    )
    .await
    .unwrap();

    // 2. Fetch crew members
    let crew = get_job_crew("u-crew-staff1".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(crew.len(), 2);
    assert!(crew.iter().any(|u| u.id == "u-crew-staff2"));
    assert!(crew.iter().any(|u| u.id == "u-crew-staff3"));

    // 3. Remove a crew member
    remove_crew_member(
        "u-crew-staff1".to_string(),
        job.id.clone(),
        "u-crew-staff2".to_string(),
    )
    .await
    .unwrap();

    // Verify updated crew list
    let crew_after = get_job_crew("u-crew-staff1".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(crew_after.len(), 1);
    assert_eq!(crew_after[0].id, "u-crew-staff3");

    // Cleanup
    conn.execute(
        "DELETE FROM job_crew WHERE job_ticket_id = ?1",
        crate::params![&job.id],
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-crew-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-crew-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_vehicle_capacity_validation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-cap-test', 'Cap Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-cap-staff', 'ws-cap-test', 'staff@cap.io', 'admin')", ()).await.unwrap();

    // 2. Register a vehicle with 5.0 m3 capacity
    conn.execute(
        "INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status) VALUES ('v-cap-1', 'ws-cap-test', 'Small Truck', 'MIN-123', 5.0, 'active')",
        ()
    ).await.unwrap();

    // 3. Create a move job ticket
    let job = create_job_ticket(
        "u-cap-staff".to_string(),
        "ws-cap-test".to_string(),
        "Capacity Test Move".to_string(),
        "Move items".to_string(),
        "Origin address".to_string(),
        "medium".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // 4. Add inventory exceeding capacity (6.0 m3)
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-cap-1', 'ws-cap-test', ?1, 'Möbler', 'Huge Sofa', 1, 6.0)",
        crate::params![&job.id]
    ).await.unwrap();

    // 5. Verify assignment succeeds by default (enforce_single_trip_capacity = false)
    let assign_res_default = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-cap-1".to_string()),
    )
    .await;
    assert!(assign_res_default.is_ok());

    // 5b. Update workspace settings to enforce single trip capacity
    conn.execute(
        "UPDATE workspaces SET settings = '{\"enforce_single_trip_capacity\":true}' WHERE id = 'ws-cap-test'",
        ()
    ).await.unwrap();

    // 5c. Verify assignment fails when enforce_single_trip_capacity is true
    let assign_res_fail = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-cap-1".to_string()),
    )
    .await;
    assert!(assign_res_fail.is_err());
    let err_msg = assign_res_fail.unwrap_err().to_string();
    assert!(err_msg.contains("exceeds vehicle capacity"));

    // 6. Test spatial packing buffer edge case: 30 m3 cargo assigned to 32 m3 vehicle
    // 30 m3 * 1.2 buffer = 36 m3 required volume > 32 m3 vehicle capacity
    conn.execute(
        "INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status) VALUES ('v-cap-32', 'ws-cap-test', 'Mid Truck', 'MID-320', 32.0, 'active')",
        ()
    ).await.unwrap();
    conn.execute(
        "DELETE FROM move_inventory WHERE job_ticket_id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-cap-30', 'ws-cap-test', ?1, 'Möbler', '30m3 Cargo', 1, 30.0)",
        crate::params![&job.id]
    ).await.unwrap();

    let assign_res_buffer_fail = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-cap-32".to_string()),
    )
    .await;
    assert!(assign_res_buffer_fail.is_err());
    let err_buf_msg = assign_res_buffer_fail.unwrap_err().to_string();
    assert!(err_buf_msg.contains("20% packing buffer"));

    // 7. Test max_payload_kg enforcement: 1200 kg cargo assigned to 1000 kg payload vehicle
    conn.execute(
        "INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, max_payload_kg, status) VALUES ('v-payload-1000', 'ws-cap-test', 'Light Van', 'PAY-100', 20.0, 1000.0, 'active')",
        ()
    ).await.unwrap();
    conn.execute(
        "DELETE FROM move_inventory WHERE job_ticket_id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, estimated_weight_kg) VALUES ('inv-heavy-weight', 'ws-cap-test', ?1, 'Möbler', 'Heavy Safe', 1, 2.0, 1200.0)",
        crate::params![&job.id]
    ).await.unwrap();

    let assign_res_payload_fail = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-payload-1000".to_string()),
    )
    .await;
    assert!(assign_res_payload_fail.is_err());
    let err_pay_msg = assign_res_payload_fail.unwrap_err().to_string();
    assert!(err_pay_msg.contains("exceeds vehicle max payload limit"));

    // 8. Delete heavy item and insert small item (3.0 m3, 50 kg)
    conn.execute(
        "DELETE FROM move_inventory WHERE id = 'inv-heavy-weight'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, estimated_weight_kg) VALUES ('inv-cap-2', 'ws-cap-test', ?1, 'Möbler', 'Small Table', 1, 3.0, 50.0)",
        crate::params![&job.id]
    ).await.unwrap();

    // 9. Verify assignment succeeds with smaller volume and weight
    let assign_res2 = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-cap-1".to_string()),
    )
    .await;
    assert!(assign_res2.is_ok());

    // 8. Clean up
    conn.execute(
        "DELETE FROM move_inventory WHERE workspace_id = 'ws-cap-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-cap-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM vehicles WHERE workspace_id = 'ws-cap-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-cap-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-cap-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_driver_license_and_tachograph_compliance() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    use crate::services::jobs::validate_driver_tachograph_compliance;

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-comp-test', 'Comp Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-comp-admin', 'ws-comp-test', 'admin@comp.io', 'admin')", ()).await.unwrap();

    // User A: Category B license only
    let meta_b = serde_json::json!({ "driver_license_class": "B" }).to_string();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-driver-b', 'ws-comp-test', 'driverB@comp.io', 'mover', ?1)", crate::params![&meta_b]).await.unwrap();

    // Heavy Truck: 25.0 m3 capacity (requires C/CE)
    conn.execute(
        "INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status) VALUES ('v-heavy-1', 'ws-comp-test', 'Heavy Truck C', 'HEV-999', 25.0, 'active')",
        ()
    ).await.unwrap();

    let job = create_job_ticket(
        "u-comp-admin".to_string(),
        "ws-comp-test".to_string(),
        "Heavy Haul Move".to_string(),
        "Relocating entire mansion".to_string(),
        "Origin St 1".to_string(),
        "high".to_string(),
        None,
        "2026-08-30".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    assign_vehicle_to_job(
        "u-comp-admin".to_string(),
        job.id.clone(),
        Some("v-heavy-1".to_string()),
    )
    .await
    .unwrap();

    // 1. Attempt to add Category B driver to Heavy Truck job (should fail validation)
    let add_res = add_crew_member(
        "u-comp-admin".to_string(),
        job.id.clone(),
        "u-driver-b".to_string(),
        "driver".to_string(),
    )
    .await;
    assert!(add_res.is_err());
    let err_str = add_res.unwrap_err().to_string();
    assert!(err_str.contains("Driver license violation"));

    // 2. Direct compliance check
    let compliance = validate_driver_tachograph_compliance(
        "u-comp-admin".to_string(),
        "u-driver-b".to_string(),
        job.id.clone(),
    )
    .await
    .unwrap();
    assert!(!compliance.is_compliant);
    assert_eq!(compliance.license_class, "B");
    assert_eq!(compliance.required_license_class, "C/CE");

    // Cleanup
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-comp-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM vehicles WHERE workspace_id = 'ws-comp-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-comp-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-comp-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_over_capacity_vehicle_dispatch_rejection() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let settings = serde_json::json!({
        "enforce_single_trip_capacity": true
    })
    .to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-disp-cap-test', 'Disp Cap WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-disp-admin', 'ws-disp-cap-test', 'admin@dispcap.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, max_payload_kg, status) VALUES ('v-small-van', 'ws-disp-cap-test', 'City Van 10m3', 'VAN-111', 10.0, 800.0, 'active')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-disp-admin".to_string(),
        "ws-disp-cap-test".to_string(),
        "Overload Move".to_string(),
        "Large move".to_string(),
        "Main St 1".to_string(),
        "medium".to_string(),
        Some("u-disp-admin".to_string()),
        "2026-09-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // Add 15m3 cargo (> 10m3 capacity with 20% buffer = 18m3 required)
    crate::services::jobs::moves::create_move_inventory_item(
        "u-disp-admin".to_string(),
        job.id.clone(),
        "Möbler".to_string(),
        "Big Wardrobe".to_string(),
        3,
        5.0,
        None,
    )
    .await
    .unwrap();

    // 1. Vehicle assignment to over-capacity job ticket should fail
    let assign_res = assign_vehicle_to_job(
        "u-disp-admin".to_string(),
        job.id.clone(),
        Some("v-small-van".to_string()),
    )
    .await;
    assert!(assign_res.is_err());
    assert!(
        assign_res
            .unwrap_err()
            .to_string()
            .contains("exceeds vehicle capacity")
    );

    // Cleanup
    conn.execute(
        "DELETE FROM move_invoices WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM move_quotes WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM move_inventory WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM vehicles WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = 'ws-disp-cap-test'",
        (),
    )
    .await
    .ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-disp-cap-test'", ())
        .await
        .ok();
}
