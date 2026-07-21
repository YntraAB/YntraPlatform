use crate::database;
use crate::services::jobs::{
    create_job_ticket, submit_damaged_item_claim, update_claim_status,
    process_claim_payout, get_job_claims,
};

#[tokio::test]
async fn test_damaged_item_claims_workflow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-claims-test', 'Claims WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-claims-staff', 'ws-claims-test', 'staff@claims.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-claims-client', 'ws-claims-test', 'client@claims.io', 'client')", ()).await.unwrap();

    // 2. Create job ticket
    let job = create_job_ticket(
        "u-claims-staff".to_string(),
        "ws-claims-test".to_string(),
        "Claims Test Move".to_string(),
        "Moving fragile glassware and dining table".to_string(),
        "Original Address".to_string(),
        "high".to_string(),
        Some("u-claims-client".to_string()),
        "2026-10-25".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    // 3. Customer submits damaged item claim with photo evidence
    let photos_json = r#"["https://media.yntra.se/claims/scratch_table_1.webp", "https://media.yntra.se/claims/scratch_table_2.webp"]"#;
    let claim = submit_damaged_item_claim(
        "u-claims-client".to_string(),
        job.id.clone(),
        "Mahogany Dining Table".to_string(),
        "Deep scratch across table surface occurred during transit".to_string(),
        3500.0,
        photos_json.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(claim.item_name, "Mahogany Dining Table");
    assert_eq!(claim.claimed_amount, 3500.0);
    assert_eq!(claim.status, "submitted");

    // 4. Retrieve job claims list
    let claims = get_job_claims("u-claims-client".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].id, claim.id);

    // 5. Staff coordinator reviews claim & enters repair quote estimate
    let reviewed_claim = update_claim_status(
        "u-claims-staff".to_string(),
        claim.id.clone(),
        "under_review".to_string(),
        Some(3000.0),
        Some(2800.0),
        Some("TRYGG-HANSA-POL-9988".to_string()),
        Some("Repair quote received from WoodRestoration AB: 2,800 SEK.".to_string()),
    )
    .await
    .unwrap();

    assert_eq!(reviewed_claim.status, "under_review");
    assert_eq!(reviewed_claim.approved_amount, Some(3000.0));
    assert_eq!(reviewed_claim.repair_quote_amount, Some(2800.0));

    // 6. Process insurance claim payout
    let payout_res = process_claim_payout(
        "u-claims-staff".to_string(),
        claim.id.clone(),
        2800.0,
        "TRYGG-HANSA-POL-9988".to_string(),
    )
    .await
    .unwrap();

    assert!(payout_res.success);
    assert_eq!(payout_res.payout_amount, 2800.0);
    assert_eq!(payout_res.new_status, "paid");

    let final_claims = get_job_claims("u-claims-staff".to_string(), job.id.clone()).await.unwrap();
    assert_eq!(final_claims[0].status, "paid");

    // Cleanup
    conn.execute("DELETE FROM damaged_item_claims WHERE workspace_id = 'ws-claims-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-claims-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-claims-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-claims-test'", ()).await.unwrap();
}
