use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;

pub async fn calculate_eligible_labor_cost(
    conn: &crate::database::DbConnection,
    job_ticket_id: &str,
    base_price: f64,
    settings_json: &serde_json::Value,
) -> Result<f64, YntraError> {
    let mut inv_stmt = conn.prepare(
        "SELECT quantity, item_name, item_category, handling_notes FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;
    let mut inv_rows = inv_stmt.query(crate::params![job_ticket_id]).await?;
    let mut specialty_surcharge = 0.0;

    let surcharge_piano = settings_json.get("surcharge_piano").and_then(|v| v.as_f64()).unwrap_or(1500.0);
    let surcharge_safe = settings_json.get("surcharge_safe").and_then(|v| v.as_f64()).unwrap_or(2000.0);
    let surcharge_jacuzzi = settings_json.get("surcharge_jacuzzi").and_then(|v| v.as_f64()).unwrap_or(2500.0);
    let surcharge_fragile = settings_json.get("surcharge_fragile").and_then(|v| v.as_f64()).unwrap_or(500.0);
    let surcharge_server_rack = settings_json.get("surcharge_server_rack").and_then(|v| v.as_f64()).unwrap_or(3000.0);
    let surcharge_fitness_equipment = settings_json.get("surcharge_fitness_equipment").and_then(|v| v.as_f64()).unwrap_or(800.0);
    let surcharge_marble_glass = settings_json.get("surcharge_marble_glass").and_then(|v| v.as_f64()).unwrap_or(600.0);

    while let Some(row) = inv_rows.next().await? {
        let quantity: i64 = row.get(0)?;
        let item_name: String = row.get(1)?;
        let item_category: String = row.get(2)?;
        let handling_notes: Option<String> = row.get(3)?;

        let item_fee = crate::services::jobs::calculate_item_specialty_surcharge_extended(
            item_category,
            item_name,
            handling_notes,
            surcharge_piano,
            surcharge_safe,
            surcharge_jacuzzi,
            surcharge_fragile,
            surcharge_server_rack,
            surcharge_fitness_equipment,
            surcharge_marble_glass,
        );
        specialty_surcharge += item_fee * (quantity as f64);
    }

    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    let effective_base_price = if base_price > 0.0 {
        base_price
    } else {
        settings_json.get("moving_minimum_job_price").and_then(|v| v.as_f64()).unwrap_or(0.0)
    };

    let base_labor_price = (effective_base_price - specialty_surcharge).max(0.0);

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
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

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

        let has_explicit_rates = settings_json.get("moving_hourly_rate_per_mover").is_some()
            || settings_json.get("moving_hourly_rate_vehicle").is_some();

        if has_explicit_rates {
            let total_rate = (active_crew_size * hourly_rate_per_mover) + hourly_rate_vehicle;
            let mover_ratio = if total_rate > 0.0 {
                (active_crew_size * hourly_rate_per_mover) / total_rate
            } else {
                1.0
            };
            (base_labor_price * mover_ratio) + specialty_surcharge
        } else {
            let labor_ratio = settings_json
                .get("moving_labor_ratio_hourly")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            (base_labor_price * labor_ratio) + specialty_surcharge
        }
    } else {
        let labor_ratio = settings_json
            .get("moving_labor_ratio_volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.70);
        (base_labor_price * labor_ratio) + specialty_surcharge
    };

    Ok(labor_portion.min(base_price))
}

pub async fn calculate_customer_annual_rut_used(
    conn: &crate::database::DbConnection,
    workspace_id: &str,
    customer_id: &str,
    target_year: &str,
    current_invoice_id: Option<&str>,
) -> Result<f64, YntraError> {
    let year_prefix = format!("{}%", target_year);
    // 1. Issued/paid move invoices (excluding cancelled ones)
    let mut stmt = conn.prepare(
        "SELECT id, rut_deduction FROM move_invoices WHERE workspace_id = ?1 AND customer_id = ?2 AND invoice_date LIKE ?3 AND rut_deduction > 0.0 AND status != 'cancelled'",
    ).await?;
    let mut rows = stmt.query(crate::params![workspace_id, customer_id, &year_prefix]).await?;
    let mut total_rut = 0.0;
    while let Some(row) = rows.next().await? {
        let inv_id: String = row.get(0)?;
        if let Some(curr_id) = current_invoice_id {
            if inv_id == curr_id {
                continue;
            }
        }
        let rut_val: f64 = row.get(1)?;
        total_rut += rut_val;
    }

    // 2. Committed RUT from pending accepted quotes not yet invoiced for this customer in target_year
    let mut quote_stmt = conn.prepare(
        "SELECT q.base_price, q.stairs_surcharge FROM move_quotes q
         JOIN job_tickets j ON q.job_ticket_id = j.id
         WHERE q.workspace_id = ?1
           AND (j.assigned_user_id = ?2 OR j.assigned_user_id IS NULL OR j.assigned_user_id = '' OR ?2 IN (SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client'))
           AND (j.scheduled_date LIKE ?3 OR j.scheduled_date = '' OR j.scheduled_date IS NULL)
           AND q.status = 'accepted'
           AND q.id NOT IN (SELECT quote_id FROM move_invoices WHERE workspace_id = ?1 AND status != 'cancelled')",
    ).await?;
    let mut quote_rows = quote_stmt.query(crate::params![workspace_id, customer_id, &year_prefix]).await?;
    while let Some(row) = quote_rows.next().await? {
        let base_price: f64 = row.get(0)?;
        let stairs_surcharge: f64 = row.get(1)?;
        let estimated_quote_rut = 0.5 * ((base_price * 0.70) + stairs_surcharge);
        total_rut += estimated_quote_rut;
    }

    Ok(total_rut)
}
#[uniffi::export]
pub fn validate_customer_personal_number_for_rut(personal_number: &str) -> Result<String, YntraError> {
    use chrono::Datelike;
    let current_year = chrono::Utc::now().year();
    crate::services::clients::normalize_swedish_pnum(personal_number, current_year).ok_or_else(|| {
        YntraError::ValidationError(format!(
            "Invalid Swedish personal number '{}': must be a valid 10 or 12 digit personal number with valid Luhn checksum.",
            personal_number
        ))
    })
}

#[uniffi::export]
pub async fn generate_move_invoice(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
) -> Result<crate::models::MoveInvoice, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ticket_id, base_price, _distance_fee, stairs_surcharge, _packing_supplies_fee, total_price, ws_id): (
        String,
        f64,
        f64,
        f64,
        f64,
        f64,
        String,
    ) = conn
        .query_row(
            "SELECT job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, workspace_id FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.role != "admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let customer_id: String = if auth.role == "client" {
        requester_user_id.clone()
    } else {
        conn.query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| requester_user_id.clone())
    };

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| Ok(r.get::<String>(0)?),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            settings_json
                .get("company_country")
                .and_then(|v| v.as_str())
                .unwrap_or("SE")
        });

    let currency = settings_json
        .get("currency")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match target_region {
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

    let now = chrono::Utc::now();
    let invoice_date = now.format("%Y-%m-%d").to_string();
    let due_date = (now + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let target_year = now.format("%Y").to_string();

    let is_rut_active = (target_region == "SE" || settings_json.get("use_rut_deduction").and_then(|v| v.as_bool()).unwrap_or(false)) && use_rut;

    let (rut_deduction, tax_authority_amount, customer_amount) = if is_rut_active {
        // Upfront validation of customer personal number
        let raw_pnum: Option<String> = conn
            .query_row(
                "SELECT metadata ->> 'personal_number' FROM users WHERE id = ?1",
                crate::params![&customer_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(None);

        let valid_pnum = if let Some(ref pnum_str) = raw_pnum {
            let decrypted = if pnum_str.starts_with("enc:") || pnum_str.len() > 30 {
                crate::infra::crypto::decrypt_field(pnum_str, &ws_id).unwrap_or_else(|_| pnum_str.clone())
            } else {
                pnum_str.clone()
            };
            use chrono::Datelike;
            let current_year = chrono::Utc::now().year();
            crate::services::clients::normalize_swedish_pnum(&decrypted, current_year)
        } else {
            None
        };

        if valid_pnum.is_none() {
            return Err(YntraError::ValidationError(format!(
                "Invalid or missing Swedish personal number for customer {}. RUT deduction requires a valid 10 or 12 digit personal number with a valid Luhn checksum.",
                customer_id
            )));
        }

        let eligible_labor = calculate_eligible_labor_cost(&conn, &job_ticket_id, base_price, &settings_json).await?;

        let long_carry_meters: f64 = conn
            .query_row(
                "SELECT COALESCE(long_carry_meters, 0) FROM job_tickets WHERE id = ?1",
                crate::params![&job_ticket_id],
                |r| Ok(r.get::<i64>(0)? as f64),
            )
            .await
            .unwrap_or(0.0);

        let surcharge_long_carry_per_meter = settings_json.get("surcharge_long_carry_per_meter").and_then(|v| v.as_f64()).unwrap_or(50.0);
        let surcharge_crane_hoist = settings_json.get("surcharge_crane_hoist").and_then(|v| v.as_f64()).unwrap_or(1500.0);
        let requires_crane_hoist = settings_json.get("requires_crane_hoist").and_then(|v| v.as_bool()).unwrap_or(false);

        let long_carry_fee = long_carry_meters * surcharge_long_carry_per_meter;
        let crane_hoist_fee = if requires_crane_hoist { surcharge_crane_hoist } else { 0.0 };

        let non_deductible_stair_additions = long_carry_fee + crane_hoist_fee;
        let eligible_stair_labor = (stairs_surcharge - non_deductible_stair_additions).max(0.0);

        let raw_rut = 0.5 * (eligible_labor + eligible_stair_labor);

        let used_rut = calculate_customer_annual_rut_used(&conn, &ws_id, &customer_id, &target_year, None).await.unwrap_or(0.0);
        let annual_cap = settings_json.get("annual_rut_limit_per_person").and_then(|v| v.as_f64()).unwrap_or(75000.0);
        let remaining_cap = (annual_cap - used_rut).max(0.0);
        let rut = raw_rut.min(remaining_cap);
        (rut, rut, subtotal - rut)
    } else {
        let tax = subtotal * dynamic_tax_rate;
        (0.0, tax, subtotal + tax)
    };

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

    if auth.role == "mover" || auth.role == "driver" {
        return Err(YntraError::AuthError(
            "Access denied: mover role cannot view move invoices".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, actual_hours, additional_charges, adjustment_notes FROM move_invoices WHERE quote_id = ?1 LIMIT 1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(1)?;
        let cust_id = row.get::<String>(2)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        if !is_staff(&auth) && auth.user_id != cust_id {
            return Err(YntraError::AuthError("Access denied: customer mismatch".to_string()));
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

        if !is_staff(&auth) {
            return Err(YntraError::AuthError(
                "Access denied: only authorized staff can manually mark invoices as paid. Clients must process payments via an integrated gateway.".to_string(),
            ));
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
    let (job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee) = if let Some(row) = quote_rows.next().await? {
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

    if pricing_model == "hourly" {
        if let Some(hrs) = actual_hours {
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
                .and_then(|v| v.as_f64())
                .or_else(|| settings_json.get("moving_active_crew_size").and_then(|v| v.as_f64()))
                .unwrap_or(2.0);

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

            let has_explicit_rates = settings_json.get("moving_hourly_rate_per_mover").is_some()
                || settings_json.get("moving_hourly_rate_vehicle").is_some();

            let hourly_rate = if has_explicit_rates {
                active_crew_size * hourly_rate_per_mover + hourly_rate_vehicle
            } else {
                settings_json
                    .get("moving_hourly_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1200.0)
            };

            let mut inv_stmt = conn.prepare(
                "SELECT quantity, estimated_volume_m3, item_name, item_category, handling_notes FROM move_inventory WHERE job_ticket_id = ?1",
            ).await?;
            let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
            let mut specialty_surcharge = 0.0;

            let surcharge_piano = settings_json.get("surcharge_piano").and_then(|v| v.as_f64()).unwrap_or(1500.0);
            let surcharge_safe = settings_json.get("surcharge_safe").and_then(|v| v.as_f64()).unwrap_or(2000.0);
            let surcharge_jacuzzi = settings_json.get("surcharge_jacuzzi").and_then(|v| v.as_f64()).unwrap_or(2500.0);
            let surcharge_fragile = settings_json.get("surcharge_fragile").and_then(|v| v.as_f64()).unwrap_or(500.0);
            let surcharge_server_rack = settings_json.get("surcharge_server_rack").and_then(|v| v.as_f64()).unwrap_or(3000.0);
            let surcharge_fitness_equipment = settings_json.get("surcharge_fitness_equipment").and_then(|v| v.as_f64()).unwrap_or(800.0);
            let surcharge_marble_glass = settings_json.get("surcharge_marble_glass").and_then(|v| v.as_f64()).unwrap_or(600.0);

            while let Some(row) = inv_rows.next().await? {
                let quantity: i64 = row.get(0)?;
                let _vol: f64 = row.get(1)?;
                let item_name: String = row.get(2)?;
                let item_category: String = row.get(3)?;
                let handling_notes: Option<String> = row.get(4)?;

                let item_fee = crate::services::jobs::calculate_item_specialty_surcharge_extended(
                    item_category,
                    item_name,
                    handling_notes,
                    surcharge_piano,
                    surcharge_safe,
                    surcharge_jacuzzi,
                    surcharge_fragile,
                    surcharge_server_rack,
                    surcharge_fitness_equipment,
                    surcharge_marble_glass,
                );
                specialty_surcharge += item_fee * (quantity as f64);
            }

            adjusted_base_price = (hrs * hourly_rate) + specialty_surcharge;
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
        let eligible_labor = calculate_eligible_labor_cost(&conn, &job_ticket_id, adjusted_base_price, &settings_json).await?;
        
        let long_carry_meters: f64 = conn
            .query_row(
                "SELECT COALESCE(long_carry_meters, 0) FROM job_tickets WHERE id = ?1",
                crate::params![&job_ticket_id],
                |r| Ok(r.get::<i64>(0)? as f64),
            )
            .await
            .unwrap_or(0.0);

        let surcharge_long_carry_per_meter = settings_json.get("surcharge_long_carry_per_meter").and_then(|v| v.as_f64()).unwrap_or(50.0);
        let surcharge_crane_hoist = settings_json.get("surcharge_crane_hoist").and_then(|v| v.as_f64()).unwrap_or(1500.0);
        let requires_crane_hoist = settings_json.get("requires_crane_hoist").and_then(|v| v.as_bool()).unwrap_or(false);

        let long_carry_fee = long_carry_meters * surcharge_long_carry_per_meter;
        let crane_hoist_fee = if requires_crane_hoist { surcharge_crane_hoist } else { 0.0 };

        let non_deductible_stair_additions = long_carry_fee + crane_hoist_fee;
        let eligible_stair_labor = (stairs_surcharge - non_deductible_stair_additions).max(0.0);

        let raw_rut = 0.5 * (eligible_labor + eligible_stair_labor);

        let target_year = if invoice_date.len() >= 4 { &invoice_date[0..4] } else { "2026" };
        let used_rut = calculate_customer_annual_rut_used(&conn, &auth.workspace_id, &customer_id, target_year, Some(&invoice_id)).await.unwrap_or(0.0);
        let annual_cap = settings_json.get("annual_rut_limit_per_person").and_then(|v| v.as_f64()).unwrap_or(75000.0);
        let remaining_cap = (annual_cap - used_rut).max(0.0);
        let rut = raw_rut.min(remaining_cap);
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
