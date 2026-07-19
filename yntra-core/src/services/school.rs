use crate::database;
use crate::infra::observer::notify_observers;
use crate::{
    StudentProfile, Assignment, Submission, AttendanceRecord, TermGrade, ReportCard,
    LibraryBook, SchoolInvoice, LibraryLendingLogInfo, HealthRecord, HealthIncident,
    WorkspaceUser, YntraError
};
use uuid::Uuid;

async fn verify_school_write_zkp(
    conn: &database::DbConnection,
    requester_user_id: &str,
    role: &str,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
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

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &profile.id,
            &profile.workspace_id,
            &profile.user_id,
            &profile.first_name,
            &profile.last_name,
            &profile.grade_level,
            &profile.parent_contact,
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

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &submission.id,
            &submission.workspace_id,
            &submission.assignment_id,
            &submission.student_id,
            &submission.content,
            &submission.grade,
            &submission.feedback,
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

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &record.id,
            &record.workspace_id,
            &record.student_id,
            &record.course_id,
            &record.date,
            &record.status,
            &record.notes,
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

    let now_ms = crate::infra::time::get_current_time_ms();
    let pts = grade.final_points.map(|p| p as i64);
    conn.execute(
        "INSERT OR REPLACE INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &grade.id,
            &grade.workspace_id,
            &grade.student_id,
            &grade.course_id,
            &grade.term_name,
            &grade.final_grade,
            &pts,
            &grade.teacher_comments,
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

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO health_incidents (id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &incident.id,
            &incident.workspace_id,
            &incident.student_id,
            &incident.visit_reason,
            &incident.treatment,
            &incident.checked_in_at,
            &incident.checked_out_at,
            &incident.notes,
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
    conn.execute(
        "INSERT OR REPLACE INTO timetable_slots (id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &slot.id,
            &slot.workspace_id,
            &slot.course_id,
            &(slot.day_of_week as i64),
            &slot.start_time,
            &slot.end_time,
            &slot.classroom,
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
    use crate::Course;

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
}
