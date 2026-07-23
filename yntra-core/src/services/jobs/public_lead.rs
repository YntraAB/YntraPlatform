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
    // 1. Input sanitization & validation
    let name_clean = customer_name.trim();
    if name_clean.len() < 2 {
        return Err(YntraError::ValidationError(
            "Customer name must be at least 2 characters long.".to_string(),
        ));
    }

    let customer_email_clean = customer_email.trim().to_lowercase();
    if customer_email_clean.is_empty() || !customer_email_clean.contains('@') {
        return Err(YntraError::ValidationError(format!(
            "Invalid customer email address '{}': missing '@' domain identifier",
            customer_email
        )));
    }
    let parts: Vec<&str> = customer_email_clean.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
        return Err(YntraError::ValidationError(format!(
            "Invalid customer email address '{}': must contain valid local and domain parts (e.g. user@domain.com)",
            customer_email
        )));
    }

    let origin_clean = origin_address.trim();
    let dest_clean = destination_address.trim();
    if origin_clean.is_empty() || dest_clean.is_empty() {
        return Err(YntraError::ValidationError(
            "Origin and destination addresses cannot be empty.".to_string(),
        ));
    }

    // Parse items JSON upfront before creating database records
    let items: Vec<serde_json::Value> = serde_json::from_str(&items_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid items JSON: {}", e)))?;

    let conn = database::acquire_connection().await?;

    // 2. Verify workspace exists
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

    // 3. Rate limiting check (max 5 leads per email/workspace in 15 minutes)
    let fifteen_mins_ago = crate::infra::time::get_current_time_ms() - (15 * 60 * 1000);
    let recent_leads_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM job_tickets WHERE workspace_id = ?1 AND title LIKE ?2 AND created_at >= ?3",
            crate::params![&workspace_id, format!("%{}", name_clean), fifteen_mins_ago],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    if recent_leads_count >= 5 {
        return Err(YntraError::ValidationError(
            "Rate limit exceeded: Too many booking lead requests submitted recently. Please wait before submitting another request.".to_string(),
        ));
    }

    // 4. Create or find guest client user
    let existing_uid: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND email = ?2",
            crate::params![&workspace_id, &customer_email_clean],
            |r| r.get(0),
        )
        .await
        .ok();

    let (customer_id, is_new_guest) = match existing_uid {
        Some(id) => {
            crate::services::users::ensure_user_role_signature(&conn, &id, "client", &workspace_id).await?;
            (id, false)
        }
        None => {
            let id = format!("u-guest-{}", uuid::Uuid::new_v4());
            let guest_meta = serde_json::json!({
                "unverified_guest": true,
                "verification_status": "unverified",
                "lead_source": "public_lead_widget"
            }).to_string();

            conn.execute(
                "INSERT INTO users (id, workspace_id, email, full_name, phone, role, preferences, metadata) VALUES (?1, ?2, ?3, ?4, ?5, 'client', '{}', ?6)",
                crate::params![&id, &workspace_id, &customer_email_clean, name_clean, &customer_phone, &guest_meta],
            ).await?;
            crate::services::users::ensure_user_role_signature(&conn, &id, "client", &workspace_id).await?;
            (id, true)
        }
    };

    // 5. Create job ticket for the lead
    let job_id = format!("job-lead-{}", uuid::Uuid::new_v4());
    let now_ms = crate::infra::time::get_current_time_ms();
    let now_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, origin_address, destination_address) VALUES (?1, ?2, ?3, ?4, ?5, 'medium', 'quote_requested', 'unscheduled', '[]', ?6, ?7, ?8, ?9)",
        crate::params![
            &job_id,
            &workspace_id,
            format!("Offertförfrågan - {}", name_clean),
            format!("Offertförfrågan skapad via publik lead-widget för {} [UNVERIFIED GUEST LEAD]", name_clean),
            origin_clean,
            &now_date,
            now_ms,
            origin_clean,
            dest_clean,
        ],
    ).await?;

    // 6. Insert inventory items
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

    // Drop active database connection handle before calling calculate_and_save_move_quote
    drop(conn);

    // 7. Calculate and save move quote estimate using workspace settings calculation engine
    let quote_res = crate::services::jobs::moves::calculate_and_save_move_quote(
        customer_id.clone(),
        job_id.clone(),
    )
    .await;

    if let Err(e) = quote_res {
        // Rollback orphaned database records if quote generation fails
        if let Ok(clean_conn) = database::acquire_connection().await {
            let _ = clean_conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job_id]).await;
            let _ = clean_conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job_id]).await;
            if is_new_guest {
                let _ = clean_conn.execute("DELETE FROM users WHERE id = ?1", crate::params![&customer_id]).await;
            }
        }
        return Err(e);
    }
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

