use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::school::auth::{verify_school_write_zkp, verify_school_permission};
use crate::SchoolConflict;

pub async fn record_school_conflict(
    conn: &database::DbConnection,
    workspace_id: &str,
    entity_table: &str,
    entity_id: &str,
    conflict_json: serde_json::Value,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conflict_id = format!("{}-{}-{}", entity_table, entity_id, now_ms);
    conn.execute(
        "INSERT INTO school_conflicts (id, workspace_id, entity_table, entity_id, conflict_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        crate::params![
            &conflict_id,
            workspace_id,
            entity_table,
            entity_id,
            conflict_json.to_string(),
            now_ms
        ],
    )
    .await?;
    Ok(())
}

fn choose_version_from_conflict(conflict_json_str: &str, choice: &str) -> Result<serde_json::Value, YntraError> {
    let val: serde_json::Value = serde_json::from_str(conflict_json_str)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    
    let versions = val.get("versions").and_then(|v| v.as_array())
        .ok_or_else(|| YntraError::ValidationError("Invalid conflict JSON format: versions array missing".to_string()))?;
    
    if let Ok(idx) = choice.parse::<usize>() {
        if idx < versions.len() {
            return Ok(versions[idx].clone());
        }
    }
    
    for v in versions {
        if let Some(by) = v.get("by").and_then(|b| b.as_str()) {
            if by == choice {
                return Ok(v.clone());
            }
        }
    }
    
    Err(YntraError::ValidationError(format!("Conflict resolution version choice '{}' not found", choice)))
}

