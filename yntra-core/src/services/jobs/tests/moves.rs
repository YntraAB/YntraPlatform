use crate::database;
use crate::infra::errors::YntraError;
use crate::services::jobs::{create_job_ticket, calculate_and_save_move_quote, get_move_quote, accept_move_quote, get_move_quote_revisions, accept_move_quote_with_deposit, confirm_quote_deposit_payment};
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
    assert_eq!(q.base_price, 1000.0);
    assert_eq!(q.distance_fee, 800.0);
    assert_eq!(q.stairs_surcharge, 900.0);
    assert_eq!(q.packing_supplies_fee, 200.0);
    assert_eq!(q.total_price, 2900.0);

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
    assert_eq!(q_updated.base_price, 750.0);
    assert_eq!(q_updated.total_price, 2600.0);

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
    assert_eq!(q.base_price, 1250.0);
    assert_eq!(q.distance_fee, 800.0);
    assert_eq!(q.stairs_surcharge, 600.0);
    assert_eq!(q.packing_supplies_fee, 250.0);
    assert_eq!(q.total_price, 2900.0);

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

    assert_eq!(quote.base_price, 1650.0);
    assert_eq!(quote.total_price, 2600.0); // base (1650) + distance default (800) + packaging default (1.5 * 100 = 150) = 2600

    // Direct unit test of calculate_item_specialty_surcharge matching synonyms, category, and notes
    use crate::services::jobs::calculate_item_specialty_surcharge;
    assert_eq!(calculate_item_specialty_surcharge("Möbler".into(), "Pianostol".into(), None, 1500.0, 2000.0, 2500.0, 500.0), 1500.0);
    assert_eq!(calculate_item_specialty_surcharge("Övrigt".into(), "Gammalt Klaver".into(), None, 1500.0, 2000.0, 2500.0, 500.0), 1500.0);
    assert_eq!(calculate_item_specialty_surcharge("Kontor".into(), "Skåp".into(), Some("Värdeskåp med kodlås / Tunglyft".into()), 1500.0, 2000.0, 2500.0, 500.0), 2000.0);
    assert_eq!(calculate_item_specialty_surcharge("Utomhus".into(), "Badutrustning".into(), Some("Spabad för 4 pers".into()), 1500.0, 2000.0, 2500.0, 500.0), 2500.0);
    assert_eq!(calculate_item_specialty_surcharge("Konst".into(), "Skulptur".into(), Some("Skör marmor".into()), 1500.0, 2000.0, 2500.0, 500.0), 500.0);

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
    assert_eq!(quote1.packing_supplies_fee, 200.0);

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
    assert_eq!(quote2.packing_supplies_fee, 400.0);

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
    assert_eq!(quote3.packing_supplies_fee, 300.0);

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
    assert_eq!(q.stairs_surcharge, 900.0);
    assert_eq!(q.total_price, 2900.0);

    // Verify custom admin staircase multipliers
    use crate::services::jobs::moves::{calculate_access_and_stair_surcharge, calculate_access_and_stair_surcharge_with_multipliers};
    // Default spiral staircase multiplier = 1.5 -> 2 floors * 300 kr * 1.5 = 900 kr
    assert_eq!(calculate_access_and_stair_surcharge(2, 0, false, true, Some("spiral".to_string()), None, None, None, 0, false, 300.0, 40.0, 3500.0, 500.0), 900.0);

    // Custom admin spiral multiplier = 1.0 (no added cost) -> 2 floors * 300 kr * 1.0 = 600 kr
    assert_eq!(calculate_access_and_stair_surcharge_with_multipliers(2, 0, false, true, Some("spiral".to_string()), None, None, None, 0, false, 300.0, 40.0, 3500.0, 500.0, 1.0, 1.0, 1.0), 600.0);

    // Custom admin narrow multiplier = 1.25 -> 2 floors * 300 kr * 1.25 = 750 kr
    assert_eq!(calculate_access_and_stair_surcharge_with_multipliers(2, 0, false, true, Some("narrow".to_string()), None, None, None, 0, false, 300.0, 40.0, 3500.0, 500.0, 1.5, 1.25, 1.2), 750.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-basement-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-basement-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_dynamic_pricing_models() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup workspace with weekend multiplier (1.25), peak season multiplier (1.15), long distance settings
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dyn-test', 'Dynamic WS', '[\"moving_company\"]', '{\"moving_pricing_model\":\"hourly\",\"moving_hourly_rate\":1000.0,\"moving_minimum_hours\":4.0,\"moving_weekend_multiplier\":1.25,\"moving_peak_season_multiplier\":1.15,\"estimated_distance_km\":100.0,\"moving_local_radius_km\":30.0,\"moving_per_km_rate\":15.0,\"moving_distance_fee_flat\":500.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dyn-staff', 'ws-dyn-test', 'staff@dyn.io', 'admin')", ()).await.unwrap();

    // Create job ticket scheduled on Saturday Aug 29, 2026 (Both weekend and peak season end-of-month!)
    // 2026-08-29 is Saturday (weekend = 1.25) AND day 29 >= 25 (peak = 1.15)
    let job = create_job_ticket(
        "u-dyn-staff".to_string(),
        "ws-dyn-test".to_string(),
        "Weekend Peak Intercity Move".to_string(),
        "Testing dynamic pricing".to_string(),
        "Stockholm".to_string(),
        "high".to_string(),
        None,
        "2026-08-29".to_string(),
        "[]".to_string(),
        Some("Stockholm".to_string()),
        Some("Uppsala".to_string()),
        0, 0, true, true, false, false,
    ).await.unwrap();

    // Add minimal item (0.5 m3)
    create_move_inventory_item(
        "u-dyn-staff".to_string(),
        job.id.clone(),
        "Möbler".to_string(),
        "Bord".to_string(),
        1, 0.5, None,
    ).await.unwrap();

    calculate_and_save_move_quote("u-dyn-staff".to_string(), job.id.clone()).await.unwrap();

    let q = get_move_quote("u-dyn-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // Minimum hours = 4.0 hours * 1000.0 SEK/h = 4000 SEK base labor
    // Weekend (1.25) * Peak (1.15) = 1.4375 -> Base price = 4000 * 1.4375 = 5750 SEK
    assert_eq!(q.base_price, 5750.0);

    // Distance fee: Flat (500) + (100km - 30km) * 15 SEK/km (1050) = 1550 SEK
    assert_eq!(q.distance_fee, 1550.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-dyn-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_building_access_and_staircase_surcharges() {
    use crate::services::jobs::calculate_access_and_stair_surcharge;

    // 1. Standard stairs (2 floors * 300 SEK = 600 SEK)
    let s1 = calculate_access_and_stair_surcharge(
        2, 0, false, true, None, None, None, None, 0, false, 300.0, 40.0, 3500.0, 500.0
    );
    assert_eq!(s1, 600.0);

    // 2. Spiral staircase multiplier (2 floors * 300 SEK * 1.5x = 900 SEK)
    let s2 = calculate_access_and_stair_surcharge(
        2, 0, false, true, Some("spiral".into()), None, None, None, 0, false, 300.0, 40.0, 3500.0, 500.0
    );
    assert_eq!(s2, 900.0);

    // 3. Small elevator constraint (elevator present but too small for furniture, so stairs surcharge applies + 500 SEK small elevator fee)
    // 2 floors * 300 = 600 + 500 = 1100 SEK
    let s3 = calculate_access_and_stair_surcharge(
        2, 0, true, true, None, None, Some("small".into()), None, 0, false, 300.0, 40.0, 3500.0, 500.0
    );
    assert_eq!(s3, 1100.0);

    // 4. External Crane / Hoist requirement (3500 SEK) + Long carry (50m * 40 SEK/m = 2000 SEK) -> 5500 SEK
    let s4 = calculate_access_and_stair_surcharge(
        0, 0, true, true, None, None, None, None, 50, true, 300.0, 40.0, 3500.0, 500.0
    );
    assert_eq!(s4, 5500.0);
}

#[test]
fn test_tariff_estimation_and_volume_conversions() {
    use crate::services::jobs::{calculate_packing_materials_tariff_estimate, convert_volume_to_tariff_weight};

    let supplies = calculate_packing_materials_tariff_estimate(10.0);
    assert_eq!(supplies.total_volume_m3, 10.0);
    assert_eq!(supplies.small_boxes_count, 35);
    assert_eq!(supplies.large_boxes_count, 20);
    assert_eq!(supplies.wardrobe_boxes_count, 4);
    assert!(supplies.estimated_supplies_cost_sek > 0.0);

    let res_weight = convert_volume_to_tariff_weight(10.0, false);
    assert_eq!(res_weight.density_lbs_per_cu_ft, 7.0);
    assert!(res_weight.calculated_weight_lbs > 2400.0);
    assert_eq!(res_weight.requires_shuttle_truck, false);

    let comm_weight = convert_volume_to_tariff_weight(50.0, true);
    assert_eq!(comm_weight.density_lbs_per_cu_ft, 12.0);
    assert_eq!(comm_weight.requires_shuttle_truck, true);
}

#[tokio::test]
async fn test_warehouse_vault_sit_storage_workflow() {
    use crate::services::jobs::{
        assign_job_to_warehouse_vault, get_job_warehouse_vaults,
        release_job_from_warehouse_vault, calculate_sit_recurring_billing_summary,
    };

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sit-test', 'SIT WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sit-staff', 'ws-sit-test', 'staff@sit.se', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, sync_status) VALUES ('job-sit-1', 'ws-sit-test', 'Job SIT 1', 'Desc', 'Loc', 'normal', 'quote_requested', '2026-08-01', '[]', '2026-08-01', 1700000000000, 'synced')", ()).await.unwrap();

    let vault = assign_job_to_warehouse_vault(
        "u-sit-staff".to_string(),
        "job-sit-1".to_string(),
        "V-101".to_string(),
        "Central Huvudlager #1".to_string(),
        15.0,
        2200.0,
        "2026-08-01".to_string(),
        None,
    ).await.unwrap();

    assert_eq!(vault.vault_number, "V-101");
    assert_eq!(vault.status, "stored");

    let vaults = get_job_warehouse_vaults("u-sit-staff".to_string(), "job-sit-1".to_string()).await.unwrap();
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].vault_number, "V-101");

    let summary = calculate_sit_recurring_billing_summary("u-sit-staff".to_string(), "job-sit-1".to_string()).await.unwrap();
    assert_eq!(summary.vault_count, 1);
    assert_eq!(summary.total_volume_m3, 15.0);
    assert!(summary.accumulated_storage_fee_sek >= 2200.0);

    release_job_from_warehouse_vault("u-sit-staff".to_string(), vault.id).await.unwrap();
    let vaults_after = get_job_warehouse_vaults("u-sit-staff".to_string(), "job-sit-1".to_string()).await.unwrap();
    assert_eq!(vaults_after[0].status, "released");

    conn.execute("DELETE FROM warehouse_vaults WHERE workspace_id = 'ws-sit-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-sit-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sit-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-sit-test'", ()).await.ok();
}

#[tokio::test]
async fn test_tip_distribution_and_mover_payroll_split() {
    use crate::services::jobs::{
        create_job_ticket, distribute_job_customer_tip, get_job_tip_distribution,
        calculate_mover_job_payroll_split,
    };

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-pay-test', 'Pay WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pay-staff', 'ws-pay-test', 'staff@pay.se', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-pay-staff".to_string(),
        "ws-pay-test".to_string(),
        "Job Pay 1".to_string(),
        "Desc".to_string(),
        "Loc".to_string(),
        "normal".to_string(),
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
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pay-driver', 'ws-pay-test', 'driver@pay.se', 'staff')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pay-mover', 'ws-pay-test', 'mover@pay.se', 'staff')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, 'u-pay-driver', 'driver')", crate::params![&job.id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, 'u-pay-mover', 'mover')", crate::params![&job.id]).await.unwrap();

    let tip = distribute_job_customer_tip("u-pay-staff".to_string(), job.id.clone(), 600.0).await.unwrap();
    assert_eq!(tip.crew_count, 2);
    assert_eq!(tip.tip_per_member_sek, 300.0);

    let tip_opt = get_job_tip_distribution("u-pay-staff".to_string(), job.id.clone()).await.unwrap();
    assert!(tip_opt.is_some());

    let driver_pay = calculate_mover_job_payroll_split("u-pay-staff".to_string(), job.id.clone(), "u-pay-driver".to_string(), 2.5, 5.5, false).await.unwrap();
    assert_eq!(driver_pay.driving_rate_sek_per_h, 230.0);
    assert_eq!(driver_pay.tip_allocated_sek, 300.0);
    assert_eq!(driver_pay.total_gross_payout_sek, 1892.5);

    let mover_pay = calculate_mover_job_payroll_split("u-pay-staff".to_string(), job.id.clone(), "u-pay-mover".to_string(), 0.0, 10.0, true).await.unwrap();
    assert_eq!(mover_pay.overtime_hours, 2.0);
    assert_eq!(mover_pay.per_diem_allowance_sek, 290.0);
    assert_eq!(mover_pay.total_gross_payout_sek, 2995.0);

    conn.execute("DELETE FROM job_tips WHERE workspace_id = 'ws-pay-test'", ()).await.ok();
    conn.execute("DELETE FROM job_crew WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-pay-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-pay-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-pay-test'", ()).await.ok();
}