fn extract_address_from_payload(payload: &serde_json::Value, candidate_keys: &[&str]) -> Option<String> {
    for &key in candidate_keys {
        let val_opt = if key.starts_with('/') {
            payload.pointer(key)
        } else {
            payload.get(key)
        };

        if let Some(val) = val_opt {
            if let Some(s) = val.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("address not specified") {
                    return Some(trimmed.to_string());
                }
            } else if let Some(obj) = val.as_object() {
                let street = obj.get("street")
                    .or_else(|| obj.get("street_address"))
                    .or_else(|| obj.get("address_line_1"))
                    .or_else(|| obj.get("line1"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let city = obj.get("city")
                    .or_else(|| obj.get("locality"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let zip = obj.get("zip")
                    .or_else(|| obj.get("postal_code"))
                    .or_else(|| obj.get("postcode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let state = obj.get("state")
                    .or_else(|| obj.get("region"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();

                let mut parts = Vec::new();
                if !street.is_empty() { parts.push(street.to_string()); }
                if !zip.is_empty() && !city.is_empty() {
                    parts.push(format!("{} {}", zip, city));
                } else {
                    if !zip.is_empty() { parts.push(zip.to_string()); }
                    if !city.is_empty() { parts.push(city.to_string()); }
                }
                if !state.is_empty() { parts.push(state.to_string()); }

                let full_addr = parts.join(", ");
                if full_addr.len() >= 3 {
                    return Some(full_addr);
                }
            }
        }
    }
    None
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

    let (origin_keys, dest_keys) = match provider.to_lowercase().as_str() {
        "google_lsa" | "google" => (
            vec!["originAddress", "origin", "from_address", "address"],
            vec!["destinationAddress", "destination", "to_address"],
        ),
        "yelp" => (
            vec!["start_location", "origin", "address", "location"],
            vec!["end_location", "destination", "dropoff_location"],
        ),
        "moving_com" | "moving" => (
            vec!["/mover_lead/origin", "/mover_lead/origin_address", "origin", "address"],
            vec!["/mover_lead/destination", "/mover_lead/destination_address", "destination"],
        ),
        "angi" | "homeadvisor" => (
            vec!["address", "origin", "start_address"],
            vec!["destination", "destination_address", "end_address"],
        ),
        _ => (
            vec!["origin", "address", "start_location", "from_address", "/mover_lead/origin"],
            vec!["destination", "end_location", "to_address", "/mover_lead/destination"],
        ),
    };

    let origin = extract_address_from_payload(&payload, &origin_keys)
        .ok_or_else(|| YntraError::ValidationError(format!("Third-party lead webhook from '{}' missing valid origin address", provider)))?;

    let destination = extract_address_from_payload(&payload, &dest_keys)
        .ok_or_else(|| YntraError::ValidationError(format!("Third-party lead webhook from '{}' missing valid destination address", provider)))?;

    let (name, email, phone, items_json) = match provider.to_lowercase().as_str() {
        "google_lsa" | "google" => (
            payload.get("customerName").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Google LSA Lead").to_string(),
            payload.get("customerEmail").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@googlelsa.com").to_string(),
            payload.get("customerPhone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "yelp" => (
            payload.get("user_name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Yelp Lead").to_string(),
            payload.get("user_email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@yelp.com").to_string(),
            payload.get("user_phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "moving_com" | "moving" => (
            payload.pointer("/mover_lead/contact/name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Moving.com Lead").to_string(),
            payload.pointer("/mover_lead/contact/email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@moving.com").to_string(),
            payload.pointer("/mover_lead/contact/phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.pointer("/mover_lead/items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        "angi" | "homeadvisor" => (
            payload.get("contact_name").or_else(|| payload.get("name")).and_then(|v| v.as_str()).unwrap_or("Angi Lead").to_string(),
            payload.get("contact_email").or_else(|| payload.get("email")).and_then(|v| v.as_str()).unwrap_or("lead@angi.com").to_string(),
            payload.get("contact_phone").or_else(|| payload.get("phone")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            payload.get("items").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string()),
        ),
        _ => (
            payload.get("name").and_then(|v| v.as_str()).unwrap_or("Inbound Lead").to_string(),
            payload.get("email").and_then(|v| v.as_str()).unwrap_or("lead@inbound.com").to_string(),
            payload.get("phone").and_then(|v| v.as_str()).unwrap_or("").to_string(),
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
