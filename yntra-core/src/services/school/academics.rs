use crate::database;
use crate::observer::notify_observers;
use crate::{Course, Assignment, Submission, TermGrade, ReportCard, YntraError};

fn validate_grade_for_region(grade: &str, region: &str) -> Result<(), YntraError> {
    let mut clean = grade.trim().to_string();
    if clean.is_empty() {
        return Ok(());
    }
    match region {
        "SE" => {
            // Swedish grades: A-F (and lowercase), - (streck), G (Godkänd), VG (Väl godkänd), MVG (Mycket väl godkänd), U (Underkänd)
            let valid_grades = ["A", "B", "C", "D", "E", "F", "a", "b", "c", "d", "e", "f", "-", "G", "VG", "MVG", "U", "g", "vg", "mvg", "u"];
            if !valid_grades.contains(&clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Swedish grading system (expected A-F, G, VG, MVG, U, or -).", clean)));
            }
        }
        "NO" => {
            // Norwegian: 1-6, or standard Pass/Fail: Bestått (G / B), Ikke bestått (U / IB)
            // Allow modifiers: +, -, or combined like 5/6
            let mut base = clean.clone();
            if base.ends_with('+') || base.ends_with('-') {
                base.pop();
            } else if base.contains('/') {
                let parts: Vec<&str> = base.split('/').collect();
                if parts.len() == 2 {
                    let valid_grades = ["1", "2", "3", "4", "5", "6"];
                    if valid_grades.contains(&parts[0]) && valid_grades.contains(&parts[1]) {
                        return Ok(());
                    }
                }
            }
            let valid_grades = ["1", "2", "3", "4", "5", "6", "B", "b", "IB", "ib", "G", "g", "U", "u"];
            if !valid_grades.contains(&base.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Norwegian grading system (expected 1-6 or B/IB with optional modifiers).", clean)));
            }
        }
        "DK" => {
            // Normalize: "2" -> "02", "0" -> "00"
            if clean == "2" {
                clean = "02".to_string();
            } else if clean == "0" {
                clean = "00".to_string();
            }
            let valid_grades = ["-3", "00", "02", "4", "7", "10", "12"];
            if !valid_grades.contains(&clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Danish grading system (expected -3, 00, 02, 4, 7, 10, or 12).", clean)));
            }
        }
        "FI" => {
            // Finnish: 4-10 (comprehensive), 0-5 (university), S/H (suoritettu/hylätty), HYV/HYL (hyväksytty/hylätty)
            // Allow modifiers: +, -, ½, .5
            let mut base = clean.clone();
            if base.ends_with('+') || base.ends_with('-') || base.ends_with('½') {
                base.pop();
            } else if base.ends_with(".5") {
                base = base[..base.len()-2].to_string();
            }
            let valid_grades = [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
                "S", "H", "s", "h", "HYV", "HYL", "hyv", "hyl"
            ];
            if !valid_grades.contains(&base.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Finnish grading system (expected 4-10, 0-5, S, H, HYV, or HYL with optional modifier).", clean)));
            }
        }
        r if r.starts_with("US") => {
            let upper_clean = clean.to_uppercase();
            // Non-punitive US codes
            let non_punitive = ["I", "INC", "P", "NP", "W", "AU"];
            if non_punitive.contains(&upper_clean.as_str()) {
                return Ok(());
            }
            
            // Allow US percentage grade (numeric score between 0.0 and 100.0)
            if let Ok(pct) = upper_clean.parse::<f64>() {
                if pct >= 0.0 && pct <= 100.0 {
                    return Ok(());
                }
            }
            
            let first_char = upper_clean.chars().next().unwrap_or(' ');
            // US grades allow A-E (some districts use E) and F
            let valid_letters = ['A', 'B', 'C', 'D', 'E', 'F'];
            if !valid_letters.contains(&first_char) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for US grading system (expected A-F, percentage, or non-punitive codes like I, P, W).", clean)));
            }
            if upper_clean.len() > 2 {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for US grading system (too long).", clean)));
            }
            if upper_clean.len() == 2 {
                let second_char = upper_clean.chars().nth(1).unwrap_or(' ');
                if second_char != '+' && second_char != '-' {
                    return Err(YntraError::ValidationError(format!("Invalid modifier in US grade '{}' (expected + or -).", clean)));
                }
            }
        }
        "EU" => {
            // Default EU ECTS grades: A, B, C, D, E, FX, F
            let upper_clean = clean.to_uppercase();
            let valid_grades = ["A", "B", "C", "D", "E", "FX", "F"];
            if !valid_grades.contains(&upper_clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for ECTS grading system.", clean)));
            }
        }
        _ => {}
    }
    Ok(())
}

