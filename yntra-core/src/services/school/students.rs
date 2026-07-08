use crate::database;
use crate::observer::notify_observers;
use crate::{StudentProfile, YntraError};

#[uniffi::export]
pub async fn get_students(requester_user_id: String) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let has_access = super::check_permission_for_auth(&auth, "can_view_directory")
        || super::check_permission_for_auth(&auth, "can_manage_grades");
    if !has_access {
        return Err(YntraError::AuthError("Access denied: you do not have permission to view the students directory".to_string()));
    }

    let requester_ws = auth.workspace_id.clone();

    let mut stmt = conn.prepare("SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status FROM student_profiles WHERE workspace_id = ?1").await?;
    let list = stmt.query_map(crate::params![requester_ws], |row| {
        let ws_id: String = row.get(1)?;
        Ok(StudentProfile {
            id: row.get(0)?,
            workspace_id: ws_id,
            user_id: row.get(2)?,
            first_name: row.get(3)?,
            last_name: row.get(4)?,
            grade_level: row.get(5)?,
            parent_contact: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn add_student(
    requester_user_id: String,
    workspace_id: String,
    user_id: Option<String>,
    first_name: String,
    last_name: String,
    grade_level: String,
    parent_contact: Option<String>,
) -> Result<StudentProfile, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required to add student profiles".to_string()));
    }

    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let student = StudentProfile {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        user_id: user_id.clone(),
        first_name: first_name.clone(),
        last_name: last_name.clone(),
        grade_level: grade_level.clone(),
        parent_contact: parent_contact.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![&id, &workspace_id, &user_id, &first_name, &last_name, &grade_level, &parent_contact, &now_ms],
    ).await?;
    notify_observers();

    Ok(student)
}

#[uniffi::export]
pub async fn associate_parent_student(
    requester_user_id: String,
    student_id: String,
    parent_user_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required to associate parent profiles".to_string()));
    }

    let student_ws: String = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![&student_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Student not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != student_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch for student".to_string()));
    }

    let parent_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&parent_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Parent user not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != parent_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch for parent".to_string()));
    }

    conn.execute(
        "INSERT OR IGNORE INTO student_parents (student_id, parent_user_id) VALUES (?1, ?2)",
        crate::params![&student_id, &parent_user_id],
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_parent_students(
    requester_user_id: String,
    parent_user_id: String,
) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let is_self = requester_user_id == parent_user_id;
    if !is_self && !auth.is_admin {
        return Err(YntraError::AuthError("Access denied to parent profiles".to_string()));
    }

    if !is_self && auth.role != "platform_admin" {
        let parent_ws: String = conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&parent_user_id],
            |r| r.get(0)
        ).await.map_err(|_| YntraError::NotFoundError("Parent user not found".to_string()))?;

        if auth.workspace_id != parent_ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
    }

    let mut stmt = conn.prepare(
        "SELECT s.id, s.workspace_id, s.user_id, s.first_name, s.last_name, s.grade_level, s.parent_contact, s.updated_at, s.sync_status
         FROM student_profiles s
         JOIN student_parents sp ON s.id = sp.student_id
         WHERE sp.parent_user_id = ?1"
    ).await?;
    
    let list = stmt.query_map(crate::params![&parent_user_id], |row| {
        let ws_id: String = row.get(1)?;
        Ok(StudentProfile {
            id: row.get(0)?,
            workspace_id: ws_id,
            user_id: row.get(2)?,
            first_name: row.get(3)?,
            last_name: row.get(4)?,
            grade_level: row.get(5)?,
            parent_contact: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_students_directory_permission() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        
        // Insert platform_admin user (should succeed)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-stud-admin', 'workspace-1', 'admin@yntra.io', 'platform_admin')", ()).await.unwrap();
        // Insert student parent user without directory view permission (should fail)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-stud-parent', 'workspace-1', 'parent@yntra.io', 'student_parent')", ()).await.unwrap();
        
        // Verify platform_admin can access the students list
        let res1 = get_students("test-stud-admin".to_string()).await;
        assert!(res1.is_ok());

        // Verify student_parent gets AuthError
        let res2 = get_students("test-stud-parent".to_string()).await;
        assert!(res2.is_err());
        assert!(matches!(res2.unwrap_err(), YntraError::AuthError(_)));

        // Clean up
        conn.execute("DELETE FROM users WHERE id IN ('test-stud-admin', 'test-stud-parent')", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_parent_students_authorization() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspaces and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-1', 'GPS WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-2', 'GPS WS 2', '[]', '{}')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-parent', 'ws-gps-1', 'parent@gps.io', 'parent')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-admin', 'ws-gps-1', 'admin@gps.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-other-admin', 'ws-gps-2', 'admin2@gps.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-stranger', 'ws-gps-1', 'stranger@gps.io', 'parent')", ()).await.unwrap();

        // 2. Parent query self (Should Succeed)
        let res_self = get_parent_students("u-gps-parent".to_string(), "u-gps-parent".to_string()).await;
        assert!(res_self.is_ok());

        // 3. Admin query parent in same workspace (Should Succeed)
        let res_admin = get_parent_students("u-gps-admin".to_string(), "u-gps-parent".to_string()).await;
        assert!(res_admin.is_ok());

        // 4. Admin query parent in different workspace (Should Fail)
        let res_other_admin = get_parent_students("u-gps-other-admin".to_string(), "u-gps-parent".to_string()).await;
        assert!(res_other_admin.is_err());
        assert!(matches!(res_other_admin.unwrap_err(), YntraError::AuthError(_)));

        // 5. Stranger query parent (Should Fail)
        let res_stranger = get_parent_students("u-gps-stranger".to_string(), "u-gps-parent".to_string()).await;
        assert!(res_stranger.is_err());
        assert!(matches!(res_stranger.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-gps-1', 'ws-gps-2')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-gps-1', 'ws-gps-2')", ()).await.unwrap();
    }
}
