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

async fn ingest_third_party_lead_webhook_inner(
    workspace_id: String,
    provider: String,
    api_key: String,
    payload_json: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::ValidationError(format!("Workspace {} not found", workspace_id)))?;

    let settings_json: serde_json::Value = serde_json::from_str(&settings_str)
        .unwrap_or(serde_json::json!({}));

    let expected_key = settings_json
        .get("lead_webhook_api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !expected_key.is_empty() && expected_key != api_key {
        return Err(YntraError::AuthError("Invalid lead webhook API key".to_string()));
    }

    let payload: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid webhook JSON payload: {}", e)))?;

    let (name, email, phone, origin, destination, items_json) = match provider.to_lowercase().as_str() {
        "google_lsa" | "google" => (
            payload.get("customerName").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Google LSA Lead").to_string(),
            payload.get("customerEmail").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@googlelsa.com").to_string(),
            payload.get("customerPhone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("originAddress").or_else(|| payload.get("origin")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("destinationAddress").or_else(|| payload.get("destination")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "yelp" => (
            payload.get("user_name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Yelp Lead").to_string(),
            payload.get("user_email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@yelp.com").to_string(),
            payload.get("user_phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("start_location").or_else(|| payload.get("origin")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("end_location").or_else(|| payload.get("destination")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "moving_com" | "moving" => (
            payload.pointer("/mover_lead/contact/name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Moving.com Lead").to_string(),
            payload.pointer("/mover_lead/contact/email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@moving.com").to_string(),
            payload.pointer("/mover_lead/contact/phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.pointer("/mover_lead/origin").or_else(|| payload.get("origin")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.pointer("/mover_lead/destination").or_else(|| payload.get("destination")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.pointer("/mover_lead/items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "angi" | "homeadvisor" => (
            payload.get("contact_name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Angi Lead").to_string(),
            payload.get("contact_email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@angi.com").to_string(),
            payload.get("contact_phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("address").or_else(|| payload.get("origin")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("destination").and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        _ => (
            payload.get("name").and_then(|v| v.as_str()).unwrap_or("Inbound Lead").to_string(),
            payload.get("email").and_then(|v| v.as_str()).unwrap_or("lead@inbound.com").to_string(),
            payload.get("phone").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("origin").or_else(|| payload.get("address")).and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("destination").and_then(|v| v.as_str()).unwrap_or("Address Not Specified").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
    };

    let job_id = submit_public_booking_lead_inner(
        workspace_id.clone(),
        name,
        email,
        phone,
        origin,
        destination,
        items_json,
    ).await?;

    let provider_tag = match provider.to_lowercase().as_str() {
        "google_lsa" | "google" => "Google LSA",
        "yelp" => "Yelp",
        "moving_com" | "moving" => "Moving.com",
        "angi" | "homeadvisor" => "Angi / HomeAdvisor",
        _ => "Partner API",
    };

    conn.execute(
        "UPDATE job_tickets SET title = title || ' [' || ?1 || ']', description = description || ' (Importerad via ' || ?1 || ' webhook-integration)', priority = 'high' WHERE id = ?2",
        crate::params![provider_tag, &job_id],
    ).await?;

    crate::infra::observer::notify_observers();
    Ok(job_id)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn ingest_third_party_lead_webhook(
    workspace_id: String,
    provider: String,
    api_key: String,
    payload_json: String,
) -> Result<String, YntraError> {
    let fut = ingest_third_party_lead_webhook_inner(workspace_id, provider, api_key, payload_json);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_third_party_lead_webhook(
    workspace_id: String,
    provider: String,
    api_key: String,
    payload_json: String,
) -> Result<String, YntraError> {
    ingest_third_party_lead_webhook_inner(workspace_id, provider, api_key, payload_json).await
}
