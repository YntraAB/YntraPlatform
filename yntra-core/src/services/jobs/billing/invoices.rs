use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;
use super::helpers::get_config_val;

pub async fn calculate_eligible_labor_cost(
    conn: &crate::database::DbConnection,
    job_ticket_id: &str,
    base_price: f64,
    settings_json: &serde_json::Value,
) -> Result<f64, YntraError> {
    let mut inv_stmt = conn.prepare(
        "SELECT quantity, estimated_volume_m3, item_name FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;
    let mut inv_rows = inv_stmt.query(crate::params![job_ticket_id]).await?;
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

    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    let labor_portion = if pricing_model == "hourly" {
        let crew_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_crew WHERE job_ticket_id = ?1",
                crate::params![job_ticket_id],
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

        let _hourly_rate_vehicle = settings_json
            .get("moving_hourly_rate_vehicle")
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0);

        let hours_per_m3 = settings_json
            .get("moving_hours_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.15);
        let minimum_hours = settings_json
            .get("moving_minimum_hours")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);

        let has_explicit_rates = settings_json.get("moving_hourly_rate_per_mover").is_some()
            || settings_json.get("moving_hourly_rate_vehicle").is_some();

        if has_explicit_rates {
            estimated_hours * active_crew_size * hourly_rate_per_mover + specialty_surcharge
        } else {
            let hourly_rate = settings_json
                .get("moving_hourly_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(1200.0);
            let labor_ratio = settings_json
                .get("moving_labor_ratio_hourly")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.70);
            estimated_hours * hourly_rate * labor_ratio + specialty_surcharge
        }
    } else {
        let base_rate_per_m3 = settings_json
            .get("moving_base_rate_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(500.0);
        let labor_ratio = settings_json
            .get("moving_labor_ratio_volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.70);
        total_volume * base_rate_per_m3 * labor_ratio + specialty_surcharge
    };

    Ok(labor_portion.min(base_price))
}

#[uniffi::export]
pub async fn generate_move_invoice(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
) -> Result<crate::models::MoveInvoice, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status FROM move_quotes WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    let (ws_id, job_ticket_id, base_price, _distance_fee, stairs_surcharge, _packing_supplies_fee, total_price, _quote_status) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<f64>(2)?,
            row.get::<f64>(3)?,
            row.get::<f64>(4)?,
            row.get::<f64>(5)?,
            row.get::<f64>(6)?,
            row.get::<String>(7)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Quote not found".to_string()));
    };

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let customer_id = if auth.role == "client" {
        requester_user_id.clone()
    } else {
        conn.query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "client-1".to_string())
    };

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    let currency = settings_json
        .get("currency")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match target_region.as_str() {
            "US" => "USD".to_string(),
            "DE" => "EUR".to_string(),
            _ => "SEK".to_string(),
        });

    let subtotal = total_price;
    
    let dynamic_tax_rate = settings_json
        .get("tax_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| {
            if target_region == "US" {
                settings_json.get("sales_tax_rate").and_then(|v| v.as_f64()).unwrap_or(0.08)
            } else if target_region == "DE" {
                settings_json.get("vat_rate").and_then(|v| v.as_f64()).unwrap_or(0.19)
            } else {
                0.0
            }
        });

    let is_rut_active = (target_region == "SE" || settings_json.get("use_rut_deduction").and_then(|v| v.as_bool()).unwrap_or(false)) && use_rut;

    let (rut_deduction, tax_authority_amount, customer_amount) = if is_rut_active {
        let eligible_labor = calculate_eligible_labor_cost(&conn, &job_ticket_id, base_price, &settings_json).await?;
        let rut = 0.5 * (eligible_labor + stairs_surcharge);
        (rut, rut, subtotal - rut)
    } else {
        let tax = subtotal * dynamic_tax_rate;
        (0.0, tax, subtotal + tax)
    };

    let now = chrono::Utc::now();
    let invoice_date = now.format("%Y-%m-%d").to_string();
    let due_date = (now + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let now_ms = now.timestamp_millis();

    let mut inv_stmt = conn.prepare("SELECT id FROM move_invoices WHERE quote_id = ?1").await?;
    let mut inv_rows = inv_stmt.query(crate::params![&quote_id]).await?;
    let invoice_id = if let Some(row) = inv_rows.next().await? {
        row.get::<String>(0)?
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let invoice = crate::models::MoveInvoice {
        id: invoice_id.clone(),
        workspace_id: ws_id.clone(),
        quote_id: quote_id.clone(),
        customer_id: customer_id.clone(),
        invoice_date: invoice_date.clone(),
        due_date: due_date.clone(),
        subtotal,
        rut_deduction,
        customer_amount,
        tax_authority_amount,
        status: "unpaid".to_string(),
        currency,
        actual_hours: None,
        additional_charges: None,
        adjustment_notes: None,
    };

    conn.execute(
        "INSERT OR REPLACE INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status, actual_hours, additional_charges, adjustment_notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending', ?13, ?14, ?15)",
        crate::params![
            invoice.id,
            invoice.workspace_id,
            invoice.quote_id,
            invoice.customer_id,
            invoice.invoice_date,
            invoice.due_date,
            invoice.subtotal,
            invoice.rut_deduction,
            invoice.customer_amount,
            invoice.tax_authority_amount,
            invoice.status,
            now_ms,
            invoice.actual_hours,
            invoice.additional_charges,
            invoice.adjustment_notes
        ]
    ).await?;

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id.clone(),
        ws_id,
        customer_id,
        "invoice_created".to_string(),
        None,
    ).await;

    notify_observers();
    Ok(invoice)
}