#[tokio::test]
async fn test_manual_override_dynamic_item_additions() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-ovr-test', 'Override WS', '[\"moving_company\"]', '{\"moving_base_rate_per_m3\":500.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-ovr-staff', 'ws-ovr-test', 'staff@ovr.io', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-ovr-staff".to_string(),
        "ws-ovr-test".to_string(),
        "Override Job".to_string(),
        "Testing manual override dynamic additions".to_string(),
        "Addr 1".to_string(),
        "medium".to_string(),
        None,
        "2026-09-10".to_string(),
        "[]".to_string(),
        Some("Addr 1".to_string()),
        Some("Addr 2".to_string()),
        0, 0, true, true, false, false,
    ).await.unwrap();

    // 1. Initial inventory: 1 Sofa @ 2.0 m3 -> volume cost = 1000 SEK. Distance fee = 800 SEK, supplies = 200 SEK.
    // Total calculated = 2000 SEK
    create_move_inventory_item("u-ovr-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Sofa".to_string(), 1, 2.0, None).await.unwrap();
    calculate_and_save_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap();

    let q1 = get_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap().unwrap();
    assert_eq!(q1.total_price, 2000.0);

    // 2. Admin sets a manual price override of 3000 SEK (a custom price agreement)
    crate::services::jobs::update_move_quote_price_adjustments("u-ovr-staff".to_string(), job.id.clone(), Some(3000.0), None).await.unwrap();

    let q2 = get_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap().unwrap();
    assert_eq!(q2.manual_price_override, Some(3000.0));
    assert_eq!(q2.total_price, 4000.0);

    // 3. Customer/Admin adds an extra item (Armchair @ 1.0 m3 -> volume cost +500 SEK, supplies +100 SEK -> calculated total increases to 2600 SEK, but manual override remains fixed at 3000 SEK)
    create_move_inventory_item("u-ovr-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Fåtölj".to_string(), 1, 1.0, None).await.unwrap();
    calculate_and_save_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap();

    // The agreed manual price override remains preserved at 3000 SEK base
    let q3 = get_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap().unwrap();
    assert_eq!(q3.manual_price_override, Some(3000.0));
    assert_eq!(q3.total_price, 4100.0);

    // 4. Admin clears the manual price override -> total price reverts to updated calculated total (2600.0 SEK)
    crate::services::jobs::update_move_quote_price_adjustments("u-ovr-staff".to_string(), job.id.clone(), None, None).await.unwrap();
    let q4 = get_move_quote("u-ovr-staff".to_string(), job.id.clone()).await.unwrap().unwrap();
    assert_eq!(q4.manual_price_override, None);
    assert_eq!(q4.total_price, 2600.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-ovr-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-ovr-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-ovr-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-ovr-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-ovr-test'", ()).await.ok();
}

