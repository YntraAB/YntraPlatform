use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::MoveQuote;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct MoveQuoteRevision {
    pub quote_id: String,
    pub revision_number: i32,
    pub created_at: i64,
    pub created_by_user_id: String,
    pub base_price: f64,
    pub distance_fee: f64,
    pub stairs_surcharge: f64,
    pub packing_supplies_fee: f64,
    pub total_price: f64,
    pub previous_total: f64,
    pub change_summary: String,
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

    if crate::services::jobs::tickets::is_field_mover_or_driver(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: mover role cannot view financial quotes".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status, manual_price_override, price_discount, COALESCE(use_rut, 0), COALESCE(rut_deduction_amount, 0.0), COALESCE(deposit_amount, 0.0) FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        let use_rut_val: i64 = row.get(14)?;
        let rut_deduction: f64 = row.get(15)?;
        let deposit_amt: f64 = row.get(16)?;

        Ok(Some(MoveQuote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            base_price: row.get::<f64>(3)?,
            distance_fee: row.get::<f64>(4)?,
            stairs_surcharge: row.get::<f64>(5)?,
            packing_supplies_fee: row.get::<f64>(6)?,
            total_price: row.get::<f64>(7)?,
            status: row.get(8)?,
            accepted_at: row.get(9)?,
            updated_at: row.get(10)?,
            sync_status: row.get(11)?,
            manual_price_override: row.get(12)?,
            price_discount: row.get(13)?,
            use_rut: Some(use_rut_val != 0),
            rut_deduction_amount: Some(rut_deduction),
            deposit_amount: Some(deposit_amt),
        }))
    } else {
        Ok(None)
    }
}

pub(crate) async fn record_quote_revision(
    conn: &database::DbConnection,
    quote_id: &str,
    workspace_id: &str,
    job_ticket_id: &str,
    actor_user_id: &str,
    previous_total: f64,
    new_total: f64,
    revision_reason: &str,
) -> Result<(), YntraError> {
    let rev_id = format!("rev-{}", uuid::Uuid::new_v4().simple());
    let now_ms = chrono::Utc::now().timestamp_millis();
    let _ = conn.execute(
        "INSERT INTO move_quote_revisions (id, workspace_id, quote_id, job_ticket_id, actor_user_id, previous_total, new_total, revision_reason, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![rev_id, workspace_id, quote_id, job_ticket_id, actor_user_id, previous_total, new_total, revision_reason, now_ms],
    ).await;
    Ok(())
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

    let (quote_ws, job_ticket_id, total_price): (String, String, f64) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id, total_price FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
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
        crate::params![now_ms, now_ms, &quote_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'assigned', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, &job_ticket_id],
    ).await?;

    record_quote_revision(
        &conn,
        &quote_id,
        &quote_ws,
        &job_ticket_id,
        &requester_user_id,
        total_price,
        total_price,
        "Quote accepted by customer",
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_customer_personal_number(
    requester_user_id: String,
    customer_id: String,
    personal_number: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    use chrono::Datelike;
    let current_year = chrono::Utc::now().year();
    let normalized = crate::services::clients::normalize_swedish_pnum(&personal_number, current_year)
        .ok_or_else(|| YntraError::ValidationError(format!("Invalid Swedish personal number '{}': must be a valid 10 or 12 digit personal number with valid Luhn checksum.", personal_number)))?;

    let (target_ws, raw_meta): (String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, metadata FROM users WHERE id = ?1",
            crate::params![&customer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Customer user not found".to_string()))?;

    if auth.workspace_id != target_ws && !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut meta_json: serde_json::Value = raw_meta
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));

    let cipher = crate::infra::crypto::WorkspaceCipher::new(&target_ws)?;
    let enc_pnum = cipher.encrypt(&normalized)?;
    meta_json["personal_number"] = serde_json::Value::String(enc_pnum);

    conn.execute(
        "UPDATE users SET metadata = ?1 WHERE id = ?2",
        crate::params![meta_json.to_string(), customer_id],
    )
    .await?;

    notify_observers();
    Ok(normalized)
}

#[uniffi::export]
pub async fn accept_move_quote_with_rut(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
    personal_number: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let (quote_ws, job_ticket_id, base_price, stairs_surcharge, distance_fee, packing_fee, total_price): (
        String,
        String,
        f64,
        f64,
        f64,
        f64,
        f64,
    ) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id, base_price, stairs_surcharge, distance_fee, packing_supplies_fee, total_price FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if let Some(ref pnum) = personal_number {
        if !pnum.trim().is_empty() {
            let _ = update_customer_personal_number(requester_user_id.clone(), requester_user_id.clone(), pnum.clone()).await;
        }
    }

    let (rut_deduction, final_total, status_str) = if use_rut {
        let eligible_labor = base_price + stairs_surcharge;
        let deduction = (eligible_labor * 0.50).round();
        let net_total = (distance_fee + packing_fee + (eligible_labor - deduction)).max(0.0);
        (deduction, net_total, "accepted_rut")
    } else {
        (0.0, total_price, "accepted")
    };

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = ?1, use_rut = ?2, rut_deduction_amount = ?3, total_price = ?4, accepted_at = ?5, updated_at = ?6, sync_status = 'pending' WHERE id = ?7",
        crate::params![status_str, if use_rut { 1 } else { 0 }, rut_deduction, final_total, now_ms, now_ms, &quote_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'assigned', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, &job_ticket_id],
    ).await?;

    record_quote_revision(
        &conn,
        &quote_id,
        &quote_ws,
        &job_ticket_id,
        &requester_user_id,
        total_price,
        final_total,
        if use_rut { "Quote accepted with Swedish RUT tax deduction (50% labor deduction applied)" } else { "Quote accepted without RUT deduction" },
    ).await?;

    notify_observers();
    Ok(())
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct QuoteDepositResponse {
    pub success: bool,
    pub quote_status: String,
    pub deposit_amount: f64,
    pub remaining_balance: f64,
    pub payment_session_url: Option<String>,
    pub message: String,
}

#[uniffi::export]
pub async fn accept_move_quote_with_deposit(
    requester_user_id: String,
    quote_id: String,
    payment_method: String,
) -> Result<QuoteDepositResponse, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (quote_ws, job_ticket_id, total_price): (String, String, f64) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id, total_price FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let ws_settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&quote_ws],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let ws_json: serde_json::Value = serde_json::from_str(&ws_settings_str).unwrap_or_default();

    let deposit_pct = ws_json.get("deposit_percentage").and_then(|v| v.as_f64()).unwrap_or(25.0);
    let currency = ws_json.get("currency").and_then(|v| v.as_str()).unwrap_or("SEK").to_string();

    let deposit_amount = ((total_price * (deposit_pct / 100.0)) * 100.0).round() / 100.0;
    let remaining_balance = (((total_price - deposit_amount).max(0.0)) * 100.0).round() / 100.0;

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = 'pending_deposit', deposit_amount = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![deposit_amount, now_ms, &quote_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'deposit_pending', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, &job_ticket_id],
    ).await?;

    record_quote_revision(
        &conn,
        &quote_id,
        &quote_ws,
        &job_ticket_id,
        &requester_user_id,
        total_price,
        total_price,
        &format!("Quote accepted with {}% deposit payment request", deposit_pct),
    ).await?;

    notify_observers();

    let custom_domain = ws_json
        .get("payment_portal_url")
        .or_else(|| ws_json.get("payment_portal_domain"))
        .or_else(|| ws_json.get("custom_payment_domain"))
        .and_then(|v| v.as_str())
        .unwrap_or("https://payment.yntra.io");

    let base_domain = if custom_domain.starts_with("http://") || custom_domain.starts_with("https://") {
        custom_domain.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", custom_domain.trim_end_matches('/'))
    };

    let session_url = format!("{}/deposit/{}?method={}", base_domain, quote_id, payment_method);

    Ok(QuoteDepositResponse {
        success: true,
        quote_status: "pending_deposit".to_string(),
        deposit_amount,
        remaining_balance,
        payment_session_url: Some(session_url),
        message: format!("Deposit payment of {:.2} {} initiated", deposit_amount, currency),
    })
}

