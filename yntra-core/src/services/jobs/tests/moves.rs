use crate::database;
use crate::services::jobs::{create_job_ticket, calculate_and_save_move_quote, get_move_quote};
use crate::services::jobs::{create_move_inventory_item, get_move_inventory, delete_move_inventory_item};
use crate::services::jobs::{add_job_packaging_item, get_job_packaging_items, remove_job_packaging_item};

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
async fn test_specialty_item_surcharges() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-spec-test', 'Specialty WS', '[\"moving_company\"]', '{\"moving_base_rate_per_m3\":100.0,\"surcharge_piano\":1500.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-spec-staff', 'ws-spec-test', 'staff@spec.io', 'admin')", ()).await.unwrap();

    // 2. Setup job ticket and inventory items (1 Piano @ 1.5 m3 volume)
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator) VALUES ('job-spec-test', 'ws-spec-test', 'Spec Job', 'Desc', 'Addr', 'medium', 'quote_requested', '2026-07-21', '[]', ?1, ?2, 0, 0, 1, 1)", crate::params![now_ms, now_ms]).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-piano', 'ws-spec-test', 'job-spec-test', 'Möbler', 'Grand Piano / Flygel', 1, 1.5)", ()).await.unwrap();

    // 3. Calculate move quote
    calculate_and_save_move_quote(
        "u-spec-staff".to_string(),
        "job-spec-test".to_string(),
    ).await.unwrap();

    // 4. Retrieve quote and verify base_price has the 1500 kr piano surcharge added
    // Expected base_price = volume (1.5) * rate (100.0) + piano surcharge (1500.0) = 150 + 1500 = 1650 kr
    let quote = get_move_quote(
        "u-spec-staff".to_string(),
        "job-spec-test".to_string(),
    ).await.unwrap().unwrap();

    assert_eq!(quote.base_price, 1650);
    assert_eq!(quote.total_price, 2600); // base (1650) + distance default (800) + packaging default (1.5 * 100 = 150) = 2600

    // 5. Clean up
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-spec-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-spec-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-spec-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-spec-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-spec-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_packaging_inventory_system() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-pack-test', 'Pack WS', '[\"moving_company\"]', '{\"moving_base_rate_per_m3\":100.0,\"moving_packing_supplies_fee_per_m3\":20.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pack-staff', 'ws-pack-test', 'staff@pack.io', 'admin')", ()).await.unwrap();

    // 2. Setup job ticket (1 item of 10.0 m3 volume)
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters, toll_fees) VALUES ('job-pack-test', 'ws-pack-test', 'Pack Job', 'Desc', 'Addr', 'medium', 'quote_requested', '2026-07-21', '[]', ?1, ?2, 0, 0, 1, 1, 0, 0.0)",
        crate::params![now_ms, now_ms]
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-sofa-pack', 'ws-pack-test', 'job-pack-test', 'Möbler', 'Sofa', 1, 10.0)", ()).await.unwrap();

    // 3. Initial quote calculation with NO actual packaging items (should fallback to volumetric estimation)
    // Volumetric packing supplies fee = 10.0 m3 * 20.0 SEK/m3 = 200.0 SEK
    calculate_and_save_move_quote(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
    ).await.unwrap();

    let quote1 = get_move_quote(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote1.packing_supplies_fee, 200);

    // 4. Add packaging items (reactive trigger should update the quote automatically)
    // Item 1: Moving Box (Qty 10, Price 30.0 SEK) -> 300.0 SEK
    // Item 2: Tape (Qty 2, Price 50.0 SEK) -> 100.0 SEK
    // Expected packing supplies fee = 300.0 + 100.0 = 400.0 SEK
    add_job_packaging_item(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
        "Flyttkartong".to_string(),
        10,
        30.0,
        false,
    ).await.unwrap();

    add_job_packaging_item(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
        "Packtejp".to_string(),
        2,
        50.0,
        false,
    ).await.unwrap();

    let items = get_job_packaging_items(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
    ).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item_name, "Flyttkartong");
    assert_eq!(items[1].item_name, "Packtejp");

    let quote2 = get_move_quote(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote2.packing_supplies_fee, 400);

    // 5. Remove one packaging item
    // Remove Item 2 (Tape). Expected supplies fee = 300.0 SEK
    let tape_item_id = items[1].id.clone();
    remove_job_packaging_item(
        "u-pack-staff".to_string(),
        tape_item_id,
    ).await.unwrap();

    let quote3 = get_move_quote(
        "u-pack-staff".to_string(),
        "job-pack-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote3.packing_supplies_fee, 300);

    // 6. Clean up
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-pack-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-pack-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_packaging_items WHERE job_ticket_id = 'job-pack-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-pack-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-pack-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-pack-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_basement_carrying_surcharges() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-basement-test', 'Basement Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-base-staff', 'ws-basement-test', 'staff@base.io', 'admin')", ()).await.unwrap();

    // Create a job ticket: -2 floor (basement 2), no elevator at origin, destination floor -1 (basement 1) no elevator
    let job = create_job_ticket(
        "u-base-staff".to_string(),
        "ws-basement-test".to_string(),
        "Basement Move".to_string(),
        "Testing basement carrying".to_string(),
        "Basement Origin".to_string(),
        "medium".to_string(),
        None,
        "2026-08-10".to_string(),
        "[]".to_string(),
        Some("Basement Origin".to_string()),
        Some("Basement Dest".to_string()),
        -2,     // origin floor
        -1,     // destination floor
        false,  // origin elevator
        false,  // destination elevator
        false,
        false,
    )
    .await
    .unwrap();

    // Add Sofa (Furniture) - Qty 1, Vol 2.0
    create_move_inventory_item(
        "u-base-staff".to_string(),
        job.id.clone(),
        "Furniture".to_string(),
        "Sofa".to_string(),
        1,
        2.0,
        None,
    )
    .await
    .unwrap();

    // Calculate Quote
    calculate_and_save_move_quote("u-base-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let quote_opt = get_move_quote("u-base-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(quote_opt.is_some());
    let q = quote_opt.unwrap();
    
    // Calculations verification:
    // Volume = 2.0 m3
    // Base Price = 2.0 * 500 = 1000 SEK
    // Distance Fee = 800 SEK
    // Stairs Surcharge = |-2| + |-1| = 3 floors * 300 SEK = 900 SEK
    // Packing supplies fee = 2.0 * 100 = 200 SEK
    // Total = 1000 + 800 + 900 + 200 = 2900 SEK
    assert_eq!(q.stairs_surcharge, 900);
    assert_eq!(q.total_price, 2900);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-basement-test'", ()).await.unwrap();
}
