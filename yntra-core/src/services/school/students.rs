#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{StudentProfile, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_students(requester_user_id: String) -> Result<Vec<StudentProfile>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let requester_ws: String = conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| r.get(0)
        ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let ws_id = requester.workspace_id.clone().unwrap_or_default();
        Ok(store.student_profiles.iter().filter(|s| s.workspace_id == ws_id).cloned().collect())
    }
}

#[uniffi::export]
pub async fn add_student(
    workspace_id: String,
    user_id: Option<String>,
    first_name: String,
    last_name: String,
    grade_level: String,
    parent_contact: Option<String>,
) -> Result<StudentProfile, YntraError> {
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        conn.execute(
            "INSERT INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
            crate::params![&id, &workspace_id, &user_id, &first_name, &last_name, &grade_level, &parent_contact, &now_ms],
        ).await?;
        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.student_profiles.push(student.clone());
        notify_observers();
    }

    Ok(student)
}

#[uniffi::export]
pub async fn associate_parent_student(
    student_id: String,
    parent_user_id: String,
) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        conn.execute(
            "INSERT OR IGNORE INTO student_parents (student_id, parent_user_id) VALUES (?1, ?2)",
            crate::params![&student_id, &parent_user_id],
        ).await?;
        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = student_id;
        let _ = parent_user_id;
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn get_parent_students(parent_user_id: String) -> Result<Vec<StudentProfile>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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

    #[cfg(target_arch = "wasm32")]
    {
        let _ = parent_user_id;
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.student_profiles.clone())
    }
}