#[tokio::test]
async fn test_quote_revision_audit_trail_and_accepted_lock() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rev-test', 'Revision WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rev-staff', 'ws-rev-test', 'staff@rev.se', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rev-client', 'ws-rev-test', 'client@rev.se', 'client')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-rev-staff".to_string(),
        "ws-rev-test".to_string(),
        "Revision Job".to_string(),
        "Audit test".to_string(),
        "Street 1".to_string(),
        "medium".to_string(),
        Some("u-rev-client".to_string()),
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        true,
        true,
        false,
        false,
    ).await.unwrap();

    // 1. Initial inventory item and quote
    create_move_inventory_item("u-rev-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Bord".to_string(), 1, 2.0, None).await.unwrap();
    calculate_and_save_move_quote("u-rev-staff".to_string(), job.id.clone()).await.unwrap();

    let q1 = get_move_quote("u-rev-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // 2. Client adds a second item -> recalculates quote and creates audit revision record
    create_move_inventory_item("u-rev-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Soffa".to_string(), 1, 3.0, None).await.unwrap();
    calculate_and_save_move_quote("u-rev-staff".to_string(), job.id.clone()).await.unwrap();

    let revs = get_move_quote_revisions("u-rev-staff".to_string(), q1.id.clone()).await.unwrap();
    assert!(!revs.is_empty());
    assert_eq!(revs[0].quote_id, q1.id);
    assert_eq!(revs[0].previous_total, q1.total_price);

    // 3. Accept quote
    accept_move_quote("u-rev-staff".to_string(), q1.id.clone()).await.unwrap();

    // 4. Guest / client attempts to modify inventory after acceptance must fail
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-client-guest', 'ws-rev-test', 'client@rev.se', 'client')", ()).await.unwrap();
    let guest_err = create_move_inventory_item("u-client-guest".to_string(), job.id.clone(), "Möbler".to_string(), "Stol".to_string(), 1, 0.5, None).await;
    assert!(guest_err.is_err());
    assert!(matches!(guest_err.unwrap_err(), YntraError::ValidationError(_)));

    // 5. Authorized field crew / staff CAN add items on move day post-acceptance
    create_move_inventory_item("u-rev-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Stol".to_string(), 1, 0.5, None).await.unwrap();
    calculate_and_save_move_quote("u-rev-staff".to_string(), job.id.clone()).await.unwrap();
    let updated_quote = get_move_quote("u-rev-staff".to_string(), job.id.clone()).await.unwrap().unwrap();
    assert_eq!(updated_quote.status, "revised");

    // Cleanup
    conn.execute("DELETE FROM move_quote_revisions WHERE workspace_id = 'ws-rev-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-rev-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-rev-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-rev-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rev-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rev-test'", ()).await.ok();
}