#[uniffi::export]
pub async fn get_move_invoice(
    requester_user_id: String,
    quote_id: String,
) -> Result<Option<crate::models::MoveInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, actual_hours, additional_charges, adjustment_notes FROM move_invoices WHERE quote_id = ?1 LIMIT 1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(1)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        
        let settings_str: String = conn
            .query_row(
                "SELECT settings FROM workspaces WHERE id = ?1",
                crate::params![&ws_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or_else(|_| "{}".to_string());
        let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
        let target_region = settings_json
            .get("target_region")
            .and_then(|v| v.as_str())
            .unwrap_or("SE")
            .to_uppercase();

        let currency = settings_json
            .get("currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| match target_region.as_str() {
                "US" => "USD".to_string(),
                "DE" => "EUR".to_string(),
                _ => "SEK".to_string(),
            });

        Ok(Some(crate::models::MoveInvoice {
            id: row.get::<String>(0)?,
            workspace_id: ws_id,
            quote_id,
            customer_id: row.get::<String>(2)?,
            invoice_date: row.get::<String>(3)?,
            due_date: row.get::<String>(4)?,
            subtotal: row.get::<f64>(5)?,
            rut_deduction: row.get::<f64>(6)?,
            customer_amount: row.get::<f64>(7)?,
            tax_authority_amount: row.get::<f64>(8)?,
            status: row.get::<String>(9)?,
            currency,
            actual_hours: row.get::<Option<f64>>(10)?,
            additional_charges: row.get::<Option<f64>>(11)?,
            adjustment_notes: row.get::<Option<String>>(12)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn pay_move_invoice(
    requester_user_id: String,
    invoice_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT workspace_id, customer_id FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, customer_id) = if let Some(row) = rows.next().await? {
        let ws = row.get::<String>(0)?;
        let cust = row.get::<String>(1)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }

        let is_staff = auth.role == "platform_admin"
            || auth.role == "admin"
            || auth.role == "assistant"
            || auth.role == "workspace_admin";

        if !is_staff {
            let settings_str: String = conn
                .query_row(
                    "SELECT settings FROM workspaces WHERE id = ?1",
                    crate::params![&ws],
                    |r| r.get(0),
                )
                .await
                .unwrap_or_else(|_| "{}".to_string());
            let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
            
            let gateway_url = get_config_val("swish_gateway_url", "SWISH_GATEWAY_URL", &settings_json).await
                .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await)
                .or(get_config_val("stripe_gateway_url", "STRIPE_GATEWAY_URL", &settings_json).await);

            let swish_sandbox = settings_json.get("swish_use_sandbox").and_then(|v| v.as_bool()).unwrap_or(false);
            let stripe_sandbox = settings_json.get("stripe_use_sandbox").and_then(|v| v.as_bool()).unwrap_or(false);
            
            if gateway_url.is_some() && !swish_sandbox && !stripe_sandbox {
                return Err(YntraError::AuthError("Access denied: only staff can manually mark production invoices as paid".to_string()));
            }
        }

        (ws, cust)
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, invoice_id],
    ).await?;

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id.clone(),
        ws_id,
        customer_id,
        "invoice_paid".to_string(),
        None,
    ).await;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn adjust_invoice_for_actuals(
    requester_user_id: String,
    invoice_id: String,
    actual_hours: Option<f64>,
    additional_charges: Option<f64>,
    notes: Option<String>,
) -> Result<crate::models::MoveInvoice, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }
    if !is_staff(&auth) {
        return Err(YntraError::AuthError("Access denied: only staff can adjust invoices".to_string()));
    }

    // 1. Fetch the existing invoice details
    let mut stmt = conn.prepare(
        "SELECT quote_id, customer_id, invoice_date, due_date, status FROM move_invoices WHERE id = ?1 AND workspace_id = ?2"
    ).await?;
    let mut rows = stmt.query(crate::params![&invoice_id, &auth.workspace_id]).await?;
    let (quote_id, customer_id, invoice_date, due_date, current_status) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<String>(2)?,
            row.get::<String>(3)?,
            row.get::<String>(4)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    // 2. Fetch quote and workspace settings to perform recalculated pricing
    let mut quote_stmt = conn.prepare(
        "SELECT job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee FROM move_quotes WHERE id = ?1"
    ).await?;
    let mut quote_rows = quote_stmt.query(crate::params![&quote_id]).await?;
    let (_job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee) = if let Some(row) = quote_rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<f64>(1)?,
            row.get::<f64>(2)?,
            row.get::<f64>(3)?,
            row.get::<f64>(4)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Corresponding quote not found".to_string()));
    };

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    let currency = settings_json
        .get("currency")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match target_region.as_str() {
            "US" => "USD".to_string(),
            "DE" => "EUR".to_string(),
            _ => "SEK".to_string(),
        });

    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    // 3. Recalculate adjusted pricing
    let mut adjusted_base_price = base_price;
    let mut actual_labor_cost = base_price;

    if pricing_model == "hourly" {
        if let Some(hrs) = actual_hours {
            // Fetch crew size from active_crew_size or fall back to default
            let crew_size = settings_json
                .get("moving_active_crew_size")
                .and_then(|v| v.as_f64())
                .unwrap_or(2.0);
            let hourly_rate_mover = settings_json
                .get("moving_hourly_rate_per_mover")
                .and_then(|v| v.as_f64())
                .unwrap_or(450.0);
            
            // Recalculate base price based on actual hours worked
            adjusted_base_price = hrs * crew_size * hourly_rate_mover;
            actual_labor_cost = adjusted_base_price;
        }
    }

    let add_charges = additional_charges.unwrap_or(0.0);
    let subtotal = adjusted_base_price + distance_fee + stairs_surcharge + packing_supplies_fee + add_charges;

    let dynamic_tax_rate = settings_json
        .get("tax_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| {
            if target_region == "US" {
                settings_json.get("sales_tax_rate").and_then(|v| v.as_f64()).unwrap_or(0.08)
            } else if target_region == "DE" {
                settings_json.get("vat_rate").and_then(|v| v.as_f64()).unwrap_or(0.19)
            } else {
                0.0
            }
        });

    // Check if RUT is enabled
    let is_rut_active = target_region == "SE" || settings_json.get("use_rut_deduction").and_then(|v| v.as_bool()).unwrap_or(false);

    let (rut_deduction, tax_authority_amount, customer_amount) = if is_rut_active {
        // Recalculate eligible labor based on the adjusted actual base price
        let eligible_labor = if pricing_model == "hourly" {
            actual_labor_cost
        } else {
            let labor_ratio = settings_json
                .get("moving_labor_ratio_volume")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.70);
            adjusted_base_price * labor_ratio
        };
        let rut = 0.5 * (eligible_labor + stairs_surcharge);
        (rut, rut, subtotal - rut)
    } else {
        let tax = subtotal * dynamic_tax_rate;
        (0.0, tax, subtotal + tax)
    };

    let invoice = crate::models::MoveInvoice {
        id: invoice_id.clone(),
        workspace_id: auth.workspace_id.clone(),
        quote_id: quote_id.clone(),
        customer_id: customer_id.clone(),
        invoice_date: invoice_date.clone(),
        due_date: due_date.clone(),
        subtotal,
        rut_deduction,
        customer_amount,
        tax_authority_amount,
        status: current_status,
        currency,
        actual_hours,
        additional_charges,
        adjustment_notes: notes.clone(),
    };

    // Update in database
    conn.execute(
        "UPDATE move_invoices SET subtotal = ?1, rut_deduction = ?2, customer_amount = ?3, tax_authority_amount = ?4, updated_at = ?5, sync_status = 'pending', actual_hours = ?6, additional_charges = ?7, adjustment_notes = ?8 WHERE id = ?9",
        crate::params![
            invoice.subtotal,
            invoice.rut_deduction,
            invoice.customer_amount,
            invoice.tax_authority_amount,
            now_ms,
            invoice.actual_hours,
            invoice.additional_charges,
            invoice.adjustment_notes,
            invoice.id
        ],
    ).await?;

    notify_observers();
    Ok(invoice)
}
