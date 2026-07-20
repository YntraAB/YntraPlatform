use crate::database;
use crate::infra::observer::notify_observers;
use crate::services::notes::verify_zkp_if_encrypted;
use crate::{
    StudentProfile, Assignment, Submission, AttendanceRecord, TermGrade, ReportCard,
    LibraryBook, SchoolInvoice, LibraryLendingLogInfo, HealthRecord, HealthIncident,
    WorkspaceUser, YntraError, SchoolConflict
};
use uuid::Uuid;

async fn verify_school_write_zkp(
    conn: &database::DbConnection,
    requester_user_id: &str,
    role: &str,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let is_dev_bypass = !crate::infra::auth::is_production() && requester_user_id.starts_with("test-");

    if !is_dev_bypass {
        let (u_role, workspace_id, role_signature, creator_public_key): (String, String, Option<String>, Option<String>) = match conn
            .query_row(
                "SELECT u.role, u.workspace_id, u.role_signature, w.creator_public_key \
                 FROM users u \
                 JOIN workspaces w ON u.workspace_id = w.id \
                 WHERE u.id = ?1",
                crate::params![requester_user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .await
        {
            Ok(val) => val,
            Err(_) => return Err(YntraError::AuthError("User or workspace not found".to_string())),
        };

        let sig = match role_signature {
            Some(s) => s,
            None => {
                if !crate::infra::auth::is_production() {
                    return Ok(());
                } else {
                    return Err(YntraError::AuthError("Missing role signature: offline database tampering suspected".to_string()));
                }
            }
        };
        let pk = match creator_public_key {
            Some(p) => p,
            None => {
                if !crate::infra::auth::is_production() {
                    return Ok(());
                } else {
                    return Err(YntraError::AuthError("Workspace public key not found".to_string()));
                }
            }
        };

        if !crate::infra::crypto::verify_role_signature(&pk, requester_user_id, &u_role, &workspace_id, &sig) {
            return Err(YntraError::CryptoError(
                "Role signature verification failed: offline database tampering detected".to_string(),
            ));
        }
    }

    let is_proof_required = if crate::infra::auth::is_production() {
        true
    } else {
        role_proof.is_some()
    };

    if is_proof_required {
        let proof = role_proof.ok_or_else(|| {
            YntraError::AuthError("Zero-Knowledge Role Proof is required for write operations".to_string())
        })?;

        let metadata_str: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![requester_user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        let public_key_hex = if let Some(ref meta) = metadata_str {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                val.get("public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        val.get("siths_public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if public_key_hex.is_empty() {
            return Err(YntraError::AuthError(
                "Cryptographic role verification failed: User public key not found".to_string(),
            ));
        }

        let trust = crate::ZkCryptoTrust::new();
        if !trust.verify_proof(
            proof,
            requester_user_id.to_string(),
            role.to_string(),
            public_key_hex,
        ) {
            return Err(YntraError::CryptoError(
                "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
            ));
        }
    }

    Ok(())
}

#[uniffi::export]
pub async fn save_blob(
    requester_user_id: String,
    sha256: String,
    workspace_id: String,
    base64_data: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO local_blobs (sha256, workspace_id, data, created_at) VALUES (?1, ?2, ?3, ?4)",
        crate::params![sha256, workspace_id, base64_data, now_ms],
    )
    .await?;

    Ok(())
}

#[uniffi::export]
pub async fn get_blob(
    requester_user_id: String,
    sha256: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (data, ws_id): (String, String) = conn
        .query_row(
            "SELECT data, workspace_id FROM local_blobs WHERE sha256 = ?1",
            crate::params![sha256],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Blob not found: {}", sha256)))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    Ok(data)
}

#[uniffi::export]
pub async fn check_school_permission(
    requester_user_id: String,
    permission_name: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    Ok(has_school_permission(&auth, &permission_name))
}

fn has_school_permission(auth: &crate::AuthContext, permission_name: &str) -> bool {
    if auth.role == "platform_admin" || auth.role == "admin" || auth.role == "school-admin" || auth.role == "role-school-admin" || auth.role == "principal" || auth.role == "role-school-principal" {
        return true;
    }
    if let Some(ref settings_str) = auth.workspace_settings {
        if let Ok(settings_val) = serde_json::from_str::<serde_json::Value>(settings_str) {
            if let Some(roles_arr) = settings_val.get("roles").and_then(|r| r.as_array()) {
                for r in roles_arr {
                    let r_id = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let r_name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let is_match = r_id == auth.role
                        || r_id.ends_with(&format!("-{}", auth.role))
                        || r_id.strip_prefix("role-").map(|s| s == auth.role).unwrap_or(false)
                        || r_id.strip_prefix("role-school-").map(|s| s == auth.role).unwrap_or(false)
                        || r_name.to_lowercase() == auth.role.to_lowercase();
                    if is_match {
                        if let Some(permissions) = r.get("permissions") {
                            if let Some(val) = permissions.get(permission_name).and_then(|v| v.as_bool()) {
                                return val;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn verify_school_permission(auth: &crate::AuthContext, permission_name: &str) -> Result<(), YntraError> {
    if has_school_permission(auth, permission_name) {
        Ok(())
    } else {
        Err(YntraError::AuthError(format!(
            "Access denied: role '{}' does not have permission '{}'",
            auth.role, permission_name
        )))
    }
}

async fn verify_student_access(
    conn: &database::DbConnection,
    auth: &crate::AuthContext,
    student_id: &str,
) -> Result<(), YntraError> {
    if auth.role == "platform_admin" {
        return Ok(());
    }

    let role_lower = auth.role.to_lowercase();
    if role_lower == "student" || role_lower == "role-school-student" {
        let profile_user_id: Option<String> = conn
            .query_row(
                "SELECT user_id FROM student_profiles WHERE id = ?1 AND workspace_id = ?2",
                crate::params![student_id, &auth.workspace_id],
                |r| r.get(0),
            )
            .await
            .map_err(|_| YntraError::NotFoundError(format!("Student profile not found: {}", student_id)))?;

        if let Some(uid) = profile_user_id {
            if uid == auth.user_id {
                return Ok(());
            }
        }
        return Err(YntraError::AuthError("Access denied: You can only view your own student records".to_string()));
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        let linked: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2 AND workspace_id = ?3",
                crate::params![student_id, &auth.user_id, &auth.workspace_id],
                |r| r.get(0),
            )
            .await
            .ok();

        if linked.is_some() {
            return Ok(());
        }
        return Err(YntraError::AuthError("Access denied: You are not linked to this student".to_string()));
    }

    Ok(())
}

#[uniffi::export]
pub async fn get_student_profiles(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let role_lower = auth.role.to_lowercase();
    let query_str = if role_lower == "student" || role_lower == "role-school-student" {
        "SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1 AND user_id = ?2"
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        "SELECT s.id, s.workspace_id, s.user_id, s.first_name, s.last_name, s.grade_level, s.parent_contact, s.updated_at FROM student_profiles s JOIN student_parents sp ON s.id = sp.student_id WHERE s.workspace_id = ?1 AND sp.parent_user_id = ?2"
    } else {
        "SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1"
    };

    let mut stmt = conn.prepare(query_str).await?;

    let list = if role_lower == "student" || role_lower == "role-school-student" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        }).await?
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        }).await?
    } else {
        stmt.query_map(crate::params![workspace_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        }).await?
    };

    Ok(list)
}

async fn record_school_conflict(
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

#[uniffi::export]
pub async fn save_student_profile(
    requester_user_id: String,
    profile: StudentProfile,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != profile.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_students")?;

    let team_id = "";
    verify_zkp_if_encrypted(&conn, &profile.first_name, &auth.user_id, &auth.role, &profile.workspace_id, team_id).await?;
    verify_zkp_if_encrypted(&conn, &profile.last_name, &auth.user_id, &auth.role, &profile.workspace_id, team_id).await?;
    verify_zkp_if_encrypted(&conn, &profile.grade_level, &auth.user_id, &auth.role, &profile.workspace_id, team_id).await?;
    if let Some(ref contact) = profile.parent_contact {
        verify_zkp_if_encrypted(&conn, contact, &auth.user_id, &auth.role, &profile.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, String, String, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&profile.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        } else {
            None
        }
    };

    let mut first_name = profile.first_name.clone();
    let mut last_name = profile.last_name.clone();
    let mut grade_level = profile.grade_level.clone();
    let mut parent_contact = profile.parent_contact.clone();

    let incoming_updated_at = if profile.updated_at > now_ms + 5000 { now_ms } else { profile.updated_at };
    if let Some((old_first_name, old_last_name, old_grade, old_parent_contact, old_updated_at)) = existing {
        if old_updated_at > incoming_updated_at {
            let first_diff = old_first_name != profile.first_name;
            let last_diff = old_last_name != profile.last_name;
            let grade_diff = old_grade != profile.grade_level;
            let parent_diff = old_parent_contact != profile.parent_contact;

            if first_diff || last_diff || grade_diff || parent_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "first_name": old_first_name.clone(),
                            "last_name": old_last_name.clone(),
                            "grade_level": old_grade.clone(),
                            "parent_contact": old_parent_contact.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "first_name": profile.first_name.clone(),
                            "last_name": profile.last_name.clone(),
                            "grade_level": profile.grade_level.clone(),
                            "parent_contact": profile.parent_contact.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &profile.workspace_id, "student_profiles", &profile.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > profile.updated_at)
                first_name = old_first_name;
                last_name = old_last_name;
                grade_level = old_grade;
                parent_contact = old_parent_contact;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &profile.id,
            &profile.workspace_id,
            &profile.user_id,
            &first_name,
            &last_name,
            &grade_level,
            &parent_contact,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_student_profile(
    requester_user_id: String,
    id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    let ws_id: String = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute("DELETE FROM student_profiles WHERE id = ?1", crate::params![&id]).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn save_course(
    requester_user_id: String,
    workspace_id: String,
    course: crate::Course,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_schedule")?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
        crate::params![
            &course.id,
            &workspace_id,
            &course.name,
            &course.subject,
            &course.teacher_id,
            &course.classroom,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_course(
    requester_user_id: String,
    id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    let ws_id: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute("DELETE FROM courses WHERE id = ?1", crate::params![&id]).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_assignments(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
) -> Result<Vec<Assignment>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, course_id, title, description, due_date, max_points, updated_at FROM assignments WHERE workspace_id = ?1 AND course_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, course_id], |row| {
            Ok(Assignment {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                course_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                due_date: row.get(5)?,
                max_points: row.get::<i64>(6)? as i32,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_assignment(
    requester_user_id: String,
    assignment: Assignment,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != assignment.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if assignment.max_points >= 0 {
        verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
        verify_school_permission(&auth, "can_manage_grades")?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &assignment.id,
            &assignment.workspace_id,
            &assignment.course_id,
            &assignment.title,
            &assignment.description,
            &assignment.due_date,
            &(assignment.max_points as i64),
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_assignment(
    requester_user_id: String,
    id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    let ws_id: String = conn.query_row(
        "SELECT workspace_id FROM assignments WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await?;

    conn.execute(
        "DELETE FROM assignments WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&id, &ws_id]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn save_submission(
    requester_user_id: String,
    submission: Submission,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != submission.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    if auth.role != "platform_admin" && auth.role != "admin" && auth.role != "school-admin" {
        let is_student = auth.role == "student" || auth.role == "role-school-student" || has_school_permission(&auth, "can_manage_grades");
        if !is_student {
            return Err(YntraError::AuthError("Access denied: insufficient permissions to manage submissions".to_string()));
        }
    }

    let team_id = "";
    verify_zkp_if_encrypted(&conn, &submission.content, &auth.user_id, &auth.role, &submission.workspace_id, team_id).await?;
    if let Some(ref grade) = submission.grade {
        verify_zkp_if_encrypted(&conn, grade, &auth.user_id, &auth.role, &submission.workspace_id, team_id).await?;
    }
    if let Some(ref feedback) = submission.feedback {
        verify_zkp_if_encrypted(&conn, feedback, &auth.user_id, &auth.role, &submission.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications and preserve encrypted content if modified by a grader
    let existing: Option<(String, Option<String>, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT content, grade, feedback, updated_at FROM submissions WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&submission.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        } else {
            None
        }
    };

    let mut content = submission.content.clone();
    let mut grade = submission.grade.clone();
    let mut feedback = submission.feedback.clone();

    let incoming_updated_at = if submission.updated_at > now_ms + 5000 { now_ms } else { submission.updated_at };
    if let Some((old_content, old_grade, old_feedback, old_updated_at)) = existing {
        if !content.starts_with("zero_copy_enc:") && old_content.starts_with("zero_copy_enc:") {
            content = old_content;
        }

        // If the DB version is newer than the incoming base version timestamp
        if old_updated_at > incoming_updated_at {
            let grade_diff = old_grade != submission.grade;
            let feedback_diff = old_feedback != submission.feedback;

            if grade_diff || feedback_diff {
                // Conflict! Store concurrent values inside a Multi-Value Register JSON structure
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "grade": old_grade.clone(),
                            "feedback": old_feedback.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "grade": submission.grade.clone(),
                            "feedback": submission.feedback.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &submission.workspace_id, "submissions", &submission.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > submission.updated_at)
                grade = old_grade;
                feedback = old_feedback;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &submission.id,
            &submission.workspace_id,
            &submission.assignment_id,
            &submission.student_id,
            &content,
            &grade,
            &feedback,
            &submission.submitted_at,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_schedule")?;

    let team_id = "";
    if let Some(ref notes) = record.notes {
        verify_zkp_if_encrypted(&conn, notes, &auth.user_id, &auth.role, &record.workspace_id, team_id).await?;
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

    let incoming_updated_at = if record.updated_at > now_ms + 5000 { now_ms } else { record.updated_at };
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
                
                record_school_conflict(&conn, &record.workspace_id, "attendance_records", &record.id, mvr).await?;

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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
pub async fn get_term_grades(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<TermGrade>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at FROM term_grades WHERE workspace_id = ?1 AND student_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, student_id], |row| {
            Ok(TermGrade {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                term_name: row.get(4)?,
                final_grade: row.get(5)?,
                final_points: row.get::<Option<i64>>(6)?.map(|i| i as i32),
                teacher_comments: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_course_term_grades(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
) -> Result<Vec<TermGrade>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at FROM term_grades WHERE workspace_id = ?1 AND course_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, course_id], |row| {
            Ok(TermGrade {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                course_id: row.get(3)?,
                term_name: row.get(4)?,
                final_grade: row.get(5)?,
                final_points: row.get::<Option<i64>>(6)?.map(|i| i as i32),
                teacher_comments: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_term_grade(
    requester_user_id: String,
    grade: TermGrade,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != grade.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_grades")?;

    let team_id = "";
    if let Some(ref final_g) = grade.final_grade {
        verify_zkp_if_encrypted(&conn, final_g, &auth.user_id, &auth.role, &grade.workspace_id, team_id).await?;
    }
    if let Some(ref comments) = grade.teacher_comments {
        verify_zkp_if_encrypted(&conn, comments, &auth.user_id, &auth.role, &grade.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(Option<String>, Option<i64>, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT final_grade, final_points, teacher_comments, updated_at FROM term_grades WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&grade.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        } else {
            None
        }
    };

    let mut final_grade = grade.final_grade.clone();
    let mut final_points = grade.final_points.map(|p| p as i64);
    let mut teacher_comments = grade.teacher_comments.clone();

    let incoming_updated_at = if grade.updated_at > now_ms + 5000 { now_ms } else { grade.updated_at };
    if let Some((old_grade, old_points, old_comments, old_updated_at)) = existing {
        // If the DB version is newer than the incoming base version timestamp
        if old_updated_at > incoming_updated_at {
            let grade_diff = old_grade != grade.final_grade;
            let points_diff = old_points != grade.final_points.map(|p| p as i64);
            let comments_diff = old_comments != grade.teacher_comments;

            if grade_diff || points_diff || comments_diff {
                // Conflict! Store concurrent values inside a Multi-Value Register JSON structure
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "grade": old_grade.clone(),
                            "points": old_points,
                            "teacher_comments": old_comments.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "grade": grade.final_grade.clone(),
                            "points": grade.final_points.map(|p| p as i64),
                            "teacher_comments": grade.teacher_comments.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &grade.workspace_id, "term_grades", &grade.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > grade.updated_at)
                final_grade = old_grade;
                final_points = old_points;
                teacher_comments = old_comments;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &grade.id,
            &grade.workspace_id,
            &grade.student_id,
            &grade.course_id,
            &grade.term_name,
            &final_grade,
            &final_points,
            &teacher_comments,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn publish_report_card(
    requester_user_id: String,
    report: ReportCard,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != report.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_publish_report_cards")?;

    let team_id = "";
    if let Some(ref comments) = report.principal_comments {
        verify_zkp_if_encrypted(&conn, comments, &auth.user_id, &auth.role, &report.workspace_id, team_id).await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO report_cards (id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &report.id,
            &report.workspace_id,
            &report.student_id,
            &report.term_name,
            &report.gpa,
            &report.principal_comments,
            &report.status,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_report_cards(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<ReportCard>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at FROM report_cards WHERE workspace_id = ?1 AND student_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, student_id], |row| {
            Ok(ReportCard {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                term_name: row.get(3)?,
                gpa: row.get(4)?,
                principal_comments: row.get(5)?,
                status: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_library_books(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<LibraryBook>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at FROM library_books WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
            Ok(LibraryBook {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                isbn: row.get(4)?,
                copies_available: row.get::<i64>(5)? as i32,
                total_copies: row.get::<i64>(6)? as i32,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_library_book(
    requester_user_id: String,
    book: LibraryBook,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != book.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_library")?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &book.id,
            &book.workspace_id,
            &book.title,
            &book.author,
            &book.isbn,
            &(book.copies_available as i64),
            &(book.total_copies as i64),
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn checkout_book(
    requester_user_id: String,
    workspace_id: String,
    book_id: String,
    student_id: String,
    due_date: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_library")?;

    conn.begin_transaction().await?;

    let book_opt: Option<(i64, i64)> = conn.query_row(
        "SELECT copies_available, total_copies FROM library_books WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&book_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((available, _total)) = book_opt {
        if available <= 0 {
            let _ = conn.rollback().await;
            return Err(YntraError::ValidationError("No copies available for checkout".to_string()));
        }

        let now_ms = crate::infra::time::get_current_time_ms();
        let log_id = Uuid::new_v4().to_string();
        let now_str = crate::infra::time::get_current_datetime_str();

        conn.execute(
            "UPDATE library_books SET copies_available = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&(available - 1), &now_ms, &book_id]
        ).await?;

        conn.execute(
            "INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, 'borrowed', ?7, 'pending')",
            crate::params![
                &log_id,
                &workspace_id,
                &book_id,
                &student_id,
                &now_str,
                &due_date,
                &now_ms
            ]
        ).await?;

        conn.commit().await?;
        notify_observers();
        Ok(())
    } else {
        let _ = conn.rollback().await;
        Err(YntraError::NotFoundError("Book not found".to_string()))
    }
}

#[uniffi::export]
pub async fn return_book(
    requester_user_id: String,
    workspace_id: String,
    lending_log_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_library")?;

    conn.begin_transaction().await?;

    let log_opt: Option<(String, String)> = conn.query_row(
        "SELECT book_id, status FROM library_lending_logs WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&lending_log_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((book_id, status)) = log_opt {
        if status == "returned" {
            let _ = conn.rollback().await;
            return Err(YntraError::ValidationError("Book already returned".to_string()));
        }

        let available: i64 = conn.query_row(
            "SELECT copies_available FROM library_books WHERE id = ?1",
            crate::params![&book_id],
            |r| r.get(0)
        ).await.unwrap_or(0);

        let now_ms = crate::infra::time::get_current_time_ms();
        let now_str = crate::infra::time::get_current_datetime_str();

        conn.execute(
            "UPDATE library_books SET copies_available = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&(available + 1), &now_ms, &book_id]
        ).await?;

        conn.execute(
            "UPDATE library_lending_logs SET returned_at = ?1, status = 'returned', updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&now_str, &now_ms, &lending_log_id]
        ).await?;

        conn.commit().await?;
        notify_observers();
        Ok(())
    } else {
        let _ = conn.rollback().await;
        Err(YntraError::NotFoundError("Lending log not found".to_string()))
    }
}

#[uniffi::export]
pub async fn renew_book(
    requester_user_id: String,
    workspace_id: String,
    log_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;

    let log_opt: Option<(String, String)> = conn.query_row(
        "SELECT student_id, status FROM library_lending_logs WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&log_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((student_id, status)) = log_opt {
        if status != "borrowed" {
            return Err(YntraError::ValidationError("Book is not currently borrowed".to_string()));
        }

        verify_student_access(&conn, &auth, &student_id).await?;

        let now_ms = crate::infra::time::get_current_time_ms();
        let new_due_date = (chrono::Local::now() + chrono::Duration::days(14)).format("%Y-%m-%d").to_string();

        conn.execute(
            "UPDATE library_lending_logs SET due_date = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&new_due_date, &now_ms, &log_id]
        ).await?;

        notify_observers();
        Ok(())
    } else {
        Err(YntraError::NotFoundError("Lending log not found".to_string()))
    }
}

#[uniffi::export]
pub async fn reserve_book(
    requester_user_id: String,
    workspace_id: String,
    book_id: String,
    student_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_student_access(&conn, &auth, &student_id).await?;

    conn.begin_transaction().await?;

    let book_opt: Option<(i64, i64)> = conn.query_row(
        "SELECT copies_available, total_copies FROM library_books WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&book_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((available, _total)) = book_opt {
        if available <= 0 {
            let _ = conn.rollback().await;
            return Err(YntraError::ValidationError("No copies available for reservation".to_string()));
        }

        let now_ms = crate::infra::time::get_current_time_ms();
        let log_id = Uuid::new_v4().to_string();
        let now_str = crate::infra::time::get_current_datetime_str();
        let due_date = (chrono::Local::now() + chrono::Duration::days(14)).format("%Y-%m-%d").to_string();

        conn.execute(
            "UPDATE library_books SET copies_available = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&(available - 1), &now_ms, &book_id]
        ).await?;

        conn.execute(
            "INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, 'reserved', ?7, 'pending')",
            crate::params![
                &log_id,
                &workspace_id,
                &book_id,
                &student_id,
                &now_str,
                &due_date,
                &now_ms
            ]
        ).await?;

        conn.commit().await?;
        notify_observers();
        Ok(())
    } else {
        let _ = conn.rollback().await;
        Err(YntraError::NotFoundError("Book not found".to_string()))
    }
}

#[uniffi::export]
pub async fn get_school_invoices(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let role_lower = auth.role.to_lowercase();
    let query_str = if role_lower == "student" || role_lower == "role-school-student" {
        "SELECT s.id, s.workspace_id, s.student_id, s.title, s.amount, s.due_date, s.status, s.paid_at, s.updated_at FROM school_invoices s JOIN student_profiles p ON s.student_id = p.id WHERE s.workspace_id = ?1 AND p.user_id = ?2"
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        "SELECT s.id, s.workspace_id, s.student_id, s.title, s.amount, s.due_date, s.status, s.paid_at, s.updated_at FROM school_invoices s JOIN student_profiles p ON s.student_id = p.id JOIN student_parents sp ON p.id = sp.student_id WHERE s.workspace_id = ?1 AND sp.parent_user_id = ?2"
    } else {
        "SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at FROM school_invoices WHERE workspace_id = ?1"
    };

    let mut stmt = conn.prepare(query_str).await?;

    let list = if role_lower == "student" || role_lower == "role-school-student" || role_lower == "parent" || role_lower == "role-school-parent" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(SchoolInvoice {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                title: row.get(3)?,
                amount: row.get(4)?,
                due_date: row.get(5)?,
                status: row.get(6)?,
                paid_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).await?
    } else {
        stmt.query_map(crate::params![workspace_id], |row| {
            Ok(SchoolInvoice {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                student_id: row.get(2)?,
                title: row.get(3)?,
                amount: row.get(4)?,
                due_date: row.get(5)?,
                status: row.get(6)?,
                paid_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).await?
    };

    Ok(list)
}

#[uniffi::export]
pub async fn create_school_invoice(
    requester_user_id: String,
    invoice: SchoolInvoice,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != invoice.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_billing")?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO school_invoices (id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &invoice.id,
            &invoice.workspace_id,
            &invoice.student_id,
            &invoice.title,
            &invoice.amount,
            &invoice.due_date,
            &invoice.status,
            &invoice.paid_at,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn record_school_payment(
    requester_user_id: String,
    workspace_id: String,
    invoice_id: String,
    payment_method: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_billing")?;

    conn.begin_transaction().await?;

    let invoice_opt: Option<(f64, String)> = conn.query_row(
        "SELECT amount, status FROM school_invoices WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&invoice_id, &workspace_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    if let Some((amount, status)) = invoice_opt {
        if status == "paid" {
            let _ = conn.rollback().await;
            return Err(YntraError::ValidationError("Invoice is already paid".to_string()));
        }

        let now_ms = crate::infra::time::get_current_time_ms();
        let now_str = crate::infra::time::get_current_datetime_str();
        let payment_id = Uuid::new_v4().to_string();

        conn.execute(
            "UPDATE school_invoices SET status = 'paid', paid_at = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&now_str, &now_ms, &invoice_id]
        ).await?;

        conn.execute(
            "INSERT INTO school_payments (id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
            crate::params![
                &payment_id,
                &workspace_id,
                &invoice_id,
                &amount,
                &payment_method,
                &now_str,
                &now_ms
            ]
        ).await?;

        conn.commit().await?;
        notify_observers();
        Ok(())
    } else {
        let _ = conn.rollback().await;
        Err(YntraError::NotFoundError("Invoice not found".to_string()))
    }
}

#[uniffi::export]
pub async fn get_library_lending_logs(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<LibraryLendingLogInfo>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let role_lower = auth.role.to_lowercase();
    let query_str = if role_lower == "student" || role_lower == "role-school-student" {
        "SELECT l.id, b.title, s.first_name || ' ' || s.last_name, l.checked_out_at, l.due_date, l.returned_at, l.status, l.student_id
         FROM library_lending_logs l
         JOIN library_books b ON l.book_id = b.id
         JOIN student_profiles s ON l.student_id = s.id
         WHERE l.workspace_id = ?1 AND s.user_id = ?2"
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        "SELECT l.id, b.title, s.first_name || ' ' || s.last_name, l.checked_out_at, l.due_date, l.returned_at, l.status, l.student_id
         FROM library_lending_logs l
         JOIN library_books b ON l.book_id = b.id
         JOIN student_profiles s ON l.student_id = s.id
         JOIN student_parents sp ON s.id = sp.student_id
         WHERE l.workspace_id = ?1 AND sp.parent_user_id = ?2"
    } else {
        "SELECT l.id, b.title, s.first_name || ' ' || s.last_name, l.checked_out_at, l.due_date, l.returned_at, l.status, l.student_id
         FROM library_lending_logs l
         JOIN library_books b ON l.book_id = b.id
         JOIN student_profiles s ON l.student_id = s.id
         WHERE l.workspace_id = ?1"
    };

    let mut stmt = conn.prepare(query_str).await?;

    let list = if role_lower == "student" || role_lower == "role-school-student" || role_lower == "parent" || role_lower == "role-school-parent" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(LibraryLendingLogInfo {
                id: row.get(0)?,
                book_title: row.get(1)?,
                student_name: row.get(2)?,
                checked_out_at: row.get(3)?,
                due_date: row.get(4)?,
                returned_at: row.get(5)?,
                status: row.get(6)?,
                student_id: row.get(7)?,
            })
        }).await?
    } else {
        stmt.query_map(crate::params![workspace_id], |row| {
            Ok(LibraryLendingLogInfo {
                id: row.get(0)?,
                book_title: row.get(1)?,
                student_name: row.get(2)?,
                checked_out_at: row.get(3)?,
                due_date: row.get(4)?,
                returned_at: row.get(5)?,
                status: row.get(6)?,
                student_id: row.get(7)?,
            })
        }).await?
    };

    Ok(list)
}

#[uniffi::export]
pub async fn link_parent_to_student(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    parent_user_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_students")?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
        crate::params![&student_id, &parent_user_id, &workspace_id, &now_ms]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn link_student_self_service(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let parent_email: String = conn
        .query_row(
            "SELECT email FROM users WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&auth.user_id, &workspace_id],
            |r| r.get(0)
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Parent user not found".to_string()))?;

    let student_parent_contact: Option<String> = conn
        .query_row(
            "SELECT parent_contact FROM student_profiles WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&student_id, &workspace_id],
            |r| r.get(0)
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if let Some(contact) = student_parent_contact {
        if contact.trim().to_lowercase() == parent_email.trim().to_lowercase() {
            let now_ms = crate::infra::time::get_current_time_ms();
            conn.execute(
                "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
                crate::params![&student_id, &auth.user_id, &workspace_id, &now_ms]
            ).await?;

            notify_observers();
            return Ok(());
        }
    }

    Err(YntraError::AuthError("Verification failed: parent contact info does not match student profile".to_string()))
}

#[uniffi::export]
pub async fn get_student_parents(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<WorkspaceUser>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("
            SELECT u.id, u.workspace_id, u.email, u.full_name, u.phone, u.role, u.preferences, u.metadata, u.updated_at, u.sync_status
            FROM users u
            JOIN student_parents sp ON u.id = sp.parent_user_id
            WHERE sp.student_id = ?1 AND sp.workspace_id = ?2
        ")
        .await?;

    let list = stmt
        .query_map(crate::params![student_id, workspace_id], |row| {
            let id: String = row.get(0)?;
            let metadata_str: Option<String> = row.get(7)?;
            
            let mut siths_card_id = None;
            let mut nfc_badge_uid = None;
            let mut personal_number = None;
            let mut public_key = None;

            if let Some(ref m_str) = metadata_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                    public_key = meta_val
                        .get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            meta_val
                                .get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });

                    siths_card_id = meta_val
                        .get("siths_card_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    nfc_badge_uid = meta_val
                        .get("nfc_badge_uid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    personal_number = meta_val
                        .get("personal_number")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }

            Ok(WorkspaceUser {
                id,
                workspace_id: row.get(1)?,
                email: row.get(2)?,
                full_name: row.get(3)?,
                phone: row.get(4)?,
                role: row.get(5)?,
                preferences: row.get(6)?,
                siths_card_id,
                nfc_badge_uid,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                personal_number,
                public_key,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_parent_students(
    requester_user_id: String,
    workspace_id: String,
    parent_user_id: String,
) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("
            SELECT s.id, s.workspace_id, s.user_id, s.first_name, s.last_name, s.grade_level, s.parent_contact, s.updated_at
            FROM student_profiles s
            JOIN student_parents sp ON s.id = sp.student_id
            WHERE sp.parent_user_id = ?1 AND sp.workspace_id = ?2
        ")
        .await?;

    let list = stmt
        .query_map(crate::params![parent_user_id, workspace_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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

#[uniffi::export]
pub async fn get_student_submissions(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<Submission>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at FROM submissions WHERE workspace_id = ?1 AND student_id = ?2")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id, student_id], |row| {
            Ok(Submission {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                assignment_id: row.get(2)?,
                student_id: row.get(3)?,
                content: row.get(4)?,
                grade: row.get(5)?,
                feedback: row.get(6)?,
                submitted_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_timetable_slots(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<crate::TimetableSlot>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
            Ok(crate::TimetableSlot {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                course_id: row.get(2)?,
                day_of_week: row.get::<i64>(3)? as i32,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
                classroom: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_timetable_slot(
    requester_user_id: String,
    slot: crate::TimetableSlot,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != slot.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_schedule")?;

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, i64, String, String, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&slot.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        } else {
            None
        }
    };

    let mut course_id = slot.course_id.clone();
    let mut day_of_week = slot.day_of_week as i64;
    let mut start_time = slot.start_time.clone();
    let mut end_time = slot.end_time.clone();
    let mut classroom = slot.classroom.clone();

    let incoming_updated_at = if slot.updated_at > now_ms + 5000 { now_ms } else { slot.updated_at };
    if let Some((old_course_id, old_day_of_week, old_start_time, old_end_time, old_classroom, old_updated_at)) = existing {
        if old_updated_at > incoming_updated_at {
            let course_diff = old_course_id != slot.course_id;
            let day_diff = old_day_of_week != slot.day_of_week as i64;
            let start_diff = old_start_time != slot.start_time;
            let end_diff = old_end_time != slot.end_time;
            let class_diff = old_classroom != slot.classroom;

            if course_diff || day_diff || start_diff || end_diff || class_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "course_id": old_course_id.clone(),
                            "day_of_week": old_day_of_week,
                            "start_time": old_start_time.clone(),
                            "end_time": old_end_time.clone(),
                            "classroom": old_classroom.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "course_id": slot.course_id.clone(),
                            "day_of_week": slot.day_of_week as i64,
                            "start_time": slot.start_time.clone(),
                            "end_time": slot.end_time.clone(),
                            "classroom": slot.classroom.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &slot.workspace_id, "timetable_slots", &slot.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > slot.updated_at)
                course_id = old_course_id;
                day_of_week = old_day_of_week;
                start_time = old_start_time;
                end_time = old_end_time;
                classroom = old_classroom;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO timetable_slots (id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &slot.id,
            &slot.workspace_id,
            &course_id,
            &day_of_week,
            &start_time,
            &end_time,
            &classroom,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::{Course, TimetableSlot};

    #[tokio::test]
    async fn test_school_service_crud() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace & user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-school-test', 'School WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-admin', 'ws-school-test', 'sch@admin.com', 'admin')", ()).await.unwrap();

        // 1. Student Profile CRUD
        let profile = StudentProfile {
            id: "stud-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            user_id: None,
            first_name: "Jane".to_string(),
            last_name: "Smith".to_string(),
            grade_level: "10A".to_string(),
            parent_contact: Some("parent@smith.com".to_string()),
            updated_at: 0,
        };
        save_student_profile("u-school-admin".to_string(), profile.clone(), None).await.unwrap();

        let list = get_student_profiles("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].first_name, "Jane");

        // 2. Library CRUD
        let book = LibraryBook {
            id: "bk-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            title: "Physics I".to_string(),
            author: "Isaac Newton".to_string(),
            isbn: "111-222".to_string(),
            copies_available: 2,
            total_copies: 2,
            updated_at: 0,
        };
        save_library_book("u-school-admin".to_string(), book.clone(), None).await.unwrap();

        checkout_book("u-school-admin".to_string(), "ws-school-test".to_string(), "bk-1".to_string(), "stud-1".to_string(), "2026-08-01".to_string(), None).await.unwrap();
        let bk_list = get_library_books("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(bk_list[0].copies_available, 1);

        // 3. Billing CRUD
        let invoice = SchoolInvoice {
            id: "inv-s1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            student_id: "stud-1".to_string(),
            title: "Lab Fee".to_string(),
            amount: 50.0,
            due_date: "2026-08-01".to_string(),
            status: "unpaid".to_string(),
            paid_at: None,
            updated_at: 0,
        };
        create_school_invoice("u-school-admin".to_string(), invoice, None).await.unwrap();
        record_school_payment("u-school-admin".to_string(), "ws-school-test".to_string(), "inv-s1".to_string(), "Card".to_string(), None).await.unwrap();

        let invoices = get_school_invoices("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(invoices[0].status, "paid");
        assert!(invoices[0].paid_at.is_some());

        // 4. Parent Linking
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-parent-1', 'ws-school-test', 'parent@smith.com', 'parent')", ()).await.unwrap();
        link_parent_to_student("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string(), "u-parent-1".to_string(), None).await.unwrap();
        let parents = get_student_parents("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].id, "u-parent-1");

        // 5. Health Record & Incident CRUD
        let hr = HealthRecord {
            id: "hr-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            student_id: "stud-1".to_string(),
            vaccine_name: "MMR".to_string(),
            status: "administered".to_string(),
            administered_at: Some("2026-05-01".to_string()),
            updated_at: 0,
        };
        save_student_health_record("u-school-admin".to_string(), hr, None).await.unwrap();
        let hr_list = get_student_health_records("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
        assert_eq!(hr_list.len(), 1);
        assert_eq!(hr_list[0].vaccine_name, "MMR");

        let incident = HealthIncident {
            id: "inc-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            student_id: "stud-1".to_string(),
            visit_reason: "Fever".to_string(),
            treatment: "Paracetamol 500mg".to_string(),
            checked_in_at: "10:00".to_string(),
            checked_out_at: Some("10:30".to_string()),
            notes: Some("Slight headache".to_string()),
            updated_at: 0,
        };
        save_health_incident("u-school-admin".to_string(), incident, None).await.unwrap();
        let inc_list = get_health_incidents("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(inc_list.len(), 1);
        assert_eq!(inc_list[0].visit_reason, "Fever");

        // 6. Grading & Report Cards
        let test_course = Course {
            id: "crs-1".to_string(),
            name: "Math 101".to_string(),
            subject: "Math".to_string(),
            teacher_id: Some("u-school-admin".to_string()),
            classroom: Some("Room 101".to_string()),
        };
        save_course("u-school-admin".to_string(), "ws-school-test".to_string(), test_course, None).await.unwrap();

        let tg = TermGrade {
            id: "tg-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            student_id: "stud-1".to_string(),
            course_id: "crs-1".to_string(),
            term_name: "Fall 2026".to_string(),
            final_grade: Some("A".to_string()),
            final_points: Some(95),
            teacher_comments: Some("Excellent work".to_string()),
            updated_at: 0,
        };
        save_term_grade("u-school-admin".to_string(), tg, None).await.unwrap();
        let tg_list = get_term_grades("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
        assert_eq!(tg_list.len(), 1);
        assert_eq!(tg_list[0].final_grade, Some("A".to_string()));

        let c_tg_list = get_course_term_grades("u-school-admin".to_string(), "ws-school-test".to_string(), "crs-1".to_string()).await.unwrap();
        assert_eq!(c_tg_list.len(), 1);
        assert_eq!(c_tg_list[0].final_grade, Some("A".to_string()));

        let rc = ReportCard {
            id: "rc-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            student_id: "stud-1".to_string(),
            term_name: "Fall 2026".to_string(),
            gpa: 4.0,
            principal_comments: Some("Outstanding student".to_string()),
            status: "published".to_string(),
            updated_at: 0,
        };
        publish_report_card("u-school-admin".to_string(), rc, None).await.unwrap();
        let rc_list = get_report_cards("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
        assert_eq!(rc_list.len(), 1);
        assert_eq!(rc_list[0].gpa, 4.0);

        // 7. Submissions CRUD
        let test_assignment = Assignment {
            id: "assign-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            course_id: "crs-1".to_string(),
            title: "Math Homework 1".to_string(),
            description: "Solve page 10".to_string(),
            due_date: "2026-08-01".to_string(),
            max_points: 100,
            updated_at: 0,
        };
        save_assignment("u-school-admin".to_string(), test_assignment, None).await.unwrap();

        let sub = crate::Submission {
            id: "sub-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            assignment_id: "assign-1".to_string(),
            student_id: "stud-1".to_string(),
            content: "My homework answer".to_string(),
            grade: None,
            feedback: None,
            submitted_at: "2026-07-16T12:00:00Z".to_string(),
            updated_at: 0,
        };
        save_submission("u-school-admin".to_string(), sub.clone(), None).await.unwrap();
        let sub_list = get_student_submissions("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
        assert_eq!(sub_list.len(), 1);
        assert_eq!(sub_list[0].content, "My homework answer");

        // 8. Timetable slots CRUD
        let slot = crate::TimetableSlot {
            id: "slot-1".to_string(),
            workspace_id: "ws-school-test".to_string(),
            course_id: "crs-1".to_string(),
            day_of_week: 1,
            start_time: "08:30".to_string(),
            end_time: "09:45".to_string(),
            classroom: Some("Room 101".to_string()),
            updated_at: 0,
        };
        save_timetable_slot("u-school-admin".to_string(), slot.clone(), None).await.unwrap();
        let slots = get_timetable_slots("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].start_time, "08:30");

        // Cleanup
        conn.execute("DELETE FROM submissions WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM timetable_slots WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM assignments WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM report_cards WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM term_grades WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM health_incidents WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM health_records WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM school_payments WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-school-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_school_permission_validation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace with role templates in settings
        let settings_json = r#"{
            "roles": [
                {
                    "id": "role-school-teacher",
                    "name": "Teacher",
                    "permissions": {
                        "can_manage_schedule": true,
                        "can_manage_grades": true
                    }
                },
                {
                    "id": "role-school-student",
                    "name": "Student",
                    "permissions": {
                        "can_manage_schedule": false,
                        "can_manage_grades": false
                    }
                }
            ]
        }"#;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-perm-test', 'Perm WS', '[]', ?1)", crate::params![settings_json]).await.unwrap();
        
        // Insert users
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-teacher', 'ws-perm-test', 'teacher@school.com', 'teacher')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student', 'ws-perm-test', 'student@school.com', 'student')", ()).await.unwrap();

        // Setup course to test
        let course = Course {
            id: "crs-p1".to_string(),
            name: "History".to_string(),
            subject: "History".to_string(),
            teacher_id: Some("u-teacher".to_string()),
            classroom: Some("Room 202".to_string()),
        };

        // 1. Teacher has can_manage_schedule, so save_course should succeed
        let save_res = save_course("u-teacher".to_string(), "ws-perm-test".to_string(), course.clone(), None).await;
        assert!(save_res.is_ok(), "Teacher should be authorized: {:?}", save_res.err());

        // 2. Student does NOT have can_manage_schedule, so save_course should fail
        let save_student_res = save_course("u-student".to_string(), "ws-perm-test".to_string(), course.clone(), None).await;
        assert!(save_student_res.is_err(), "Student should be denied");
        if let Err(YntraError::AuthError(msg)) = save_student_res {
            assert!(msg.contains("does not have permission 'can_manage_schedule'"));
        } else {
            panic!("Expected AuthError for student course save");
        }

        // Cleanup
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-perm-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-perm-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-perm-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_grading_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-grade-test";
        let settings_json = r#"{
            "roles": [
                {
                    "id": "role-school-admin",
                    "name": "Admin",
                    "permissions": {
                        "can_manage_grades": true
                    }
                }
            ]
        }"#;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Grade WS', '[]', ?2)", crate::params![ws_id, settings_json]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-g', ?1, 'admin@g.com', 'admin')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-g', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-g', 'Math', 'Math', 'Room 1', ?1, 0)", crate::params![ws_id]).await.unwrap();

        // 1. Initial grade (v1)
        let grade_v1 = TermGrade {
            id: "tg-g".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-g".to_string(),
            course_id: "crs-g".to_string(),
            term_name: "Fall 2026".to_string(),
            final_grade: Some("B".to_string()),
            final_points: Some(80),
            teacher_comments: Some("Good progress".to_string()),
            updated_at: 1000,
        };

        save_term_grade("u-admin-g".to_string(), grade_v1, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // Fetch to find the actual updated_at saved in the DB
        let saved_time_a: i64 = conn.query_row(
            "SELECT updated_at FROM term_grades WHERE id = 'tg-g'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 2. Editor A updates the grade (having read the initial grade version)
        let grade_v2_a = TermGrade {
            id: "tg-g".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-g".to_string(),
            course_id: "crs-g".to_string(),
            term_name: "Fall 2026".to_string(),
            final_grade: Some("A-".to_string()),
            final_points: Some(90),
            teacher_comments: Some("Excellent progress".to_string()),
            updated_at: saved_time_a, // read version matches
        };
        save_term_grade("u-admin-g".to_string(), grade_v2_a, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // Fetch updated_at after A's write
        let saved_time_b: i64 = conn.query_row(
            "SELECT updated_at FROM term_grades WHERE id = 'tg-g'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 3. Editor B concurrent offline update (holds stale parent updated_at)
        let grade_v2_b = TermGrade {
            id: "tg-g".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-g".to_string(),
            course_id: "crs-g".to_string(),
            term_name: "Fall 2026".to_string(),
            final_grade: Some("B+".to_string()),
            final_points: Some(85),
            teacher_comments: Some("Steady progress".to_string()),
            updated_at: saved_time_a, // stale! (points to v1, but database is now at v2a)
        };

        // This save should trigger conflict detection!
        save_term_grade("u-admin-g".to_string(), grade_v2_b, None).await.unwrap();

        // 4. Assert conflict encoding
        let (final_grade, final_points, teacher_comments): (String, Option<i64>, String) = conn.query_row(
            "SELECT final_grade, final_points, teacher_comments FROM term_grades WHERE id = 'tg-g'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).await.unwrap();

        // The main table fields remain clean under Last-Write-Wins (which keeps Editor A's version as A wrote a newer version saved_time_b > saved_time_a)
        assert_eq!(final_grade, "A-");
        assert_eq!(final_points, Some(90));
        assert_eq!(teacher_comments, "Excellent progress");

        // The conflict table should record details of both versions
        let conflict_json: String = conn.query_row(
            "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'term_grades' AND entity_id = 'tg-g'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        assert!(conflict_json.contains("\"conflict\":true"));
        assert!(conflict_json.contains("A-"));
        assert!(conflict_json.contains("B+"));

        // Cleanup
        conn.execute("DELETE FROM term_grades WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_student_profile_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-profile-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Profile WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-p', ?1, 'admin@p.com', 'admin')", crate::params![ws_id]).await.unwrap();

        // 1. Initial version
        let profile_v1 = StudentProfile {
            id: "stud-p".to_string(),
            workspace_id: ws_id.to_string(),
            user_id: None,
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            grade_level: "10A".to_string(),
            parent_contact: Some("parent@doe.com".to_string()),
            updated_at: 1000,
        };
        save_student_profile("u-admin-p".to_string(), profile_v1, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        let saved_time_a: i64 = conn.query_row(
            "SELECT updated_at FROM student_profiles WHERE id = 'stud-p'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 2. Editor A updates
        let profile_v2_a = StudentProfile {
            id: "stud-p".to_string(),
            workspace_id: ws_id.to_string(),
            user_id: None,
            first_name: "Johnny".to_string(),
            last_name: "Doe".to_string(),
            grade_level: "10A".to_string(),
            parent_contact: Some("parent-new@doe.com".to_string()),
            updated_at: saved_time_a,
        };
        save_student_profile("u-admin-p".to_string(), profile_v2_a, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // 3. Editor B updates concurrently (using stale v1 time)
        let profile_v2_b = StudentProfile {
            id: "stud-p".to_string(),
            workspace_id: ws_id.to_string(),
            user_id: None,
            first_name: "John-Boy".to_string(),
            last_name: "Doe".to_string(),
            grade_level: "10B".to_string(),
            parent_contact: Some("parent-stale@doe.com".to_string()),
            updated_at: saved_time_a, // stale!
        };
        save_student_profile("u-admin-p".to_string(), profile_v2_b, None).await.unwrap();

        // 4. Assert conflict
        let (first_name, last_name, grade_level, parent_contact): (String, String, String, String) = conn.query_row(
            "SELECT first_name, last_name, grade_level, parent_contact FROM student_profiles WHERE id = 'stud-p'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).await.unwrap();

        assert_eq!(first_name, "Johnny");
        assert_eq!(last_name, "Doe");
        assert_eq!(grade_level, "10A");
        assert_eq!(parent_contact, "parent-new@doe.com");

        let conflict_json: String = conn.query_row(
            "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'student_profiles' AND entity_id = 'stud-p'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        assert!(conflict_json.contains("\"conflict\":true"));
        assert!(conflict_json.contains("Johnny"));
        assert!(conflict_json.contains("John-Boy"));

        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_attendance_record_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-att-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Attendance WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-a', ?1, 'admin@a.com', 'admin')", crate::params![ws_id]).await.unwrap();

        // Insert required relations to satisfy FOREIGN KEY checks
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-a', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-a', 'Math', 'Math', 'Room A', ?1, 0)", crate::params![ws_id]).await.unwrap();

        // 1. Initial version
        let rec_v1 = AttendanceRecord {
            id: "att-1".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-a".to_string(),
            course_id: "crs-a".to_string(),
            date: "2026-07-19".to_string(),
            status: "present".to_string(),
            notes: Some("On time".to_string()),
            updated_at: 1000,
        };
        save_attendance_record("u-admin-a".to_string(), rec_v1, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        let saved_time_a: i64 = conn.query_row(
            "SELECT updated_at FROM attendance_records WHERE id = 'att-1'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 2. Editor A updates
        let rec_v2_a = AttendanceRecord {
            id: "att-1".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-a".to_string(),
            course_id: "crs-a".to_string(),
            date: "2026-07-19".to_string(),
            status: "late".to_string(),
            notes: Some("Late 5 minutes".to_string()),
            updated_at: saved_time_a,
        };
        save_attendance_record("u-admin-a".to_string(), rec_v2_a, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // 3. Editor B updates concurrently
        let rec_v2_b = AttendanceRecord {
            id: "att-1".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-a".to_string(),
            course_id: "crs-a".to_string(),
            date: "2026-07-19".to_string(),
            status: "excused".to_string(),
            notes: Some("Parent called".to_string()),
            updated_at: saved_time_a, // stale!
        };
        save_attendance_record("u-admin-a".to_string(), rec_v2_b, None).await.unwrap();

        // 4. Assert conflict
        let (status, notes): (String, String) = conn.query_row(
            "SELECT status, notes FROM attendance_records WHERE id = 'att-1'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).await.unwrap();

        assert_eq!(status, "late");
        assert_eq!(notes, "Late 5 minutes");

        let conflict_json: String = conn.query_row(
            "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'attendance_records' AND entity_id = 'att-1'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        assert!(conflict_json.contains("\"conflict\":true"));
        assert!(conflict_json.contains("late"));
        assert!(conflict_json.contains("excused"));

        conn.execute("DELETE FROM attendance_records WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_timetable_slot_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-slot-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Slot WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-s', ?1, 'admin@s.com', 'admin')", crate::params![ws_id]).await.unwrap();

        // Insert required relations to satisfy FOREIGN KEY checks
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-slot-t', 'Math', 'Math', 'Room A', ?1, 0)", crate::params![ws_id]).await.unwrap();

        // 1. Initial version
        let slot_v1 = TimetableSlot {
            id: "slot-t".to_string(),
            workspace_id: ws_id.to_string(),
            course_id: "crs-slot-t".to_string(),
            day_of_week: 1,
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
            classroom: Some("Room A".to_string()),
            updated_at: 1000,
        };
        save_timetable_slot("u-admin-s".to_string(), slot_v1, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        let saved_time_a: i64 = conn.query_row(
            "SELECT updated_at FROM timetable_slots WHERE id = 'slot-t'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 2. Editor A updates
        let slot_v2_a = TimetableSlot {
            id: "slot-t".to_string(),
            workspace_id: ws_id.to_string(),
            course_id: "crs-slot-t".to_string(),
            day_of_week: 1,
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
            classroom: Some("Room B".to_string()),
            updated_at: saved_time_a,
        };
        save_timetable_slot("u-admin-s".to_string(), slot_v2_a, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // 3. Editor B updates concurrently
        let slot_v2_b = TimetableSlot {
            id: "slot-t".to_string(),
            workspace_id: ws_id.to_string(),
            course_id: "crs-slot-t".to_string(),
            day_of_week: 1,
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
            classroom: Some("Room C".to_string()),
            updated_at: saved_time_a, // stale!
        };
        save_timetable_slot("u-admin-s".to_string(), slot_v2_b, None).await.unwrap();

        // 4. Assert conflict
        let (course_id, day_of_week, start_time, end_time, classroom): (String, i64, String, String, String) = conn.query_row(
            "SELECT course_id, day_of_week, start_time, end_time, classroom FROM timetable_slots WHERE id = 'slot-t'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        ).await.unwrap();

        assert_eq!(course_id, "crs-slot-t");
        assert_eq!(day_of_week, 1);
        assert_eq!(start_time, "09:00");
        assert_eq!(end_time, "10:00");
        assert_eq!(classroom, "Room B");

        let conflict_json: String = conn.query_row(
            "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'timetable_slots' AND entity_id = 'slot-t'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        assert!(conflict_json.contains("\"conflict\":true"));
        assert!(conflict_json.contains("Room B"));
        assert!(conflict_json.contains("Room C"));

        conn.execute("DELETE FROM timetable_slots WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_health_incident_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-health-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Health WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-h', ?1, 'admin@h.com', 'admin')", crate::params![ws_id]).await.unwrap();

        // Insert required relations to satisfy FOREIGN KEY checks
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-h', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();

        // 1. Initial version
        let inc_v1 = HealthIncident {
            id: "inc-t".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-h".to_string(),
            visit_reason: "Cough".to_string(),
            treatment: "Cough Syrup".to_string(),
            checked_in_at: "09:00".to_string(),
            checked_out_at: Some("09:15".to_string()),
            notes: Some("Slight cold".to_string()),
            updated_at: 1000,
        };
        save_health_incident("u-admin-h".to_string(), inc_v1, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        let saved_time_a: i64 = conn.query_row(
            "SELECT updated_at FROM health_incidents WHERE id = 'inc-t'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        // 2. Editor A updates
        let inc_v2_a = HealthIncident {
            id: "inc-t".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-h".to_string(),
            visit_reason: "Cough".to_string(),
            treatment: "Cough Syrup + Tea".to_string(),
            checked_in_at: "09:00".to_string(),
            checked_out_at: Some("09:15".to_string()),
            notes: Some("Rest advised".to_string()),
            updated_at: saved_time_a,
        };
        save_health_incident("u-admin-h".to_string(), inc_v2_a, None).await.unwrap();
        crate::infra::time::sleep_ms(10).await;

        // 3. Editor B updates concurrently
        let inc_v2_b = HealthIncident {
            id: "inc-t".to_string(),
            workspace_id: ws_id.to_string(),
            student_id: "stud-h".to_string(),
            visit_reason: "Fever".to_string(),
            treatment: "Paracetamol".to_string(),
            checked_in_at: "09:10".to_string(),
            checked_out_at: Some("09:30".to_string()),
            notes: Some("Temp 38.5C".to_string()),
            updated_at: saved_time_a, // stale!
        };
        save_health_incident("u-admin-h".to_string(), inc_v2_b, None).await.unwrap();

        // 4. Assert conflict
        let (reason, treatment, checked_in_at, notes): (String, String, String, String) = conn.query_row(
            "SELECT visit_reason, treatment, checked_in_at, notes FROM health_incidents WHERE id = 'inc-t'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).await.unwrap();

        assert_eq!(reason, "Cough");
        assert_eq!(treatment, "Cough Syrup + Tea");
        assert_eq!(checked_in_at, "09:00");
        assert_eq!(notes, "Rest advised");

        let conflict_json: String = conn.query_row(
            "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'health_incidents' AND entity_id = 'inc-t'",
            (),
            |r| r.get(0),
        ).await.unwrap();

        assert!(conflict_json.contains("\"conflict\":true"));
        assert!(conflict_json.contains("Cough"));
        assert!(conflict_json.contains("Fever"));

        conn.execute("DELETE FROM health_incidents WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_school_row_level_sync() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspace and users
        let ws_id = "ws-sync-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Sync WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        
        // Unprivileged student user
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student-sync', ?1, 'std@school.com', 'student')", crate::params![ws_id]).await.unwrap();
        
        // Admins and other students
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-other-student', ?1, 'other@school.com', 'student')", crate::params![ws_id]).await.unwrap();

        // 2. Setup Student Profiles
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-sync-1', ?1, 'SyncStudent', 'One', '10A', 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-sync-2', ?1, 'OtherStudent', 'Two', '10A', 0)", crate::params![ws_id]).await.unwrap();

        // Map users to students so partitioning knows which students belong to the requester
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('u-student-sync', ?1, 'std@school.com', 'student', 'valid-sig')", crate::params![ws_id]).await.unwrap();
        // Insert student profile link for student-sync
        conn.execute("UPDATE student_profiles SET user_id = 'u-student-sync' WHERE id = 'stud-sync-1'", ()).await.unwrap();

        // Insert Course and Assignment to satisfy foreign key constraints
        conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at) VALUES ('crs-sync-1', ?1, 'Math', 'MATH101', 'u-teacher', 'Room 101', 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at) VALUES ('assign-sync-1', ?1, 'crs-sync-1', 'HW1', 'Homework 1', '2026-08-01', 100, 0)", crate::params![ws_id]).await.unwrap();

        // 3. Create a pending local write on submissions (e.g. submitting an assignment)
        conn.execute("INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, submitted_at, updated_at, sync_status) VALUES ('sub-sync-1', ?1, 'assign-sync-1', 'stud-sync-1', 'My Homework Content', '2026-07-20', 0, 'pending')", crate::params![ws_id]).await.unwrap();

        // 4. Configure database sync to enable sync loop (local/mock mode)
        crate::database::sync::configure_database_sync("local_url".to_string(), "local_token".to_string());

        // 5. Trigger sync
        crate::database::sync::sync_database().await.unwrap();

        // 6. Verify that submissions is marked as synced locally
        let sync_status: String = conn.query_row(
            "SELECT sync_status FROM submissions WHERE id = 'sub-sync-1'",
            (),
            |r| r.get(0),
        ).await.unwrap();
        assert_eq!(sync_status, "synced");

        // Clean up
        conn.execute("DELETE FROM submissions WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM assignments WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }

    #[tokio::test]
    async fn test_school_conflict_resolution() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-conflict-test";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Conflict WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-conflict-admin', ?1, 'admin@conf.com', 'admin')", crate::params![ws_id]).await.unwrap();

        // 1. Setup a conflict record in school_conflicts
        let conflict_json = serde_json::json!({
            "conflict": true,
            "versions": [
                {
                    "status": "absent",
                    "notes": "Original note",
                    "by": "Concurrent Editor",
                    "updated_at": 100
                },
                {
                    "status": "present",
                    "notes": "New note",
                    "by": "u-conflict-admin",
                    "updated_at": 200
                }
            ]
        });

        // Insert a dummy student_profile, course, and attendance_record to satisfy foreign keys
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-c', ?1, NULL, 'John', 'Doe', '10', 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-c', 'Math', 'Math', 'Room 1', ?1, 0)", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at) VALUES ('att-c', ?1, 'stud-c', 'crs-c', '2026-07-20', 'absent', 'Original note', 100)", crate::params![ws_id]).await.unwrap();

        let conflict_id = "attendance_records-att-c-12345";
        conn.execute(
            "INSERT INTO school_conflicts (id, workspace_id, entity_table, entity_id, conflict_json, updated_at) VALUES (?1, ?2, 'attendance_records', 'att-c', ?3, 200)",
            crate::params![conflict_id, ws_id, conflict_json.to_string()]
        ).await.unwrap();

        // 2. Verify get_school_conflicts retrieves it
        let conflicts = get_school_conflicts("u-conflict-admin".to_string(), ws_id.to_string()).await.unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].id, conflict_id);
        assert_eq!(conflicts[0].entity_table, "attendance_records");

        // 3. Resolve using version choice "1" (present, New note)
        resolve_school_conflict(
            "u-conflict-admin".to_string(),
            ws_id.to_string(),
            conflict_id.to_string(),
            "1".to_string(),
            None,
            None
        ).await.unwrap();

        // 4. Verify record in database has been updated and conflict has been removed
        let (resolved_status, resolved_notes): (String, Option<String>) = conn.query_row(
            "SELECT status, notes FROM attendance_records WHERE id = 'att-c'",
            (),
            |r| Ok((r.get(0)?, r.get(1)?))
        ).await.unwrap();
        assert_eq!(resolved_status, "present");
        assert_eq!(resolved_notes.unwrap(), "New note");

        let remaining = get_school_conflicts("u-conflict-admin".to_string(), ws_id.to_string()).await.unwrap();
        assert_eq!(remaining.len(), 0);

        // 5. Clean up
        conn.execute("DELETE FROM attendance_records WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }
}

#[uniffi::export]
pub async fn get_student_profiles_rkyv(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<u8>, YntraError> {
    let profiles = get_student_profiles(requester_user_id, workspace_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&profiles)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[uniffi::export]
pub async fn get_assignments_rkyv(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
) -> Result<Vec<u8>, YntraError> {
    let assignments = get_assignments(requester_user_id, workspace_id, course_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&assignments)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[uniffi::export]
pub async fn get_student_submissions_rkyv(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<u8>, YntraError> {
    let submissions = get_student_submissions(requester_user_id, workspace_id, student_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&submissions)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
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
