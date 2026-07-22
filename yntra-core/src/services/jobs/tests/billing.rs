use crate::database;
use crate::YntraError;
use crate::services::jobs::{
    create_job_ticket, create_move_inventory_item,
    calculate_and_save_move_quote, get_move_quote,
    generate_move_invoice, get_move_invoice, pay_move_invoice,
    validate_customer_personal_number_for_rut,
    initiate_swish_payment, check_swish_payment_status,
    process_swish_payment_webhook, initiate_stripe_payment, process_stripe_payment_webhook,
    export_skatteverket_claims, get_rut_invoices,
    initiate_bankid_skatteverket_session, submit_skatteverket_claim_direct,
    adjust_invoice_for_actuals, process_onsite_mpos_card_payment,
    sync_invoice_to_erp, reconcile_erp_payments,
    assign_vehicle_to_job, add_crew_member, remove_crew_member,
};

#[tokio::test]
async fn test_invoice_and_rut_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace, staff/client user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-inv-test', 'Invoice Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-inv-staff', 'ws-inv-test', 'staff@inv.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-inv-client', 'ws-inv-test', 'client@inv.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

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

    // Insert an inventory item to set volume to 4.0 m3
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-item-1', 'ws-inv-test', ?1, 'furniture', 'Soffa', 2, 2.0)",
        crate::params![&job.id],
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
    assert_eq!(invoice.rut_deduction, 1000.0); // 50% of (1400 eligible labor + 600 stairs)
    assert_eq!(invoice.customer_amount, 2800.0);
    assert_eq!(invoice.tax_authority_amount, 1000.0);
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
    assert_eq!(fetched_invoice.rut_deduction, 1000.0);
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
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
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
        "2026-08-12".to_string(),
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
    assert_eq!(q.base_price, 600.0);
    assert_eq!(q.distance_fee, 1000.0);
    assert_eq!(q.stairs_surcharge, 800.0);
    assert_eq!(q.packing_supplies_fee, 150.0);
    assert_eq!(q.total_price, 2550.0);

    // Cleanup
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-price-staff'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-price-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_hourly_pricing_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace with hourly pricing model
    let settings = r#"{"moving_pricing_model":"hourly","moving_hourly_rate":1200.0,"moving_distance_fee_flat":800.0,"moving_stairs_surcharge_per_floor":300.0,"moving_packing_supplies_fee_per_m3":100.0,"moving_hours_per_m3":0.15,"moving_minimum_hours":2.0,"moving_labor_ratio_hourly":0.70}"#;
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hourly-test', 'Hourly Test WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-hourly-staff', 'ws-hourly-test', 'staff@hourly.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-hourly-client', 'ws-hourly-test', 'client@hourly.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    // Create job ticket: 3 floors, no elevator
    let job = create_job_ticket(
        "u-hourly-staff".to_string(),
        "ws-hourly-test".to_string(),
        "Hourly Relocation".to_string(),
        "Hourly move test".to_string(),
        "Main St 1".to_string(),
        "medium".to_string(),
        Some("u-hourly-client".to_string()),
        "2026-09-15".to_string(),
        "[]".to_string(),
        None,
        None,
        3, 0, false, true, false, false,
    )
    .await
    .unwrap();

    // Add inventory (total volume = 10.0 m3) -> estimated hours = 10.0 * 0.15 = 1.5, capped at 2.0 minimum
    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-hourly-1', 'ws-hourly-test', ?1, 'furniture', 'Big Sofa', 1, 10.0)",
        crate::params![&job.id],
    )
    .await
    .unwrap();

    // Calculate Quote
    calculate_and_save_move_quote("u-hourly-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    let q = get_move_quote("u-hourly-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    // Hours = 2.0
    // Base price = 2.0 * 1200.0 = 2400 SEK
    // Distance flat = 800 SEK
    // Stairs surcharge = 3 floors * 300.0 = 900 SEK
    // Packing supplies = 10.0 * 100 = 1000 SEK
    // Total price = 2400 + 800 + 900 + 1000 = 5100 SEK
    assert_eq!(q.base_price, 2400.0);
    assert_eq!(q.distance_fee, 800.0);
    assert_eq!(q.stairs_surcharge, 900.0);
    assert_eq!(q.packing_supplies_fee, 1000.0);
    assert_eq!(q.total_price, 5100.0);

    // Generate invoice with RUT enabled
    let invoice = generate_move_invoice("u-hourly-staff".to_string(), q.id.clone(), true).await.unwrap();

    // Recalculated eligible labor cost:
    // base_price (2400) * labor_ratio (0.70) = 1680 SEK
    // RUT deduction = 50% of (1680 labor + 900 stairs) = 1290 SEK
    // Customer payment = 5100 - 1290 = 3810 SEK
    assert_eq!(invoice.subtotal, 5100.0);
    assert_eq!(invoice.rut_deduction, 1290.0);
    assert_eq!(invoice.customer_amount, 3810.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE quote_id = ?1", crate::params![&q.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE id = ?1", crate::params![&q.id]).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id IN ('u-hourly-staff', 'u-hourly-client')", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-hourly-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_swish_payment_flow() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-swish-test', 'Swish WS', '[\"moving_company\"]', '{\"swish_payee_alias\":\"1234567890\",\"swish_use_sandbox\":true}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-swish-staff', 'ws-swish-test', 'staff@swish.io', 'admin')", ()).await.unwrap();

    // 2. Create invoice
    let job = create_job_ticket(
        "u-swish-staff".to_string(),
        "ws-swish-test".to_string(),
        "Swish Job".to_string(),
        "Testing swish flow".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-swish-1', 'ws-swish-test', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    let inv = generate_move_invoice("u-swish-staff".to_string(), "quote-swish-1".to_string(), false).await.unwrap();

    // 3. Initiate Swish Payment Session
    let session = initiate_swish_payment("u-swish-staff".to_string(), inv.id.clone()).await.unwrap();
    assert_eq!(session.amount, 1500.0);
    assert_eq!(session.status, "pending");
    assert!(!session.qr_code_base64.is_empty());
    assert!(session.swish_url.contains("paymentrequest?token="));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-swish-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-swish-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_check_swish_payment_status() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-swish-status', 'Swish WS', '[\"moving_company\"]', '{\"swish_payee_alias\":\"1234567890\"}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-swish-status-staff', 'ws-swish-status', 'staff@swish.io', 'admin')", ()).await.unwrap();

    // 2. Create invoice
    let job = create_job_ticket(
        "u-swish-status-staff".to_string(),
        "ws-swish-status".to_string(),
        "Swish Job".to_string(),
        "Testing swish flow".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-swish-status-1', 'ws-swish-status', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    let inv = generate_move_invoice("u-swish-status-staff".to_string(), "quote-swish-status-1".to_string(), false).await.unwrap();

    // 3. Query status (should be pending by default since no gateway is set up)
    let status = check_swish_payment_status("u-swish-status-staff".to_string(), inv.id.clone(), "some-token-123".to_string()).await.unwrap();
    assert_eq!(status, "pending");

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-swish-status'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-swish-status'", ()).await.unwrap();
}

#[tokio::test]
async fn test_swish_unauthorized_client_access() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup workspace & users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-swish-auth', 'Swish Auth WS', '[\"moving_company\"]', '{\"swish_payee_alias\":\"1234567890\",\"swish_use_sandbox\":true}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-swish-owner', 'ws-swish-auth', 'owner@swish.io', 'client')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-swish-attacker', 'ws-swish-auth', 'attacker@swish.io', 'client')", ()).await.unwrap();

    // Create job and invoice for owner client
    let job = create_job_ticket(
        "u-swish-owner".to_string(),
        "ws-swish-auth".to_string(),
        "Owner Job".to_string(),
        "Testing auth flow".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-swish-auth-1', 'ws-swish-auth', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    let inv = generate_move_invoice("u-swish-owner".to_string(), "quote-swish-auth-1".to_string(), false).await.unwrap();

    // 1. Attacker client attempts to initiate payment on owner's invoice -> should fail
    let err_initiate = initiate_swish_payment("u-swish-attacker".to_string(), inv.id.clone()).await;
    assert!(err_initiate.is_err());
    if let Err(YntraError::AuthError(msg)) = err_initiate {
        assert!(msg.contains("customer mismatch"));
    } else {
        panic!("Expected AuthError for unauthorized initiate_swish_payment");
    }

    // 2. Attacker client attempts to check status on owner's invoice -> should fail
    let err_check = check_swish_payment_status("u-swish-attacker".to_string(), inv.id.clone(), "token-123".to_string()).await;
    assert!(err_check.is_err());
    if let Err(YntraError::AuthError(msg)) = err_check {
        assert!(msg.contains("customer mismatch"));
    } else {
        panic!("Expected AuthError for unauthorized check_swish_payment_status");
    }

    // 3. Owner client initiates payment on own invoice -> should succeed
    let session = initiate_swish_payment("u-swish-owner".to_string(), inv.id.clone()).await;
    assert!(session.is_ok());

    // 4. Owner client checks status on own invoice -> should succeed
    let status = check_swish_payment_status("u-swish-owner".to_string(), inv.id.clone(), "token-123".to_string()).await;
    assert!(status.is_ok());

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-swish-auth'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-swish-auth'", ()).await.unwrap();
}

#[tokio::test]
async fn test_webhook_and_rbac_flows() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace with webhook configuration and a production gateway setting
    let settings = serde_json::json!({
        "swish_gateway_url": "https://api.gateway.yntra.se/v1/swish",
        "swish_webhook_token": "secret-swish-token-abc",
        "stripe_webhook_signing_secret": "whsec_testsecret123"
    }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-webhook-test', 'Webhook WS', '[\"moving_company\"]', ?1)", crate::params![&settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-webhook-client', 'ws-webhook-test', 'client-web@swish.io', 'client')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-webhook-staff', 'ws-webhook-test', 'staff-web@swish.io', 'admin')", ()).await.unwrap();

    // 2. Create job, quote, and invoice
    let job = create_job_ticket(
        "u-webhook-staff".to_string(),
        "ws-webhook-test".to_string(),
        "Webhook Job".to_string(),
        "Testing webhooks".to_string(),
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

    let quote_id = "q-webhook-test".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-webhook-test', ?2, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    let inv = generate_move_invoice("u-webhook-staff".to_string(), quote_id.clone(), false)
        .await
        .unwrap();

    // 3. Test RBAC: A client should NOT be allowed to manually mark a production invoice as paid
    let err_rbac = pay_move_invoice("u-webhook-client".to_string(), inv.id.clone()).await;
    assert!(err_rbac.is_err());
    assert!(matches!(err_rbac.unwrap_err(), YntraError::AuthError(_)));

    // 4. Test RBAC: A staff member should still be allowed to manually mark it as paid
    let ok_rbac = pay_move_invoice("u-webhook-staff".to_string(), inv.id.clone()).await;
    assert!(ok_rbac.is_ok());

    // Reset invoice status to unpaid
    conn.execute("UPDATE move_invoices SET status = 'unpaid' WHERE id = ?1", crate::params![&inv.id]).await.unwrap();

    // 5. Test Swish webhook with invalid token
    let payload = serde_json::json!({
        "status": "PAID",
        "payeePaymentReference": inv.id.clone()
    }).to_string();
    let err_swish = process_swish_payment_webhook("ws-webhook-test".to_string(), "bad-token".to_string(), payload.clone()).await;
    assert!(err_swish.is_err());

    // 6. Test Swish webhook with valid token
    let ok_swish = process_swish_payment_webhook("ws-webhook-test".to_string(), "secret-swish-token-abc".to_string(), payload.clone()).await;
    assert!(ok_swish.is_ok());

    // Verify invoice is paid
    let inv_swish = get_move_invoice("u-webhook-staff".to_string(), quote_id.clone()).await.unwrap().unwrap();
    assert_eq!(inv_swish.status, "paid");

    // Reset invoice status to unpaid
    conn.execute("UPDATE move_invoices SET status = 'unpaid' WHERE id = ?1", crate::params![&inv.id]).await.unwrap();

    // 7. Test Stripe webhook verification with valid signature
    let stripe_payload = serde_json::json!({
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "client_reference_id": inv.id.clone()
            }
        }
    }).to_string();

    let timestamp = chrono::Utc::now().timestamp();
    let signed_payload = format!("{}.{}", timestamp, stripe_payload);
    
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(b"whsec_testsecret123").unwrap();
    mac.update(signed_payload.as_bytes());
    let sig_bytes = mac.finalize().into_bytes();
    let valid_signature = const_hex::encode(sig_bytes);

    let sig_header = format!("t={},v1={}", timestamp, valid_signature);

    let ok_stripe = process_stripe_payment_webhook("ws-webhook-test".to_string(), sig_header, stripe_payload.clone()).await;
    assert!(ok_stripe.is_ok());

    // Verify invoice is paid
    let inv_stripe = get_move_invoice("u-webhook-staff".to_string(), quote_id.clone()).await.unwrap().unwrap();
    assert_eq!(inv_stripe.status, "paid");

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id IN ('u-webhook-client', 'u-webhook-staff')", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-webhook-test'", ()).await.unwrap();
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
        "personal_number": "19811218-9876"
    }).to_string();

    conn.execute(
        "INSERT OR REPLACE INTO users (id, workspace_id, email, role, full_name, metadata) VALUES ('client-1', 'ws-rut-test', 'client@rut.se', 'client', 'Anna Andersson', ?1)",
        crate::params![&client_metadata]
    ).await.unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO users (id, workspace_id, email, role, full_name) VALUES ('staff-1', 'ws-rut-test', 'staff@rut.se', 'admin', 'Staff Admin')",
        ()
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

    // Pay invoice as staff
    pay_move_invoice("staff-1".to_string(), inv.id.clone())
        .await
        .unwrap();

    // Verify security: client role calls must be rejected
    assert!(get_rut_invoices("client-1".to_string()).await.is_err());
    assert!(export_skatteverket_claims("client-1".to_string(), vec![inv.id.clone()], "xml".to_string()).await.is_err());

    // 3. Query RUT invoices listing as staff
    let list = get_rut_invoices("staff-1".to_string())
        .await
        .unwrap();
    
    let overview = list.iter().find(|i| i.invoice_id == inv.id).unwrap();
    assert_eq!(overview.customer_name, "Anna Andersson");
    assert_eq!(overview.customer_pnum, "19811218-9876");
    assert_eq!(overview.rut_amount, 600.0);
    assert_eq!(overview.status, "paid");

    // 4. Export XML as staff
    let xml = export_skatteverket_claims("staff-1".to_string(), vec![inv.id.clone()], "xml".to_string())
        .await
        .unwrap();
    
    assert!(xml.contains("<BegaranFil xmlns=\"http://xmls.skatteverket.se/se/skatteverket/us/omr/rotrut/begaran/6.0\">"));
    assert!(xml.contains("<UtforareOrgNr>556999-9999</UtforareOrgNr>"));
    assert!(xml.contains("<KoparePersnr>198112189876</KoparePersnr>"));
    assert!(xml.contains("<BegartBelopp>600</BegartBelopp>"));
    assert!(xml.contains("<RutArbete>"));
    assert!(xml.contains("<Flyttjanster>3</Flyttjanster>"));

    // 5. Export CSV as staff
    let csv = export_skatteverket_claims("staff-1".to_string(), vec![inv.id.clone()], "csv".to_string())
        .await
        .unwrap();
    
    assert!(csv.contains("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours"));
    assert!(csv.contains("556999-9999,198112189876"));
    assert!(csv.contains(",1200,600,3,3"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id IN ('client-1', 'staff-1')", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_skatteverket_batch_export_resilience_to_invalid_pnums() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-batch-resil-test', 'Resil WS', '[\"moving_company\"]', '{\"company_org_number\":\"556888-8888\"}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-resil-staff', 'ws-batch-resil-test', 'staff@resil.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-valid', 'ws-batch-resil-test', 'valid@resil.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-invalid', 'ws-batch-resil-test', 'invalid@resil.io', 'client', '{\"personal_number\":\"invalid-pnum\"}')", ()).await.unwrap();

    let job_valid = create_job_ticket(
        "u-resil-staff".to_string(), "ws-batch-resil-test".to_string(), "Valid Move".to_string(), "Desc".to_string(), "Addr".to_string(), "medium".to_string(), Some("client-valid".to_string()), "2026-08-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false,
    ).await.unwrap();
    conn.execute("INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-valid', 'ws-batch-resil-test', ?1, 1000.0, 0.0, 0.0, 0.0, 1000.0, 'accepted')", crate::params![&job_valid.id]).await.unwrap();
    let inv_valid = generate_move_invoice("u-resil-staff".to_string(), "quote-valid".to_string(), true).await.unwrap();

    let job_invalid = create_job_ticket(
        "u-resil-staff".to_string(), "ws-batch-resil-test".to_string(), "Invalid Move".to_string(), "Desc".to_string(), "Addr".to_string(), "medium".to_string(), Some("client-invalid".to_string()), "2026-08-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false,
    ).await.unwrap();
    conn.execute("INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-invalid', 'ws-batch-resil-test', ?1, 1000.0, 0.0, 0.0, 0.0, 1000.0, 'accepted')", crate::params![&job_invalid.id]).await.unwrap();
    // Directly insert an invoice for client-invalid to simulate historical/legacy data with invalid personal number
    let now_date = crate::infra::time::get_current_datetime_str();
    conn.execute(
        "INSERT INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, tax_authority_amount, customer_amount, status, updated_at) VALUES ('inv-invalid', 'ws-batch-resil-test', 'quote-invalid', 'client-invalid', ?1, ?1, 1000.0, 500.0, 500.0, 500.0, 'issued', 100000)",
        crate::params![&now_date],
    ).await.unwrap();

    // Export batch containing BOTH valid and invalid invoices
    let batch_ids = vec![inv_valid.id.clone(), "inv-invalid".to_string()];
    let xml = export_skatteverket_claims("u-resil-staff".to_string(), batch_ids.clone(), "xml".to_string()).await.unwrap();
    
    // Assert XML exported valid claim and skipped invalid claim without failing batch
    assert!(xml.contains("198112189876"));
    assert!(!xml.contains("invalid-pnum"));

    let csv = export_skatteverket_claims("u-resil-staff".to_string(), batch_ids, "csv".to_string()).await.unwrap();
    assert!(csv.contains("198112189876"));
    assert!(!csv.contains("invalid-pnum"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-batch-resil-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-batch-resil-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-batch-resil-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-batch-resil-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-batch-resil-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_skatteverket_export_validation_failures() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user with mock settings (missing or wrong personal number)
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-fail-test', 'Fail WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fail-staff', 'ws-fail-test', 'staff@fail.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-fail-1', 'ws-fail-test', 'client@fail.io', 'client', '{}')", ()).await.unwrap(); // Empty metadata -> missing personal number

    // 2. Setup job ticket & quote
    let job = create_job_ticket(
        "u-fail-staff".to_string(),
        "ws-fail-test".to_string(),
        "Fail Move".to_string(),
        "Testing Skatteverket fail path".to_string(),
        "Start Address".to_string(),
        "high".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        Some("Start Address".to_string()),
        Some("End Address".to_string()),
        0,
        0,
        true,
        true,
        false,
        false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-fail-1', 'ws-fail-test', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    // 3. Invoice generation should fail upfront with ValidationError because client has no personal number
    let inv_result = generate_move_invoice("u-fail-staff".to_string(), "quote-fail-1".to_string(), true).await;
    assert!(inv_result.is_err());
    match inv_result {
        Err(YntraError::ValidationError(msg)) => {
            assert!(msg.contains("personal number"));
        }
        _ => panic!("Expected ValidationError due to missing personal number"),
    }

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-fail-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-fail-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-fail-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fail-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-fail-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_skatteverket_direct_submission() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-direct-test', 'Direct RUT WS', '[\"moving_company\"]', '{\"skatteverket_api_url\":\"https://test.skatteverket.se/service/rotrut/v6\"}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-direct-staff', 'ws-direct-test', 'staff@direct.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-direct-1', 'ws-direct-test', 'client@direct.io', 'client', '{\"personal_number\":\"19811218-9876\"}')", ()).await.unwrap();

    // 2. Create job, quote, and invoice with RUT deduction
    let job = create_job_ticket(
        "u-direct-staff".to_string(),
        "ws-direct-test".to_string(),
        "Direct Move".to_string(),
        "Testing direct Skatteverket submission".to_string(),
        "Start Address".to_string(),
        "high".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        Some("Start Address".to_string()),
        Some("End Address".to_string()),
        0,
        0,
        true,
        true,
        false,
        false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('quote-direct-1', 'ws-direct-test', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    let inv = generate_move_invoice("u-direct-staff".to_string(), "quote-direct-1".to_string(), true).await.unwrap();
    assert_eq!(inv.status, "unpaid");

    // Pay invoice
    pay_move_invoice("u-direct-staff".to_string(), inv.id.clone()).await.unwrap();

    // 3. Initiate BankID session for Skatteverket
    let session = initiate_bankid_skatteverket_session("u-direct-staff".to_string()).await.unwrap();
    assert_eq!(session.challenge, Some("skatteverket-rut-signing".to_string()));

    // 4. Submit claim directly without setting status to success - should fail with AuthError
    let result_pending_err = submit_skatteverket_claim_direct("u-direct-staff".to_string(), session.id.clone(), vec![inv.id.clone()]).await;
    assert!(result_pending_err.is_err());
    assert!(matches!(result_pending_err.unwrap_err(), YntraError::AuthError(_)));

    // Update status to success
    conn.execute("UPDATE bankid_auth_sessions SET status = 'success' WHERE id = ?1", crate::params![&session.id]).await.unwrap();

    // 4b. Submit claim directly without certificate - should fail with ValidationError
    let result_err = submit_skatteverket_claim_direct("u-direct-staff".to_string(), session.id.clone(), vec![inv.id.clone()]).await;
    assert!(result_err.is_err());
    match result_err {
        Err(YntraError::ValidationError(msg)) => {
            assert!(msg.contains("Missing Skatteverket corporate certificate"));
        }
        _ => panic!("Expected ValidationError due to missing certificate"),
    }

    // 5. Add a dummy corporate certificate to settings
    conn.execute(
        "UPDATE workspaces SET settings = '{\"skatteverket_api_url\":\"https://test.skatteverket.se/service/rotrut/v6\",\"skatteverket_corporate_cert\":\"dummy_base64_cert\"}' WHERE id = 'ws-direct-test'",
        ()
    ).await.unwrap();

    // 6. Submit claim directly with certificate - should attempt connection and return failed status
    let result_ok = submit_skatteverket_claim_direct("u-direct-staff".to_string(), session.id, vec![inv.id.clone()]).await.unwrap();
    assert_eq!(result_ok.status, "failed");
    assert!(result_ok.message.contains("Skatteverket connection failed") || result_ok.message.contains("rejected"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-direct-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-direct-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-direct-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-direct-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-direct-test'", ()).await.unwrap();
}

#[test]
fn test_skatteverket_receipt_reference_parsing() {
    use crate::services::jobs::billing::extract_skatteverket_receipt_reference;

    // XML response format with <Mottagningsreferens>
    let xml_body = "<Svar><Mottagningsreferens>SKV-REC-2026-991234</Mottagningsreferens><Status>OK</Status></Svar>";
    assert_eq!(extract_skatteverket_receipt_reference(xml_body), "SKV-REC-2026-991234");

    // XML response format with <Journalnummer>
    let xml_jn_body = "<Response><Journalnummer>JN-8884920</Journalnummer></Response>";
    assert_eq!(extract_skatteverket_receipt_reference(xml_jn_body), "JN-8884920");

    // JSON response format with "mottagningsreferens"
    let json_body = r#"{"mottagningsreferens": "JSON-REC-10020", "status": "APPROVED"}"#;
    assert_eq!(extract_skatteverket_receipt_reference(json_body), "JSON-REC-10020");

    // JSON response format with "reference_number"
    let json_ref_body = r#"{"reference_number": "REF-994821"}"#;
    assert_eq!(extract_skatteverket_receipt_reference(json_ref_body), "REF-994821");

    // Plain text fallback
    let plain_body = "REF-DIRECT-STRING-1234";
    assert_eq!(extract_skatteverket_receipt_reference(plain_body), "REF-DIRECT-STRING-1234");
}

#[tokio::test]
async fn test_dynamic_tax_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace with US region settings (8% sales tax rate)
    let settings_us = serde_json::json!({
        "target_region": "US",
        "sales_tax_rate": 0.08,
    }).to_string();
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-tax-us', 'US Tax WS', '[\"moving_company\"]', ?1)", crate::params![&settings_us]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-tax-us', 'ws-tax-us', 'staff@us.io', 'admin')", ()).await.unwrap();

    let job_us = create_job_ticket("u-tax-us".to_string(), "ws-tax-us".to_string(), "US Move".to_string(), "desc".to_string(), "Addr".to_string(), "medium".to_string(), None, "2026-08-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false).await.unwrap();
    conn.execute("INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('q-tax-us', 'ws-tax-us', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')", crate::params![&job_us.id]).await.unwrap();

    let inv_us = generate_move_invoice("u-tax-us".to_string(), "q-tax-us".to_string(), false).await.unwrap();
    // Subtotal = 1500.0, Tax (8%) = 120.0, Total customer_amount = 1620.0
    assert_eq!(inv_us.currency, "USD");
    assert_eq!(inv_us.rut_deduction, 0.0);
    assert_eq!(inv_us.tax_authority_amount, 120.0);
    assert_eq!(inv_us.customer_amount, 1620.0);

    // 2. Setup workspace with DE region settings (19% VAT rate)
    let settings_de = serde_json::json!({
        "target_region": "DE",
        "vat_rate": 0.19,
    }).to_string();
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-tax-de', 'DE Tax WS', '[\"moving_company\"]', ?1)", crate::params![&settings_de]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-tax-de', 'ws-tax-de', 'staff@de.io', 'admin')", ()).await.unwrap();

    let job_de = create_job_ticket("u-tax-de".to_string(), "ws-tax-de".to_string(), "DE Move".to_string(), "desc".to_string(), "Addr".to_string(), "medium".to_string(), None, "2026-08-01".to_string(), "[]".to_string(), None, None, 0, 0, true, true, false, false).await.unwrap();
    conn.execute("INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('q-tax-de', 'ws-tax-de', ?1, 1000.0, 500.0, 0.0, 0.0, 1500.0, 'accepted')", crate::params![&job_de.id]).await.unwrap();

    let inv_de = generate_move_invoice("u-tax-de".to_string(), "q-tax-de".to_string(), false).await.unwrap();
    // Subtotal = 1500.0, Tax (19%) = 285.0, Total customer_amount = 1785.0
    assert_eq!(inv_de.currency, "EUR");
    assert_eq!(inv_de.rut_deduction, 0.0);
    assert_eq!(inv_de.tax_authority_amount, 285.0);
    assert_eq!(inv_de.customer_amount, 1785.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id IN ('ws-tax-us', 'ws-tax-de')", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id IN ('ws-tax-us', 'ws-tax-de')", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id IN ('ws-tax-us', 'ws-tax-de')", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-tax-us', 'ws-tax-de')", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id IN ('ws-tax-us', 'ws-tax-de')", ()).await.unwrap();
}

#[tokio::test]
async fn test_long_carry_and_toll_surcharges() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-carry-test', 'Carry WS', '[\"moving_company\"]', '{\"moving_base_rate_per_m3\":100.0,\"surcharge_long_carry_per_meter\":50.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-carry-staff', 'ws-carry-test', 'staff@carry.io', 'admin')", ()).await.unwrap();

    // 2. Setup job ticket with 20 meters of long carry and 350.0 SEK in toll fees (1 item of 1.0 m3 volume)
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters, toll_fees) VALUES ('job-carry-test', 'ws-carry-test', 'Carry Job', 'Desc', 'Addr', 'medium', 'quote_requested', '2026-07-21', '[]', ?1, ?2, 0, 0, 1, 1, 20, 350.0)",
        crate::params![now_ms, now_ms]
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-box', 'ws-carry-test', 'job-carry-test', 'Möbler', 'Box', 1, 1.0)", ()).await.unwrap();

    // 3. Calculate move quote
    calculate_and_save_move_quote(
        "u-carry-staff".to_string(),
        "job-carry-test".to_string(),
    ).await.unwrap();

    // 4. Retrieve quote and verify surcharges
    // Base price = 1.0 * 100.0 = 100
    // Stairs surcharge = long_carry_meters (20) * rate (50.0) = 1000
    // Distance fee = distance default (800) + tolls (350) = 1150
    // Packing supplies fee = 1.0 * 100 = 100
    // Expected total_price = 100 + 1000 + 1150 + 100 = 2350
    let quote = get_move_quote(
        "u-carry-staff".to_string(),
        "job-carry-test".to_string(),
    ).await.unwrap().unwrap();

    assert_eq!(quote.stairs_surcharge, 1000.0);
    assert_eq!(quote.distance_fee, 1150.0);
    assert_eq!(quote.total_price, 2350.0);

    // 5. Clean up
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-carry-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-carry-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-carry-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-carry-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-carry-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_crew_size_pricing_adjustments() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace with hourly pricing configuration
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-crew-test', 'Crew WS', '[\"moving_company\"]', '{\"moving_pricing_model\":\"hourly\",\"moving_default_crew_size\":2,\"moving_hourly_rate_per_mover\":450.0,\"moving_hourly_rate_vehicle\":300.0,\"moving_hours_per_m3\":0.15,\"moving_minimum_hours\":2.0}')",
        ()
    ).await.unwrap();

    // Staff and crew users
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff', 'ws-crew-test', 'staff@crew.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-m1', 'ws-crew-test', 'm1@crew.io', 'mover')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-m2', 'ws-crew-test', 'm2@crew.io', 'mover')", ()).await.unwrap();

    // 2. Setup job ticket (10.0 m3 volume -> estimated hours = 10.0 * 0.15 = 1.5, capped at 2.0 minimum)
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters, toll_fees) VALUES ('job-crew-test', 'ws-crew-test', 'Crew Job', 'Desc', 'Addr', 'medium', 'quote_requested', '2026-07-21', '[]', ?1, ?2, 0, 0, 1, 1, 0, 0.0)",
        crate::params![now_ms, now_ms]
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-sofa', 'ws-crew-test', 'job-crew-test', 'Möbler', 'Sofa', 1, 10.0)", ()).await.unwrap();

    // 3. Calculate initial quote with 0 assigned crew (should use default_crew_size = 2)
    // Hourly rate = 2 * 450 + 300 = 1200 SEK
    // Base price = 2.0 hours * 1200 = 2400 SEK
    calculate_and_save_move_quote(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
    ).await.unwrap();

    let quote1 = get_move_quote(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote1.base_price, 2400.0);

    // 4. Assign 1 crew member (reactive trigger recalculates the quote)
    // Assigned crew count = 1 -> Hourly rate = 1 * 450 + 300 = 750 SEK
    // Base price = 2.0 hours * 750 = 1500 SEK
    assign_vehicle_to_job(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
        Some("vehicle-1".to_string()), // dummy vehicle
    ).await.unwrap_or_default(); // Ignore capacity validation details

    add_crew_member(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
        "u-crew-m1".to_string(),
        "driver".to_string(),
    ).await.unwrap();

    let quote2 = get_move_quote(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote2.base_price, 1500.0);

    // 5. Assign a second crew member
    // Assigned crew count = 2 -> Hourly rate = 2 * 450 + 300 = 1200 SEK
    // Base price = 2.0 hours * 1200 = 2400 SEK
    add_crew_member(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
        "u-crew-m2".to_string(),
        "mover".to_string(),
    ).await.unwrap();

    let quote3 = get_move_quote(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote3.base_price, 2400.0);

    // 6. Remove one crew member
    // Assigned crew count = 1 -> Hourly rate = 750 SEK
    // Base price = 1500 SEK
    remove_crew_member(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
        "u-crew-m2".to_string(),
    ).await.unwrap();

    let quote4 = get_move_quote(
        "u-crew-staff".to_string(),
        "job-crew-test".to_string(),
    ).await.unwrap().unwrap();
    assert_eq!(quote4.base_price, 1500.0);

    // 7. Clean up
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_crew WHERE job_ticket_id = 'job-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-crew-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_adjust_invoice_for_actuals() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user with hourly model
    let settings = serde_json::json!({
        "company_country": "SE",
        "target_region": "SE",
        "moving_pricing_model": "hourly",
        "moving_default_crew_size": 2.0,
        "moving_hourly_rate_per_mover": 450.0,
        "moving_hourly_rate_vehicle": 300.0,
        "use_rut_deduction": true,
    }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-adj-test', 'Adj WS', '[\"moving_company\"]', ?1)", crate::params![&settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-adj-staff', 'ws-adj-test', 'staff@adj.se', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-adj', 'ws-adj-test', 'client@adj.se', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    // 2. Create job ticket, quote, and initial invoice
    let job = create_job_ticket(
        "u-adj-staff".to_string(),
        "ws-adj-test".to_string(),
        "Relocation job".to_string(),
        "Testing actuals adjustment".to_string(),
        "Start Addr".to_string(),
        "medium".to_string(),
        None,
        "2026-08-01".to_string(),
        "[]".to_string(),
        Some("Start Addr".to_string()),
        Some("End Addr".to_string()),
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES ('q-adj-1', 'ws-adj-test', ?1, 1800.0, 500.0, 200.0, 100.0, 2600.0, 'accepted')",
        crate::params![&job.id]
    ).await.unwrap();

    let initial_inv = generate_move_invoice("u-adj-staff".to_string(), "q-adj-1".to_string(), true).await.unwrap();

    // 3. Log actual hours and additional charges, adjusting final invoice
    let adjusted_inv = adjust_invoice_for_actuals(
        "u-adj-staff".to_string(),
        initial_inv.id.clone(),
        Some(6.0),
        Some(300.0),
        Some("Actual hours: 6. Disassembly surcharge added.".to_string()),
    ).await.unwrap();

    assert_eq!(adjusted_inv.subtotal, 8300.0);
    assert_eq!(adjusted_inv.rut_deduction, 2800.0);
    assert_eq!(adjusted_inv.customer_amount, 5500.0);
    assert_eq!(adjusted_inv.actual_hours, Some(6.0));
    assert_eq!(adjusted_inv.additional_charges, Some(300.0));
    assert_eq!(adjusted_inv.adjustment_notes, Some("Actual hours: 6. Disassembly surcharge added.".to_string()));

    // Verify it persists in database by querying get_move_invoice
    let fetched_inv = get_move_invoice("u-adj-staff".to_string(), "q-adj-1".to_string()).await.unwrap().unwrap();
    assert_eq!(fetched_inv.subtotal, 8300.0);
    assert_eq!(fetched_inv.rut_deduction, 2800.0);
    assert_eq!(fetched_inv.actual_hours, Some(6.0));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-adj-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_onsite_mpos_card_payment_processing() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-onsite-test', 'OnSite Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-driver-1', 'ws-onsite-test', 'driver@onsite.io', 'staff')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-cust-1', 'ws-onsite-test', 'client@onsite.io', 'client')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-driver-1".to_string(),
        "ws-onsite-test".to_string(),
        "On-Site Job".to_string(),
        "Collect payment on completion".to_string(),
        "Origin Address".to_string(),
        "medium".to_string(),
        Some("u-cust-1".to_string()),
        "2026-09-20".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    let quote_id = "q-onsite-1".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-onsite-test', ?2, 2500.0, 0.0, 0.0, 0.0, 2500.0, 'accepted', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    let inv = generate_move_invoice("u-cust-1".to_string(), quote_id, false).await.unwrap();
    assert_eq!(inv.status, "unpaid");

    let payment_res = process_onsite_mpos_card_payment(
        "u-driver-1".to_string(),
        inv.id.clone(),
        "stripe_terminal".to_string(),
        Some("STRIPE-READER-999".to_string()),
    )
    .await
    .unwrap();

    assert!(payment_res.success);
    assert_eq!(payment_res.amount_collected, 2500.0);
    assert_eq!(payment_res.payment_method, "Stripe Tap-to-Pay / Card Reader");

    let updated_inv = get_move_invoice("u-driver-1".to_string(), "q-onsite-1".to_string()).await.unwrap().unwrap();
    assert_eq!(updated_inv.status, "paid");
    assert!(updated_inv.adjustment_notes.unwrap().contains("Stripe Tap-to-Pay"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-onsite-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-onsite-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-onsite-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-onsite-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-onsite-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_accounting_erp_sync_and_reconciliation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-erp-test', 'ERP Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-erp-staff', 'ws-erp-test', 'staff@erp.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-erp-client', 'ws-erp-test', 'client@erp.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-erp-staff".to_string(),
        "ws-erp-test".to_string(),
        "ERP Move".to_string(),
        "Test ERP synchronization".to_string(),
        "Start Address".to_string(),
        "medium".to_string(),
        Some("u-erp-client".to_string()),
        "2026-11-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, true, true, false, false,
    )
    .await
    .unwrap();

    let quote_id = "q-erp-1".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-erp-test', ?2, 4000.0, 500.0, 300.0, 200.0, 5000.0, 'accepted', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    let inv = generate_move_invoice("u-erp-client".to_string(), quote_id, true).await.unwrap();
    assert_eq!(inv.status, "unpaid");

    // Test Fortnox ERP Sync
    let fortnox_res = sync_invoice_to_erp("u-erp-staff".to_string(), inv.id.clone(), "fortnox".to_string()).await.unwrap();
    assert!(fortnox_res.success);
    assert_eq!(fortnox_res.ledger_account, "3050_MOVING_SERVICES");
    assert!(fortnox_res.erp_invoice_number.starts_with("FORTNOX-INV-"));

    // Test Automated ERP Payment Reconciliation
    let reconciled_count = reconcile_erp_payments("u-erp-staff".to_string(), "fortnox".to_string()).await.unwrap();
    assert_eq!(reconciled_count, 1);

    let updated_inv = get_move_invoice("u-erp-staff".to_string(), "q-erp-1".to_string()).await.unwrap().unwrap();
    assert_eq!(updated_inv.status, "paid");
    assert_eq!(updated_inv.adjustment_notes, Some("Reconciled from ERP bank ledger".to_string()));

    // Cleanup
    conn.execute("DELETE FROM erp_sync_logs WHERE workspace_id = 'ws-erp-test'", ()).await.ok();
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-erp-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-erp-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-erp-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-erp-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-erp-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_annual_personal_rut_cap_enforcement() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rut-cap-test', 'RUT Cap WS', '[\"moving_company\"]', '{\"target_region\":\"SE\",\"moving_base_rate_per_m3\":1000.0,\"annual_rut_limit_per_person\":75000.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-staff', 'ws-rut-cap-test', 'staff@rutcap.se', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-cust', 'ws-rut-cap-test', 'customer@rutcap.se', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-rut-old', 'ws-rut-cap-test', 'Old Job', 'Desc', 'Addr', 'medium', 'completed', '2026-03-15', '[]', '2026-03-15', 100)",
        ()
    ).await.unwrap();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-rut-new', 'ws-rut-cap-test', 'New Job', 'Desc', 'Addr', 'medium', 'open', '2026-09-01', '[]', '2026-09-01', 100)",
        ()
    ).await.unwrap();

    // Insert an existing invoice for this customer in 2026 claiming 70,000 SEK of RUT
    let current_year = chrono::Utc::now().format("%Y").to_string();
    let past_inv_date = format!("{}-03-15", current_year);

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-rut-old', 'ws-rut-cap-test', 'job-rut-old', 140000.0, 0.0, 0.0, 0.0, 140000.0, 'sent', 100, 'pending')",
        ()
    ).await.unwrap();

    conn.execute(
        "INSERT INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES ('inv-rut-old', 'ws-rut-cap-test', 'q-rut-old', 'u-rut-cust', ?1, ?1, 140000.0, 70000.0, 70000.0, 70000.0, 'paid', 100, 'pending')",
        crate::params![&past_inv_date]
    ).await.unwrap();

    // Now create a new quote for 20,000 SEK labor base_price (raw RUT would be 0.5 * 20000 = 10,000 SEK)
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-rut-new', 'ws-rut-cap-test', 'job-rut-new', 20000.0, 0.0, 0.0, 0.0, 20000.0, 'sent', 100, 'pending')",
        ()
    ).await.unwrap();

    // Generate new invoice for the customer
    let new_inv = generate_move_invoice("u-rut-cust".to_string(), "q-rut-new".to_string(), true).await.unwrap();

    // Since customer used 70,000 SEK out of 75,000 SEK cap, remaining cap is 5,000 SEK!
    // Raw RUT was 10,000 SEK, but rut_deduction must be capped at 5,000.0 SEK!
    assert_eq!(new_inv.rut_deduction, 5000.0);
    assert_eq!(new_inv.customer_amount, 15000.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-rut-cap-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-rut-cap-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-rut-cap-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rut-cap-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-cap-test'", ()).await.ok();
}

#[tokio::test]
async fn test_annual_rut_cap_race_condition_pending_accepted_quotes() {
    use crate::services::jobs::billing::calculate_customer_annual_rut_used;
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rut-pending-test', 'RUT Pending WS', '[\"moving_company\"]', '{\"target_region\":\"SE\",\"moving_base_rate_per_m3\":1000.0,\"annual_rut_limit_per_person\":75000.0}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-p-staff', 'ws-rut-pending-test', 'staff@rutp.se', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-p-cust', 'ws-rut-pending-test', 'customer@rutp.se', 'client', '{\"personal_number\":\"19811218-9876\"}')", ()).await.unwrap();

    let current_year = chrono::Utc::now().format("%Y").to_string();
    let sched_date = format!("{}-06-15", current_year);

    // Job 1 & accepted quote for 100,000 SEK base price (RUT deduction portion = 0.5 * (100000 * 0.70) = 35,000 SEK)
    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, assigned_user_id) VALUES ('job-p-1', 'ws-rut-pending-test', 'Job 1', 'Desc', 'Addr', 'medium', 'open', ?1, '[]', 100, 100, 'u-rut-p-cust')",
        crate::params![&sched_date]
    ).await.unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-p-1', 'ws-rut-pending-test', 'job-p-1', 100000.0, 0.0, 0.0, 0.0, 100000.0, 'accepted', 100, 'pending')",
        ()
    ).await.unwrap();

    // Existing billed invoice for 30,000 SEK RUT
    let inv_date = format!("{}-02-10", current_year);
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-p-billed', 'ws-rut-pending-test', 'job-p-1', 60000.0, 0.0, 0.0, 0.0, 60000.0, 'accepted', 100, 'pending')",
        ()
    ).await.unwrap();
    conn.execute(
        "INSERT INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES ('inv-p-billed', 'ws-rut-pending-test', 'q-p-billed', 'u-rut-p-cust', ?1, ?1, 60000.0, 30000.0, 30000.0, 30000.0, 'paid', 100, 'pending')",
        crate::params![&inv_date]
    ).await.unwrap();

    // Now calculate customer annual RUT used for 2026:
    // 30,000 (billed invoice) + 35,000 (accepted unbilled quote q-p-1) = 65,000 SEK total used RUT!
    let used_rut = calculate_customer_annual_rut_used(&conn, "ws-rut-pending-test", "u-rut-p-cust", &current_year, None).await.unwrap();
    assert_eq!(used_rut, 65000.0);

    // Create a 2nd new job & quote for 40,000 SEK base price (raw RUT would be 14,000 SEK)
    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, assigned_user_id) VALUES ('job-p-2', 'ws-rut-pending-test', 'Job 2', 'Desc', 'Addr', 'medium', 'open', ?1, '[]', 100, 100, 'u-rut-p-cust')",
        crate::params![&sched_date]
    ).await.unwrap();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-p-2', 'ws-rut-pending-test', 'job-p-2', 40000.0, 0.0, 0.0, 0.0, 40000.0, 'sent', 100, 'pending')",
        ()
    ).await.unwrap();

    // Generate invoice for q-p-2
    let inv2 = generate_move_invoice("u-rut-p-cust".to_string(), "q-p-2".to_string(), true).await.unwrap();

    // 75,000 limit - 65,000 used = 10,000 remaining RUT cap.
    // Raw RUT was 14,000 SEK, but rut_deduction MUST be capped at 10,000 SEK!
    assert_eq!(inv2.rut_deduction, 10000.0);
    assert_eq!(inv2.customer_amount, 30000.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-rut-pending-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-rut-pending-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-rut-pending-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rut-pending-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-pending-test'", ()).await.ok();
}

#[tokio::test]
async fn test_custom_flat_pricing_zero_volume_rut_calculation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-flat-test', 'Flat Price WS', '[\"moving_company\"]', '{\"target_region\":\"SE\",\"moving_pricing_model\":\"volume\",\"moving_labor_ratio_volume\":0.70}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-flat-staff', 'ws-flat-test', 'staff@flat.se', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-flat-cust', 'ws-flat-test', 'cust@flat.se', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-flat-1', 'ws-flat-test', 'Flat Job', 'No itemized volume', 'Addr', 'medium', 'open', '2026-09-01', '[]', '2026-09-01', 100)",
        ()
    ).await.unwrap();

    // Flat price agreement: base_price = 8000.0 SEK, total_price = 8000.0 SEK. Zero inventory items added.
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-flat-1', 'ws-flat-test', 'job-flat-1', 8000.0, 0.0, 0.0, 0.0, 8000.0, 'sent', 100, 'pending')",
        ()
    ).await.unwrap();

    let inv = generate_move_invoice("u-flat-cust".to_string(), "q-flat-1".to_string(), true).await.unwrap();

    // Eligible labor = 8000 * 0.70 = 5600.0 SEK.
    // RUT deduction = 50% * 5600.0 = 2800.0 SEK.
    // Customer amount = 8000.0 - 2800.0 = 5200.0 SEK.
    assert_eq!(inv.subtotal, 8000.0);
    assert_eq!(inv.rut_deduction, 2800.0);
    assert_eq!(inv.customer_amount, 5200.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-flat-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-flat-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-flat-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-flat-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-flat-test'", ()).await.ok();
}

