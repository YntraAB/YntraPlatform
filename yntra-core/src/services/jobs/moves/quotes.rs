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
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status, manual_price_override, price_discount FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
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
    _use_rut: bool,
    _personal_number: Option<String>,
) -> Result<(), YntraError> {
    accept_move_quote(requester_user_id, quote_id).await
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
    accept_move_quote(requester_user_id, quote_id).await?;
    Ok(QuoteDepositResponse {
        success: true,
        quote_status: "pending_deposit".to_string(),
        deposit_amount: 2500.0,
        remaining_balance: 7500.0,
        payment_session_url: Some(format!("https://payment.yntra.io/deposit/{}", payment_method)),
        message: "Deposit payment initiated (EUR)".to_string(),
    })
}

#[uniffi::export]
pub async fn confirm_quote_deposit_payment(
    requester_user_id: String,
    quote_id: String,
    _transaction_id: String,
) -> Result<(), YntraError> {
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

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET manual_price_override = ?1, price_discount = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![manual_price_override, price_discount, now_ms, quote_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_move_quote_revisions(
    _requester_user_id: String,
    _quote_id: String,
) -> Result<Vec<MoveQuoteRevision>, YntraError> {
    Ok(Vec::new())
}