#[tokio::test]
async fn test_quote_deposit_flow_prevents_premature_accepted_lock() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dep-test', 'Deposit WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dep-staff', 'ws-dep-test', 'staff@dep.se', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-dep-staff".to_string(),
        "ws-dep-test".to_string(),
        "Deposit Job".to_string(),
        "Deposit test".to_string(),
        "Addr 1".to_string(),
        "medium".to_string(),
        None,
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        true,
        true,
        false,
        false,
    ).await.unwrap();

    create_move_inventory_item("u-dep-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Bord".to_string(), 1, 2.0, None).await.unwrap();
    calculate_and_save_move_quote("u-dep-staff".to_string(), job.id.clone()).await.unwrap();
    let q1 = get_move_quote("u-dep-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // 1. Initiate deposit payment approval
    let dep_res = accept_move_quote_with_deposit("u-dep-staff".to_string(), q1.id.clone(), "swish".to_string()).await.unwrap();
    assert!(dep_res.success);
    assert_eq!(dep_res.quote_status, "pending_deposit");
    assert!(dep_res.payment_session_url.is_some());

    // Verify quote and job ticket statuses are NOT locked as accepted/assigned prematurely
    let q_status: String = conn.query_row("SELECT status FROM move_quotes WHERE id = ?1", crate::params![&q1.id], |r| r.get(0)).await.unwrap();
    let j_status: String = conn.query_row("SELECT status FROM job_tickets WHERE id = ?1", crate::params![&job.id], |r| r.get(0)).await.unwrap();
    assert_eq!(q_status, "pending_deposit");
    assert_eq!(j_status, "deposit_pending");

    // 2. Deposit payment completes -> confirm deposit payment
    confirm_quote_deposit_payment("u-dep-staff".to_string(), q1.id.clone(), "ref-12345".to_string()).await.unwrap();

    // Verify status transitions to accepted and assigned
    let q_status_after: String = conn.query_row("SELECT status FROM move_quotes WHERE id = ?1", crate::params![&q1.id], |r| r.get(0)).await.unwrap();
    let j_status_after: String = conn.query_row("SELECT status FROM job_tickets WHERE id = ?1", crate::params![&job.id], |r| r.get(0)).await.unwrap();
    assert_eq!(q_status_after, "accepted");
    assert_eq!(j_status_after, "assigned");

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-dep-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-dep-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-dep-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dep-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-dep-test'", ()).await.ok();
}

