#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{LibraryBook, LibraryLendingLog, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_library_books() -> Result<Vec<LibraryBook>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let mut stmt = conn.prepare("SELECT id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status FROM library_books").await?;
        let list = stmt.query_map((), |row| {
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.library_books.clone())
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_library").await? {
            return Err(YntraError::AuthError("Access denied: cannot manage library catalog".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("librarian") || requester.role.contains("bibliotekarie") || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.library_books.retain(|b| b.id != book.id);
        store.library_books.push(book.clone());
    }

    notify_observers();
    Ok(book)
}

#[uniffi::export]
pub async fn get_library_lending_logs(
    requester_user_id: String,
    student_id: Option<String>,
) -> Result<Vec<LibraryLendingLog>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let is_staff = super::check_permission(&conn, &requester_user_id, "can_manage_library").await?;
        
        let list = if let Some(sid) = student_id {
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
            let mut stmt = conn.prepare("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status FROM library_lending_logs").await?;
            stmt.query_map((), |row| {
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let is_staff = {
            let requester = store.users.iter().find(|u| u.id == requester_user_id);
            requester.map(|u| u.role == "admin" || u.role == "platform_admin" || u.role.contains("librarian") || u.role.contains("bibliotekarie") || u.role.contains("teacher") || u.role.contains("rektor") || u.role.contains("principal")).unwrap_or(false)
        };

        let list: Vec<LibraryLendingLog> = if let Some(sid) = student_id {
            if !is_staff && !super::has_academic_access_wasm(&store, &requester_user_id, &sid) {
                return Err(YntraError::AuthError("Access denied".to_string()));
            }
            store.library_lending_logs.iter().filter(|l| l.student_id == sid).cloned().collect()
        } else {
            if !is_staff {
                return Err(YntraError::AuthError("Access denied".to_string()));
            }
            store.library_lending_logs.clone()
        };
        Ok(list)
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
            return Err(YntraError::AuthError("Access denied: cannot checkout for this student".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        if !super::has_academic_access_wasm(&store, &requester_user_id, &student_id) {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        
        // 1. Check if copies are available
        let copies: i32 = conn.query_row(
            "SELECT copies_available FROM library_books WHERE id = ?1",
            crate::params![&book_id],
            |r| r.get(0)
        ).await.unwrap_or(0);
        if copies <= 0 {
            return Err(YntraError::DbError("No copies available for checkout".to_string()));
        }

        // 2. Decrement copy count
        conn.execute(
            "UPDATE library_books SET copies_available = copies_available - 1, updated_at = ?1 WHERE id = ?2",
            crate::params![&now_ms, &book_id],
        ).await?;

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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        // 1. Decrement copies
        if let Some(book) = store.library_books.iter_mut().find(|b| b.id == book_id) {
            if book.copies_available <= 0 {
                return Err(YntraError::DbError("No copies available".to_string()));
            }
            book.copies_available -= 1;
            book.updated_at = now_ms;
        } else {
            return Err(YntraError::NotFoundError("Book not found".to_string()));
        }
        
        // 2. Log checkout
        store.library_lending_logs.push(log.clone());
    }

    notify_observers();
    Ok(log)
}

#[uniffi::export]
pub async fn return_library_book(
    requester_user_id: String,
    log_id: String,
    returned_at: String,
) -> Result<LibraryLendingLog, YntraError> {
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        
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

        notify_observers();
        Ok(log)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        let mut book_id = None;
        let mut result_log = None;
        
        // Find log first to check access
        {
            let log_opt = store.library_lending_logs.iter().find(|l| l.id == log_id);
            if let Some(log) = log_opt {
                if !super::has_academic_access_wasm(&store, &requester_user_id, &log.student_id) {
                    return Err(YntraError::AuthError("Access denied".to_string()));
                }
            } else {
                return Err(YntraError::NotFoundError("Log not found".to_string()));
            }
        }

        if let Some(log) = store.library_lending_logs.iter_mut().find(|l| l.id == log_id) {
            if log.status == "returned" {
                return Ok(log.clone());
            }
            log.status = "returned".to_string();
            log.returned_at = Some(returned_at);
            log.updated_at = now_ms;
            book_id = Some(log.book_id.clone());
            result_log = Some(log.clone());
        }

        if let Some(bid) = book_id {
            if let Some(book) = store.library_books.iter_mut().find(|b| b.id == bid) {
                book.copies_available = (book.copies_available + 1).min(book.total_copies);
                book.updated_at = now_ms;
            }
            notify_observers();
            Ok(result_log.unwrap())
        } else {
            Err(YntraError::NotFoundError("Log not found".to_string()))
        }
    }
}
