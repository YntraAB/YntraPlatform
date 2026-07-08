pub mod grading;

use crate::database;
use crate::observer::notify_observers;
use crate::{Course, Assignment, Submission, TermGrade, ReportCard, YntraError};
use grading::*;

#[uniffi::export]
pub async fn get_courses(requester_user_id: String) -> Result<Vec<Course>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status FROM courses WHERE workspace_id = ?1").await?;
    let list = stmt.query_map(crate::params![auth.workspace_id], |row| {
        Ok(Course {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            name: row.get(2)?,
            subject: row.get(3)?,
            teacher_id: row.get(4)?,
            classroom: row.get(5)?,
            updated_at: row.get(6)?,
            sync_status: row.get(7)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn add_course(
    requester_user_id: String,
    workspace_id: String,
    name: String,
    subject: String,
    teacher_id: Option<String>,
    classroom: Option<String>,
) -> Result<Course, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }
    if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage courses".to_string()));
    }

    if let Some(ref tid) = teacher_id {
        let teacher_ws: String = conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![tid],
            |r| r.get(0)
        ).await.map_err(|_| YntraError::NotFoundError("Teacher user not found".to_string()))?;
        if teacher_ws != workspace_id {
            return Err(YntraError::ValidationError("Teacher does not belong to the specified workspace".to_string()));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let course = Course {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        name: name.clone(),
        subject: subject.clone(),
        teacher_id: teacher_id.clone(),
        classroom: classroom.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
        crate::params![&id, &workspace_id, &name, &subject, &teacher_id, &classroom, &now_ms],
    ).await?;
    notify_observers();

    Ok(course)
}

#[uniffi::export]
pub async fn update_course(
    requester_user_id: String,
    id: String,
    name: String,
    subject: String,
    teacher_id: Option<String>,
    classroom: Option<String>,
) -> Result<Course, YntraError> {
    let conn = database::acquire_connection().await?;
    let course_ws: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != course_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage courses".to_string()));
    }

    if let Some(ref tid) = teacher_id {
        let teacher_ws: String = conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![tid],
            |r| r.get(0)
        ).await.map_err(|_| YntraError::NotFoundError("Teacher user not found".to_string()))?;
        if teacher_ws != course_ws {
            return Err(YntraError::ValidationError("Teacher does not belong to the specified workspace".to_string()));
        }
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    
    conn.execute(
        "UPDATE courses SET name = ?1, subject = ?2, teacher_id = ?3, classroom = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
        crate::params![&name, &subject, &teacher_id, &classroom, &now_ms, &id],
    ).await?;
    
    let row: (String, String, String, String, Option<String>, Option<String>, i64, String) = conn.query_row(
        "SELECT id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status FROM courses WHERE id = ?1",
        crate::params![&id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
    ).await?;
    
    let updated_course = Course {
        id: row.0,
        workspace_id: row.1,
        name: row.2,
        subject: row.3,
        teacher_id: row.4,
        classroom: row.5,
        updated_at: row.6,
        sync_status: row.7,
    };
    
    notify_observers();

    Ok(updated_course)
}

#[uniffi::export]
pub async fn get_assignments(requester_user_id: String, course_id: String) -> Result<Vec<Assignment>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    let course_ws: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&course_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != course_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn.prepare("SELECT id, workspace_id, course_id, title, description, due_date, max_points, updated_at, sync_status FROM assignments WHERE course_id = ?1").await?;
    let list = stmt.query_map(crate::params![&course_id], |row| {
        Ok(Assignment {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            course_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            due_date: row.get(5)?,
            max_points: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn add_assignment(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
    title: String,
    description: String,
    due_date: String,
    max_points: i32,
) -> Result<Assignment, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let course_ws: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&course_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if course_ws != workspace_id {
        return Err(YntraError::ValidationError("Course does not belong to the specified workspace".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_assignments").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage assignments".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let assignment = Assignment {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        course_id: course_id.clone(),
        title: title.clone(),
        description: description.clone(),
        due_date: due_date.clone(),
        max_points,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![&id, &workspace_id, &course_id, &title, &description, &due_date, &max_points, &now_ms],
    ).await?;
    notify_observers();

    Ok(assignment)
}

#[uniffi::export]
pub async fn get_submissions(
    requester_user_id: String,
    assignment_id: String,
) -> Result<Vec<Submission>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let assignment_ws: String = conn.query_row(
        "SELECT workspace_id FROM assignments WHERE id = ?1",
        crate::params![&assignment_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Assignment not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != assignment_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let is_teacher = super::check_permission(&conn, &requester_user_id, "can_manage_grades").await?;
    
    let mut stmt = conn.prepare("SELECT id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at, sync_status FROM submissions WHERE assignment_id = ?1").await?;
    let list = stmt.query_map(crate::params![&assignment_id], |row| {
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
            sync_status: row.get(9)?,
        })
    }).await?;

    if is_teacher {
        Ok(list)
    } else {
        let mut filtered = Vec::new();
        for sub in list {
            if super::has_academic_access(&conn, &requester_user_id, &sub.student_id).await? {
                filtered.push(sub);
            }
        }
        Ok(filtered)
    }
}

#[uniffi::export]
pub async fn add_submission(
    requester_user_id: String,
    workspace_id: String,
    assignment_id: String,
    student_id: String,
    content: String,
    grade: Option<String>,
    feedback: Option<String>,
) -> Result<Submission, YntraError> {
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

    let assignment_ws: String = conn.query_row(
        "SELECT workspace_id FROM assignments WHERE id = ?1",
        crate::params![&assignment_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Assignment not found".to_string()))?;

    if assignment_ws != workspace_id {
        return Err(YntraError::ValidationError("Assignment does not belong to the specified workspace".to_string()));
    }

    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied: cannot submit for this student".to_string()));
    }

    let can_grade = super::check_permission(&conn, &requester_user_id, "can_manage_grades").await?;
    if (grade.is_some() || feedback.is_some()) && !can_grade {
        return Err(YntraError::AuthError("Access denied: you do not have permission to grade submissions".to_string()));
    }

    let settings_json = if auth.role != "platform_admin" && auth.workspace_id == workspace_id {
        auth.workspace_settings.clone().unwrap_or_else(|| "{}".to_string())
    } else {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "{}".to_string())
    };

    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    let grade = match grade {
        Some(ref g) => Some(validate_grade_for_region(g, &target_region)?),
        None => None,
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let submitted_at = crate::infra::time::get_current_datetime_str();
    let submission = Submission {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        assignment_id: assignment_id.clone(),
        student_id: student_id.clone(),
        content: content.clone(),
        grade: grade.clone(),
        feedback: feedback.clone(),
        submitted_at: submitted_at.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![&id, &workspace_id, &assignment_id, &student_id, &content, &grade, &feedback, &submitted_at, &now_ms],
    ).await?;
    notify_observers();

    Ok(submission)
}

#[uniffi::export]
pub async fn update_submission_grade(
    requester_user_id: String,
    submission_id: String,
    grade: Option<String>,
    feedback: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let submission_ws: String = conn.query_row(
        "SELECT workspace_id FROM submissions WHERE id = ?1",
        crate::params![&submission_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Submission not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != submission_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let settings_json = if auth.role != "platform_admin" && auth.workspace_id == submission_ws {
        auth.workspace_settings.clone().unwrap_or_else(|| "{}".to_string())
    } else {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&submission_ws],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "{}".to_string())
    };

    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    let grade = match grade {
        Some(ref g) => Some(validate_grade_for_region(g, &target_region)?),
        None => None,
    };

    if !super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? {
        return Err(YntraError::AuthError("Access denied: cannot grade submissions".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "UPDATE submissions SET grade = ?1, feedback = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![&grade, &feedback, &now_ms, &submission_id],
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_term_grades(
    requester_user_id: String,
    student_id: String,
    term_name: String,
) -> Result<Vec<TermGrade>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied to student grades".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at, sync_status FROM term_grades WHERE student_id = ?1 AND term_name = ?2").await?;
    let list = stmt.query_map(crate::params![&student_id, &term_name], |row| {
        Ok(TermGrade {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            student_id: row.get(2)?,
            course_id: row.get(3)?,
            term_name: row.get(4)?,
            final_grade: row.get(5)?,
            final_points: row.get(6)?,
            teacher_comments: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn save_term_grade(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    course_id: String,
    term_name: String,
    final_grade: Option<String>,
    final_points: Option<i32>,
    teacher_comments: Option<String>,
) -> Result<TermGrade, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let settings_json = if auth.role != "platform_admin" && auth.workspace_id == workspace_id {
        auth.workspace_settings.clone().unwrap_or_else(|| "{}".to_string())
    } else {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "{}".to_string())
    };

    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    let final_grade = match final_grade {
        Some(ref g) => Some(validate_grade_for_region(g, &target_region)?),
        None => None,
    };

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
        return Err(YntraError::AuthError("Access denied: cannot modify term grades".to_string()));
    }
    let existing_id: Option<String> = conn.query_row(
        "SELECT id FROM term_grades WHERE student_id = ?1 AND course_id = ?2 AND term_name = ?3",
        crate::params![&student_id, &course_id, &term_name],
        |row| row.get(0)
    ).await.ok();
    
    let now_ms = crate::infra::time::get_current_time_ms();
    let record = if let Some(id) = existing_id {
        conn.execute(
            "UPDATE term_grades SET final_grade = ?1, final_points = ?2, teacher_comments = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
            crate::params![&final_grade, &final_points, &teacher_comments, &now_ms, &id]
        ).await?;
        TermGrade {
            id,
            workspace_id,
            student_id,
            course_id,
            term_name,
            final_grade,
            final_points,
            teacher_comments,
            updated_at: now_ms,
            sync_status: "pending".to_string(),
        }
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
            crate::params![&id, &workspace_id, &student_id, &course_id, &term_name, &final_grade, &final_points, &teacher_comments, &now_ms]
        ).await?;
        TermGrade {
            id,
            workspace_id,
            student_id,
            course_id,
            term_name,
            final_grade,
            final_points,
            teacher_comments,
            updated_at: now_ms,
            sync_status: "pending".to_string(),
        }
    };
    notify_observers();
    Ok(record)
}

#[uniffi::export]
pub async fn get_report_cards(requester_user_id: String, student_id: String) -> Result<Vec<ReportCard>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied to student report cards".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at, sync_status FROM report_cards WHERE student_id = ?1").await?;
    let list = stmt.query_map(crate::params![&student_id], |row| {
        Ok(ReportCard {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            student_id: row.get(2)?,
            term_name: row.get(3)?,
            gpa: row.get(4)?,
            principal_comments: row.get(5)?,
            status: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;

    let is_staff = super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? 
        || super::check_permission(&conn, &requester_user_id, "can_publish_report_cards").await?;

    let mut filtered = Vec::new();
    for rc in list {
        if is_staff || rc.status == "published" {
            filtered.push(rc);
        }
    }
    Ok(filtered)
}

#[uniffi::export]
pub async fn publish_report_card(
    requester_user_id: String,
    report_card_id: String,
    principal_comments: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let rc_ws: String = conn.query_row(
        "SELECT workspace_id FROM report_cards WHERE id = ?1",
        crate::params![&report_card_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Report card not found".to_string()))?;

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != rc_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_publish_report_cards").await? {
        return Err(YntraError::AuthError("Access denied: cannot publish report cards".to_string()));
    }
    conn.execute(
        "UPDATE report_cards SET status = 'published', principal_comments = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![&principal_comments, &crate::infra::time::get_current_time_ms(), &report_card_id]
    ).await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn calculate_and_save_gpa(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    term_name: String,
) -> Result<ReportCard, YntraError> {
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

    if !super::check_permission(&conn, &requester_user_id, "can_manage_grades").await? {
        return Err(YntraError::AuthError("Access denied: cannot calculate GPA".to_string()));
    }
    let mut stmt = conn.prepare("SELECT final_grade FROM term_grades WHERE student_id = ?1 AND term_name = ?2").await?;
    let grades: Vec<Option<String>> = stmt.query_map(crate::params![&student_id, &term_name], |row| row.get(0)).await?;
    
    let settings_json = if auth.role != "platform_admin" && auth.workspace_id == workspace_id {
        auth.workspace_settings.clone().unwrap_or_else(|| "{}".to_string())
    } else {
        conn.query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "{}".to_string())
    };
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    let gpa = calculate_gpa(&grades, &target_region);
    
    let existing_id: Option<String> = conn.query_row(
        "SELECT id FROM report_cards WHERE student_id = ?1 AND term_name = ?2",
        crate::params![&student_id, &term_name],
        |row| row.get(0)
    ).await.ok();
    
    let now_ms = crate::infra::time::get_current_time_ms();
    let record = if let Some(id) = existing_id {
        conn.execute(
            "UPDATE report_cards SET gpa = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![&gpa, &now_ms, &id]
        ).await?;
        
        let principal_comments: Option<String> = conn.query_row(
            "SELECT principal_comments FROM report_cards WHERE id = ?1",
            crate::params![&id],
            |row| row.get(0)
        ).await.unwrap_or(None);
        
        let status: String = conn.query_row(
            "SELECT status FROM report_cards WHERE id = ?1",
            crate::params![&id],
            |row| row.get(0)
        ).await.unwrap_or_else(|_| "draft".to_string());
        
        ReportCard {
            id,
            workspace_id,
            student_id,
            term_name,
            gpa,
            principal_comments,
            status,
            updated_at: now_ms,
            sync_status: "pending".to_string(),
        }
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO report_cards (id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'draft', ?6, 'pending')",
            crate::params![&id, &workspace_id, &student_id, &term_name, &gpa, &now_ms]
        ).await?;
        ReportCard {
            id,
            workspace_id,
            student_id,
            term_name,
            gpa,
            principal_comments: None,
            status: "draft".to_string(),
            updated_at: now_ms,
            sync_status: "pending".to_string(),
        }
    };
    notify_observers();
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_get_submissions_workspace_mismatch() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup
        let _ = conn.execute("DELETE FROM submissions WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM assignments WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM courses WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('ws-acad-a', 'ws-acad-b')", ()).await;

        // Setup two workspaces
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-acad-a', 'WS A', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-acad-b', 'WS B', '[]', '{}')", ()).await.unwrap();

        // Teacher in WS A
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-acad-teacher-a', 'ws-acad-a', 'teacher@acad.a', 'teacher')", ()).await.unwrap();
        // Student/other user in WS B
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-acad-user-b', 'ws-acad-b', 'student@acad.b', 'student')", ()).await.unwrap();

        // Course and assignment in WS B
        conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at) VALUES ('course-b', 'ws-acad-b', 'Math B', 'Math', 'u-acad-user-b', 'Room 1', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at) VALUES ('assign-b', 'ws-acad-b', 'course-b', 'Homework 1', 'Solve problems', '2026-07-20', 100, 0)", ()).await.unwrap();

        // Teacher from WS A queries submissions of assignment from WS B -> should fail with AuthError
        let res = get_submissions("u-acad-teacher-a".to_string(), "assign-b".to_string()).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        let _ = conn.execute("DELETE FROM assignments WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM courses WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('ws-acad-a', 'ws-acad-b')", ()).await;
    }

    #[tokio::test]
    async fn test_add_submission_grade_validation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Setup
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-grade-val', 'WS A', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-teacher', 'ws-grade-val', 't@sch.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student', 'ws-grade-val', 's@sch.io', 'student')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('s-prof', 'ws-grade-val', 'u-student', 'Bob', 'Smith', 'Grade 5', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at) VALUES ('course-v', 'ws-grade-val', 'Math', 'Math', 'u-teacher', 'Room 1', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at) VALUES ('assign-v', 'ws-grade-val', 'course-v', 'Homework', 'Solve', '2026-07-20', 100, 0)", ()).await.unwrap();

        // 1. Invalid grade "Z" (SE only allows A-F or U) -> should fail
        let res = add_submission(
            "u-teacher".to_string(),
            "ws-grade-val".to_string(),
            "assign-v".to_string(),
            "s-prof".to_string(),
            "Content".to_string(),
            Some("Z".to_string()),
            None,
        ).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        // 2. Valid grade "A" -> should succeed
        let res = add_submission(
            "u-teacher".to_string(),
            "ws-grade-val".to_string(),
            "assign-v".to_string(),
            "s-prof".to_string(),
            "Content".to_string(),
            Some("A".to_string()),
            None,
        ).await;
        assert!(res.is_ok());

        // Cleanup
        conn.execute("DELETE FROM submissions WHERE workspace_id = 'ws-grade-val'", ()).await.unwrap();
        conn.execute("DELETE FROM assignments WHERE workspace_id = 'ws-grade-val'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-grade-val'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-grade-val'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-grade-val'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-grade-val'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_report_cards_draft_filtering() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Setup
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rc-val', 'WS A', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-teacher-rc', 'ws-rc-val', 't@sch.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student-rc', 'ws-rc-val', 's@sch.io', 'student')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('s-prof-rc', 'ws-rc-val', 'u-student-rc', 'Bob', 'Smith', 'Grade 5', 0)", ()).await.unwrap();

        // Create draft report card directly in DB
        let rc_id = "rc-test-1".to_string();
        conn.execute(
            "INSERT INTO report_cards (id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at, sync_status) VALUES (?1, 'ws-rc-val', 's-prof-rc', 'Term 1', 4.0, NULL, 'draft', 0, 'synced')",
            crate::params![&rc_id],
        ).await.unwrap();

        // 1. Query as teacher (staff) -> should return the draft report card
        let res_teacher = get_report_cards("u-teacher-rc".to_string(), "s-prof-rc".to_string()).await.unwrap();
        assert_eq!(res_teacher.len(), 1);
        assert_eq!(res_teacher[0].status, "draft");

        // 2. Query as student -> should filter out draft report card, returning 0
        let res_student = get_report_cards("u-student-rc".to_string(), "s-prof-rc".to_string()).await.unwrap();
        assert_eq!(res_student.len(), 0);

        // 3. Publish report card
        publish_report_card("u-teacher-rc".to_string(), rc_id, Some("Good job".to_string())).await.unwrap();

        // 4. Query as student again -> should now return the published report card
        let res_student_pub = get_report_cards("u-student-rc".to_string(), "s-prof-rc".to_string()).await.unwrap();
        assert_eq!(res_student_pub.len(), 1);
        assert_eq!(res_student_pub[0].status, "published");

        // Cleanup
        conn.execute("DELETE FROM report_cards WHERE workspace_id = 'ws-rc-val'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-rc-val'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rc-val'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-rc-val'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_course_teacher_workspace_mismatch() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Setup two workspaces
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-course-a', 'WS A', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-course-b', 'WS B', '[]', '{}')", ()).await.unwrap();

        // Admin of A
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-a', 'ws-course-a', 'admina@sch.io', 'admin')", ()).await.unwrap();
        // Teacher of B
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-teacher-b', 'ws-course-b', 'teacherb@sch.io', 'teacher')", ()).await.unwrap();

        // Try to add course in workspace A with teacher from B -> should fail
        let res = add_course(
            "u-admin-a".to_string(),
            "ws-course-a".to_string(),
            "Math".to_string(),
            "Math".to_string(),
            Some("u-teacher-b".to_string()),
            None,
        ).await;

        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-course-a', 'ws-course-b')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-course-a', 'ws-course-b')", ()).await.unwrap();
    }
}
