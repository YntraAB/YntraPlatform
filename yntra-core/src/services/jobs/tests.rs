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
                route_stops_json: None,
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

#[tokio::test]
async fn test_client_self_service_inventory_flow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace and a client user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-client-inv-test', 'Client Inv WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-client-inv', 'ws-client-inv-test', 'client@selfservice.se', 'client')", ()).await.unwrap();

    // 2. Create a job ticket (staff creator or admin, but assigned to client)
    let job = create_job_ticket(
        "u-client-inv".to_string(), // In this mock setup, we authorize the create with the client's ID
        "ws-client-inv-test".to_string(),
        "Client Relocation".to_string(),
        "Self service inventory".to_string(),
        "Client Home 1".to_string(),
        "medium".to_string(),
        Some("u-client-inv".to_string()),
        "2026-09-10".to_string(),
        "[]".to_string(),
        Some("Client Home 1".to_string()),
        Some("New Apartment 2".to_string()),
        1,      // origin floor
        2,      // destination floor
        true,   // origin elevator
        false,  // destination elevator (no elevator)
        false,
        false,
    )
    .await
    .unwrap();

    // 3. Client adds a Sofa (Qty 1, Vol 1.5 m3)
    create_move_inventory_item(
        "u-client-inv".to_string(),
        job.id.clone(),
        "Möbler".to_string(),
        "Soffa".to_string(),
        1,
        1.5,
        Some("Tung soffa".to_string()),
    )
    .await
    .unwrap();

    // 4. Client adds a Box (Qty 10, Vol 0.1 m3)
    create_move_inventory_item(
        "u-client-inv".to_string(),
        job.id.clone(),
        "Kartonger".to_string(),
        "Kartong".to_string(),
        10,
        0.1,
        None,
    )
    .await
    .unwrap();

    // 5. Verify inventory has been successfully created
    let inv = get_move_inventory("u-client-inv".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(inv.len(), 2);
    assert!(inv.iter().any(|i| i.item_name == "Soffa" && i.quantity == 1));
    assert!(inv.iter().any(|i| i.item_name == "Kartong" && i.quantity == 10));

    // 6. Client calculates quote
    calculate_and_save_move_quote("u-client-inv".to_string(), job.id.clone())
        .await
        .unwrap();

    let quote_opt = get_move_quote("u-client-inv".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(quote_opt.is_some());
    let q = quote_opt.unwrap();

    // Total Volume = 1.5 * 1 + 0.1 * 10 = 2.5 m3
    // Base Price = 2.5 * 500 = 1250 SEK
    // Distance Fee = 800 SEK
    // Stairs Surcharge = 2 floors * 300 SEK (destination floor 2, no elevator) = 600 SEK
    // Packing supplies fee = 2.5 * 100 = 250 SEK
    // Total = 1250 + 800 + 600 + 250 = 2900 SEK
    assert_eq!(q.base_price, 1250);
    assert_eq!(q.distance_fee, 800);
    assert_eq!(q.stairs_surcharge, 600);
    assert_eq!(q.packing_supplies_fee, 250);
    assert_eq!(q.total_price, 2900);

    // 7. Client deletes Sofa
    let sofa_item = inv.iter().find(|i| i.item_name == "Soffa").unwrap();
    delete_move_inventory_item("u-client-inv".to_string(), sofa_item.id.clone())
        .await
        .unwrap();

    // 8. Verify count decreased
    let inv_after = get_move_inventory("u-client-inv".to_string(), job.id.clone())
        .await
        .unwrap();
    assert_eq!(inv_after.len(), 1);
    assert_eq!(inv_after[0].item_name, "Kartong");

    // Cleanup
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-client-inv'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-client-inv-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_hourly_pricing_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace with custom hourly settings
    let settings = r#"{"moving_pricing_model":"hourly","moving_hourly_rate":1500.0,"moving_hours_per_m3":0.2,"moving_minimum_hours":3.0,"moving_distance_fee_flat":500.0}"#;
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hourly-test', 'Hourly Test WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-hourly-staff', 'ws-hourly-test', 'staff@hourly.io', 'admin')", ()).await.unwrap();

    // 2. Create a job ticket (ground floor, no stairs surcharge)
    let job = create_job_ticket(
        "u-hourly-staff".to_string(),
        "ws-hourly-test".to_string(),
        "Hourly Relocation".to_string(),
        "Testing hourly calculator".to_string(),
        "Office 1".to_string(),
        "medium".to_string(),
        None,
        "2026-09-12".to_string(),
        "[]".to_string(),
        Some("Office 1".to_string()),
        Some("Office 2".to_string()),
        0,      // origin floor
        0,      // destination floor
        true,   // origin elevator
        true,   // destination elevator
        false,
        false,
    )
    .await
    .unwrap();

    // 3. Add inventory items: Qty 5, Vol 2.0 m3 each => Total Volume = 10.0 m3
    create_move_inventory_item(
        "u-hourly-staff".to_string(),
        job.id.clone(),
        "Möbler".to_string(),
        "Bord".to_string(),
        5,
        2.0,
        None,
    )
    .await
    .unwrap();

    // 4. Calculate Quote
    calculate_and_save_move_quote("u-hourly-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let quote_opt = get_move_quote("u-hourly-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(quote_opt.is_some());
    let q = quote_opt.unwrap();

    // Total = 4500 + 500 + 1000 (packing supplies) = 6000 SEK
    assert_eq!(q.base_price, 4500);
    assert_eq!(q.distance_fee, 500);
    assert_eq!(q.total_price, 6000);

    // 5. Add more items to exceed the minimum hours
    // Add Qty 10, Vol 2.0 m3 each => Extra 20 m3 => Total Volume = 30.0 m3
    create_move_inventory_item(
        "u-hourly-staff".to_string(),
        job.id.clone(),
        "Möbler".to_string(),
        "Skåp".to_string(),
        10,
        2.0,
        None,
    )
    .await
    .unwrap();

    // Recalculate
    calculate_and_save_move_quote("u-hourly-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let q_updated = get_move_quote("u-hourly-staff".to_string(), job.id.clone())
        .await
        .unwrap()
        .unwrap();

    // Total = 9000 + 500 + 3000 (packing supplies) = 12500 SEK
    assert_eq!(q_updated.base_price, 9000);
    assert_eq!(q_updated.total_price, 12500);

    // Cleanup
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-hourly-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-hourly-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_swish_payment_flow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-swish-test', 'Swish Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-swish-client', 'ws-swish-test', 'client@swish.io', 'client')", ()).await.unwrap();

    // 2. Create job ticket & quote & invoice
    let job = create_job_ticket(
        "u-swish-client".to_string(),
        "ws-swish-test".to_string(),
        "Swish Job".to_string(),
        "Testing Swish payment integration".to_string(),
        "Address 1".to_string(),
        "medium".to_string(),
        None,
        "2026-09-12".to_string(),
        "[]".to_string(),
        Some("Address 1".to_string()),
        Some("Address 2".to_string()),
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    // Insert dummy quote
    let quote_id = "q-swish-test".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-swish-test', ?2, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    // 3. Generate invoice
    let inv = generate_move_invoice("u-swish-client".to_string(), quote_id, false)
        .await
        .unwrap();

    // 4. Initiate Swish Payment Session
    let session = initiate_swish_payment("u-swish-client".to_string(), inv.id.clone())
        .await
        .unwrap();

    assert_eq!(session.amount, 1500.0);
    assert_eq!(session.status, "pending");
    assert!(session.swish_url.contains("swish://paymentrequest?token="));
    assert!(session.qr_code_base64.contains("data:image/svg+xml;base64,"));

    // 5. Complete payment using pay_move_invoice
    pay_move_invoice("u-swish-client".to_string(), inv.id.clone())
        .await
        .unwrap();

    let inv_after = get_move_invoice("u-swish-client".to_string(), "q-swish-test".to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(inv_after.status, "paid");

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-swish-client'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-swish-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_skatteverket_rut_export_flow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user with personal number
    let settings = serde_json::json!({
        "company_org_number": "556999-9999",
        "moving_pricing_model": "hourly",
        "moving_hourly_rate": 1000.0,
        "moving_hours_per_m3": 0.2,
        "moving_minimum_hours": 3.0
    }).to_string();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rut-test', 'RUT Test WS', '[\"moving_company\"]', ?1)",
        crate::params![&settings]
    ).await.unwrap();

    let client_metadata = serde_json::json!({
        "personal_number": "19880808-8888"
    }).to_string();

    conn.execute(
        "INSERT OR REPLACE INTO users (id, workspace_id, email, role, full_name, metadata) VALUES ('client-1', 'ws-rut-test', 'client@rut.se', 'client', 'Anna Andersson', ?1)",
        crate::params![&client_metadata]
    ).await.unwrap();

    // 2. Create job ticket, quote, and invoice
    let job = create_job_ticket(
        "client-1".to_string(),
        "ws-rut-test".to_string(),
        "RUT Job".to_string(),
        "Testing Skatteverket export".to_string(),
        "Address 1".to_string(),
        "medium".to_string(),
        None,
        "2026-10-15".to_string(),
        "[]".to_string(),
        Some("Address 1".to_string()),
        Some("Address 2".to_string()),
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    // Insert quote with base_price = 1200, stairs = 0, packing = 0. Total = 1200.
    // Use RUT deduction
    let quote_id = "q-rut-test".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-rut-test', ?2, 1200.0, 0.0, 0.0, 0.0, 1200.0, 'accepted', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    // Generate invoice with use_rut = true
    let inv = generate_move_invoice("client-1".to_string(), quote_id, true)
        .await
        .unwrap();

    // Assert RUT values: 50% of (base + stairs) = 50% of 1200 = 600.
    assert_eq!(inv.rut_deduction, 600.0);
    assert_eq!(inv.customer_amount, 600.0);

    // Pay invoice
    pay_move_invoice("client-1".to_string(), inv.id.clone())
        .await
        .unwrap();

    // 3. Query RUT invoices listing
    let list = get_rut_invoices("client-1".to_string())
        .await
        .unwrap();
    
    let overview = list.iter().find(|i| i.invoice_id == inv.id).unwrap();
    assert_eq!(overview.customer_name, "Anna Andersson");
    assert_eq!(overview.customer_pnum, "19880808-8888");
    assert_eq!(overview.rut_amount, 600.0);
    assert_eq!(overview.status, "paid");

    // 4. Export XML
    let xml = export_skatteverket_claims("client-1".to_string(), vec![inv.id.clone()], "xml".to_string())
        .await
        .unwrap();
    
    assert!(xml.contains("<BegaranFil xmlns=\"http://xmls.skatteverket.se/se/skatteverket/us/omr/rotrut/begaran/6.0\">"));
    assert!(xml.contains("<UtforareOrgNr>556999-9999</UtforareOrgNr>"));
    assert!(xml.contains("<KoparePersnr>19880808-8888</KoparePersnr>"));
    assert!(xml.contains("<BegartBelopp>600</BegartBelopp>"));
    assert!(xml.contains("<RutArbete>"));
    assert!(xml.contains("<Flyttjanster>2</Flyttjanster>")); // 1200 / 600 = 2 hours

    // 5. Export CSV
    let csv = export_skatteverket_claims("client-1".to_string(), vec![inv.id.clone()], "csv".to_string())
        .await
        .unwrap();
    
    assert!(csv.contains("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours"));
    assert!(csv.contains("556999-9999,19880808-8888"));
    assert!(csv.contains(",1200,600,2,2"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'client-1'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_multi_stop_route_optimization() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-route-test', 'Route Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-route-staff', 'ws-route-test', 'staff@route.io', 'admin')", ()).await.unwrap();

    // 2. Create job ticket
    let job = create_job_ticket(
        "u-route-staff".to_string(),
        "ws-route-test".to_string(),
        "Relocation route".to_string(),
        "Testing TSP optimization".to_string(),
        "Recycling Center".to_string(),
        "medium".to_string(),
        None,
        "2026-08-15".to_string(),
        "[]".to_string(),
        Some("Warehouse".to_string()),
        Some("Drop-off B".to_string()),
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // Verify initial stops
    assert_eq!(job.route_stops_json, Some("[]".to_string()));

    // 3. Update route stops
    let stops = vec![
        "Recycling Center".to_string(),
        "Pickup A".to_string(),
    ];
    update_route_stops("u-route-staff".to_string(), job.id.clone(), stops.clone())
        .await
        .unwrap();

    // Reload job and check stops
    let tickets = get_job_tickets("u-route-staff".to_string()).await.unwrap();
    let reloaded = tickets.iter().find(|t| t.id == job.id).unwrap();
    assert_eq!(
        reloaded.route_stops_json.as_deref(),
        Some("[\"Recycling Center\",\"Pickup A\"]")
    );

    // 4. Optimize stops
    let optimized = optimize_job_route("u-route-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    
    assert_eq!(optimized.len(), 2);
    let tickets2 = get_job_tickets("u-route-staff".to_string()).await.unwrap();
    let reloaded2 = tickets2.iter().find(|t| t.id == job.id).unwrap();
    let reloaded_stops: Vec<String> = serde_json::from_str(reloaded2.route_stops_json.as_deref().unwrap()).unwrap();
    assert_eq!(reloaded_stops, optimized);

    // 5. Directions URL verification
    let directions_url = get_directions_url("u-route-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    
    assert!(directions_url.contains("origin=Warehouse"));
    assert!(directions_url.contains("destination=Drop-off%20B"));
    assert!(directions_url.contains("waypoints="));
    assert!(directions_url.contains("Recycling%20Center"));
    assert!(directions_url.contains("Pickup%20A"));

    // Cleanup
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-route-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-route-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-route-test'", ()).await.unwrap();
}