#[uniffi::export]
pub async fn get_school_conflicts(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<SchoolConflict>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let is_privileged = auth.role == "platform_admin"
        || auth.role == "admin"
        || auth.role == "school-admin"
        || auth.role == "role-school-admin"
        || auth.role == "teacher"
        || auth.role == "role-school-teacher"
        || auth.role == "principal"
        || auth.role == "role-school-principal";

    if !is_privileged {
        return Err(YntraError::AuthError("Access denied: only administrators and teachers can access conflicts".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, entity_table, entity_id, conflict_json, updated_at FROM school_conflicts WHERE workspace_id = ?1 ORDER BY updated_at DESC")
        .await?;

    let mut rows = stmt.query(crate::params![&workspace_id]).await?;
    let mut conflicts = Vec::new();
    while let Some(row) = rows.next().await? {
        conflicts.push(SchoolConflict {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            entity_table: row.get(2)?,
            entity_id: row.get(3)?,
            conflict_json: row.get(4)?,
            updated_at: row.get(5)?,
        });
    }

    Ok(conflicts)
}

#[uniffi::export]
pub async fn resolve_school_conflict(
    requester_user_id: String,
    workspace_id: String,
    conflict_id: String,
    resolution_choice: String,
    custom_resolved_json: Option<String>,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;

    let conflict: (String, String, String) = conn.query_row(
        "SELECT entity_table, entity_id, conflict_json FROM school_conflicts WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&conflict_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).await.map_err(|_| YntraError::NotFoundError("Conflict record not found".to_string()))?;

    let entity_table = conflict.0;
    let entity_id = conflict.1;
    let conflict_json_str = conflict.2;

    if auth.role != "platform_admin" && auth.role != "admin" && auth.role != "school-admin" {
        match entity_table.as_str() {
            "student_profiles" => verify_school_permission(&auth, "can_manage_students")?,
            "submissions" | "term_grades" => verify_school_permission(&auth, "can_manage_grades")?,
            "attendance_records" => {
                if !has_school_permission(&auth, "can_manage_grades") && !has_school_permission(&auth, "can_manage_schedule") {
                    return Err(YntraError::AuthError("Access denied: insufficient permissions to resolve attendance conflicts".to_string()));
                }
            }
            "timetable_slots" => verify_school_permission(&auth, "can_manage_schedule")?,
            "health_incidents" => verify_school_permission(&auth, "can_access_health_records")?,
            _ => return Err(YntraError::AuthError("Access denied: insufficient permissions to resolve conflicts".to_string())),
        }
    }

    let resolved_val = if let Some(ref custom_str) = custom_resolved_json {
        if !custom_str.is_empty() {
            serde_json::from_str::<serde_json::Value>(custom_str)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        } else {
            choose_version_from_conflict(&conflict_json_str, &resolution_choice)?
        }
    } else {
        choose_version_from_conflict(&conflict_json_str, &resolution_choice)?
    };

    let now_ms = crate::infra::time::get_current_time_ms();
    match entity_table.as_str() {
        "student_profiles" => {
            let first_name = resolved_val.get("first_name").and_then(|v| v.as_str()).unwrap_or_default();
            let last_name = resolved_val.get("last_name").and_then(|v| v.as_str()).unwrap_or_default();
            let grade_level = resolved_val.get("grade_level").and_then(|v| v.as_str()).unwrap_or_default();
            let parent_contact = resolved_val.get("parent_contact").and_then(|v| v.as_str());

            conn.execute(
                "UPDATE student_profiles SET first_name = ?1, last_name = ?2, grade_level = ?3, parent_contact = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
                crate::params![first_name, last_name, grade_level, parent_contact, now_ms, &entity_id]
            ).await?;
        }
        "submissions" => {
            let grade = resolved_val.get("grade").and_then(|v| v.as_str());
            let feedback = resolved_val.get("feedback").and_then(|v| v.as_str());
            let content = resolved_val.get("content").and_then(|v| v.as_str()).unwrap_or_default();

            conn.execute(
                "UPDATE submissions SET grade = ?1, feedback = ?2, content = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
                crate::params![grade, feedback, content, now_ms, &entity_id]
            ).await?;
        }
        "attendance_records" => {
            let status = resolved_val.get("status").and_then(|v| v.as_str()).unwrap_or_default();
            let notes = resolved_val.get("notes").and_then(|v| v.as_str());

            conn.execute(
                "UPDATE attendance_records SET status = ?1, notes = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
                crate::params![status, notes, now_ms, &entity_id]
            ).await?;
        }
        "term_grades" => {
            let final_grade = resolved_val.get("grade").and_then(|v| v.as_str());
            let final_grade_val = final_grade.or_else(|| resolved_val.get("final_grade").and_then(|v| v.as_str()));

            let final_points = resolved_val.get("points").and_then(|v| v.as_i64())
                .or_else(|| resolved_val.get("final_points").and_then(|v| v.as_i64()));

            let teacher_comments = resolved_val.get("teacher_comments").and_then(|v| v.as_str())
                .or_else(|| resolved_val.get("comments").and_then(|v| v.as_str()));

            conn.execute(
                "UPDATE term_grades SET final_grade = ?1, final_points = ?2, teacher_comments = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
                crate::params![final_grade_val, final_points, teacher_comments, now_ms, &entity_id]
            ).await?;
        }
        "health_incidents" => {
            let visit_reason = resolved_val.get("visit_reason").and_then(|v| v.as_str()).unwrap_or_default();
            let treatment = resolved_val.get("treatment").and_then(|v| v.as_str());
            let checked_in_at = resolved_val.get("checked_in_at").and_then(|v| v.as_str()).unwrap_or_default();
            let checked_out_at = resolved_val.get("checked_out_at").and_then(|v| v.as_str());
            let notes = resolved_val.get("notes").and_then(|v| v.as_str());

            conn.execute(
                "UPDATE health_incidents SET visit_reason = ?1, treatment = ?2, checked_in_at = ?3, checked_out_at = ?4, notes = ?5, updated_at = ?6, sync_status = 'pending' WHERE id = ?7",
                crate::params![visit_reason, treatment, checked_in_at, checked_out_at, notes, now_ms, &entity_id]
            ).await?;
        }
        "timetable_slots" => {
            let course_id = resolved_val.get("course_id").and_then(|v| v.as_str()).unwrap_or_default();
            let day_of_week = resolved_val.get("day_of_week").and_then(|v| v.as_i64()).unwrap_or_default();
            let start_time = resolved_val.get("start_time").and_then(|v| v.as_str()).unwrap_or_default();
            let end_time = resolved_val.get("end_time").and_then(|v| v.as_str()).unwrap_or_default();
            let classroom = resolved_val.get("classroom").and_then(|v| v.as_str());

            conn.execute(
                "UPDATE timetable_slots SET course_id = ?1, day_of_week = ?2, start_time = ?3, end_time = ?4, classroom = ?5, updated_at = ?6, sync_status = 'pending' WHERE id = ?7",
                crate::params![course_id, day_of_week, start_time, end_time, classroom, now_ms, &entity_id]
            ).await?;
        }
        _ => {
            return Err(YntraError::ValidationError(format!("Unsupported conflict entity table: {}", entity_table)));
        }
    }

    conn.execute(
        "DELETE FROM school_conflicts WHERE id = ?1",
        crate::params![&conflict_id]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_school_conflict(
    requester_user_id: String,
    workspace_id: String,
    conflict_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;

    let entity_table: String = conn.query_row(
        "SELECT entity_table FROM school_conflicts WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&conflict_id, &workspace_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Conflict record not found".to_string()))?;

    if auth.role != "platform_admin" && auth.role != "admin" && auth.role != "school-admin" {
        match entity_table.as_str() {
            "student_profiles" => verify_school_permission(&auth, "can_manage_students")?,
            "submissions" | "term_grades" => verify_school_permission(&auth, "can_manage_grades")?,
            "attendance_records" => {
                if !has_school_permission(&auth, "can_manage_grades") && !has_school_permission(&auth, "can_manage_schedule") {
                    return Err(YntraError::AuthError("Access denied: insufficient permissions to delete attendance conflicts".to_string()));
                }
            }
            "timetable_slots" => verify_school_permission(&auth, "can_manage_schedule")?,
            "health_incidents" => verify_school_permission(&auth, "can_access_health_records")?,
            _ => return Err(YntraError::AuthError("Access denied: insufficient permissions to delete conflicts".to_string())),
        }
    }

    conn.execute(
        "DELETE FROM school_conflicts WHERE id = ?1",
        crate::params![&conflict_id]
    ).await?;

    notify_observers();
    Ok(())
}

fn has_school_permission(auth: &crate::AuthContext, permission_name: &str) -> bool {
    crate::services::school::auth::has_school_permission(auth, permission_name)
}