#[uniffi::export]
pub async fn get_courses(requester_user_id: String) -> Result<Vec<Course>, YntraError> {
    let conn = database::acquire_connection().await?;
    let requester_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

    let mut stmt = conn.prepare("SELECT id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status FROM courses WHERE workspace_id = ?1").await?;
    let list = stmt.query_map(crate::params![requester_ws], |row| {
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
pub async fn get_assignments(course_id: String) -> Result<Vec<Assignment>, YntraError> {
    let conn = database::acquire_connection().await?;
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
    let submission_ws: String = conn.query_row(
        "SELECT workspace_id FROM submissions WHERE id = ?1",
        crate::params![&submission_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Submission not found".to_string()))?;

    // Determine target region from workspace settings
    let settings_json: String = conn.query_row(
        "SELECT settings FROM workspaces WHERE id = ?1",
        crate::params![&submission_ws],
        |r| r.get(0)
    ).await.unwrap_or_else(|_| "{}".to_string());
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    if let Some(ref g) = grade {
        validate_grade_for_region(g, &target_region)?;
    }

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != submission_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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

    // Determine target region from workspace settings
    let settings_json: String = conn.query_row(
        "SELECT settings FROM workspaces WHERE id = ?1",
        crate::params![&workspace_id],
        |r| r.get(0)
    ).await.unwrap_or_else(|_| "{}".to_string());
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region_raw = settings.get("target_region").and_then(|v| v.as_str()).unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    if let Some(ref g) = final_grade {
        validate_grade_for_region(g, &target_region)?;
    }
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
    Ok(list)
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
    
    // Determine target region from workspace settings
    let settings_json: String = conn.query_row(
        "SELECT settings FROM workspaces WHERE id = ?1",
        crate::params![&workspace_id],
        |r| r.get(0)
    ).await.unwrap_or_else(|_| "{}".to_string());
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

pub(crate) fn calculate_gpa(grades: &[Option<String>], region: &str) -> f64 {
    let mut total_points = 0.0;
    let mut count = 0;
    let upper_region = region.to_uppercase();
    
    let mut use_fi_university = false;
    if upper_region == "FI" {
        let mut has_uni_indicators = false;
        let mut has_comp_indicators = false;
        for g_opt in grades {
            if let Some(g) = g_opt {
                let clean = g.trim().to_uppercase();
                if ["0", "1", "2", "3", "5"].contains(&clean.as_str()) {
                    has_uni_indicators = true;
                }
                if ["6", "7", "8", "9", "10"].contains(&clean.as_str()) {
                    has_comp_indicators = true;
                }
            }
        }
        if has_uni_indicators && !has_comp_indicators {
            use_fi_university = true;
        }
    }
    
    for g_opt in grades {
        if let Some(g) = g_opt {
            let clean = g.trim().to_uppercase();
            if clean.is_empty() {
                continue;
            }
            let pts = match upper_region.as_str() {
                "SE" => match clean.as_str() {
                    "A" => Some(4.0),
                    "B" => Some(3.0),
                    "C" => Some(2.0),
                    "D" => Some(1.5),
                    "E" => Some(1.0),
                    "F" | "U" | "IG" => Some(0.0),
                    "G" => Some(2.0),
                    "VG" => Some(3.5),
                    "MVG" => Some(4.0),
                    _ => None,
                },
                "NO" => match clean.as_str() {
                    "6" => Some(4.0),
                    "5" => Some(3.0),
                    "4" => Some(2.0),
                    "3" => Some(1.5),
                    "2" => Some(1.0),
                    "1" | "U" | "IB" => Some(0.0),
                    "G" | "B" => Some(3.0),
                    _ => None,
                },
                "DK" => match clean.as_str() {
                    "12" => Some(4.0),
                    "10" => Some(3.5),
                    "7" => Some(3.0),
                    "4" => Some(2.0),
                    "02" => Some(1.0),
                    "00" | "0" | "-3" => Some(0.0),
                    _ => None,
                },
                "FI" => {
                    if use_fi_university {
                        match clean.as_str() {
                            "5" => Some(4.0),
                            "4" => Some(3.5),
                            "3" => Some(3.0),
                            "2" => Some(2.0),
                            "1" => Some(1.0),
                            "0" | "H" | "HYL" | "I" => Some(0.0),
                            "S" | "HYV" => Some(3.0),
                            "L" => Some(4.0),
                            "E" => Some(3.5),
                            "M" => Some(3.0),
                            "C" => Some(2.5),
                            "B" => Some(2.0),
                            "A" => Some(1.0),
                            _ => None,
                        }
                    } else {
                        match clean.as_str() {
                            "10" => Some(4.0),
                            "9" => Some(3.5),
                            "8" => Some(3.0),
                            "7" => Some(2.0),
                            "6" => Some(1.5),
                            "5" => Some(1.0),
                            "4" | "H" | "HYL" | "I" => Some(0.0),
                            "S" | "HYV" => Some(3.0),
                            "L" => Some(4.0),
                            "E" => Some(3.5),
                            "M" => Some(3.0),
                            "C" => Some(2.5),
                            "B" => Some(2.0),
                            "A" => Some(1.0),
                            _ => None,
                        }
                    }
                }
                r if r.starts_with("US") => {
                    match clean.as_str() {
                        "A" | "A+" => Some(4.0),
                        "A-" => Some(3.7),
                        "B+" => Some(3.3),
                        "B" => Some(3.0),
                        "B-" => Some(2.7),
                        "C+" => Some(2.3),
                        "C" => Some(2.0),
                        "C-" => Some(1.7),
                        "D+" => Some(1.3),
                        "D" | "E" | "E+" => Some(1.0),
                        "D-" | "E-" => Some(0.7),
                        "F" => Some(0.0),
                        _ => {
                            // Check if it's a numeric percentage grade
                            let mut clean_num = clean.clone();
                            while clean_num.ends_with('%') || clean_num.ends_with('+') || clean_num.ends_with('-') {
                                clean_num.pop();
                            }
                            if let Ok(pct) = clean_num.parse::<f64>() {
                                if pct >= 90.0 { Some(4.0) }
                                else if pct >= 80.0 { Some(3.0) }
                                else if pct >= 70.0 { Some(2.0) }
                                else if pct >= 60.0 { Some(1.0) }
                                else { Some(0.0) }
                            } else {
                                None
                            }
                        }
                    }
                },
                _ => {
                    // Default EU ECTS scale
                    match clean.as_str() {
                        "A" => Some(4.0),
                        "B" => Some(3.0),
                        "C" => Some(2.0),
                        "D" => Some(1.0),
                        "E" => Some(1.0),
                        "FX" | "F" => Some(0.0),
                        _ => None,
                    }
                }
            };
            if let Some(p) = pts {
                total_points += p;
                count += 1;
            }
        }
    }
    if count > 0 { total_points / (count as f64) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_gpa_all_cases() {
        // Test standard values
        assert_eq!(calculate_gpa(&[Some("A".to_string())], "US"), 4.0);
        assert_eq!(calculate_gpa(&[Some("B".to_string())], "US"), 3.0);
        assert_eq!(calculate_gpa(&[Some("C".to_string())], "US"), 2.0);
        assert_eq!(calculate_gpa(&[Some("D".to_string())], "US"), 1.0);
        assert_eq!(calculate_gpa(&[Some("E".to_string())], "US"), 1.0);
        assert_eq!(calculate_gpa(&[Some("F".to_string())], "US"), 0.0);

        // Test Norwegian grades
        assert_eq!(calculate_gpa(&[Some("6".to_string()), Some("5".to_string())], "NO"), 3.5);
        assert_eq!(calculate_gpa(&[Some("2".to_string()), Some("1".to_string())], "NO"), 0.5);

        // Test Danish grades (including Danish 4)
        assert_eq!(calculate_gpa(&[Some("12".to_string()), Some("4".to_string())], "DK"), 3.0);
        assert_eq!(calculate_gpa(&[Some("02".to_string()), Some("-3".to_string())], "DK"), 0.5);

        // Test Finnish grades (comprehensive & university)
        assert_eq!(calculate_gpa(&[Some("10".to_string()), Some("8".to_string())], "FI"), 3.5);
        assert_eq!(calculate_gpa(&[Some("3".to_string()), Some("1".to_string())], "FI"), 2.0);
        assert_eq!(calculate_gpa(&[Some("5".to_string()), Some("4".to_string())], "FI"), 3.75);
        assert_eq!(calculate_gpa(&[Some("10".to_string()), Some("5".to_string()), Some("4".to_string())], "FI"), 5.0 / 3.0);

        // Test Swedish old scale fail grade IG
        assert_eq!(calculate_gpa(&[Some("MVG".to_string()), Some("IG".to_string())], "SE"), 2.0);

        // Test US percentage grades
        assert_eq!(calculate_gpa(&[Some("95".to_string()), Some("85".to_string())], "US-CA"), 3.5);

        // Test average calculations
        assert_eq!(calculate_gpa(&[Some("A".to_string()), Some("B".to_string())], "US"), 3.5);
        assert_eq!(calculate_gpa(&[Some("A".to_string()), Some("F".to_string())], "US"), 2.0);

        // Test lowercase and whitespaces
        assert_eq!(calculate_gpa(&[Some("  a  ".to_string()), Some("b\n".to_string())], "US"), 3.5);

        // Test invalid and None grades ignored
        assert_eq!(
            calculate_gpa(&[
                Some("A".to_string()),
                None,
                Some("INVALID".to_string()),
                Some("B".to_string())
            ], "US"),
            3.5
        );

        // Test empty/only invalid inputs
        assert_eq!(calculate_gpa(&[], "US"), 0.0);
        assert_eq!(calculate_gpa(&[None, Some("X".to_string())], "US"), 0.0);
    }

    #[test]
    fn test_validate_grade_for_region_all_cases() {
        assert!(validate_grade_for_region("A", "SE").is_ok());
        assert!(validate_grade_for_region("F", "SE").is_ok());
        assert!(validate_grade_for_region("a", "SE").is_ok());
        assert!(validate_grade_for_region("-", "SE").is_ok());
        assert!(validate_grade_for_region("G", "SE").is_ok());
        assert!(validate_grade_for_region("VG", "SE").is_ok());
        assert!(validate_grade_for_region("X", "SE").is_err());

        assert!(validate_grade_for_region("6", "NO").is_ok());
        assert!(validate_grade_for_region("1", "NO").is_ok());
        assert!(validate_grade_for_region("B", "NO").is_ok());
        assert!(validate_grade_for_region("X", "NO").is_err());

        assert!(validate_grade_for_region("12", "DK").is_ok());
        assert!(validate_grade_for_region("-3", "DK").is_ok());
        assert!(validate_grade_for_region("02", "DK").is_ok());
        assert!(validate_grade_for_region("2", "DK").is_ok()); // Normalizes to 02
        assert!(validate_grade_for_region("A", "DK").is_err());

        assert!(validate_grade_for_region("10", "FI").is_ok());
        assert!(validate_grade_for_region("4", "FI").is_ok());
        assert!(validate_grade_for_region("5", "FI").is_ok()); // University scale
        assert!(validate_grade_for_region("S", "FI").is_ok());
        assert!(validate_grade_for_region("HYV", "FI").is_ok());
        assert!(validate_grade_for_region("A", "FI").is_err());

        assert!(validate_grade_for_region("A+", "US-CA").is_ok());
        assert!(validate_grade_for_region("B-", "US-FED").is_ok());
        assert!(validate_grade_for_region("F", "US-NY").is_ok());
        assert!(validate_grade_for_region("E", "US-NY").is_ok()); // Some US school districts use E
        assert!(validate_grade_for_region("I", "US-CA").is_ok());  // Incomplete
        assert!(validate_grade_for_region("INC", "US-CA").is_ok());
        assert!(validate_grade_for_region("W", "US-CA").is_ok());  // Withdrawn
        assert!(validate_grade_for_region("A++", "US-CA").is_err());
        assert!(validate_grade_for_region("X", "US-CA").is_err());

        assert!(validate_grade_for_region("A", "EU").is_ok());
        assert!(validate_grade_for_region("FX", "EU").is_ok());
        assert!(validate_grade_for_region("X", "EU").is_err());
    }
}

