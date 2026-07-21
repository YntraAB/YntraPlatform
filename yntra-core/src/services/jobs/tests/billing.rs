use crate::database;
use crate::YntraError;
use crate::services::jobs::{
    create_job_ticket, create_move_inventory_item,
    calculate_and_save_move_quote, get_move_quote,
    generate_move_invoice, get_move_invoice, pay_move_invoice,
    initiate_swish_payment, check_swish_payment_status,
    process_swish_payment_webhook, process_stripe_payment_webhook,
    export_skatteverket_claims, get_rut_invoices,
    initiate_bankid_skatteverket_session, submit_skatteverket_claim_direct,
    adjust_invoice_for_actuals,
    assign_vehicle_to_job, add_crew_member, remove_crew_member,
};

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
async fn test_hourly_pricing_calculations() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace with hourly pricing model
    let settings = r#"{"moving_pricing_model":"hourly","moving_hourly_rate":1200.0,"moving_distance_fee_flat":800.0,"moving_stairs_surcharge_per_floor":300.0,"moving_packing_supplies_fee_per_m3":100.0,"moving_hours_per_m3":0.15,"moving_minimum_hours":2.0,"moving_labor_ratio_hourly":0.70}"#;
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hourly-test', 'Hourly Test WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-hourly-staff', 'ws-hourly-test', 'staff@hourly.io', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-hourly-client', 'ws-hourly-test', 'client@hourly.io', 'client')", ()).await.unwrap();

    // Create job ticket: 3 floors, no elevator
    let job = create_job_ticket(
        "u-hourly-staff".to_string(),
        "ws-hourly-test".to_string(),
        "Hourly Relocation".to_string(),
        "Hourly move test".to_string(),
        "Main St 1".to_string(),
        "medium".to_string(),
        Some("u-hourly-client".to_string()),
        "2026-09-01".to_string(),
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
    assert_eq!(q.base_price, 2400);
    assert_eq!(q.distance_fee, 800);
    assert_eq!(q.stairs_surcharge, 900);
    assert_eq!(q.packing_supplies_fee, 1000);
    assert_eq!(q.total_price, 5100);

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
    assert!(xml.contains("<KoparePersnr>198808088888</KoparePersnr>"));
    assert!(xml.contains("<BegartBelopp>600</BegartBelopp>"));
    assert!(xml.contains("<RutArbete>"));
    assert!(xml.contains("<Flyttjanster>3</Flyttjanster>")); // minimum_hours = 3.0

    // 5. Export CSV
    let csv = export_skatteverket_claims("client-1".to_string(), vec![inv.id.clone()], "csv".to_string())
        .await
        .unwrap();
    
    assert!(csv.contains("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours"));
    assert!(csv.contains("556999-9999,198808088888"));
    assert!(csv.contains(",1200,600,3,3"));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE id = ?1", crate::params![&inv.id]).await.ok();
    conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
    conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE id = 'client-1'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-rut-test'", ()).await.unwrap();
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

    let inv = generate_move_invoice("u-fail-staff".to_string(), "quote-fail-1".to_string(), true).await.unwrap();

    // 3. Export CSV - should fail with ValidationError because client has no personal number
    let result_csv = export_skatteverket_claims("u-fail-staff".to_string(), vec![inv.id.clone()], "csv".to_string()).await;
    assert!(result_csv.is_err());
    match result_csv {
        Err(YntraError::ValidationError(msg)) => {
            assert!(msg.contains("personal number is missing"));
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
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('client-direct-1', 'ws-direct-test', 'client@direct.io', 'client', '{\"personal_number\":\"19900101-1234\"}')", ()).await.unwrap();

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

    assert_eq!(quote.stairs_surcharge, 1000);
    assert_eq!(quote.distance_fee, 1150);
    assert_eq!(quote.total_price, 2350);

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
    assert_eq!(quote1.base_price, 2400);

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
    assert_eq!(quote2.base_price, 1500);

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
    assert_eq!(quote3.base_price, 2400);

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
    assert_eq!(quote4.base_price, 1500);

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
        "moving_active_crew_size": 2.0,
        "moving_hourly_rate_per_mover": 450.0,
        "use_rut_deduction": true,
    }).to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-adj-test', 'Adj WS', '[\"moving_company\"]', ?1)", crate::params![&settings]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-adj-staff', 'ws-adj-test', 'staff@adj.se', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('client-adj', 'ws-adj-test', 'client@adj.se', 'client')", ()).await.unwrap();

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

    assert_eq!(adjusted_inv.subtotal, 6500.0);
    assert_eq!(adjusted_inv.rut_deduction, 2800.0);
    assert_eq!(adjusted_inv.customer_amount, 3700.0);
    assert_eq!(adjusted_inv.actual_hours, Some(6.0));
    assert_eq!(adjusted_inv.additional_charges, Some(300.0));
    assert_eq!(adjusted_inv.adjustment_notes, Some("Actual hours: 6. Disassembly surcharge added.".to_string()));

    // Verify it persists in database by querying get_move_invoice
    let fetched_inv = get_move_invoice("u-adj-staff".to_string(), "q-adj-1".to_string()).await.unwrap().unwrap();
    assert_eq!(fetched_inv.subtotal, 6500.0);
    assert_eq!(fetched_inv.rut_deduction, 2800.0);
    assert_eq!(fetched_inv.actual_hours, Some(6.0));

    // Cleanup
    conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-adj-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-adj-test'", ()).await.unwrap();
}