#[tokio::test]
async fn test_deposit_payment_link_desynchronization_prevention() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dep-desync-test', 'Desync WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-desync-staff', 'ws-dep-desync-test', 'staff@desync.se', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-desync-staff".to_string(), "ws-dep-desync-test".to_string(), "Desync Job".to_string(), "Desc".to_string(), "Addr 1".to_string(), "medium".to_string(), None, "2026-10-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false,
    ).await.unwrap();

    create_move_inventory_item("u-desync-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Soffa".to_string(), 1, 3.0, None).await.unwrap();
    calculate_and_save_move_quote("u-desync-staff".to_string(), job.id.clone()).await.unwrap();
    let q1 = get_move_quote("u-desync-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // 1. Accept quote with deposit (status: pending_deposit)
    let dep_res = accept_move_quote_with_deposit("u-desync-staff".to_string(), q1.id.clone(), "swish".to_string()).await.unwrap();
    assert_eq!(dep_res.quote_status, "pending_deposit");

    // 2. Client/staff modifies inventory after deposit link generation (adds heavy item)
    create_move_inventory_item("u-desync-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Pianobord".to_string(), 1, 5.0, None).await.unwrap();

    // Verify quote status was invalidated and moved to 'revised', and job ticket returned to 'pending'
    let q_status: String = conn.query_row("SELECT status FROM move_quotes WHERE id = ?1", crate::params![&q1.id], |r| r.get(0)).await.unwrap();
    let j_status: String = conn.query_row("SELECT status FROM job_tickets WHERE id = ?1", crate::params![&job.id], |r| r.get(0)).await.unwrap();
    assert_eq!(q_status, "revised");
    assert_eq!(j_status, "pending");

    // 3. Attempting to confirm stale deposit payment link must be rejected
    let confirm_stale = confirm_quote_deposit_payment("u-desync-staff".to_string(), q1.id.clone(), "ref-stale".to_string()).await;
    assert!(confirm_stale.is_err());
    assert!(confirm_stale.unwrap_err().to_string().contains("deposit payment link invalidated due to quote revisions"));

    // 4. Re-accepting quote with deposit produces new deposit amount based on updated total
    let dep_res2 = accept_move_quote_with_deposit("u-desync-staff".to_string(), q1.id.clone(), "swish".to_string()).await.unwrap();
    assert_eq!(dep_res2.quote_status, "pending_deposit");
    assert!(dep_res2.deposit_amount > dep_res.deposit_amount);

    // 5. Confirming payment on new deposit link succeeds
    let confirm_fresh = confirm_quote_deposit_payment("u-desync-staff".to_string(), q1.id.clone(), "ref-fresh".to_string()).await;
    assert!(confirm_fresh.is_ok());

    let final_q_status: String = conn.query_row("SELECT status FROM move_quotes WHERE id = ?1", crate::params![&q1.id], |r| r.get(0)).await.unwrap();
    assert_eq!(final_q_status, "accepted");

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-dep-desync-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-dep-desync-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-dep-desync-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dep-desync-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-dep-desync-test'", ()).await.ok();
}

