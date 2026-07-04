#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{AttendanceRecord, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_attendance(
    requester_user_id: String,
    course_id: String,
    date: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? {
            return Err(YntraError::AuthError("Access denied: cannot view course attendance".to_string()));
        }
        let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status FROM attendance_records WHERE course_id = ?1 AND date = ?2").await?;
        let list = stmt.query_map(crate::params![&course_id, &date], |row| {
            Ok(AttendanceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                date: row.get(4)?,
                status: row.get(5)?,
                notes: row.get(6)?,
                updated_at: row.get(7)?,
                sync_status: row.get(8)?,
            })
        }).await?;
        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("teacher") || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
        Ok(store.attendance_records.iter().filter(|r| r.course_id == course_id && r.date == date).cloned().collect())
    }
}

#[uniffi::export]
pub async fn save_attendance_record(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    course_id: String,
    date: String,
    status: String,
    notes: Option<String>,
) -> Result<AttendanceRecord, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? {
            return Err(YntraError::AuthError("Access denied: cannot modify attendance".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("teacher") || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        
        // Find if record already exists
        let existing_id: Option<String> = conn.query_row(
            "SELECT id FROM attendance_records WHERE student_id = ?1 AND course_id = ?2 AND date = ?3",
            crate::params![&student_id, &course_id, &date],
            |row| row.get(0)
        ).await.ok();

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE attendance_records SET status = ?1, notes = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
                crate::params![&status, &notes, &now_ms, &id],
            ).await?;
            
            let record = AttendanceRecord {
                id,
                workspace_id,
                student_id,
                course_id,
                date,
                status,
                notes,
                updated_at: now_ms,
                sync_status: "pending".to_string(),
            };
            notify_observers();
            Ok(record)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
                crate::params![&id, &workspace_id, &student_id, &course_id, &date, &status, &notes, &now_ms],
            ).await?;
            
            let record = AttendanceRecord {
                id,
                workspace_id,
                student_id,
                course_id,
                date,
                status,
                notes,
                updated_at: now_ms,
                sync_status: "pending".to_string(),
            };
            notify_observers();
            Ok(record)
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        let existing = store.attendance_records.iter_mut().find(|r| r.student_id == student_id && r.course_id == course_id && r.date == date);
        
        let record = if let Some(r) = existing {
            r.status = status;
            r.notes = notes;
            r.updated_at = now_ms;
            r.sync_status = "pending".to_string();
            r.clone()
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            let new_rec = AttendanceRecord {
                id,
                workspace_id,
                student_id,
                course_id,
                date,
                status,
                notes,
                updated_at: now_ms,
                sync_status: "pending".to_string(),
            };
            store.attendance_records.push(new_rec.clone());
            new_rec
        };
        notify_observers();
        Ok(record)
    }
}

#[uniffi::export]
pub async fn get_student_attendance(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
            return Err(YntraError::AuthError("Access denied to student attendance records".to_string()));
        }
        let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status FROM attendance_records WHERE student_id = ?1").await?;
        let list = stmt.query_map(crate::params![&student_id], |row| {
            Ok(AttendanceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                date: row.get(4)?,
                status: row.get(5)?,
                notes: row.get(6)?,
                updated_at: row.get(7)?,
                sync_status: row.get(8)?,
            })
        }).await?;
        Ok(list)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        if !super::has_academic_access_wasm(&store, &requester_user_id, &student_id) {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
        Ok(store.attendance_records.iter().filter(|r| r.student_id == student_id).cloned().collect())
    }
}
