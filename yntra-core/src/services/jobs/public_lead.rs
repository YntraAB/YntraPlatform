use crate::database;
use crate::infra::errors::YntraError;

async fn submit_public_booking_lead_inner(
    workspace_id: String,
    customer_name: String,
    customer_email: String,
    customer_phone: String,
    origin_address: String,
    destination_address: String,
    items_json: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;

    // 1. Verify workspace exists
    let ws_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await?;

    if !ws_exists {
        return Err(YntraError::ValidationError(format!(
            "Workspace {} does not exist",
            workspace_id
        )));
    }

    // 2. Create or find guest client user
    let existing_uid: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND email = ?2",
            crate::params![&workspace_id, &customer_email],
            |r| r.get(0),
        )
        .await
        .ok();

    let customer_id = match existing_uid {
        Some(id) => {
            crate::services::users::ensure_user_role_signature(&conn, &id, "client", &workspace_id).await?;
            id
        }
        None => {
            let id = format!("u-guest-{}", uuid::Uuid::new_v4());
            conn.execute(
                "INSERT INTO users (id, workspace_id, email, full_name, phone, role, preferences) VALUES (?1, ?2, ?3, ?4, ?5, 'client', '{}')",
                crate::params![&id, &workspace_id, &customer_email, &customer_name, &customer_phone],
            ).await?;
            crate::services::users::ensure_user_role_signature(&conn, &id, "client", &workspace_id).await?;
            id
        }
    };

    // 3. Create job ticket for the lead
    let job_id = format!("job-lead-{}", uuid::Uuid::new_v4());
    let now_ms = crate::infra::time::get_current_time_ms();
    let now_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_address, destination_address) VALUES (?1, ?2, ?3, ?4, ?5, 'medium', 'quote_requested', 'unscheduled', '[]', ?6, ?7, ?8, ?9)",
        crate::params![
            &job_id,
            &workspace_id,
            format!("Offertförfrågan - {}", customer_name),
            format!("Offertförfrågan skapad via publik lead-widget för {}", customer_name),
            &origin_address,
            &now_date,
            now_ms,
            &origin_address,
            &destination_address,
        ],
    ).await?;

    // 4. Parse and insert inventory items
    let items: Vec<serde_json::Value> = serde_json::from_str(&items_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid items JSON: {}", e)))?;

    for item in items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("Möbel");
        let quantity = item.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
        let volume = item.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let inv_id = format!("inv-lead-{}", uuid::Uuid::new_v4());
        
        conn.execute(
            "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES (?1, ?2, ?3, 'Möbler', ?4, ?5, ?6)",
            crate::params![
                &inv_id,
                &workspace_id,
                &job_id,
                name,
                quantity,
                volume,
            ],
        ).await?;
    }

    // 5. Calculate and save move quote estimate (1500 kr base + 150 kr per m3 volume)
    let total_volume: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * estimated_volume_m3), 0.0) FROM move_inventory WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    let base_price = 1500.0;
    let volume_fee = total_volume * 150.0;
    let total_price = base_price + volume_fee;

    let quote_id = format!("quote-lead-{}", uuid::Uuid::new_v4());
    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status) VALUES (?1, ?2, ?3, ?4, 0.0, 0.0, 0.0, ?5, 'pending')",
        crate::params![
            &quote_id,
            &workspace_id,
            &job_id,
            base_price,
            total_price,
        ],
    ).await?;

    // 6. Trigger automated email/SMS receipt to customer
    let _ = crate::services::jobs::notifications::send_external_notification(
        customer_id.clone(),
        workspace_id.clone(),
        customer_id.clone(),
        "booking_confirmation".to_string(),
        None,
    )
    .await;

    // Notify UI observers
    crate::infra::observer::notify_observers();

    Ok(job_id)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn submit_public_booking_lead(
    workspace_id: String,
    customer_name: String,
    customer_email: String,
    customer_phone: String,
    origin_address: String,
    destination_address: String,
    items_json: String,
) -> Result<String, YntraError> {
    let fut = submit_public_booking_lead_inner(workspace_id, customer_name, customer_email, customer_phone, origin_address, destination_address, items_json);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn submit_public_booking_lead(
    workspace_id: String,
    customer_name: String,
    customer_email: String,
    customer_phone: String,
    origin_address: String,
    destination_address: String,
    items_json: String,
) -> Result<String, YntraError> {
    submit_public_booking_lead_inner(workspace_id, customer_name, customer_email, customer_phone, origin_address, destination_address, items_json).await
}