#[tokio::test]
async fn test_dynamic_currency_labels_in_deposit_approval() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-curr-test', 'Euro Movers WS', '[\"moving_company\"]', '{\"currency\":\"EUR\",\"target_region\":\"DE\"}')",
        ()
    ).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-curr-staff', 'ws-curr-test', 'staff@euromovers.de', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-curr-staff".to_string(), "ws-curr-test".to_string(), "Euro Move".to_string(), "Desc".to_string(), "Berlin 1".to_string(), "medium".to_string(), None, "2026-10-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false,
    ).await.unwrap();

    create_move_inventory_item("u-curr-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Tisch".to_string(), 1, 2.0, None).await.unwrap();
    calculate_and_save_move_quote("u-curr-staff".to_string(), job.id.clone()).await.unwrap();
    let q1 = get_move_quote("u-curr-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    let dep_res = accept_move_quote_with_deposit("u-curr-staff".to_string(), q1.id.clone(), "swish".to_string()).await.unwrap();
    assert!(dep_res.message.contains("EUR"));
    assert!(!dep_res.message.contains("SEK"));

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-curr-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-curr-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-curr-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-curr-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-curr-test'", ()).await.ok();
}

#[tokio::test]
async fn test_custom_domain_deposit_link_generation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-customdomain-test', 'White-Label WS', '[\"moving_company\"]', '{\"payment_portal_url\":\"https://pay.nordicmovers.se\"}')",
        ()
    ).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-cd-staff', 'ws-customdomain-test', 'staff@nordic.se', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-cd-staff".to_string(),
        "ws-customdomain-test".to_string(),
        "White-label Job".to_string(),
        "Domain test".to_string(),
        "Addr 1".to_string(),
        "medium".to_string(),
        None,
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        true,
        true,
        false,
        false,
    ).await.unwrap();

    create_move_inventory_item("u-cd-staff".to_string(), job.id.clone(), "Möbler".to_string(), "Bord".to_string(), 1, 2.0, None).await.unwrap();
    calculate_and_save_move_quote("u-cd-staff".to_string(), job.id.clone()).await.unwrap();
    let q1 = get_move_quote("u-cd-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    let dep_res = accept_move_quote_with_deposit("u-cd-staff".to_string(), q1.id.clone(), "swish".to_string()).await.unwrap();
    assert!(dep_res.payment_session_url.is_some());
    let url = dep_res.payment_session_url.unwrap();
    assert!(url.starts_with("https://pay.nordicmovers.se/deposit/"));
    assert!(url.contains(&q1.id));

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-customdomain-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-customdomain-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-customdomain-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-customdomain-test'", ()).await.ok();
}

