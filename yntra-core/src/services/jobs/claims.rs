use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;
use crate::DamagedItemClaim;
use uuid::Uuid;

async fn ensure_damaged_item_claims_schema(conn: &database::DbConnection) -> Result<(), YntraError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS damaged_item_claims (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            item_name TEXT NOT NULL,
            description TEXT NOT NULL,
            claimed_amount REAL NOT NULL,
            approved_amount REAL,
            repair_quote_amount REAL,
            insurance_reference TEXT,
            photo_urls_json TEXT NOT NULL DEFAULT '[]',
            status TEXT NOT NULL DEFAULT 'submitted',
            settlement_notes TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE
        )",
        (),
    ).await?;
    Ok(())
}

#[uniffi::export]
pub async fn submit_damaged_item_claim(
    requester_user_id: String,
    job_ticket_id: String,
    item_name: String,
    description: String,
    claimed_amount: f64,
    photo_urls_json: String,
) -> Result<DamagedItemClaim, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if claimed_amount <= 0.0 {
        return Err(YntraError::ValidationError("Claimed amount must be greater than 0".to_string()));
    }

    ensure_damaged_item_claims_schema(&conn).await?;

    let claim_id = format!("claim-{}", Uuid::new_v4().simple());
    let now_ms = chrono::Utc::now().timestamp_millis();

    let claim = DamagedItemClaim {
        id: claim_id.clone(),
        workspace_id: job_ws.clone(),
        job_ticket_id: job_ticket_id.clone(),
        item_name: item_name.clone(),
        description: description.clone(),
        claimed_amount,
        approved_amount: None,
        repair_quote_amount: None,
        insurance_reference: None,
        photo_urls_json: if photo_urls_json.trim().is_empty() { "[]".to_string() } else { photo_urls_json },
        status: "submitted".to_string(),
        settlement_notes: None,
        created_at: now_ms,
        updated_at: now_ms,
    };

    conn.execute(
        "INSERT INTO damaged_item_claims (id, workspace_id, job_ticket_id, item_name, description, claimed_amount, approved_amount, repair_quote_amount, insurance_reference, photo_urls_json, status, settlement_notes, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        crate::params![
            claim.id,
            claim.workspace_id,
            claim.job_ticket_id,
            claim.item_name,
            claim.description,
            claim.claimed_amount,
            claim.approved_amount,
            claim.repair_quote_amount,
            claim.insurance_reference,
            claim.photo_urls_json,
            claim.status,
            claim.settlement_notes,
            claim.created_at,
            claim.updated_at
        ],
    ).await?;

    notify_observers();
    Ok(claim)
}

#[uniffi::export]
pub async fn update_claim_status(
    requester_user_id: String,
    claim_id: String,
    status: String,
    approved_amount: Option<f64>,
    repair_quote_amount: Option<f64>,
    insurance_reference: Option<String>,
    settlement_notes: Option<String>,
) -> Result<DamagedItemClaim, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError("Access denied: only staff coordinators can update claims".to_string()));
    }

    ensure_damaged_item_claims_schema(&conn).await?;

    let (claim_ws, _job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM damaged_item_claims WHERE id = ?1",
            crate::params![&claim_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Claim {} not found", claim_id)))?;

    if auth.workspace_id != claim_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let new_status = status.to_lowercase();

    conn.execute(
        "UPDATE damaged_item_claims SET status = ?1, approved_amount = ?2, repair_quote_amount = ?3, insurance_reference = ?4, settlement_notes = ?5, updated_at = ?6 WHERE id = ?7 AND workspace_id = ?8",
        crate::params![
            new_status,
            approved_amount,
            repair_quote_amount,
            insurance_reference,
            settlement_notes,
            now_ms,
            claim_id,
            auth.workspace_id
        ],
    ).await?;

    notify_observers();

    let updated_claim = conn.query_row(
        "SELECT id, workspace_id, job_ticket_id, item_name, description, claimed_amount, approved_amount, repair_quote_amount, insurance_reference, photo_urls_json, status, settlement_notes, created_at, updated_at FROM damaged_item_claims WHERE id = ?1",
        crate::params![&claim_id],
        |r| Ok(DamagedItemClaim {
            id: r.get(0)?,
            workspace_id: r.get(1)?,
            job_ticket_id: r.get(2)?,
            item_name: r.get(3)?,
            description: r.get(4)?,
            claimed_amount: r.get(5)?,
            approved_amount: r.get(6)?,
            repair_quote_amount: r.get(7)?,
            insurance_reference: r.get(8)?,
            photo_urls_json: r.get(9)?,
            status: r.get(10)?,
            settlement_notes: r.get(11)?,
            created_at: r.get(12)?,
            updated_at: r.get(13)?,
        }),
    ).await?;

    Ok(updated_claim)
}

#[uniffi::export]
pub async fn process_claim_payout(
    requester_user_id: String,
    claim_id: String,
    payout_amount: f64,
    insurance_policy_claim_ref: String,
) -> Result<crate::models::ClaimPayoutResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError("Access denied: only staff coordinators can process claim payouts".to_string()));
    }

    let claim = update_claim_status(
        requester_user_id,
        claim_id.clone(),
        "paid".to_string(),
        Some(payout_amount),
        None,
        Some(insurance_policy_claim_ref.clone()),
        Some(format!("Insurance payout processed: {:.2} SEK approved under policy ref {}", payout_amount, insurance_policy_claim_ref)),
    ).await?;

    Ok(crate::models::ClaimPayoutResult {
        success: true,
        claim_id: claim.id,
        payout_amount,
        insurance_reference: insurance_policy_claim_ref,
        new_status: "paid".to_string(),
        message: format!("Successfully disbursed insurance payout of {:.2} SEK.", payout_amount),
    })
}

#[uniffi::export]
pub async fn get_job_claims(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<DamagedItemClaim>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    ensure_damaged_item_claims_schema(&conn).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_name, description, claimed_amount, approved_amount, repair_quote_amount, insurance_reference, photo_urls_json, status, settlement_notes, created_at, updated_at FROM damaged_item_claims WHERE job_ticket_id = ?1"
    ).await?;

    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let mut claims = Vec::new();
    while let Some(row) = rows.next().await? {
        claims.push(DamagedItemClaim {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            item_name: row.get(3)?,
            description: row.get(4)?,
            claimed_amount: row.get(5)?,
            approved_amount: row.get(6)?,
            repair_quote_amount: row.get(7)?,
            insurance_reference: row.get(8)?,
            photo_urls_json: row.get(9)?,
            status: row.get(10)?,
            settlement_notes: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        });
    }

    Ok(claims)
}
