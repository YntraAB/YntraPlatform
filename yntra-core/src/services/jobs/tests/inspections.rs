use crate::database;
use crate::services::jobs::{
    acknowledge_damage_inspection_by_client, create_job_ticket, delete_damage_inspection,
    get_job_damage_inspections, record_damage_inspection,
};

#[tokio::test]
async fn test_damage_inspection_workflow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup test workspace and user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dmg-test', 'Damage Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dmg-crew', 'ws-dmg-test', 'crew@moving.io', 'admin')", ()).await.unwrap();

    // 2. Create job ticket
    let job = create_job_ticket(
        "u-dmg-crew".to_string(),
        "ws-dmg-test".to_string(),
        "Damage Inspection Move".to_string(),
        "Relocating antique sofa".to_string(),
        "Sveavägen 50".to_string(),
        "high".to_string(),
        None,
        "2026-08-25".to_string(),
        "[]".to_string(),
        Some("Kungsgatan 1".to_string()),
        Some("Sveavägen 50".to_string()),
        0, 0, false, false, false, false,
    ).await.unwrap();

    // 3. Record pre-existing damage prior to loading
    let inspection = record_damage_inspection(
        "u-dmg-crew".to_string(),
        job.id.clone(),
        None,
        "Antique Oak Dining Table".to_string(),
        "scratch".to_string(),
        "severe".to_string(),
        Some("Deep 15cm scratch on top surface near left corner prior to loading.".to_string()),
        Some("https://storage.yntra.se/inspections/table_scratch_001.jpg".to_string()),
    ).await.unwrap();

    assert_eq!(inspection.item_name, "Antique Oak Dining Table");
    assert_eq!(inspection.damage_type, "scratch");
    assert_eq!(inspection.severity, "severe");
    assert!(!inspection.client_acknowledged);

    // 4. Fetch job inspections list
    let list = get_job_damage_inspections("u-dmg-crew".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, inspection.id);

    // 5. Client sign-off acknowledgement
    acknowledge_damage_inspection_by_client(
        "u-dmg-crew".to_string(),
        inspection.id.clone(),
        Some("<svg>signature_data</svg>".to_string()),
    ).await.unwrap();

    let list_after_ack = get_job_damage_inspections("u-dmg-crew".to_string(), job.id.clone()).await.unwrap();
    assert!(list_after_ack[0].client_acknowledged);
    assert_eq!(list_after_ack[0].client_signature_svg.as_deref(), Some("<svg>signature_data</svg>"));

    // 6. Delete inspection record
    delete_damage_inspection("u-dmg-crew".to_string(), inspection.id.clone()).await.unwrap();

    let list_empty = get_job_damage_inspections("u-dmg-crew".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(list_empty.len(), 0);

    // Cleanup
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-dmg-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dmg-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-dmg-test'", ()).await.unwrap();
}