#[tokio::test]
async fn test_per_job_distance_isolation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dist-iso-test', 'Dist Iso WS', '[\"moving_company\"]', '{\"moving_distance_fee_flat\":500.0,\"moving_local_radius_km\":30.0,\"moving_per_km_rate\":10.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dist-staff', 'ws-dist-iso-test', 'staff@dist.io', 'admin')", ()).await.unwrap();

    // Job A: Long distance (100 km) stored on Job A ticket
    let job_a = create_job_ticket(
        "u-dist-staff".to_string(),
        "ws-dist-iso-test".to_string(),
        "Long Distance Move".to_string(),
        "Job A Desc".to_string(),
        "City A".to_string(),
        "medium".to_string(),
        None,
        "2026-10-01".to_string(),
        "[]".to_string(),
        Some("City A".to_string()),
        Some("City B".to_string()),
        0, 0, true, true, false, false,
    ).await.unwrap();
    conn.execute("UPDATE job_tickets SET route_stops_json = '{\"estimated_distance_km\": 100.0}' WHERE id = ?1", crate::params![&job_a.id]).await.unwrap();

    // Job B: Local move (10 km) stored on Job B ticket
    let job_b = create_job_ticket(
        "u-dist-staff".to_string(),
        "ws-dist-iso-test".to_string(),
        "Local Move".to_string(),
        "Job B Desc".to_string(),
        "City A".to_string(),
        "medium".to_string(),
        None,
        "2026-10-01".to_string(),
        "[]".to_string(),
        Some("City A".to_string()),
        Some("City A East".to_string()),
        0, 0, true, true, false, false,
    ).await.unwrap();
    conn.execute("UPDATE job_tickets SET route_stops_json = '{\"estimated_distance_km\": 10.0}' WHERE id = ?1", crate::params![&job_b.id]).await.unwrap();

    // Calculate quotes for both jobs
    calculate_and_save_move_quote("u-dist-staff".to_string(), job_a.id.clone()).await.unwrap();
    calculate_and_save_move_quote("u-dist-staff".to_string(), job_b.id.clone()).await.unwrap();

    let quote_a = get_move_quote("u-dist-staff".to_string(), job_a.id.clone()).await.unwrap().unwrap();
    let quote_b = get_move_quote("u-dist-staff".to_string(), job_b.id.clone()).await.unwrap().unwrap();

    // Job A: 500 flat + (100 - 30)*10 = 1200 SEK distance fee
    assert_eq!(quote_a.distance_fee, 1200.0);
    // Job B: 10 km <= 30 km radius -> 500 SEK flat distance fee
    assert_eq!(quote_b.distance_fee, 500.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-dist-iso-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-dist-iso-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dist-iso-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-dist-iso-test'", ()).await.ok();
}

