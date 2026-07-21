use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::{MoveInventoryItem, MoveQuote, JobPackagingItem};

#[uniffi::export]
pub async fn get_move_inventory(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<MoveInventoryItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id], |row| {
            Ok(MoveInventoryItem {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                item_category: row.get(3)?,
                item_name: row.get(4)?,
                quantity: row.get::<i64>(5)? as i32,
                estimated_volume_m3: row.get(6)?,
                handling_notes: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<MoveQuote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(MoveQuote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            base_price: row.get::<f64>(3)? as i64,
            distance_fee: row.get::<f64>(4)? as i64,
            stairs_surcharge: row.get::<f64>(5)? as i64,
            packing_supplies_fee: row.get::<f64>(6)? as i64,
            total_price: row.get::<f64>(7)? as i64,
            status: row.get(8)?,
            accepted_at: row.get(9)?,
            updated_at: row.get(10)?,
            sync_status: row.get(11)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn accept_move_quote(
    requester_user_id: String,
    quote_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (quote_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![now_ms, now_ms, quote_id],
    ).await?;

    let customer_id = conn.query_row(
        "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
        crate::params![&quote_ws],
        |r| r.get::<String>(0),
    )
    .await
    .unwrap_or_else(|_| "client-1".to_string());

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id.clone(),
        quote_ws,
        customer_id,
        "booking_confirmation".to_string(),
        None,
    ).await;

    Ok(())
}

#[uniffi::export]
pub async fn create_move_inventory_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_category: String,
    item_name: String,
    quantity: i32,
    estimated_volume_m3: f64,
    handling_notes: Option<String>,
) -> Result<(), YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Verify workspace scoping of the job ticket
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            id,
            job_ws,
            job_ticket_id,
            item_category,
            item_name,
            quantity as i64,
            estimated_volume_m3,
            handling_notes,
            now_ms
        ],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_move_inventory_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_inventory WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Inventory item not found".to_string()))?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM move_inventory WHERE id = ?1",
        crate::params![item_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn calculate_and_save_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Job Ticket details
    let mut stmt = conn.prepare(
        "SELECT workspace_id, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, COALESCE(long_carry_meters, 0), COALESCE(toll_fees, 0.0) FROM job_tickets WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let (job_ws, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters, toll_fees) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<i64>(1)? as i32,
            row.get::<i64>(2)? as i32,
            row.get::<bool>(3)?,
            row.get::<bool>(4)?,
            row.get::<i64>(5)? as i32,
            row.get::<f64>(6)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Job not found".to_string()));
    };

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // 2. Fetch workspace settings:
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&job_ws],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    // 3. Fetch inventory items and calculate volume & specialty surcharges
    let mut inv_stmt = conn.prepare(
        "SELECT quantity, estimated_volume_m3, item_name FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;
    let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
    let mut total_volume = 0.0;
    let mut specialty_surcharge = 0.0;

    let surcharge_piano = settings_json.get("surcharge_piano").and_then(|v| v.as_f64()).unwrap_or(1500.0);
    let surcharge_safe = settings_json.get("surcharge_safe").and_then(|v| v.as_f64()).unwrap_or(2000.0);
    let surcharge_jacuzzi = settings_json.get("surcharge_jacuzzi").and_then(|v| v.as_f64()).unwrap_or(2500.0);
    let surcharge_fragile = settings_json.get("surcharge_fragile").and_then(|v| v.as_f64()).unwrap_or(500.0);

    while let Some(row) = inv_rows.next().await? {
        let quantity: i64 = row.get(0)?;
        let vol: f64 = row.get(1)?;
        let item_name: String = row.get(2)?;
        total_volume += (quantity as f64) * vol;

        let item_name_lower = item_name.to_lowercase();
        let item_fee = if item_name_lower.contains("piano") || item_name_lower.contains("flygel") {
            surcharge_piano
        } else if item_name_lower.contains("safe") || item_name_lower.contains("kassaskåp") {
            surcharge_safe
        } else if item_name_lower.contains("jacuzzi") || item_name_lower.contains("badkar") || item_name_lower.contains("spa") {
            surcharge_jacuzzi
        } else if item_name_lower.contains("konst") || item_name_lower.contains("tavla") || item_name_lower.contains("painting") || item_name_lower.contains("fragile") {
            surcharge_fragile
        } else {
            0.0
        };
        specialty_surcharge += item_fee * (quantity as f64);
    }

    // 4. Quoting Calculations:
    let base_rate_per_m3 = settings_json
        .get("moving_base_rate_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(500.0);
    let distance_fee_flat = settings_json
        .get("moving_distance_fee_flat")
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    let stairs_surcharge_per_floor = settings_json
        .get("moving_stairs_surcharge_per_floor")
        .and_then(|v| v.as_f64())
        .unwrap_or(300.0);
    let packing_supplies_fee_per_m3 = settings_json
        .get("moving_packing_supplies_fee_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);

    // Base hourly/labor rate depending on configured pricing model
    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    let mut base_price = if pricing_model == "hourly" {
        let crew_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_crew WHERE job_ticket_id = ?1",
                crate::params![&job_ticket_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);

        let default_crew_size = settings_json
            .get("moving_default_crew_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(2) as f64;

        let active_crew_size = if crew_count > 0 {
            crew_count as f64
        } else {
            default_crew_size
        };

        let hourly_rate_per_mover = settings_json
            .get("moving_hourly_rate_per_mover")
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0);

        let hourly_rate_vehicle = settings_json
            .get("moving_hourly_rate_vehicle")
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0);

        let hourly_rate = if settings_json.get("moving_hourly_rate_per_mover").is_some()
            || settings_json.get("moving_hourly_rate_vehicle").is_some()
        {
            active_crew_size * hourly_rate_per_mover + hourly_rate_vehicle
        } else {
            settings_json
                .get("moving_hourly_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(1200.0)
        };

        let hours_per_m3 = settings_json
            .get("moving_hours_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.15);
        let minimum_hours = settings_json
            .get("moving_minimum_hours")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);
        (estimated_hours * hourly_rate) as i64
    } else {
        (total_volume * base_rate_per_m3) as i64
    };

    // Add specialty/heavy item handling fees to base labor price
    base_price += specialty_surcharge as i64;

    let surcharge_long_carry_per_meter = settings_json
        .get("surcharge_long_carry_per_meter")
        .and_then(|v| v.as_f64())
        .unwrap_or(40.0);

    // Flat distance rate + tolls
    let distance_fee = distance_fee_flat as i64 + toll_fees as i64;
    // Stairs surcharge (stairs_surcharge_per_floor per floor if no elevator)
    let mut stairs_surcharge = 0;
    if !origin_has_elevator && origin_floor > 0 {
        stairs_surcharge += (origin_floor as i64) * (stairs_surcharge_per_floor as i64);
    }
    if !destination_has_elevator && destination_floor > 0 {
        stairs_surcharge += (destination_floor as i64) * (stairs_surcharge_per_floor as i64);
    }
    // Add long carry surcharge to labor stairs surcharge for RUT tax deductibility eligibility
    if long_carry_meters > 0 {
        stairs_surcharge += (long_carry_meters as i64) * (surcharge_long_carry_per_meter as i64);
    }
    let actual_supplies_cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * price_per_unit), 0.0) FROM job_packaging_items WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    let packing_supplies_fee = if actual_supplies_cost > 0.0 {
        actual_supplies_cost as i64
    } else {
        (total_volume * packing_supplies_fee_per_m3) as i64
    };
    let total_price = base_price + distance_fee + stairs_surcharge + packing_supplies_fee;

    let now_ms = chrono::Utc::now().timestamp_millis();
    
    // Check if quote exists to keep its status, default to "sent"
    let mut quote_stmt = conn.prepare(
        "SELECT id, status FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let (quote_id, quote_status) = if let Some(row) = q_rows.next().await? {
        (row.get::<String>(0)?, row.get::<String>(1)?)
    } else {
        (uuid::Uuid::new_v4().to_string(), "sent".to_string())
    };

    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        crate::params![
            quote_id,
            job_ws,
            job_ticket_id,
            base_price as f64,
            distance_fee as f64,
            stairs_surcharge as f64,
            packing_supplies_fee as f64,
            total_price as f64,
            quote_status,
            now_ms
        ],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn add_job_packaging_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_name: String,
    quantity: i32,
    price_per_unit: f64,
    is_leased: bool,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let item_id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let is_leased_int = if is_leased { 1 } else { 0 };

    conn.execute(
        "INSERT INTO job_packaging_items (id, workspace_id, job_ticket_id, item_name, quantity, price_per_unit, is_leased, returned_quantity, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)",
        crate::params![
            item_id,
            job_ws,
            job_ticket_id,
            item_name,
            quantity,
            price_per_unit,
            is_leased_int,
            now_ms
        ],
    ).await?;

    // Check if move quote exists
    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn remove_job_packaging_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM job_packaging_items WHERE id = ?1",
        crate::params![item_id],
    ).await?;

    // Recalculate quote if exists
    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn update_job_packaging_item_returned(
    requester_user_id: String,
    item_id: String,
    returned_quantity: i32,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE job_packaging_items SET returned_quantity = ?1, updated_at = ?2 WHERE id = ?3",
        crate::params![returned_quantity, now_ms, item_id],
    ).await?;

    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn get_job_packaging_items(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<JobPackagingItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_name, quantity, price_per_unit, is_leased, returned_quantity, created_at, updated_at, sync_status FROM job_packaging_items WHERE job_ticket_id = ?1 ORDER BY created_at ASC",
    ).await?;

    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(JobPackagingItem {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            item_name: row.get(3)?,
            quantity: row.get(4)?,
            price_per_unit: row.get(5)?,
            is_leased: row.get::<i32>(6)? != 0,
            returned_quantity: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row.get::<Option<String>>(10)?.unwrap_or_else(|| "pending".to_string()),
        });
    }

    Ok(items)
}
