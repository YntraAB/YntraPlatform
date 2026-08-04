use crate::AttendanceRecord;
use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::notes::verify_zkp_if_encrypted;
use crate::services::school::auth::{
    verify_school_permission, verify_school_write_zkp, verify_student_access,
};
use crate::services::school::conflicts::record_school_conflict;

#[uniffi::export]
pub async fn get_attendance_records(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
    date: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at FROM attendance_records WHERE workspace_id = ?1 AND course_id = ?2 AND date = ?3")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, course_id, date], |row| {
            Ok(AttendanceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                date: row.get(4)?,
                status: row.get(5)?,
                notes: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_attendance_record(
    requester_user_id: String,
    record: AttendanceRecord,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != record.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_schedule")?;

    let team_id = "";
    if let Some(ref notes) = record.notes {
        verify_zkp_if_encrypted(
            &conn,
            notes,
            &auth.user_id,
            &auth.role,
            &record.workspace_id,
            team_id,
        )
        .await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT status, notes, updated_at FROM attendance_records WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&record.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?))
        } else {
            None
        }
    };

    let mut status = record.status.clone();
    let mut notes = record.notes.clone();

    let incoming_updated_at = if record.updated_at > now_ms + 5000 {
        now_ms
    } else {
        record.updated_at
    };
    if let Some((old_status, old_notes, old_updated_at)) = existing {
        if old_updated_at > incoming_updated_at {
            let status_diff = old_status != record.status;
            let notes_diff = old_notes != record.notes;

            if status_diff || notes_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "status": old_status.clone(),
                            "notes": old_notes.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "status": record.status.clone(),
                            "notes": record.notes.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });

                record_school_conflict(
                    &conn,
                    &record.workspace_id,
                    "attendance_records",
                    &record.id,
                    mvr,
                )
                .await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > record.updated_at)
                status = old_status;
                notes = old_notes;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &record.id,
            &record.workspace_id,
            &record.student_id,
            &record.course_id,
            &record.date,
            &status,
            &notes,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn report_student_absence(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    date: String,
    reason: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_student_access(&conn, &auth, &student_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();
    let record_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT OR REPLACE INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &record_id,
            &workspace_id,
            &student_id,
            "",
            &date,
            "absent",
            &format!("Parent Reported: {}", reason),
            &now_ms
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_student_attendance_records(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at FROM attendance_records WHERE workspace_id = ?1 AND student_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, student_id], |row| {
            Ok(AttendanceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                date: row.get(4)?,
                status: row.get(5)?,
                notes: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}
