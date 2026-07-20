use crate::database;
use super::tickets::*;
use super::crew::*;
use super::signatures::*;
use super::moves::*;
use super::billing::*;
use super::routing::*;
use crate::{JobTicket, MoveInventoryItem, MoveQuote, MoveSignature, WorkspaceUser, YntraError};

#[tokio::test]
async fn test_job_tickets_workspace_scoping() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Workspaces and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-job-1', 'Job WS 1', '[]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-job-2', 'Job WS 2', '[]', '{}')", ()).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-job-user1', 'ws-job-1', 'u1@job.io', 'user')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-job-user2', 'ws-job-2', 'u2@job.io', 'user')", ()).await.unwrap();

    // Create a job in ws-job-1
    let job1 = create_job_ticket(
        "u-job-user1".to_string(),
        "ws-job-1".to_string(),
        "Move office chair".to_string(),
        "Heavy chair".to_string(),
        "123 Main St".to_string(),
        "high".to_string(),
        Some("u-job-user1".to_string()),
        "2026-07-05".to_string(),
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

    // Retrieve tickets as user 1 (should see job1)
    let list1 = get_job_tickets("u-job-user1".to_string()).await.unwrap();
    assert_eq!(list1.len(), 1);
    assert_eq!(list1[0].id, job1.id);

    // Retrieve tickets as user 1 with rkyv
    let bytes = get_job_tickets_rkyv("u-job-user1".to_string())
        .await
        .unwrap();
    let rkyv_list: Vec<JobTicket> =
        rkyv::from_bytes::<Vec<JobTicket>, rkyv::rancor::Error>(&bytes).unwrap();
    assert_eq!(rkyv_list.len(), 1);
    assert_eq!(rkyv_list[0].id, job1.id);

    // Retrieve tickets as user 2 (should see 0, since ws-job-2 has no jobs)
    let list2 = get_job_tickets("u-job-user2".to_string()).await.unwrap();
    assert_eq!(list2.len(), 0);

    // Cleanup
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id IN ('ws-job-1', 'ws-job-2')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM users WHERE workspace_id IN ('ws-job-1', 'ws-job-2')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM workspaces WHERE id IN ('ws-job-1', 'ws-job-2')",
        (),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_move_operations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, staff user (role = 'admin')
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-move-test', 'Move Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-move-staff', 'ws-move-test', 'staff@move.io', 'admin')", ()).await.unwrap();

    // Create a job ticket: 3rd floor, no elevator at origin
    let job = create_job_ticket(
        "u-move-staff".to_string(),
        "ws-move-test".to_string(),
        "Move Sofa and Boxes".to_string(),
        "Client moving".to_string(),
        "Origin St 5".to_string(),
        "medium".to_string(),
        None,
        "2026-08-10".to_string(),
        "[]".to_string(),
        Some("Origin St 5".to_string()),
        Some("Dest St 10".to_string()),
        3,      // origin floor
        1,      // destination floor
        false,  // origin elevator
        true,   // destination elevator
        false,
        false,
    )
    .await
    .unwrap();

    // 1. Add Sofa (Furniture) - Qty 1, Vol 1.5
    create_move_inventory_item(
        "u-move-staff".to_string(),
        job.id.clone(),
        "Furniture".to_string(),
        "Sofa".to_string(),
        1,
        1.5,
        Some("Leather sofa".to_string()),
    )
    .await
    .unwrap();

    // 2. Add Books (Boxes) - Qty 5, Vol 0.1
    create_move_inventory_item(
        "u-move-staff".to_string(),
        job.id.clone(),
        "Boxes".to_string(),
        "Books".to_string(),
        5,
        0.1,
        None,
    )
    .await
    .unwrap();

    // Verify inventory
    let inv = get_move_inventory("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(inv.len(), 2);
    
    let sofa_item = inv.iter().find(|i| i.item_name == "Sofa").unwrap();
    let books_item = inv.iter().find(|i| i.item_name == "Books").unwrap();
    assert_eq!(sofa_item.quantity, 1);
    assert_eq!(books_item.quantity, 5);

    // 3. Calculate Quote
    calculate_and_save_move_quote("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let quote_opt = get_move_quote("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(quote_opt.is_some());
    let q = quote_opt.unwrap();
    
    // Calculations verification:
    // Volume = 1.5 * 1 + 0.1 * 5 = 2.0 m3
    // Base Price = 2.0 * 500 = 1000 SEK
    // Distance Fee = 800 SEK
    // Stairs Surcharge = 3 floors * 300 SEK (since origin has no elevator, dest has elevator so 0 surcharge) = 900 SEK
    // Packing supplies fee = 2.0 * 100 = 200 SEK
    // Total = 1000 + 800 + 900 + 200 = 2900 SEK
    assert_eq!(q.base_price, 1000);
    assert_eq!(q.distance_fee, 800);
    assert_eq!(q.stairs_surcharge, 900);
    assert_eq!(q.packing_supplies_fee, 200);
    assert_eq!(q.total_price, 2900);

    // 4. Delete the Books item
    delete_move_inventory_item("u-move-staff".to_string(), books_item.id.clone())
        .await
        .unwrap();

    // Verify inventory count decreased
    let inv_after = get_move_inventory("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(inv_after.len(), 1);
    assert_eq!(inv_after[0].item_name, "Sofa");

    // 5. Recalculate Quote (Volume drops to 1.5 m3)
    // Base Price = 1.5 * 500 = 750 SEK
    // Distance Fee = 800 SEK
    // Stairs Surcharge = 900 SEK
    // Packing supplies = 1.5 * 100 = 150 SEK
    // Total = 750 + 800 + 900 + 150 = 2600 SEK
    calculate_and_save_move_quote("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let q_updated = get_move_quote("u-move-staff".to_string(), job.id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(q_updated.base_price, 750);
    assert_eq!(q_updated.total_price, 2600);

    // Cleanup
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-move-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-move-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_scheduling_and_sync() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, staff user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test', 'Sync Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sync-staff', 'ws-sync-test', 'staff@sync.io', 'admin')", ()).await.unwrap();

    // Create job ticket
    let job = create_job_ticket(
        "u-sync-staff".to_string(),
        "ws-sync-test".to_string(),
        "Office Relocation".to_string(),
        "Large office relocation".to_string(),
        "Main St 1".to_string(),
        "high".to_string(),
        None,
        "".to_string(), // Unscheduled
        "[]".to_string(),
        None,
        None,
        0,
        0,
        true,
        true,
        false,
        false,
    )
    .await
    .unwrap();

    // Verify initial state
    assert_eq!(job.scheduled_date, "");
    assert_eq!(job.status, "pending");

    // 1. Schedule the job ticket
    schedule_job_ticket(
        "u-sync-staff".to_string(),
        job.id.clone(),
        "2026-08-15".to_string(),
        Some("u-sync-staff".to_string()),
    )
    .await
    .unwrap();

    // Verify job ticket updated
    let updated_job: JobTicket = conn
        .query_row(
            "SELECT scheduled_date, status, assigned_user_id FROM job_tickets WHERE id = ?1",
            crate::params![&job.id],
            |r| Ok(JobTicket {
                id: job.id.clone(),
                workspace_id: "ws-sync-test".to_string(),
                title: "".to_string(),
                description: "".to_string(),
                location_address: "".to_string(),
                priority: "".to_string(),
                status: r.get(1)?,
                assigned_user_id: r.get(2)?,
                scheduled_date: r.get(0)?,
                checklist_json: "[]".to_string(),
                completion_report: None,
                created_at: "".to_string(),
                updated_at: 0,
                sync_status: "pending".to_string(),
                origin_address: None,
                destination_address: None,
                origin_floor: 0,
                destination_floor: 0,
                origin_has_elevator: false,
                destination_has_elevator: false,
                origin_parking_permit_needed: false,
                destination_parking_permit_needed: false,
                assigned_vehicle_id: None,
            }),
        )
        .await
        .unwrap();
    
    assert_eq!(updated_job.scheduled_date, "2026-08-15");
    assert_eq!(updated_job.status, "assigned");
    assert_eq!(updated_job.assigned_user_id, Some("u-sync-staff".to_string()));

    // Verify calendar event created
    let (event_id, event_start, event_assignee): (String, String, Option<String>) = conn
        .query_row(
            "SELECT id, start_time, assignee_id FROM events WHERE metadata LIKE ?1",
            crate::params![format!("%\"job_ticket_id\":\"{}\"%", job.id)],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .unwrap();

    assert_eq!(event_start, "2026-08-15 09:00");
    assert_eq!(event_assignee, Some("u-sync-staff".to_string()));

    // 2. Drag & Drop update (calls update_event_time)
    crate::services::teams::update_event_time(
        "u-sync-staff".to_string(),
        event_id.clone(),
        "2026-08-20 10:00".to_string(),
        "2026-08-20 18:00".to_string(),
    )
    .await
    .unwrap();

    // Verify job ticket updated to new date YYYY-MM-DD
    let res_date: String = conn
        .query_row(
            "SELECT scheduled_date FROM job_tickets WHERE id = ?1",
            crate::params![&job.id],
            |r| r.get(0),
        )
        .await
        .unwrap();
    assert_eq!(res_date, "2026-08-20");

    // 3. Delete event
    crate::services::teams::delete_event("u-sync-staff".to_string(), event_id.clone())
        .await
        .unwrap();

    // Verify job ticket reset to unscheduled
    let (res_date_2, res_status, res_assignee): (String, String, Option<String>) = conn
        .query_row(
            "SELECT scheduled_date, status, assigned_user_id FROM job_tickets WHERE id = ?1",
            crate::params![&job.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .unwrap();

    assert_eq!(res_date_2, "");
    assert_eq!(res_status, "pending");
    assert_eq!(res_assignee, None);

    // Cleanup
    conn.execute("DELETE FROM events WHERE id = ?1", crate::params![&event_id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-sync-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-sync-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_invoice_and_rut_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, staff/client user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-inv-test', 'Invoice Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-inv-staff', 'ws-inv-test', 'staff@inv.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-inv-client', 'ws-inv-test', 'client@inv.io', 'client')", ()).await.unwrap();

    // Create job ticket
    let job = create_job_ticket(
        "u-inv-staff".to_string(),
        "ws-inv-test".to_string(),
        "RUT Relocation".to_string(),
        "Move with tax deductions".to_string(),
        "Main St 1".to_string(),
        "medium".to_string(),
        Some("u-inv-client".to_string()),
        "2026-09-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        true,
        true,
        false,
        false,
    )
    .await
    .unwrap();

    // Setup a mock quote
    let quote_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-inv-test', ?2, 2000.0, 800.0, 600.0, 400.0, 3800.0, 'sent', 123456)",
        crate::params![&quote_id, &job.id],
    ).await.unwrap();

    // 1. Generate invoice with RUT deduction enabled
    let invoice = generate_move_invoice(
        "u-inv-staff".to_string(),
        quote_id.clone(),
        true,
    )
    .await
    .unwrap();

    assert_eq!(invoice.subtotal, 3800.0);
    assert_eq!(invoice.rut_deduction, 1300.0);
    assert_eq!(invoice.customer_amount, 2500.0);
    assert_eq!(invoice.tax_authority_amount, 1300.0);
    assert_eq!(invoice.status, "unpaid");

    // 2. Fetch the invoice
    let fetched_invoice = get_move_invoice(
        "u-inv-staff".to_string(),
        quote_id.clone(),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(fetched_invoice.id, invoice.id);
    assert_eq!(fetched_invoice.rut_deduction, 1300.0);
    assert_eq!(fetched_invoice.status, "unpaid");

    // 3. Pay the invoice
    pay_move_invoice(
        "u-inv-staff".to_string(),
        invoice.id.clone(),
    )
    .await
    .unwrap();

    // Verify status updated in database
    let status_res: String = conn.query_row(
        "SELECT status FROM move_invoices WHERE id = ?1",
        crate::params![&invoice.id],
        |r| r.get(0),
    ).await.unwrap();
    assert_eq!(status_res, "paid");

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE quote_id = ?1", crate::params![&quote_id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE id = ?1", crate::params![&quote_id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-inv-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-inv-client'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-inv-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_configurable_pricing_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace with custom pricing settings
    let settings = r#"{"moving_base_rate_per_m3":600.0,"moving_distance_fee_flat":1000.0,"moving_stairs_surcharge_per_floor":400.0,"moving_packing_supplies_fee_per_m3":150.0}"#;
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-price-test', 'Price Test WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-price-staff', 'ws-price-test', 'staff@price.io', 'admin')", ()).await.unwrap();

    // Create job ticket: 2nd floor, no elevator at origin
    let job = create_job_ticket(
        "u-price-staff".to_string(),
        "ws-price-test".to_string(),
        "Move Desk".to_string(),
        "Custom pricing check".to_string(),
        "Origin St 50".to_string(),
        "medium".to_string(),
        None,
        "2026-08-15".to_string(),
        "[]".to_string(),
        Some("Origin St 50".to_string()),
        Some("Dest St 100".to_string()),
        2,      // origin floor
        0,      // destination floor
        false,  // origin elevator
        true,   // destination elevator
        false,
        false,
    )
    .await
    .unwrap();

    // Add 1 Desk - Qty 1, Vol 1.0 m3
    create_move_inventory_item(
        "u-price-staff".to_string(),
        job.id.clone(),
        "Furniture".to_string(),
        "Desk".to_string(),
        1,
        1.0,
        None,
    )
    .await
    .unwrap();

    // Calculate Quote
    calculate_and_save_move_quote("u-price-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let quote_opt = get_move_quote("u-price-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(quote_opt.is_some());
    let q = quote_opt.unwrap();

    // Custom Calculations verification:
    // Volume = 1.0 m3
    // Base Price = 1.0 * 600.0 = 600 SEK
    // Distance Fee = 1000 SEK
    // Stairs Surcharge = 2 floors * 400 SEK = 800 SEK
    // Packing supplies fee = 1.0 * 150.0 = 150 SEK
    // Total = 600 + 1000 + 800 + 150 = 2550 SEK
    assert_eq!(q.base_price, 600);
    assert_eq!(q.distance_fee, 1000);
    assert_eq!(q.stairs_surcharge, 800);
    assert_eq!(q.packing_supplies_fee, 150);
    assert_eq!(q.total_price, 2550);

    // Cleanup
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-price-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-price-test'", ()).await.unwrap();
}

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
async fn test_digital_signature_capture() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sig-test', 'Sig Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sig-staff', 'ws-sig-test', 'staff@sig.io', 'admin')", ()).await.unwrap();

    // Create job ticket
    let job = create_job_ticket(
        "u-sig-staff".to_string(),
        "ws-sig-test".to_string(),
        "Cabinet relocation".to_string(),
        "Delicate office cabinets".to_string(),
        "Cabinet Road 10".to_string(),
        "medium".to_string(),
        None,
        "2026-08-14".to_string(),
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

    // 1. Initially verify no signature exists
    let sig_opt = get_job_signature("u-sig-staff".to_string(), job.id.clone()).await.unwrap();
    assert!(sig_opt.is_none());

    // 2. Save signature
    let mock_signature = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADIA...";
    save_job_signature("u-sig-staff".to_string(), job.id.clone(), "John Doe (Customer)".to_string(), mock_signature.to_string()).await.unwrap();

    // 3. Retrieve and verify signature details
    let sig_opt_2 = get_job_signature("u-sig-staff".to_string(), job.id.clone()).await.unwrap();
    assert!(sig_opt_2.is_some());
    let sig = sig_opt_2.unwrap();
    assert_eq!(sig.signer_name, "John Doe (Customer)");
    assert_eq!(sig.signature_data_base64, mock_signature);
    assert_eq!(sig.job_ticket_id, job.id);
    assert_eq!(sig.workspace_id, "ws-sig-test");

    // Cleanup
    conn.execute("DELETE FROM move_signatures WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sig-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_gps_routing_urls() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-test', 'GPS Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-staff', 'ws-gps-test', 'staff@gps.io', 'admin')", ()).await.unwrap();

    // 1. Create job ticket with both origin and destination addresses
    let job1 = create_job_ticket(
        "u-gps-staff".to_string(),
        "ws-gps-test".to_string(),
        "Cabinet relocation".to_string(),
        "Delicate office cabinets".to_string(),
        "Dest Road 10".to_string(),
        "medium".to_string(),
        None,
        "2026-08-14".to_string(),
        "[]".to_string(),
        Some("Origin St 1".to_string()),
        Some("Dest St 5".to_string()),
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // Verify routing URL with both origin and destination
    let url1 = get_directions_url("u-gps-staff".to_string(), job1.id.clone()).await.unwrap();
    assert_eq!(url1, "https://www.google.com/maps/dir/?api=1&origin=Origin%20St%201&destination=Dest%20St%205");

    // 2. Create job ticket with destination only (relying on fallback to location_address)
    let job2 = create_job_ticket(
        "u-gps-staff".to_string(),
        "ws-gps-test".to_string(),
        "Cabinet relocation".to_string(),
        "Delicate office cabinets".to_string(),
        "Location St 20".to_string(),
        "medium".to_string(),
        None,
        "2026-08-14".to_string(),
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

    let url2 = get_directions_url("u-gps-staff".to_string(), job2.id.clone()).await.unwrap();
    assert_eq!(url2, "https://www.google.com/maps/dir/?api=1&destination=Location%20St%2020");

    // Cleanup
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-gps-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-gps-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-gps-test'", ()).await.unwrap();
}
