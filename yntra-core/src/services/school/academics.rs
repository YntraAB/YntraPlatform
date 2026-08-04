use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::notes::verify_zkp_if_encrypted;
use crate::services::school::auth::{
    verify_school_permission, verify_school_write_zkp, verify_student_access,
};
use crate::services::school::conflicts::record_school_conflict;
use crate::{Assignment, Course, ReportCard, Submission, TermGrade};

#[uniffi::export]
pub async fn save_course(
    requester_user_id: String,
    workspace_id: String,
    course: Course,
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
pub async fn delete_course(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let ws_id: String = conn
        .query_row(
            "SELECT workspace_id FROM courses WHERE id = ?1",
            crate::params![&id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute("DELETE FROM courses WHERE id = ?1", crate::params![&id])
        .await?;
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
pub async fn delete_assignment(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let ws_id: String = conn
        .query_row(
            "SELECT workspace_id FROM assignments WHERE id = ?1",
            crate::params![&id],
            |r| r.get(0),
        )
        .await?;

    conn.execute(
        "DELETE FROM assignments WHERE id = ?1 AND workspace_id = ?2",
        crate::params![&id, &ws_id],
    )
    .await?;

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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    if auth.role != "platform_admin" && auth.role != "admin" && auth.role != "school-admin" {
        let is_student = auth.role == "student"
            || auth.role == "role-school-student"
            || has_school_permission(&auth, "can_manage_grades");
        if !is_student {
            return Err(YntraError::AuthError(
                "Access denied: insufficient permissions to manage submissions".to_string(),
            ));
        }
    }

    let team_id = "";
    verify_zkp_if_encrypted(
        &conn,
        &submission.content,
        &auth.user_id,
        &auth.role,
        &submission.workspace_id,
        team_id,
    )
    .await?;
    if let Some(ref grade) = submission.grade {
        verify_zkp_if_encrypted(
            &conn,
            grade,
            &auth.user_id,
            &auth.role,
            &submission.workspace_id,
            team_id,
        )
        .await?;
    }
    if let Some(ref feedback) = submission.feedback {
        verify_zkp_if_encrypted(
            &conn,
            feedback,
            &auth.user_id,
            &auth.role,
            &submission.workspace_id,
            team_id,
        )
        .await?;
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

    let incoming_updated_at = if submission.updated_at > now_ms + 5000 {
        now_ms
    } else {
        submission.updated_at
    };
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

                record_school_conflict(
                    &conn,
                    &submission.workspace_id,
                    "submissions",
                    &submission.id,
                    mvr,
                )
                .await?;

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

fn has_school_permission(auth: &crate::AuthContext, permission_name: &str) -> bool {
    crate::services::school::auth::has_school_permission(auth, permission_name)
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
pub async fn get_term_grades(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<TermGrade>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_grades")?;

    let team_id = "";
    if let Some(ref final_g) = grade.final_grade {
        verify_zkp_if_encrypted(
            &conn,
            final_g,
            &auth.user_id,
            &auth.role,
            &grade.workspace_id,
            team_id,
        )
        .await?;
    }
    if let Some(ref comments) = grade.teacher_comments {
        verify_zkp_if_encrypted(
            &conn,
            comments,
            &auth.user_id,
            &auth.role,
            &grade.workspace_id,
            team_id,
        )
        .await?;
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

    let incoming_updated_at = if grade.updated_at > now_ms + 5000 {
        now_ms
    } else {
        grade.updated_at
    };
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

                record_school_conflict(&conn, &grade.workspace_id, "term_grades", &grade.id, mvr)
                    .await?;

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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_publish_report_cards")?;

    let team_id = "";
    if let Some(ref comments) = report.principal_comments {
        verify_zkp_if_encrypted(
            &conn,
            comments,
            &auth.user_id,
            &auth.role,
            &report.workspace_id,
            team_id,
        )
        .await?;
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
