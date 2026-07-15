use crate::database;
use crate::{AcademicOverview, LibraryOverview, FinanceOverview, YntraError};

#[uniffi::export]
pub async fn get_academic_overview(
    requester_user_id: String,
    workspace_id: String,
) -> Result<AcademicOverview, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Query courses count
    let course_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM courses WHERE workspace_id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    // Query timetable slots count
    let slot_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timetable_slots WHERE workspace_id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    Ok(AcademicOverview {
        course_count: course_count as i32,
        slot_count: slot_count as i32,
    })
}

#[uniffi::export]
pub async fn get_library_overview(
    requester_user_id: String,
    workspace_id: String,
) -> Result<LibraryOverview, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Query total books
    let total_books: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_copies), 0) FROM library_books WHERE workspace_id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    // Query available copies
    let available_copies: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(copies_available), 0) FROM library_books WHERE workspace_id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    Ok(LibraryOverview {
        total_books: total_books as i32,
        available_copies: available_copies as i32,
    })
}

#[uniffi::export]
pub async fn get_finance_overview(
    requester_user_id: String,
    workspace_id: String,
) -> Result<FinanceOverview, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Query unpaid invoice count (status is typically 'unpaid' or 'pending')
    let unpaid_invoice_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM school_invoices WHERE workspace_id = ?1 AND status != 'paid'",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    // Query total due amount
    let total_due_amount: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0.0) FROM school_invoices WHERE workspace_id = ?1 AND status != 'paid'",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    Ok(FinanceOverview {
        unpaid_invoice_count: unpaid_invoice_count as i32,
        total_due_amount,
    })
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct LibraryBookInfo {
    pub title: String,
    pub author: String,
    pub copies_available: i32,
    pub total_copies: i32,
}

#[uniffi::export]
pub async fn pay_outstanding_invoices(
    requester_user_id: String,
    workspace_id: String,
) -> Result<(), YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE school_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE workspace_id = ?2 AND status != 'paid'",
        crate::params![now_ms, workspace_id],
    )
    .await?;

    crate::infra::observer::notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn search_library_book(
    requester_user_id: String,
    workspace_id: String,
    query: String,
) -> Result<Option<LibraryBookInfo>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let like_query = format!("%{}%", query);
    let mut stmt = conn
        .prepare(
            "SELECT title, author, copies_available, total_copies FROM library_books WHERE workspace_id = ?1 AND (title LIKE ?2 OR author LIKE ?2) LIMIT 1",
        )
        .await?;

    let mut rows = stmt.query(crate::params![workspace_id, like_query]).await?;
    if let Some(row) = rows.next().await? {
        let title: String = row.get(0)?;
        let author: String = row.get(1)?;
        let copies_available: i64 = row.get(2)?;
        let total_copies: i64 = row.get(3)?;
        Ok(Some(LibraryBookInfo {
            title,
            author,
            copies_available: copies_available as i32,
            total_copies: total_copies as i32,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn get_workspace_courses(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<crate::Course>, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, name, subject, teacher_id, classroom FROM courses WHERE workspace_id = ?1 ORDER BY name ASC",
        )
        .await?;

    let mut rows = stmt.query(crate::params![workspace_id]).await?;
    let mut courses = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let subject: String = row.get(2)?;
        let teacher_id: Option<String> = row.get(3)?;
        let classroom: Option<String> = row.get(4)?;
        courses.push(crate::Course {
            id,
            name,
            subject,
            teacher_id,
            classroom,
        });
    }
    Ok(courses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_dashboard_overviews() {
        let conn = crate::database::acquire_connection().await.unwrap();

        // Set up workspaces, users, courses, slot, books, invoices
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-test-dash', 'Test WS', '[\"academics\",\"library\",\"finance\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, preferences) VALUES ('user-test-dash', 'ws-test-dash', 'test@test.com', 'admin', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, updated_at) VALUES ('course-1', 'ws-test-dash', 'Math', 'Calculus', 12345)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO timetable_slots (id, workspace_id, course_id, day_of_week, start_time, end_time, updated_at) VALUES ('slot-1', 'ws-test-dash', 'course-1', 1, '09:00', '10:00', 12345)", ()).await.unwrap();
        
        // Insert library book
        conn.execute(
            "INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at) VALUES ('book-1', 'ws-test-dash', 'Intro to Algorithms', 'CLRS', '123', 3, 5, 12345)",
            ()
        ).await.unwrap();

        // Insert student profile (required for invoice foreign key constraint)
        conn.execute(
            "INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('student-1', 'ws-test-dash', 'John', 'Doe', 'Grade 10', 12345)",
            ()
        ).await.unwrap();

        // Insert invoice
        conn.execute(
            "INSERT OR REPLACE INTO school_invoices (id, workspace_id, student_id, title, amount, due_date, status, updated_at) VALUES ('invoice-1', 'ws-test-dash', 'student-1', 'Tuition', 1500.0, '2026-08-01', 'unpaid', 12345)",
            ()
        ).await.unwrap();


        let ac_ov = get_academic_overview("user-test-dash".to_string(), "ws-test-dash".to_string()).await.unwrap();
        assert_eq!(ac_ov.course_count, 1);
        assert_eq!(ac_ov.slot_count, 1);

        let lib_ov = get_library_overview("user-test-dash".to_string(), "ws-test-dash".to_string()).await.unwrap();
        assert_eq!(lib_ov.total_books, 5);
        assert_eq!(lib_ov.available_copies, 3);

        let fin_ov = get_finance_overview("user-test-dash".to_string(), "ws-test-dash".to_string()).await.unwrap();
        assert_eq!(fin_ov.unpaid_invoice_count, 1);
        assert_eq!(fin_ov.total_due_amount, 1500.0);

        // Verify search library book (positive case)
        let book_found = search_library_book("user-test-dash".to_string(), "ws-test-dash".to_string(), "Intro".to_string()).await.unwrap();
        assert!(book_found.is_some());
        let book = book_found.unwrap();
        assert_eq!(book.title, "Intro to Algorithms");
        assert_eq!(book.copies_available, 3);

        // Verify search library book (negative case)
        let book_not_found = search_library_book("user-test-dash".to_string(), "ws-test-dash".to_string(), "Nonexistent".to_string()).await.unwrap();
        assert!(book_not_found.is_none());

        // Verify pay outstanding invoices
        pay_outstanding_invoices("user-test-dash".to_string(), "ws-test-dash".to_string()).await.unwrap();

        // Check updated finance overview
        let fin_ov_after = get_finance_overview("user-test-dash".to_string(), "ws-test-dash".to_string()).await.unwrap();
        assert_eq!(fin_ov_after.unpaid_invoice_count, 0);
        assert_eq!(fin_ov_after.total_due_amount, 0.0);

        // Clean up
        conn.execute("DELETE FROM timetable_slots WHERE workspace_id = 'ws-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'user-test-dash'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-test-dash'", ()).await.unwrap();

    }
}
