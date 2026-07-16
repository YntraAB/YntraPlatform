use crate::database;
use crate::infra::observer::notify_observers;
use crate::{
    StudentProfile, Assignment, Submission, AttendanceRecord, TermGrade, ReportCard,
    LibraryBook, SchoolInvoice, LibraryLendingLogInfo, HealthRecord, HealthIncident,
    WorkspaceUser, YntraError
};
use uuid::Uuid;

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

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
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
pub async fn save_student_profile(
    requester_user_id: String,
    profile: StudentProfile,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != profile.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != assignment.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != submission.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != record.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != grade.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != report.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != book.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
pub async fn get_school_invoices(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at FROM school_invoices WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
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
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn create_school_invoice(
    requester_user_id: String,
    invoice: SchoolInvoice,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != invoice.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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

    let mut stmt = conn
        .prepare("
            SELECT l.id, b.title, s.first_name || ' ' || s.last_name, l.checked_out_at, l.due_date, l.returned_at, l.status
            FROM library_lending_logs l
            JOIN library_books b ON l.book_id = b.id
            JOIN student_profiles s ON l.student_id = s.id
            WHERE l.workspace_id = ?1
        ")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
            Ok(LibraryLendingLogInfo {
                id: row.get(0)?,
                book_title: row.get(1)?,
                student_name: row.get(2)?,
                checked_out_at: row.get(3)?,
                due_date: row.get(4)?,
                returned_at: row.get(5)?,
                status: row.get(6)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn link_parent_to_student(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    parent_user_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
        crate::params![&student_id, &parent_user_id, &workspace_id, &now_ms]
    ).await?;

    notify_observers();
    Ok(())
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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != record.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != incident.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != slot.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
        save_student_profile("u-school-admin".to_string(), profile.clone()).await.unwrap();

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
        save_library_book("u-school-admin".to_string(), book.clone()).await.unwrap();

        checkout_book("u-school-admin".to_string(), "ws-school-test".to_string(), "bk-1".to_string(), "stud-1".to_string(), "2026-08-01".to_string()).await.unwrap();
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
        create_school_invoice("u-school-admin".to_string(), invoice).await.unwrap();
        record_school_payment("u-school-admin".to_string(), "ws-school-test".to_string(), "inv-s1".to_string(), "Card".to_string()).await.unwrap();

        let invoices = get_school_invoices("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
        assert_eq!(invoices[0].status, "paid");
        assert!(invoices[0].paid_at.is_some());

        // 4. Parent Linking
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-parent-1', 'ws-school-test', 'parent@smith.com', 'parent')", ()).await.unwrap();
        link_parent_to_student("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string(), "u-parent-1".to_string()).await.unwrap();
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
        save_student_health_record("u-school-admin".to_string(), hr).await.unwrap();
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
        save_health_incident("u-school-admin".to_string(), incident).await.unwrap();
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
        save_course("u-school-admin".to_string(), "ws-school-test".to_string(), test_course).await.unwrap();

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
        save_term_grade("u-school-admin".to_string(), tg).await.unwrap();
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
        publish_report_card("u-school-admin".to_string(), rc).await.unwrap();
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
        save_assignment("u-school-admin".to_string(), test_assignment).await.unwrap();

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
        save_submission("u-school-admin".to_string(), sub.clone()).await.unwrap();
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
        save_timetable_slot("u-school-admin".to_string(), slot.clone()).await.unwrap();
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
}
