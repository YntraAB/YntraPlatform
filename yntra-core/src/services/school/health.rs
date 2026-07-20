use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::notes::verify_zkp_if_encrypted;
use crate::services::school::auth::{verify_school_write_zkp, verify_school_permission, verify_student_access};
use crate::services::school::conflicts::record_school_conflict;
use crate::{HealthRecord, HealthIncident};

#[uniffi::export]
pub async fn get_student_health_records(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<HealthRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at FROM health_records WHERE workspace_id = ?1 AND student_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, student_id], |row| {
            Ok(HealthRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                vaccine_name: row.get(3)?,
                status: row.get(4)?,
                administered_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_student_health_record(
    requester_user_id: String,
    record: HealthRecord,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != record.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    
    let is_parent_self = {
        let role_lower = auth.role.to_lowercase();
        (role_lower == "parent" || role_lower == "role-school-parent") 
            && verify_student_access(&conn, &auth, &record.student_id).await.is_ok()
    };
    
    if !is_parent_self {
        verify_school_permission(&auth, "can_access_health_records")?;
    }

    let team_id = "";
    verify_zkp_if_encrypted(&conn, &record.vaccine_name, &auth.user_id, &auth.role, &record.workspace_id, team_id).await?;
    verify_zkp_if_encrypted(&conn, &record.status, &auth.user_id, &auth.role, &record.workspace_id, team_id).await?;
    if let Some(ref admin_at) = record.administered_at {
        verify_zkp_if_encrypted(&conn, admin_at, &auth.user_id, &auth.role, &record.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO health_records (id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
        crate::params![
            &record.id,
            &record.workspace_id,
            &record.student_id,
            &record.vaccine_name,
            &record.status,
            &record.administered_at,
            &now_ms
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_health_incidents(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<HealthIncident>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at FROM health_incidents WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
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
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_health_incident(
    requester_user_id: String,
    incident: HealthIncident,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != incident.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_access_health_records")?;

    let team_id = "";
    verify_zkp_if_encrypted(&conn, &incident.visit_reason, &auth.user_id, &auth.role, &incident.workspace_id, team_id).await?;
    verify_zkp_if_encrypted(&conn, &incident.treatment, &auth.user_id, &auth.role, &incident.workspace_id, team_id).await?;
    verify_zkp_if_encrypted(&conn, &incident.checked_in_at, &auth.user_id, &auth.role, &incident.workspace_id, team_id).await?;
    if let Some(ref out_at) = incident.checked_out_at {
        verify_zkp_if_encrypted(&conn, out_at, &auth.user_id, &auth.role, &incident.workspace_id, team_id).await?;
    }
    if let Some(ref notes) = incident.notes {
        verify_zkp_if_encrypted(&conn, notes, &auth.user_id, &auth.role, &incident.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, String, String, Option<String>, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at FROM health_incidents WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&incident.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        } else {
            None
        }
    };

    let mut visit_reason = incident.visit_reason.clone();
    let mut treatment = incident.treatment.clone();
    let mut checked_in_at = incident.checked_in_at.clone();
    let mut checked_out_at = incident.checked_out_at.clone();
    let mut notes = incident.notes.clone();

    let incoming_updated_at = if incident.updated_at > now_ms + 5000 { now_ms } else { incident.updated_at };
    if let Some((old_reason, old_treatment, old_in_at, old_out_at, old_notes, old_updated_at)) = existing {
        if old_updated_at > incoming_updated_at {
            let reason_diff = old_reason != incident.visit_reason;
            let treatment_diff = old_treatment != incident.treatment;
            let in_diff = old_in_at != incident.checked_in_at;
            let out_diff = old_out_at != incident.checked_out_at;
            let notes_diff = old_notes != incident.notes;

            if reason_diff || treatment_diff || in_diff || out_diff || notes_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "visit_reason": old_reason.clone(),
                            "treatment": old_treatment.clone(),
                            "checked_in_at": old_in_at.clone(),
                            "checked_out_at": old_out_at.clone(),
                            "notes": old_notes.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "visit_reason": incident.visit_reason.clone(),
                            "treatment": incident.treatment.clone(),
                            "checked_in_at": incident.checked_in_at.clone(),
                            "checked_out_at": incident.checked_out_at.clone(),
                            "notes": incident.notes.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &incident.workspace_id, "health_incidents", &incident.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > incident.updated_at)
                visit_reason = old_reason;
                treatment = old_treatment;
                checked_in_at = old_in_at;
                checked_out_at = old_out_at;
                notes = old_notes;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO health_incidents (id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &incident.id,
            &incident.workspace_id,
            &incident.student_id,
            &visit_reason,
            &treatment,
            &checked_in_at,
            &checked_out_at,
            &notes,
            &now_ms
        ]
    ).await?;

    notify_observers();
    Ok(())
}