#[tokio::test]
async fn test_unvalidated_personal_number_rejection_at_intake() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Direct validation helper check
    let valid_pnum = validate_customer_personal_number_for_rut("19811218-9876");
    assert!(valid_pnum.is_ok());
    assert_eq!(valid_pnum.unwrap(), "198112189876");

    let invalid_pnum_checksum = validate_customer_personal_number_for_rut("19811218-0000");
    assert!(invalid_pnum_checksum.is_err());
    assert!(matches!(invalid_pnum_checksum.unwrap_err(), YntraError::ValidationError(_)));

    let invalid_pnum_format = validate_customer_personal_number_for_rut("invalid-pnum-123");
    assert!(invalid_pnum_format.is_err());

    // 2. Intake validation check during invoice generation
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-badpnum-test', 'Bad Pnum WS', '[\"moving_company\"]', '{\"target_region\":\"SE\"}')",
        ()
    ).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-badpnum-staff', 'ws-badpnum-test', 'staff@bad.se', 'admin')", ()).await.unwrap();
    // Insert client with invalid personal number (failed Luhn checksum)
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-badpnum-cust', 'ws-badpnum-test', 'cust@bad.se', 'client', '{\"personal_number\":\"19811218-0000\"}')", ()).await.unwrap();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-badpnum-1', 'ws-badpnum-test', 'Bad Pnum Job', 'Desc', 'Addr', 'medium', 'open', '2026-09-01', '[]', '2026-09-01', 100)",
        ()
    ).await.unwrap();

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-badpnum-1', 'ws-badpnum-test', 'job-badpnum-1', 4000.0, 0.0, 0.0, 0.0, 4000.0, 'sent', 100, 'pending')",
        ()
    ).await.unwrap();

    let result = generate_move_invoice("u-badpnum-cust".to_string(), "q-badpnum-1".to_string(), true).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        YntraError::ValidationError(msg) => {
            assert!(msg.contains("Invalid or missing Swedish personal number"));
        }
        _ => panic!("Expected ValidationError for invalid personal number at invoice intake"),
    }

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-badpnum-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-badpnum-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-badpnum-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-badpnum-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-badpnum-test'", ()).await.ok();
}

