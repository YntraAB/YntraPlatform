use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::school::auth::{verify_school_write_zkp, verify_school_permission, verify_student_access};
use crate::{LibraryBook, LibraryLendingLogInfo};
use uuid::Uuid;

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
