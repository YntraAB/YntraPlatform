use crate::database;
use crate::observer::notify_observers;
use crate::{AttendanceRecord, YntraError};

#[uniffi::export]
pub async fn get_attendance(
    requester_user_id: String,
    course_id: String,
    date: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
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

    let course_ws: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&course_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if course_ws != workspace_id {
        return Err(YntraError::ValidationError("Course does not belong to the specified workspace".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? {
        return Err(YntraError::AuthError("Access denied: cannot modify attendance".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();

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

#[uniffi::export]
pub async fn get_student_attendance(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_save_attendance_record_insert_and_update() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("DELETE FROM attendance_records WHERE workspace_id = 'ws-att-1'", ()).await;
        let _ = conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-att-1'", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-att-1'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-1'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-1'", ()).await;

        // 1. Setup workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-att-1', 'Att WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-att-admin', 'ws-att-1', 'admin@att.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at) VALUES ('course-1', 'ws-att-1', 'Math 101', 'Math', 'u-att-admin', 'Room 1', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-att-1', 'ws-att-1', NULL, 'Jane', 'Doe', 'Grade 1', 0)", ()).await.unwrap();

        // 2. Save a new record
        let record = save_attendance_record(
            "u-att-admin".to_string(),
            "ws-att-1".to_string(),
            "student-att-1".to_string(),
            "course-1".to_string(),
            "2026-07-05".to_string(),
            "Present".to_string(),
            Some("On time".to_string()),
        ).await.unwrap();

        assert_eq!(record.status, "Present");
        assert_eq!(record.notes, Some("On time".to_string()));
        assert_eq!(record.sync_status, "pending");

        // Verify it exists in DB
        let status_in_db: String = conn.query_row(
            "SELECT status FROM attendance_records WHERE id = ?1",
            crate::params![&record.id],
            |r| r.get(0)
        ).await.unwrap();
        assert_eq!(status_in_db, "Present");

        // 3. Update the record
        let updated_record = save_attendance_record(
            "u-att-admin".to_string(),
            "ws-att-1".to_string(),
            "student-att-1".to_string(),
            "course-1".to_string(),
            "2026-07-05".to_string(),
            "Absent".to_string(),
            Some("Sick leave".to_string()),
        ).await.unwrap();

        assert_eq!(updated_record.id, record.id);
        assert_eq!(updated_record.status, "Absent");
        assert_eq!(updated_record.notes, Some("Sick leave".to_string()));

        // Verify status in DB updated
        let status_in_db_updated: String = conn.query_row(
            "SELECT status FROM attendance_records WHERE id = ?1",
            crate::params![&record.id],
            |r| r.get(0)
        ).await.unwrap();
        assert_eq!(status_in_db_updated, "Absent");

        // Cleanup
        conn.execute("DELETE FROM attendance_records WHERE workspace_id = 'ws-att-1'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-att-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-1'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_attendance_permissions() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-2'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-2'", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-att-2', 'Att WS 2', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-att-teacher-2', 'ws-att-2', 'teacher@att.io', 'teacher')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-att-admin-2', 'ws-att-2', 'admin@att.io', 'admin')", ()).await.unwrap();

        // Admin has permission
        let res_admin = get_attendance(
            "u-att-admin-2".to_string(),
            "course-1".to_string(),
            "2026-07-05".to_string(),
        ).await;
        assert!(res_admin.is_ok());

        // Teacher doesn't have permission by default
        let res_teacher = get_attendance(
            "u-att-teacher-2".to_string(),
            "course-1".to_string(),
            "2026-07-05".to_string(),
        ).await;
        assert!(res_teacher.is_err());
        assert!(matches!(res_teacher.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-2'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-2'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_student_attendance_academic_access() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-att-3'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-3'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-3'", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-att-3', 'Att WS 3', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-att-student-3', 'ws-att-3', 'stud@att.io', 'student')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-att-stranger-3', 'ws-att-3', 'stranger@att.io', 'student')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-att-3', 'ws-att-3', 'u-att-student-3', 'Jane', 'Doe', 'Grade 3', 0)", ()).await.unwrap();

        // Student requesting own attendance should be fine (even if empty)
        let res_self = get_student_attendance("u-att-student-3".to_string(), "student-att-3".to_string()).await;
        assert!(res_self.is_ok());

        // Stranger requesting should fail
        let res_stranger = get_student_attendance("u-att-stranger-3".to_string(), "student-att-3".to_string()).await;
        assert!(res_stranger.is_err());
        assert!(matches!(res_stranger.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-att-3'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-att-3'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-att-3'", ()).await.unwrap();
    }
}