#[tokio::test]
async fn test_unconfigured_stripe_rejection_and_client_pay_invoice_rbac() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-uncfg-test', 'Uncfg WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-uncfg-cust', 'ws-uncfg-test', 'cust@uncfg.se', 'client')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-uncfg-staff', 'ws-uncfg-test', 'staff@uncfg.se', 'staff')", ()).await.unwrap();

    conn.execute("INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-uncfg-1', 'ws-uncfg-test', 'Job', 'Desc', 'Loc', 'normal', 'completed', '2026-08-01', '[]', 100, 100)", ()).await.unwrap();
    conn.execute("INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-uncfg-1', 'ws-uncfg-test', 'job-uncfg-1', 5000.0, 0.0, 0.0, 0.0, 5000.0, 'accepted', 100, 'synced')", ()).await.unwrap();
    conn.execute("INSERT INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES ('inv-uncfg-1', 'ws-uncfg-test', 'q-uncfg-1', 'u-uncfg-cust', '2026-08-01', '2026-08-15', 5000.0, 0.0, 5000.0, 0.0, 'unpaid', 100, 'synced')", ()).await.unwrap();

    // 1. Unconfigured Stripe payment call must fail with ValidationError
    let stripe_err = initiate_stripe_payment("u-uncfg-cust".to_string(), "inv-uncfg-1".to_string()).await;
    assert!(stripe_err.is_err());
    assert!(matches!(stripe_err.unwrap_err(), YntraError::ValidationError(_)));

    // 2. Direct pay_move_invoice call by non-staff client must fail with AuthError
    let pay_err = pay_move_invoice("u-uncfg-cust".to_string(), "inv-uncfg-1".to_string()).await;
    assert!(pay_err.is_err());
    assert!(matches!(pay_err.unwrap_err(), YntraError::AuthError(_)));

    // 3. Direct pay_move_invoice call by staff must succeed
    let pay_ok = pay_move_invoice("u-uncfg-staff".to_string(), "inv-uncfg-1".to_string()).await;
    assert!(pay_ok.is_ok());

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-uncfg-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-uncfg-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-uncfg-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-uncfg-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-uncfg-test'", ()).await.ok();
}

