use crate::database;
use crate::observer::notify_observers;
use crate::{HealthRecord, HealthIncident, YntraError};
use super::{check_permission, has_health_access};

#[uniffi::export]
pub async fn get_health_records(requester_user_id: String, student_id: String) -> Result<Vec<HealthRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !has_health_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access to health records denied".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status FROM health_records WHERE student_id = ?1").await?;
    let list = stmt.query_map(crate::params![&student_id], |row| {
        Ok(HealthRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            student_id: row.get(2)?,
            vaccine_name: row.get(3)?,
            status: row.get(4)?,
            administered_at: row.get(5)?,
            updated_at: row.get(6)?,
            sync_status: row.get(7)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn save_health_record(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    vaccine_name: String,
    status: String,
    administered_at: Option<String>,
) -> Result<HealthRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    if !check_permission(&conn, &requester_user_id, "can_access_health_records").await? {
        return Err(YntraError::AuthError("Access denied: you do not have permission to manage health records".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let actual_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let record = HealthRecord {
        id: actual_id.clone(),
        workspace_id: workspace_id.clone(),
        student_id: student_id.clone(),
        vaccine_name: vaccine_name.clone(),
        status: status.clone(),
        administered_at: administered_at.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO health_records (id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        crate::params![
            &record.id,
            &record.workspace_id,
            &record.student_id,
            &record.vaccine_name,
            &record.status,
            &record.administered_at,
            &record.updated_at,
            &record.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(record)
}

#[uniffi::export]
pub async fn get_health_incidents(requester_user_id: String, student_id: String) -> Result<Vec<HealthIncident>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !has_health_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access to health incidents denied".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status FROM health_incidents WHERE student_id = ?1 ORDER BY checked_in_at DESC").await?;
    let list = stmt.query_map(crate::params![&student_id], |row| {
        Ok(HealthIncident {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            student_id: row.get(2)?,
            visit_reason: row.get(3)?,
            treatment: row.get(4)?,
            checked_in_at: row.get(5)?,
            checked_out_at: row.get(6)?,
            notes: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn save_health_incident(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    visit_reason: String,
    treatment: String,
    checked_in_at: String,
    checked_out_at: Option<String>,
    notes: Option<String>,
) -> Result<HealthIncident, YntraError> {
    let conn = database::acquire_connection().await?;
    if !check_permission(&conn, &requester_user_id, "can_access_health_records").await? {
        return Err(YntraError::AuthError("Access denied: you do not have permission to manage health incidents".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let actual_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let incident = HealthIncident {
        id: actual_id.clone(),
        workspace_id: workspace_id.clone(),
        student_id: student_id.clone(),
        visit_reason: visit_reason.clone(),
        treatment: treatment.clone(),
        checked_in_at: checked_in_at.clone(),
        checked_out_at: checked_out_at.clone(),
        notes: notes.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO health_incidents (id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            &incident.id,
            &incident.workspace_id,
            &incident.student_id,
            &incident.visit_reason,
            &incident.treatment,
            &incident.checked_in_at,
            &incident.checked_out_at,
            &incident.notes,
            &incident.updated_at,
            &incident.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(incident)
}
