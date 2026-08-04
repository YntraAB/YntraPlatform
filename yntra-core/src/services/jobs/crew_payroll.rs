use crate::database;
use crate::infra::errors::YntraError;
use crate::models::jobs::{CrewTipDistribution, MoverPayrollBreakdown};

async fn distribute_job_customer_tip_inner(
    requester_user_id: String,
    job_ticket_id: String,
    total_tip_amount_sek: f64,
) -> Result<CrewTipDistribution, YntraError> {
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

    let crew =
        crate::services::jobs::get_job_crew(requester_user_id.clone(), job_ticket_id.clone())
            .await?;
    let crew_count = crew.len().max(1) as i32;
    let tip_per_member = (total_tip_amount_sek.max(0.0) / crew_count as f64).round();

    let tip_id = format!("tip_{}", uuid::Uuid::new_v4().simple());
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO job_tips (id, workspace_id, job_ticket_id, total_tip_amount, crew_count, tip_per_member, status, created_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'distributed', ?7, 'pending')",
        crate::params![
            &tip_id,
            &ws_id,
            &job_ticket_id,
            total_tip_amount_sek,
            crew_count,
            tip_per_member,
            now_ms,
        ],
    ).await?;

    crate::infra::observer::notify_observers();

    Ok(CrewTipDistribution {
        id: tip_id,
        workspace_id: ws_id,
        job_ticket_id,
        total_tip_amount_sek,
        crew_count,
        tip_per_member_sek: tip_per_member,
        status: "distributed".to_string(),
        created_at: now_ms,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn distribute_job_customer_tip(
    requester_user_id: String,
    job_ticket_id: String,
    total_tip_amount_sek: f64,
) -> Result<CrewTipDistribution, YntraError> {
    let fut =
        distribute_job_customer_tip_inner(requester_user_id, job_ticket_id, total_tip_amount_sek);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn distribute_job_customer_tip(
    requester_user_id: String,
    job_ticket_id: String,
    total_tip_amount_sek: f64,
) -> Result<CrewTipDistribution, YntraError> {
    distribute_job_customer_tip_inner(requester_user_id, job_ticket_id, total_tip_amount_sek).await
}

async fn get_job_tip_distribution_inner(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<CrewTipDistribution>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let res = conn
        .query_row(
            "SELECT id, workspace_id, job_ticket_id, total_tip_amount, crew_count, tip_per_member, status, created_at FROM job_tips WHERE job_ticket_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC LIMIT 1",
            crate::params![&job_ticket_id, &auth.workspace_id],
            |r| Ok(CrewTipDistribution {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                job_ticket_id: r.get(2)?,
                total_tip_amount_sek: r.get(3)?,
                crew_count: r.get(4)?,
                tip_per_member_sek: r.get(5)?,
                status: r.get(6)?,
                created_at: r.get(7)?,
            }),
        )
        .await;

    match res {
        Ok(tip) => Ok(Some(tip)),
        Err(YntraError::NoRowsReturned) => Ok(None),
        Err(e) => Err(e),
    }
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn get_job_tip_distribution(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<CrewTipDistribution>, YntraError> {
    let fut = get_job_tip_distribution_inner(requester_user_id, job_ticket_id);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_job_tip_distribution(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<CrewTipDistribution>, YntraError> {
    get_job_tip_distribution_inner(requester_user_id, job_ticket_id).await
}

async fn calculate_mover_job_payroll_split_inner(
    requester_user_id: String,
    job_ticket_id: String,
    user_id: String,
    driving_hours: f64,
    loading_hours: f64,
    is_overnight_per_diem: bool,
) -> Result<MoverPayrollBreakdown, YntraError> {
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

    // Determine crew member role (driver vs mover)
    let role: String = conn
        .query_row(
            "SELECT role FROM job_crew WHERE job_ticket_id = ?1 AND user_id = ?2",
            crate::params![&job_ticket_id, &user_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "mover".to_string());

    let driving_rate =
        if role.to_lowercase().contains("driver") || role.to_lowercase().contains("förare") {
            230.0
        } else {
            210.0
        };

    let loading_rate = 185.0;

    let total_hours = driving_hours + loading_hours;
    let overtime_hours = (total_hours - 8.0).max(0.0);
    let _regular_hours = total_hours - overtime_hours;

    // Driving pay vs Loading pay split
    let driving_pay = driving_hours * driving_rate;
    let loading_pay = loading_hours * loading_rate;
    let overtime_pay = overtime_hours * (loading_rate * 1.5);
    let per_diem = if is_overnight_per_diem { 290.0 } else { 0.0 };

    let tip_amount: f64 = conn
        .query_row(
            "SELECT tip_per_member FROM job_tips WHERE job_ticket_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC LIMIT 1",
            crate::params![&job_ticket_id, &auth.workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    let gross_total = driving_pay + loading_pay + overtime_pay + per_diem + tip_amount;

    Ok(MoverPayrollBreakdown {
        user_id,
        job_ticket_id,
        role,
        driving_hours,
        loading_hours,
        overtime_hours,
        driving_rate_sek_per_h: driving_rate,
        loading_rate_sek_per_h: loading_rate,
        overtime_multiplier: 1.5,
        per_diem_allowance_sek: per_diem,
        tip_allocated_sek: tip_amount,
        total_gross_payout_sek: gross_total,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn calculate_mover_job_payroll_split(
    requester_user_id: String,
    job_ticket_id: String,
    user_id: String,
    driving_hours: f64,
    loading_hours: f64,
    is_overnight_per_diem: bool,
) -> Result<MoverPayrollBreakdown, YntraError> {
    let fut = calculate_mover_job_payroll_split_inner(
        requester_user_id,
        job_ticket_id,
        user_id,
        driving_hours,
        loading_hours,
        is_overnight_per_diem,
    );
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn calculate_mover_job_payroll_split(
    requester_user_id: String,
    job_ticket_id: String,
    user_id: String,
    driving_hours: f64,
    loading_hours: f64,
    is_overnight_per_diem: bool,
) -> Result<MoverPayrollBreakdown, YntraError> {
    calculate_mover_job_payroll_split_inner(
        requester_user_id,
        job_ticket_id,
        user_id,
        driving_hours,
        loading_hours,
        is_overnight_per_diem,
    )
    .await
}
