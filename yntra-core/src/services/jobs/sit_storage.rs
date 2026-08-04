use crate::database;
use crate::infra::errors::YntraError;
use crate::models::jobs::{SitBillingSummary, WarehouseVaultLocation};

async fn assign_job_to_warehouse_vault_inner(
    requester_user_id: String,
    job_ticket_id: String,
    vault_number: String,
    warehouse_name: String,
    allocated_volume_m3: f64,
    monthly_rate_sek: f64,
    move_in_date: String,
    estimated_move_out_date: Option<String>,
) -> Result<WarehouseVaultLocation, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let ws_id: String = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let vault_id = format!("vault_{}", uuid::Uuid::new_v4().simple());
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO warehouse_vaults (id, workspace_id, job_ticket_id, vault_number, warehouse_name, allocated_volume_m3, monthly_rate_sek, move_in_date, estimated_move_out_date, status, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'stored', ?10, ?10, 'pending')",
        crate::params![
            &vault_id,
            &ws_id,
            &job_ticket_id,
            &vault_number,
            &warehouse_name,
            allocated_volume_m3,
            monthly_rate_sek,
            &move_in_date,
            &estimated_move_out_date,
            now_ms,
        ],
    ).await?;

    // Update job ticket status/description to reflect Storage-in-Transit (SIT) active
    conn.execute(
        "UPDATE job_tickets SET status = 'in_storage', description = description || ' [SIT Magasinering: ' || ?1 || ' / ' || ?2 || ']', updated_at = ?3 WHERE id = ?4",
        crate::params![&warehouse_name, &vault_number, now_ms, &job_ticket_id],
    ).await?;

    crate::infra::observer::notify_observers();

    Ok(WarehouseVaultLocation {
        id: vault_id,
        workspace_id: ws_id,
        job_ticket_id,
        vault_number,
        warehouse_name,
        allocated_volume_m3,
        monthly_rate_sek,
        move_in_date,
        estimated_move_out_date,
        status: "stored".to_string(),
        created_at: now_ms,
        updated_at: now_ms,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn assign_job_to_warehouse_vault(
    requester_user_id: String,
    job_ticket_id: String,
    vault_number: String,
    warehouse_name: String,
    allocated_volume_m3: f64,
    monthly_rate_sek: f64,
    move_in_date: String,
    estimated_move_out_date: Option<String>,
) -> Result<WarehouseVaultLocation, YntraError> {
    let fut = assign_job_to_warehouse_vault_inner(
        requester_user_id,
        job_ticket_id,
        vault_number,
        warehouse_name,
        allocated_volume_m3,
        monthly_rate_sek,
        move_in_date,
        estimated_move_out_date,
    );
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn assign_job_to_warehouse_vault(
    requester_user_id: String,
    job_ticket_id: String,
    vault_number: String,
    warehouse_name: String,
    allocated_volume_m3: f64,
    monthly_rate_sek: f64,
    move_in_date: String,
    estimated_move_out_date: Option<String>,
) -> Result<WarehouseVaultLocation, YntraError> {
    assign_job_to_warehouse_vault_inner(
        requester_user_id,
        job_ticket_id,
        vault_number,
        warehouse_name,
        allocated_volume_m3,
        monthly_rate_sek,
        move_in_date,
        estimated_move_out_date,
    )
    .await
}

async fn get_job_warehouse_vaults_inner(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<WarehouseVaultLocation>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, job_ticket_id, vault_number, warehouse_name, allocated_volume_m3, monthly_rate_sek, move_in_date, estimated_move_out_date, status, created_at, updated_at FROM warehouse_vaults WHERE job_ticket_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC",
        )
        .await?;

    let vaults = stmt
        .query_map(crate::params![&job_ticket_id, &auth.workspace_id], |r| {
            Ok(WarehouseVaultLocation {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                job_ticket_id: r.get(2)?,
                vault_number: r.get(3)?,
                warehouse_name: r.get(4)?,
                allocated_volume_m3: r.get(5)?,
                monthly_rate_sek: r.get(6)?,
                move_in_date: r.get(7)?,
                estimated_move_out_date: r.get(8)?,
                status: r.get(9)?,
                created_at: r.get(10)?,
                updated_at: r.get(11)?,
            })
        })
        .await?;

    Ok(vaults)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn get_job_warehouse_vaults(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<WarehouseVaultLocation>, YntraError> {
    let fut = get_job_warehouse_vaults_inner(requester_user_id, job_ticket_id);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_job_warehouse_vaults(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<WarehouseVaultLocation>, YntraError> {
    get_job_warehouse_vaults_inner(requester_user_id, job_ticket_id).await
}

async fn release_job_from_warehouse_vault_inner(
    requester_user_id: String,
    vault_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, jid): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM warehouse_vaults WHERE id = ?1",
            crate::params![&vault_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Warehouse vault not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE warehouse_vaults SET status = 'released', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &vault_id, &ws_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'in_transit', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &jid, &ws_id],
    ).await?;

    crate::infra::observer::notify_observers();
    Ok(())
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn release_job_from_warehouse_vault(
    requester_user_id: String,
    vault_id: String,
) -> Result<(), YntraError> {
    let fut = release_job_from_warehouse_vault_inner(requester_user_id, vault_id);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn release_job_from_warehouse_vault(
    requester_user_id: String,
    vault_id: String,
) -> Result<(), YntraError> {
    release_job_from_warehouse_vault_inner(requester_user_id, vault_id).await
}

async fn calculate_sit_recurring_billing_summary_inner(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<SitBillingSummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let vaults =
        get_job_warehouse_vaults_inner(requester_user_id.clone(), job_ticket_id.clone()).await?;

    let vault_count = vaults.len() as i32;
    let total_volume_m3: f64 = vaults.iter().map(|v| v.allocated_volume_m3).sum();
    let monthly_rate_total: f64 = vaults.iter().map(|v| v.monthly_rate_sek).sum();
    let rate_per_m3 = if total_volume_m3 > 0.0 {
        monthly_rate_total / total_volume_m3
    } else {
        150.0
    };

    let earliest_move_in = vaults
        .iter()
        .map(|v| v.created_at)
        .min()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let now_ms = chrono::Utc::now().timestamp_millis();
    let diff_days = ((now_ms - earliest_move_in) / (1000 * 3600 * 24)).max(1);

    let months = (diff_days as f64 / 30.0).ceil();
    let accumulated_storage = monthly_rate_total * months;
    let handling_fee = vault_count as f64 * 450.0;

    Ok(SitBillingSummary {
        job_ticket_id,
        vault_count,
        total_volume_m3,
        days_in_storage: diff_days,
        monthly_rate_per_m3_sek: rate_per_m3,
        accumulated_storage_fee_sek: accumulated_storage,
        handling_in_out_fee_sek: handling_fee,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn calculate_sit_recurring_billing_summary(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<SitBillingSummary, YntraError> {
    let fut = calculate_sit_recurring_billing_summary_inner(requester_user_id, job_ticket_id);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn calculate_sit_recurring_billing_summary(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<SitBillingSummary, YntraError> {
    calculate_sit_recurring_billing_summary_inner(requester_user_id, job_ticket_id).await
}