#[tokio::test]
async fn test_ineligible_rut_deduction_skatteverket_compliance() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let settings = r#"{"moving_pricing_model":"flat","moving_base_rate_per_m3":1000.0,"moving_stairs_surcharge_per_floor":300.0,"surcharge_long_carry_per_meter":50.0,"surcharge_crane_hoist":1500.0,"moving_labor_ratio_volume":1.0}"#;
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rut-skat-test', 'RUT Skatteverket WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-skat-staff', 'ws-rut-skat-test', 'staff@skat.se', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-skat-client', 'ws-rut-skat-test', 'client@skat.se', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-rut-skat-staff".to_string(),
        "ws-rut-skat-test".to_string(),
        "RUT Skatt Move".to_string(),
        "Desc".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        Some("u-rut-skat-client".to_string()),
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        2, 0, false, true, false, false,
    ).await.unwrap();

    conn.execute("UPDATE job_tickets SET long_carry_meters = 20 WHERE id = ?1", crate::params![&job.id]).await.unwrap();

    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-rut-skat-1', 'ws-rut-skat-test', ?1, 'Möbler', 'Standard Table', 1, 2.0)", crate::params![&job.id]).await.unwrap();

    calculate_and_save_move_quote("u-rut-skat-staff".to_string(), job.id.clone()).await.unwrap();

    let quote = get_move_quote("u-rut-skat-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    let inv = generate_move_invoice("u-rut-skat-staff".to_string(), quote.id, true).await.unwrap();

    // Total eligible labor = 2300 (base labor) + 600 (eligible stair carrying labor) = 2900 SEK
    // Expected RUT deduction = 50% * 2900 = 1450 SEK
    // (Note: Long carry 1000 SEK and equipment rentals are non-deductible under Skatteverket rules and strictly excluded from RUT!)
    assert_eq!(inv.rut_deduction, 1450.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-rut-skat-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-rut-skat-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-rut-skat-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-rut-skat-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rut-skat-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-skat-test'", ()).await.ok();
}

#[tokio::test]
async fn test_non_deductible_equipment_clipping_with_zero_stairs() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let settings = serde_json::json!({
        "moving_base_rate_per_m3": 1000.0,
        "surcharge_crane_hoist": 1500.0,
        "requires_crane_hoist": true
    }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-nonded-test', 'NonDed WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-nonded-staff', 'ws-nonded-test', 'staff@nonded.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-nonded-client', 'ws-nonded-test', 'client@nonded.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-nonded-staff".to_string(),
        "ws-nonded-test".to_string(),
        "Crane Move".to_string(),
        "Desc".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        Some("u-nonded-client".to_string()),
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, false, false, false, false, // Ground floor, elevator = false -> 0 stair carrying surcharge
    ).await.unwrap();

    // Set 4 m3 volume -> base_price = 4000 SEK (of which 70% = 2800 SEK is eligible labor)
    conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-nonded-1', 'ws-nonded-test', ?1, 'Möbler', 'Soffa', 1, 4.0)", crate::params![&job.id]).await.unwrap();

    calculate_and_save_move_quote("u-nonded-staff".to_string(), job.id.clone()).await.unwrap();
    let quote = get_move_quote("u-nonded-staff".to_string(), job.id.clone()).await.unwrap().unwrap();

    let inv = generate_move_invoice("u-nonded-staff".to_string(), quote.id, true).await.unwrap();

    // Eligible labor = 3220 SEK (70% of peak-season base labor 4600 SEK).
    // Crane hoist surcharge (1500 SEK) is non-deductible under Skatteverket rules and strictly excluded from RUT.
    // Total RUT-eligible labor = (3220 + 1500 - 1500) = 3220 SEK.
    // Expected RUT deduction = 50% * 3220 = 1610 SEK.
    assert_eq!(inv.rut_deduction, 1610.0);

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-nonded-test'", ()).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-nonded-test'", ()).await.ok();
    conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-nonded-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-nonded-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-nonded-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-nonded-test'", ()).await.ok();
}