#[tokio::test]
async fn test_depot_positioning_and_roundtrip_mileage() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-depot-test', 'Depot WS', '[\"moving_company\"]', '{\"moving_distance_fee_flat\":500.0,\"moving_local_radius_km\":30.0,\"moving_per_km_rate\":10.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-depot-staff', 'ws-depot-test', 'staff@depot.io', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-depot-staff".to_string(),
        "ws-depot-test".to_string(),
        "Depot & Roundtrip Move".to_string(),
        "Test Desc".to_string(),
        "City A".to_string(),
        "medium".to_string(),
        None,
        "2026-10-01".to_string(),
        "[]".to_string(),
        Some("City A".to_string()),
        Some("City B".to_string()),
        0, 0, true, true, false, false,
    ).await.unwrap();

    // 50 km move + 15 km depot-to-origin + 25 km destination-to-depot + roundtrip (50*2 = 100 + 15 + 25 = 140 km)
    let route_json = r#"{"estimated_distance_km": 50.0, "depot_to_origin_km": 15.0, "destination_to_depot_km": 25.0, "include_roundtrip": true}"#;
    conn.execute("UPDATE job_tickets SET route_stops_json = ?1 WHERE id = ?2", crate::params![route_json, &job.id]).await.unwrap();

    calculate_and_save_move_quote("u-depot-staff".to_string(), job.id.clone()).await.unwrap();

    let quote = get_move_quote("u-depot-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // Effective distance = 140 km. Fee = 500 + (140 - 30)*10 = 1600.0 SEK
    assert_eq!(quote.distance_fee, 1600.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-depot-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-depot-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-depot-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-depot-test'", ()).await.ok();
}
