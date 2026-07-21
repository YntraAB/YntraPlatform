use crate::database;
use crate::services::jobs::{create_job_ticket, assign_vehicle_to_job, add_crew_member, remove_crew_member, get_job_crew};

#[tokio::test]
async fn test_multi_mover_crew_assignment() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    crate::infra::crypto::set_session_key("test-session-key-for-crew-tests".to_string().into_bytes(), "ws-crew-test".to_string());
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
    add_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff2".to_string(), "driver".to_string()).await.unwrap();
    add_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff3".to_string(), "helper".to_string()).await.unwrap();

    // 2. Fetch crew members
    let crew = get_job_crew("u-crew-staff1".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(crew.len(), 2);
    assert!(crew.iter().any(|u| u.id == "u-crew-staff2"));
    assert!(crew.iter().any(|u| u.id == "u-crew-staff3"));

    // 3. Remove a crew member
    remove_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff2".to_string()).await.unwrap();

    // Verify updated crew list
    let crew_after = get_job_crew("u-crew-staff1".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(crew_after.len(), 1);
    assert_eq!(crew_after[0].id, "u-crew-staff3");

    // Cleanup
    conn.execute("DELETE FROM job_crew WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-crew-test'", ()).await.unwrap();
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
        Some("v-cap-1".to_string())
    ).await;
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
        Some("v-cap-1".to_string())
    ).await;
    assert!(assign_res_fail.is_err());
    let err_msg = assign_res_fail.unwrap_err().to_string();
    assert!(err_msg.contains("exceeds vehicle capacity"));

    // 6. Delete heavy item and insert small item (3.0 m3)
    conn.execute("DELETE FROM move_inventory WHERE id = 'inv-cap-1'", ()).await.unwrap();
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-cap-2', 'ws-cap-test', ?1, 'Möbler', 'Small Table', 1, 3.0)",
        crate::params![&job.id]
    ).await.unwrap();

    // 7. Verify assignment succeeds with smaller volume
    let assign_res2 = assign_vehicle_to_job(
        "u-cap-staff".to_string(),
        job.id.clone(),
        Some("v-cap-1".to_string())
    ).await;
    assert!(assign_res2.is_ok());

    // 8. Clean up
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-cap-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-cap-test'", ()).await.unwrap();
    conn.execute("DELETE FROM vehicles WHERE workspace_id = 'ws-cap-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-cap-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-cap-test'", ()).await.unwrap();
}
