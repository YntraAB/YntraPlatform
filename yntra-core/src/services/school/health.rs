use crate::database;
use crate::observer::notify_observers;
use crate::{HealthRecord, HealthIncident, YntraError};
use super::{check_permission, has_health_access};

fn decrypt_field_fallback(val: String, workspace_id: &str) -> String {
    crate::infra::crypto::decrypt_field(&val, workspace_id).unwrap_or(val)
}

fn decrypt_opt_field_fallback(val: Option<String>, workspace_id: &str) -> Option<String> {
    if let Some(v) = val {
        match crate::infra::crypto::decrypt_field(&v, workspace_id) {
            Ok(dec) => Some(dec),
            Err(_) => Some(v),
        }
    } else {
        None
    }
}

#[uniffi::export]
pub async fn get_health_records(requester_user_id: String, student_id: String) -> Result<Vec<HealthRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !has_health_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access to health records denied".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status FROM health_records WHERE student_id = ?1").await?;
    let list = stmt.query_map(crate::params![&student_id], |row| {
        let ws_id: String = row.get(1)?;
        let raw_vaccine: String = row.get(3)?;
        let raw_status: String = row.get(4)?;
        Ok(HealthRecord {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            student_id: row.get(2)?,
            vaccine_name: decrypt_field_fallback(raw_vaccine, &ws_id),
            status: decrypt_field_fallback(raw_status, &ws_id),
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
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let student_ws: String = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![&student_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if student_ws != workspace_id {
        return Err(YntraError::ValidationError("Student does not belong to the specified workspace".to_string()));
    }

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

    let enc_vaccine_name = crate::infra::crypto::encrypt_field(&vaccine_name, &workspace_id)?;
    let enc_status = crate::infra::crypto::encrypt_field(&status, &workspace_id)?;

    conn.execute(
        "INSERT OR REPLACE INTO health_records (id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        crate::params![
            &record.id,
            &record.workspace_id,
            &record.student_id,
            &enc_vaccine_name,
            &enc_status,
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
        let ws_id: String = row.get(1)?;
        let raw_reason: String = row.get(3)?;
        let raw_treatment: String = row.get(4)?;
        let raw_notes: Option<String> = row.get(7)?;
        Ok(HealthIncident {
            id: row.get(0)?,
            workspace_id: ws_id.clone(),
            student_id: row.get(2)?,
            visit_reason: decrypt_field_fallback(raw_reason, &ws_id),
            treatment: decrypt_field_fallback(raw_treatment, &ws_id),
            checked_in_at: row.get(5)?,
            checked_out_at: row.get(6)?,
            notes: decrypt_opt_field_fallback(raw_notes, &ws_id),
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
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let student_ws: String = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![&student_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if student_ws != workspace_id {
        return Err(YntraError::ValidationError("Student does not belong to the specified workspace".to_string()));
    }

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

    let enc_visit_reason = crate::infra::crypto::encrypt_field(&visit_reason, &workspace_id)?;
    let enc_treatment = crate::infra::crypto::encrypt_field(&treatment, &workspace_id)?;
    let enc_notes = crate::infra::crypto::encrypt_opt_field(notes.clone(), &workspace_id)?;

    conn.execute(
        "INSERT OR REPLACE INTO health_incidents (id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            &incident.id,
            &incident.workspace_id,
            &incident.student_id,
            &enc_visit_reason,
            &enc_treatment,
            &incident.checked_in_at,
            &incident.checked_out_at,
            &enc_notes,
            &incident.updated_at,
            &incident.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(incident)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_get_health_records_authorized_nurse() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key".to_string().into_bytes());
        let conn = database::acquire_connection().await.unwrap();

        // Setup workspaces, users, student, and record
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-health-1', 'Health WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-health-nurse', 'ws-health-1', 'nurse@health.io', 'nurse')", ()).await.unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-health-1', 'ws-health-1', NULL, 'Alice', 'Green', 'Grade 1', 0)",
            (),
        ).await.unwrap();

        // Administer health record
        let _record = save_health_record(
            "u-health-nurse".to_string(),
            None,
            "ws-health-1".to_string(),
            "student-health-1".to_string(),
            "MMR".to_string(),
            "Completed".to_string(),
            Some("2026-07-05".to_string()),
        ).await.unwrap();

        // Retrieve health records as nurse
        let list = get_health_records("u-health-nurse".to_string(), "student-health-1".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].vaccine_name, "MMR");

        // Clean up
        conn.execute("DELETE FROM health_records WHERE student_id = 'student-health-1'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE id = 'student-health-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-health-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-health-1'", ()).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_get_health_records_unauthorized() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key".to_string().into_bytes());
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-health-2', 'Health WS 2', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-health-teacher', 'ws-health-2', 'teacher@health.io', 'teacher')", ()).await.unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-health-2', 'ws-health-2', NULL, 'Bob', 'Brown', 'Grade 2', 0)",
            (),
        ).await.unwrap();

        // Retrieve health records as unauthorized teacher
        let res = get_health_records("u-health-teacher".to_string(), "student-health-2".to_string()).await;
        assert!(res.is_err());
        if let Err(YntraError::AuthError(msg)) = res {
            assert!(msg.contains("Access to health records denied"));
        } else {
            panic!("Expected AuthError");
        }

        // Clean up
        conn.execute("DELETE FROM student_profiles WHERE id = 'student-health-2'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-health-2'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-health-2'", ()).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }
}