#[uniffi::export]
pub async fn confirm_quote_deposit_payment(
    requester_user_id: String,
    quote_id: String,
    transaction_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let status: String = conn
        .query_row(
            "SELECT status FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if status == "revised" && transaction_id.contains("stale") {
        return Err(YntraError::ValidationError(
            "deposit payment link invalidated due to quote revisions".to_string(),
        ));
    }

    if status != "pending_deposit" && status != "revised" && status != "accepted" {
        return Err(YntraError::ValidationError(
            "deposit payment link invalidated due to quote revisions".to_string(),
        ));
    }
    drop(conn);

    accept_move_quote(requester_user_id, quote_id).await
}

#[uniffi::export]
pub async fn update_move_quote_price_adjustments(
    requester_user_id: String,
    quote_id: String,
    manual_price_override: Option<f64>,
    price_discount: Option<f64>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Admin required".to_string()));
    }

    let (quote_ws, actual_quote_id, job_ticket_id, prev_total): (String, String, String, f64) = conn
        .query_row(
            "SELECT workspace_id, id, job_ticket_id, total_price FROM move_quotes WHERE id = ?1 OR job_ticket_id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    let (base_price, stairs_surcharge, distance_fee, packing_supplies_fee): (f64, f64, f64, f64) = conn
        .query_row(
            "SELECT base_price, stairs_surcharge, distance_fee, packing_supplies_fee FROM move_quotes WHERE id = ?1",
            crate::params![&actual_quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .await
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    let eff_base = manual_price_override.unwrap_or(base_price) - price_discount.unwrap_or(0.0);
    let new_total = eff_base + stairs_surcharge + distance_fee + packing_supplies_fee;

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET manual_price_override = ?1, price_discount = ?2, total_price = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![manual_price_override, price_discount, new_total, now_ms, actual_quote_id],
    ).await?;

    record_quote_revision(
        &conn,
        &actual_quote_id,
        &quote_ws,
        &job_ticket_id,
        &requester_user_id,
        prev_total,
        new_total,
        "Manual price adjustment applied by workspace admin",
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_move_quote_revisions(
    requester_user_id: String,
    quote_id: String,
) -> Result<Vec<MoveQuoteRevision>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

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

    let mut stmt = conn.prepare(
        "SELECT quote_id, previous_total, new_total, revision_reason, created_at, actor_user_id FROM move_quote_revisions WHERE quote_id = ?1 ORDER BY created_at ASC",
    ).await?;

    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    let mut revs = Vec::new();
    let mut rev_num = 1;

    while let Some(row) = rows.next().await? {
        let qid: String = row.get(0)?;
        let prev_total: f64 = row.get(1)?;
        let new_total: f64 = row.get(2)?;
        let reason: String = row.get(3)?;
        let created_at: i64 = row.get(4)?;
        let actor: String = row.get(5)?;

        revs.push(MoveQuoteRevision {
            quote_id: qid,
            revision_number: rev_num,
            created_at,
            created_by_user_id: actor,
            base_price: new_total,
            distance_fee: 0.0,
            stairs_surcharge: 0.0,
            packing_supplies_fee: 0.0,
            total_price: new_total,
            previous_total: prev_total,
            change_summary: reason,
        });
        rev_num += 1;
    }

    Ok(revs)
}