#[tokio::test]
async fn test_annual_rut_used_with_hourly_pricing_model() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let settings = serde_json::json!({
        "moving_pricing_model": "hourly",
        "moving_hourly_rate_per_mover": 400.0,
        "moving_hourly_rate_vehicle": 400.0,
        "moving_default_crew_size": 2.0
    }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rut-hr-test', 'RUT HR WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-hr-staff', 'ws-rut-hr-test', 'staff@ruthr.io', 'admin', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rut-hr-client', 'ws-rut-hr-test', 'client@ruthr.io', 'client', '{\"personal_number\":\"198112189876\"}')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-rut-hr-staff".to_string(),
        "ws-rut-hr-test".to_string(),
        "Hourly Move".to_string(),
        "Desc".to_string(),
        "Addr".to_string(),
        "medium".to_string(),
        Some("u-rut-hr-client".to_string()),
        "2026-10-01".to_string(),
        "[]".to_string(),
        None,
        None,
        0, 0, false, false, false, false,
    ).await.unwrap();

    // Accepted quote with 3000 SEK base price
    // Under hourly model with 2 movers ($400/hr each = $800) + vehicle ($400/hr), mover ratio = 800 / 1200 = 66.6667%
    // Eligible labor = 3000 * (800/1200) = 2000.0 SEK.
    // Committed RUT = 50% * 2000.0 = 1000.0 SEK.
    let quote_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-rut-hr-test', ?2, 3000.0, 0.0, 0.0, 0.0, 3000.0, 'accepted', 123456)",
        crate::params![&quote_id, &job.id],
    ).await.unwrap();

    let used_rut = crate::services::jobs::billing::calculate_customer_annual_rut_used(&conn, "ws-rut-hr-test", "u-rut-hr-client", "2026", None).await.unwrap();

    assert_eq!(used_rut, 1000.0);

    // Cleanup
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-rut-hr-test'", ()).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-rut-hr-test'", ()).await.ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rut-hr-test'", ()).await.ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-hr-test'", ()).await.ok();
}

