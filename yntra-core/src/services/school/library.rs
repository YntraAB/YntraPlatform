use crate::database;
use crate::observer::notify_observers;
use crate::{LibraryBook, LibraryLendingLog, YntraError};

#[uniffi::export]
pub async fn get_library_books(requester_user_id: String) -> Result<Vec<LibraryBook>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let mut stmt = conn.prepare("SELECT id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status FROM library_books WHERE workspace_id = ?1").await?;
    let list = stmt.query_map(crate::params![auth.workspace_id], |row| {
        Ok(LibraryBook {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            isbn: row.get(4)?,
            copies_available: row.get(5)?,
            total_copies: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn save_library_book(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    title: String,
    author: String,
    isbn: String,
    copies_available: i32,
    total_copies: i32,
) -> Result<LibraryBook, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_library").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage library catalog".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let actual_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let book = LibraryBook {
        id: actual_id,
        workspace_id,
        title,
        author,
        isbn,
        copies_available,
        total_copies,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            &book.id,
            &book.workspace_id,
            &book.title,
            &book.author,
            &book.isbn,
            &book.copies_available,
            &book.total_copies,
            &book.updated_at,
            &book.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(book)
}

#[uniffi::export]
pub async fn get_library_lending_logs(
    requester_user_id: String,
    student_id: Option<String>,
) -> Result<Vec<LibraryLendingLog>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let is_staff = super::check_permission(&conn, &requester_user_id, "can_manage_library").await?;
    
    let list = if let Some(sid) = student_id {
        let student_ws: String = conn.query_row(
            "SELECT workspace_id FROM student_profiles WHERE id = ?1",
            crate::params![&sid],
            |r| r.get(0)
        ).await.map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

        if auth.role != "platform_admin" && auth.workspace_id != student_ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }

        if !is_staff && !super::has_academic_access(&conn, &requester_user_id, &sid).await? {
            return Err(YntraError::AuthError("Access denied to lending logs".to_string()));
        }
        let mut stmt = conn.prepare("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status FROM library_lending_logs WHERE student_id = ?1").await?;
        stmt.query_map(crate::params![&sid], |row| {
            Ok(LibraryLendingLog {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                book_id: row.get(2)?,
                student_id: row.get(3)?,
                checked_out_at: row.get(4)?,
                due_date: row.get(5)?,
                returned_at: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        }).await?
    } else {
        if !is_staff {
            return Err(YntraError::AuthError("Access denied: missing student parameter".to_string()));
        }
        
        let (query, params) = if auth.role == "platform_admin" {
            ("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status FROM library_lending_logs".to_string(), vec![])
        } else {
            ("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status FROM library_lending_logs WHERE workspace_id = ?1".to_string(), vec![auth.workspace_id.clone()])
        };

        let mut stmt = conn.prepare(&query).await?;
        stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
            Ok(LibraryLendingLog {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                book_id: row.get(2)?,
                student_id: row.get(3)?,
                checked_out_at: row.get(4)?,
                due_date: row.get(5)?,
                returned_at: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        }).await?
    };
    Ok(list)
}

#[uniffi::export]
pub async fn checkout_library_book(
    requester_user_id: String,
    workspace_id: String,
    book_id: String,
    student_id: String,
    checked_out_at: String,
    due_date: String,
) -> Result<LibraryLendingLog, YntraError> {
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

    let book_ws: String = conn.query_row(
        "SELECT workspace_id FROM library_books WHERE id = ?1",
        crate::params![&book_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Book not found in library".to_string()))?;

    if book_ws != workspace_id {
        return Err(YntraError::ValidationError("Book does not belong to the specified workspace".to_string()));
    }

    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied: cannot checkout for this student".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let log_id = uuid::Uuid::new_v4().to_string();

    let log = LibraryLendingLog {
        id: log_id.clone(),
        workspace_id: workspace_id.clone(),
        book_id: book_id.clone(),
        student_id: student_id.clone(),
        checked_out_at: checked_out_at.clone(),
        due_date: due_date.clone(),
        returned_at: None,
        status: "active".to_string(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.begin_transaction().await?;

    let res = async {
        // 1. Decrement copy count atomically if copies are available
        let affected = conn.execute(
            "UPDATE library_books SET copies_available = copies_available - 1, updated_at = ?1 WHERE id = ?2 AND copies_available > 0",
            crate::params![&now_ms, &book_id],
        ).await?;
        if affected == 0 {
            return Err(YntraError::DbError("No copies available for checkout".to_string()));
        }

        // 3. Log checkout
        conn.execute(
            "INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            crate::params![
                &log.id,
                &log.workspace_id,
                &log.book_id,
                &log.student_id,
                &log.checked_out_at,
                &log.due_date,
                &log.returned_at,
                &log.status,
                &log.updated_at,
                &log.sync_status,
            ],
        ).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            Ok(log)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn return_library_book(
    requester_user_id: String,
    log_id: String,
    returned_at: String,
) -> Result<LibraryLendingLog, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    
    // 1. Get log
    let mut log = conn.query_row(
        "SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status FROM library_lending_logs WHERE id = ?1",
        crate::params![&log_id],
        |row| {
            Ok(LibraryLendingLog {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                book_id: row.get(2)?,
                student_id: row.get(3)?,
                checked_out_at: row.get(4)?,
                due_date: row.get(5)?,
                returned_at: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        }
    ).await?;

    if !super::has_academic_access(&conn, &requester_user_id, &log.student_id).await? {
        return Err(YntraError::AuthError("Access denied: cannot return book for this student".to_string()));
    }

    if log.status == "returned" {
        return Ok(log);
    }

    log.status = "returned".to_string();
    log.returned_at = Some(returned_at.clone());
    log.updated_at = now_ms;

    conn.begin_transaction().await?;

    let res = async {
        // 2. Update log
        conn.execute(
            "UPDATE library_lending_logs SET status = 'returned', returned_at = ?1, updated_at = ?2 WHERE id = ?3",
            crate::params![&log.returned_at, &now_ms, &log_id],
        ).await?;

        // 3. Increment book copies
        conn.execute(
            "UPDATE library_books SET copies_available = copies_available + 1, updated_at = ?1 WHERE id = ?2",
            crate::params![&now_ms, &log.book_id],
        ).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            Ok(log)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_library_catalog_save_permissions() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        let _ = conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-1'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-1'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-1'", ()).await;
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lib-1', 'Lib WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-lib-admin', 'ws-lib-1', 'admin@lib.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-lib-parent', 'ws-lib-1', 'parent@lib.io', 'parent')", ()).await.unwrap();

        // Admin can save catalog books
        let book = save_library_book(
            "u-lib-admin".to_string(),
            None,
            "ws-lib-1".to_string(),
            "Book Title".to_string(),
            "Author Name".to_string(),
            "978-3-16-148410-0".to_string(),
            3,
            3,
        ).await.unwrap();

        assert_eq!(book.title, "Book Title");

        // Parent cannot save catalog books
        let res_parent = save_library_book(
            "u-lib-parent".to_string(),
            None,
            "ws-lib-1".to_string(),
            "Book Title 2".to_string(),
            "Author Name".to_string(),
            "978-3-16-148410-1".to_string(),
            3,
            3,
        ).await;

        assert!(res_parent.is_err());
        assert!(matches!(res_parent.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-1'", ()).await.unwrap();
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    }

    #[tokio::test]
    async fn test_checkout_library_book_atomic_decrement() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        let _ = conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-lib-2'", ()).await;
        let _ = conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-2'", ()).await;
        let _ = conn.execute("DELETE FROM student_parents WHERE student_id = 'student-lib-2'", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-lib-2'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-2'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-2'", ()).await;
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lib-2', 'Lib WS 2', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-lib-parent-2', 'ws-lib-2', 'parent@lib.io', 'parent')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-lib-2', 'ws-lib-2', NULL, 'Jane', 'Doe', 'Grade 3', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_parents (student_id, parent_user_id) VALUES ('student-lib-2', 'u-lib-parent-2')", ()).await.unwrap();

        // 1. Book with 1 copy available
        conn.execute("INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES ('book-2', 'ws-lib-2', 'Rust', 'Steve', '123-456', 1, 1, 0, 'synced')", ()).await.unwrap();

        // 2. Checkout (Should Succeed)
        let log = checkout_library_book(
            "u-lib-parent-2".to_string(),
            "ws-lib-2".to_string(),
            "book-2".to_string(),
            "student-lib-2".to_string(),
            "2026-07-05".to_string(),
            "2026-07-19".to_string(),
        ).await.unwrap();

        assert_eq!(log.status, "active");

        // Verify copies_available is now 0
        let copies: i32 = conn.query_row("SELECT copies_available FROM library_books WHERE id = 'book-2'", (), |r| r.get(0)).await.unwrap();
        assert_eq!(copies, 0);

        // 3. Checkout again (Should Fail - 0 copies available)
        let res2 = checkout_library_book(
            "u-lib-parent-2".to_string(),
            "ws-lib-2".to_string(),
            "book-2".to_string(),
            "student-lib-2".to_string(),
            "2026-07-05".to_string(),
            "2026-07-19".to_string(),
        ).await;

        assert!(res2.is_err());

        // Cleanup
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-lib-2'", ()).await.unwrap();
        conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-2'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE student_id = 'student-lib-2'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-lib-2'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-2'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-2'", ()).await.unwrap();
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    }

    #[tokio::test]
    async fn test_return_library_book_state_increment() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        let _ = conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-lib-3'", ()).await;
        let _ = conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-3'", ()).await;
        let _ = conn.execute("DELETE FROM student_parents WHERE student_id = 'student-lib-3'", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-lib-3'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-3'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-3'", ()).await;
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lib-3', 'Lib WS 3', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-lib-parent-3', 'ws-lib-3', 'parent@lib.io', 'parent')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-lib-3', 'ws-lib-3', NULL, 'Jane', 'Doe', 'Grade 3', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_parents (student_id, parent_user_id) VALUES ('student-lib-3', 'u-lib-parent-3')", ()).await.unwrap();

        // 1. Setup checkout log and book with 0 copies available
        conn.execute("INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES ('book-3', 'ws-lib-3', 'Rust', 'Steve', '123-456', 0, 1, 0, 'synced')", ()).await.unwrap();
        conn.execute(
            "INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES ('log-3', 'ws-lib-3', 'book-3', 'student-lib-3', '2026-07-05', '2026-07-19', NULL, 'active', 0, 'synced')",
            ()
        ).await.unwrap();

        // 2. Return book (Should Succeed)
        let log = return_library_book(
            "u-lib-parent-3".to_string(),
            "log-3".to_string(),
            "2026-07-06".to_string(),
        ).await.unwrap();

        assert_eq!(log.status, "returned");
        assert_eq!(log.returned_at, Some("2026-07-06".to_string()));

        // Verify copies_available incremented to 1
        let copies: i32 = conn.query_row("SELECT copies_available FROM library_books WHERE id = 'book-3'", (), |r| r.get(0)).await.unwrap();
        assert_eq!(copies, 1);

        // Cleanup
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-lib-3'", ()).await.unwrap();
        conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-lib-3'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE student_id = 'student-lib-3'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-lib-3'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-lib-3'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-lib-3'", ()).await.unwrap();
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    }

    #[tokio::test]
    async fn test_library_lending_logs_workspace_scoping() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        let _ = conn.execute("DELETE FROM library_lending_logs WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('ws-lib-a', 'ws-lib-b')", ()).await;
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lib-a', 'WS A', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-lib-b', 'WS B', '[]', '{}')", ()).await.unwrap();

        // Staff of ws-lib-a
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-lib-staff-a', 'ws-lib-a', 'staffa@lib.io', 'admin')", ()).await.unwrap();

        // Student of ws-lib-b
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-lib-b', 'ws-lib-b', NULL, 'Jane', 'Doe', 'Grade 3', 0)", ()).await.unwrap();

        // Book of ws-lib-b
        conn.execute("INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES ('book-b', 'ws-lib-b', 'Rust Book', 'Author', '978-3-16-148410-0', 1, 1, 0, 'synced')", ()).await.unwrap();

        // Log in ws-lib-b
        conn.execute("INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES ('log-b', 'ws-lib-b', 'book-b', 'student-lib-b', '2026-07-05', '2026-07-19', NULL, 'active', 0, 'synced')", ()).await.unwrap();

        // 1. Query all logs as staff of WS A -> should only see WS A logs (meaning 0 results, and should NOT see WS B log)
        let all_logs = get_library_lending_logs("u-lib-staff-a".to_string(), None).await.unwrap();
        assert_eq!(all_logs.len(), 0);

        // 2. Query WS B student's logs as staff of WS A -> should fail with AuthError
        let student_logs_res = get_library_lending_logs("u-lib-staff-a".to_string(), Some("student-lib-b".to_string())).await;
        assert!(student_logs_res.is_err());
        assert!(matches!(student_logs_res.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-lib-a', 'ws-lib-b')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-lib-a', 'ws-lib-b')", ()).await.unwrap();
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    }
}

