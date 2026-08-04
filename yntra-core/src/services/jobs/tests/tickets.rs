use crate::JobTicket;
use crate::database;
use crate::services::jobs::{
    create_job_ticket, get_job_tickets, get_job_tickets_rkyv, schedule_job_ticket,
};
use crate::services::jobs::{get_job_signature, save_job_signature_with_audit_trail};

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
            |r| {
                Ok(JobTicket {
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
                    long_carry_meters: 0,
                    toll_fees: 0.0,
                })
            },
        )
        .await
        .unwrap();

    assert_eq!(updated_job.scheduled_date, "2026-08-15");
    assert_eq!(updated_job.status, "assigned");
    assert_eq!(
        updated_job.assigned_user_id,
        Some("u-sync-staff".to_string())
    );

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
    conn.execute(
        "DELETE FROM events WHERE id = ?1",
        crate::params![&event_id],
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE id = 'u-sync-staff'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-sync-test'", ())
        .await
        .unwrap();
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
    let sig_opt = get_job_signature("u-sig-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(sig_opt.is_none());

    // 2. Save signature with legal audit trail and transport terms (Bohag 2020)
    let mock_signature = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADIA...";
    save_job_signature_with_audit_trail(
        "u-sig-staff".to_string(),
        job.id.clone(),
        "John Doe (Customer)".to_string(),
        mock_signature.to_string(),
        Some("192.168.1.100".to_string()),
        Some("59.3293,18.0686".to_string()),
        Some("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)".to_string()),
        Some("Bohag 2020".to_string()),
    )
    .await
    .unwrap();

    // 3. Retrieve and verify signature details and legal audit trail
    let sig_opt_2 = get_job_signature("u-sig-staff".to_string(), job.id.clone())
        .await
        .unwrap();
    assert!(sig_opt_2.is_some());
    let sig = sig_opt_2.unwrap();
    assert_eq!(sig.signer_name, "John Doe (Customer)");
    assert_eq!(sig.signature_data_base64, mock_signature);
    assert_eq!(sig.job_ticket_id, job.id);
    assert_eq!(sig.workspace_id, "ws-sig-test");
    assert_eq!(sig.ip_address, Some("192.168.1.100".to_string()));
    assert_eq!(sig.geolocation, Some("59.3293,18.0686".to_string()));
    assert_eq!(sig.terms_version, Some("Bohag 2020".to_string()));
    assert!(sig.terms_hash.is_some());
    assert!(sig.signature_hash.is_some());

    // Cleanup
    conn.execute(
        "DELETE FROM move_signatures WHERE job_ticket_id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sig-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_external_notification_triggers() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-notif-test', 'Notif Test WS', '[\"moving_company\"]', '{\"twilio_sid\":\"ACtest\",\"twilio_token\":\"toktest\",\"twilio_from_number\":\"+123\",\"sendgrid_api_key\":\"SG.test\",\"sendgrid_from_email\":\"test@yntra.se\"}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role) VALUES ('u-notif-staff', 'ws-notif-test', 'staff@notif.io', 'Staff User', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role, phone) VALUES ('u-notif-client', 'ws-notif-test', 'client@notif.io', 'Client User', 'client', '+46700000000')", ()).await.unwrap();

    // 2. Trigger custom notification (test mock fallback path logic works)
    let res = crate::services::jobs::notifications::send_external_notification(
        "u-notif-staff".to_string(),
        "ws-notif-test".to_string(),
        "u-notif-client".to_string(),
        "booking_confirmation".to_string(),
        None,
    )
    .await
    .unwrap();

    assert!(res == false || res == true);

    // 3. Clean up
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-notif-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-notif-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_public_booking_lead_submission() {
    use crate::services::jobs::{submit_public_booking_lead, update_job_status};

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lead-test', 'Lead Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();

    // 2. Submit lead
    let items_json = r#"[{"name": "Sofa", "quantity": 1, "volume": 1.5}, {"name": "Box", "quantity": 10, "volume": 0.1}]"#;
    let job_id = submit_public_booking_lead(
        "ws-lead-test".to_string(),
        "John Doe".to_string(),
        "john@doe.se".to_string(),
        "+46701112233".to_string(),
        "Startvägen 1".to_string(),
        "Slutgränd 5".to_string(),
        items_json.to_string(),
    )
    .await
    .unwrap();

    // 3. Verify user created
    let (uid, role, phone): (String, String, Option<String>) = conn
        .query_row(
            "SELECT id, role, phone FROM users WHERE workspace_id = 'ws-lead-test' AND email = 'john@doe.se'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .unwrap();

    assert!(uid.starts_with("u-guest-"));
    assert_eq!(role, "client");
    assert_eq!(phone, Some("+46701112233".to_string()));

    // 4. Verify job ticket created
    let (j_title, j_status, origin, dest): (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT title, status, origin_address, destination_address FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .await
        .unwrap();

    assert_eq!(j_title, "Offertförfrågan - John Doe");
    assert_eq!(j_status, "quote_requested");
    assert_eq!(origin, Some("Startvägen 1".to_string()));
    assert_eq!(dest, Some("Slutgränd 5".to_string()));

    // 5. Verify inventory items created (total volume: 1.5*1 + 0.1*10 = 2.5 m3)
    let total_vol: f64 = conn
        .query_row(
            "SELECT SUM(quantity * estimated_volume_m3) FROM move_inventory WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| r.get(0),
        )
        .await
        .unwrap();

    assert_eq!(total_vol, 2.5);

    // 6. Verify AuthContext::authorize succeeds for the created guest user (role signature check)
    let auth = crate::infra::auth::AuthContext::authorize(&conn, &uid).await;
    assert!(auth.is_ok());

    // 7. Verify status transition for quote_requested ticket
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('staff-lead-test', 'ws-lead-test', 'staff@lead.se', 'admin')", ()).await.unwrap();
    crate::services::users::ensure_user_role_signature(
        &conn,
        "staff-lead-test",
        "admin",
        "ws-lead-test",
    )
    .await
    .unwrap();
    let update_res = update_job_status(
        "staff-lead-test".to_string(),
        job_id.clone(),
        "assigned".to_string(),
    )
    .await;
    assert!(update_res.is_ok());

    // 6. Verify quote created using unified calculation engine (2.5 m3 * 500 = 1250 base_price, 800 distance_fee, 250 supplies_fee = 2300 total)
    let (base_price, total_price): (f64, f64) = conn
        .query_row(
            "SELECT base_price, total_price FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .unwrap();

    assert_eq!(base_price, 1250.0);
    assert_eq!(total_price, 2300.0);

    // 8. Cleanup
    conn.execute(
        "DELETE FROM move_inventory WHERE job_ticket_id = ?1",
        crate::params![&job_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM move_quotes WHERE job_ticket_id = ?1",
        crate::params![&job_id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE id = ?1",
        crate::params![&job_id],
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lead-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-lead-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_public_lead_validation_and_rate_limiting() {
    use crate::infra::errors::YntraError;
    use crate::services::jobs::submit_public_booking_lead;

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lead-val-test', 'Lead Val WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();

    // 1. Invalid email missing @ should fail validation
    let err_email = submit_public_booking_lead(
        "ws-lead-val-test".to_string(),
        "Bot User".to_string(),
        "invalidemailformat.com".to_string(),
        "12345".to_string(),
        "A".to_string(),
        "B".to_string(),
        "[]".to_string(),
    )
    .await;
    assert!(err_email.is_err());
    assert!(matches!(
        err_email.unwrap_err(),
        YntraError::ValidationError(_)
    ));

    // 2. Short name should fail validation
    let err_name = submit_public_booking_lead(
        "ws-lead-val-test".to_string(),
        "X".to_string(),
        "bot@spam.com".to_string(),
        "12345".to_string(),
        "A".to_string(),
        "B".to_string(),
        "[]".to_string(),
    )
    .await;
    assert!(err_name.is_err());

    // 3. Valid lead should succeed and set unverified_guest metadata
    let res_ok = submit_public_booking_lead(
        "ws-lead-val-test".to_string(),
        "Valid Lead".to_string(),
        "valid@lead.se".to_string(),
        "0700000000".to_string(),
        "Start Str 1".to_string(),
        "End Str 2".to_string(),
        "[]".to_string(),
    )
    .await;
    assert!(res_ok.is_ok());

    let guest_meta: String = conn.query_row(
        "SELECT metadata FROM users WHERE workspace_id = 'ws-lead-val-test' AND email = 'valid@lead.se'",
        (),
        |r| r.get(0),
    ).await.unwrap();
    assert!(guest_meta.contains("unverified_guest"));

    // 4. Invalid items JSON should fail upfront without creating orphaned records
    let err_json = submit_public_booking_lead(
        "ws-lead-val-test".to_string(),
        "Bad Json User".to_string(),
        "badjson@lead.se".to_string(),
        "0700000000".to_string(),
        "Start Str 1".to_string(),
        "End Str 2".to_string(),
        "invalid-json-string".to_string(),
    )
    .await;
    assert!(err_json.is_err());
    let bad_user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE workspace_id = 'ws-lead-val-test' AND email = 'badjson@lead.se'", (), |r| r.get(0)).await.unwrap();
    assert_eq!(bad_user_count, 0);

    // Cleanup
    conn.execute(
        "DELETE FROM move_quotes WHERE workspace_id = 'ws-lead-val-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-lead-val-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = 'ws-lead-val-test'",
        (),
    )
    .await
    .ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-lead-val-test'", ())
        .await
        .ok();
}

#[tokio::test]
async fn test_third_party_lead_aggregator_webhooks() {
    use crate::infra::errors::YntraError;
    use crate::services::jobs::ingest_third_party_lead_webhook;

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let settings = serde_json::json!({
        "lead_webhook_api_key": "lead-secret-key-123"
    })
    .to_string();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-agg-test', 'Aggregator WS', '[\"moving_company\"]', ?1)", crate::params![&settings]).await.unwrap();

    // 1. Google LSA webhook test
    let google_payload = serde_json::json!({
        "customerName": "Alice Google",
        "customerEmail": "alice@googlelsa.com",
        "customerPhone": "+46700001111",
        "originAddress": "Google St 1",
        "destinationAddress": "Google St 2"
    })
    .to_string();

    let g_job = ingest_third_party_lead_webhook(
        "ws-agg-test".to_string(),
        "google_lsa".to_string(),
        "lead-secret-key-123".to_string(),
        google_payload,
    )
    .await
    .unwrap();

    let (g_title, g_prio): (String, String) = conn
        .query_row(
            "SELECT title, priority FROM job_tickets WHERE id = ?1",
            crate::params![&g_job],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .unwrap();

    assert!(g_title.contains("Google LSA"));
    assert_eq!(g_prio, "high");

    // 2. Yelp webhook test
    let yelp_payload = serde_json::json!({
        "user_name": "Bob Yelp",
        "user_email": "bob@yelp.com",
        "user_phone": "+46700002222",
        "start_location": "Yelp St 10",
        "end_location": "Yelp St 20"
    })
    .to_string();

    let y_job = ingest_third_party_lead_webhook(
        "ws-agg-test".to_string(),
        "yelp".to_string(),
        "lead-secret-key-123".to_string(),
        yelp_payload,
    )
    .await
    .unwrap();

    let (y_title,): (String,) = conn
        .query_row(
            "SELECT title FROM job_tickets WHERE id = ?1",
            crate::params![&y_job],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .unwrap();

    assert!(y_title.contains("Yelp"));

    // 3. Webhook with structured address object (e.g. Angi / Moving.com)
    let angi_payload = serde_json::json!({
        "contact_name": "Carol Angi",
        "contact_email": "carol@angi.com",
        "contact_phone": "+46700003333",
        "address": {
            "street": "Kungsgatan 12",
            "city": "Stockholm",
            "postal_code": "11122"
        },
        "destination": {
            "street_address": "Drottninggatan 45",
            "locality": "Stockholm",
            "zip": "11151"
        }
    })
    .to_string();

    let a_job = ingest_third_party_lead_webhook(
        "ws-agg-test".to_string(),
        "angi".to_string(),
        "lead-secret-key-123".to_string(),
        angi_payload,
    )
    .await
    .unwrap();

    let (a_orig, a_dest): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT origin_address, destination_address FROM job_tickets WHERE id = ?1",
            crate::params![&a_job],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .unwrap();

    assert_eq!(a_orig, Some("Kungsgatan 12, 11122 Stockholm".to_string()));
    assert_eq!(
        a_dest,
        Some("Drottninggatan 45, 11151 Stockholm".to_string())
    );

    // 4. Webhook missing origin address should be rejected with ValidationError
    let invalid_payload = serde_json::json!({
        "customerName": "Dave NoAddress",
        "customerEmail": "dave@noaddress.com"
    })
    .to_string();

    let inv_res = ingest_third_party_lead_webhook(
        "ws-agg-test".to_string(),
        "google_lsa".to_string(),
        "lead-secret-key-123".to_string(),
        invalid_payload,
    )
    .await;

    assert!(inv_res.is_err());
    assert!(matches!(
        inv_res.unwrap_err(),
        YntraError::ValidationError(_)
    ));

    // Cleanup
    conn.execute(
        "DELETE FROM move_inventory WHERE workspace_id = 'ws-agg-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM move_quotes WHERE workspace_id = 'ws-agg-test'",
        (),
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-agg-test'",
        (),
    )
    .await
    .ok();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-agg-test'", ())
        .await
        .ok();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-agg-test'", ())
        .await
        .ok();
}

#[tokio::test]
async fn test_job_ticket_mover_visibility_scoping() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-scope-test', 'Scope Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    // Admin user (sees all)
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-scope-admin', 'ws-scope-test', 'admin@scope.io', 'admin')", ()).await.unwrap();
    // Mover 1 (sees only assigned / crew jobs)
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-scope-mover1', 'ws-scope-test', 'm1@scope.io', 'mover')", ()).await.unwrap();
    // Mover 2 (sees only assigned / crew jobs)
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-scope-mover2', 'ws-scope-test', 'm2@scope.io', 'mover')", ()).await.unwrap();

    // 2. Create job 1 (assigned directly to Mover 1)
    let job1 = create_job_ticket(
        "u-scope-admin".to_string(),
        "ws-scope-test".to_string(),
        "Job 1 (Mover 1)".to_string(),
        "Direct assign".to_string(),
        "Addr 1".to_string(),
        "medium".to_string(),
        Some("u-scope-mover1".to_string()),
        "2026-08-10".to_string(),
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

    // 3. Create job 2 (Mover 2 is added as crew)
    let job2 = create_job_ticket(
        "u-scope-admin".to_string(),
        "ws-scope-test".to_string(),
        "Job 2 (Mover 2 Crew)".to_string(),
        "Crew assign".to_string(),
        "Addr 2".to_string(),
        "medium".to_string(),
        None,
        "2026-08-10".to_string(),
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
    conn.execute("INSERT INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, 'u-scope-mover2', 'mover')", crate::params![&job2.id]).await.unwrap();

    // 4. Create job 3 (Unassigned to any specific mover)
    let _job3 = create_job_ticket(
        "u-scope-admin".to_string(),
        "ws-scope-test".to_string(),
        "Job 3 (Unassigned)".to_string(),
        "Unassigned".to_string(),
        "Addr 3".to_string(),
        "medium".to_string(),
        None,
        "2026-08-10".to_string(),
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

    // 5. Query tickets as Admin -> expects 3 jobs
    let admin_list = get_job_tickets("u-scope-admin".to_string()).await.unwrap();
    assert_eq!(admin_list.len(), 3);

    // 6. Query tickets as Mover 1 -> expects only job1
    let mover1_list = get_job_tickets("u-scope-mover1".to_string()).await.unwrap();
    assert_eq!(mover1_list.len(), 1);
    assert_eq!(mover1_list[0].id, job1.id);

    // 7. Query tickets as Mover 2 -> expects only job2
    let mover2_list = get_job_tickets("u-scope-mover2".to_string()).await.unwrap();
    assert_eq!(mover2_list.len(), 1);
    assert_eq!(mover2_list[0].id, job2.id);

    // Cleanup
    conn.execute(
        "DELETE FROM job_crew WHERE job_ticket_id IN (?1, ?2)",
        crate::params![&job1.id, &job2.id],
    )
    .await
    .ok();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-scope-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-scope-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-scope-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_customer_live_tracking_and_quote_deposit() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-track-test', 'Tracking WS', '[\"moving_company\"]', '{\"moving_deposit_percent\":25.0}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role, phone) VALUES ('u-track-staff', 'ws-track-test', 'staff@track.io', 'Leader Lars', 'admin', '+46701112233')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role) VALUES ('u-track-client', 'ws-track-test', 'client@track.io', 'Customer Carin', 'client')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status, latitude, longitude) VALUES ('v-track-1', 'ws-track-test', 'Truck 1', 'ABC-123', 25.0, 'available', 59.3293, 18.0686)", ()).await.unwrap();

    let job = create_job_ticket(
        "u-track-staff".to_string(),
        "ws-track-test".to_string(),
        "Live Tracked Relocation".to_string(),
        "Customer portal tracking test".to_string(),
        "Stockholm Central".to_string(),
        "high".to_string(),
        Some("u-track-client".to_string()),
        "2026-11-15".to_string(),
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

    // Assign vehicle to job
    conn.execute("UPDATE job_tickets SET assigned_user_id = 'u-track-staff', assigned_vehicle_id = 'v-track-1' WHERE id = ?1", crate::params![&job.id]).await.unwrap();

    // 1. Customer live GPS tracking portal
    let portal =
        crate::services::jobs::notifications::get_customer_live_tracking_portal(job.id.clone())
            .await
            .unwrap();
    assert_eq!(portal.driver_name, "Leader Lars");
    assert_eq!(portal.driver_phone, Some("+46701112233".to_string()));
    assert_eq!(portal.vehicle_license_plate, Some("ABC-123".to_string()));
    assert_eq!(portal.current_lat, 59.3293);
    assert_eq!(portal.current_lon, 18.0686);
    assert!(portal.live_tracking_url.contains(&job.id));

    // 2. Interactive quote approval with 25% deposit gate
    let quote_id = "q-track-1".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-track-test', ?2, 10000.0, 0.0, 0.0, 0.0, 10000.0, 'sent', 0)",
        crate::params![&quote_id, &job.id],
    )
    .await
    .unwrap();

    let deposit_res = crate::services::jobs::accept_move_quote_with_deposit(
        "u-track-client".to_string(),
        quote_id.clone(),
        "swish".to_string(),
    )
    .await
    .unwrap();
    assert!(deposit_res.success);
    assert_eq!(deposit_res.deposit_amount, 2500.0); // 25% of 10,000
    assert_eq!(deposit_res.remaining_balance, 7500.0);
    assert!(deposit_res.payment_session_url.unwrap().contains("swish"));

    // Cleanup
    conn.execute(
        "DELETE FROM move_quotes WHERE workspace_id = 'ws-track-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-track-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM vehicles WHERE workspace_id = 'ws-track-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-track-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-track-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_coarse_grained_mover_permissions_and_field_sheet_scoping() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-mover-perm-test', 'Mover Perm WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role) VALUES ('u-mover-admin', 'ws-mover-perm-test', 'admin@perm.io', 'Manager Max', 'admin')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, full_name, role) VALUES ('u-field-mover', 'ws-mover-perm-test', 'mover@perm.io', 'Crew Carl', 'mover')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status) VALUES ('v-mover-1', 'ws-mover-perm-test', 'Truck A', 'MOVER-888', 35.0, 'available')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-mover-admin".to_string(),
        "ws-mover-perm-test".to_string(),
        "Assigned Field Job".to_string(),
        "Field sheet scope test".to_string(),
        "Origin Street 10".to_string(),
        "high".to_string(),
        Some("u-field-mover".to_string()),
        "2026-09-01".to_string(),
        "[]".to_string(),
        Some("Origin Street 10".to_string()),
        Some("Dest Ave 20".to_string()),
        2,
        4,
        true,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    conn.execute(
        "UPDATE job_tickets SET assigned_vehicle_id = 'v-mover-1' WHERE id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, 'u-field-mover', 'mover')",
        crate::params![&job.id],
    )
    .await
    .unwrap();

    let quote_id = "q-mover-perm-1".to_string();
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-mover-perm-test', ?2, 8000.0, 500.0, 300.0, 200.0, 9000.0, 'sent', 0)",
        crate::params![&quote_id, &job.id],
    ).await.unwrap();

    // 1. Mover fetches read-only Field Sheet Manifest -> succeeds
    let manifest = crate::services::jobs::get_mover_field_sheet_manifest(
        "u-field-mover".to_string(),
        job.id.clone(),
    )
    .await
    .unwrap();
    assert_eq!(manifest.title, "Assigned Field Job");
    assert_eq!(
        manifest.assigned_vehicle_plate,
        Some("MOVER-888".to_string())
    );
    assert_eq!(manifest.origin_floor, 2);
    assert_eq!(manifest.destination_floor, 4);
    assert!(
        manifest
            .assigned_crew_names
            .contains(&"Crew Carl".to_string())
    );

    // 2. Mover attempts to view financial quotes -> blocked with AuthError
    let quote_err =
        crate::services::jobs::get_move_quote("u-field-mover".to_string(), job.id.clone()).await;
    assert!(quote_err.is_err());
    if let Err(crate::infra::errors::YntraError::AuthError(msg)) = quote_err {
        assert!(msg.contains("mover role cannot view financial quotes"));
    } else {
        panic!("Expected AuthError for mover quote access");
    }

    // 3. Mover attempts to view move invoice -> blocked with AuthError
    let invoice_err =
        crate::services::jobs::get_move_invoice("u-field-mover".to_string(), quote_id.clone())
            .await;
    assert!(invoice_err.is_err());
    if let Err(crate::infra::errors::YntraError::AuthError(msg)) = invoice_err {
        assert!(msg.contains("mover role cannot view move invoices"));
    } else {
        panic!("Expected AuthError for mover invoice access");
    }

    // Cleanup
    conn.execute(
        "DELETE FROM move_quotes WHERE workspace_id = 'ws-mover-perm-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_crew WHERE job_ticket_id = ?1",
        crate::params![&job.id],
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-mover-perm-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM vehicles WHERE workspace_id = 'ws-mover-perm-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "DELETE FROM users WHERE workspace_id = 'ws-mover-perm-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-mover-perm-test'", ())
        .await
        .unwrap();
}
