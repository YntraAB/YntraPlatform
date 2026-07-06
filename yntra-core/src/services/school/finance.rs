use crate::database;
use crate::observer::notify_observers;
use crate::{SchoolInvoice, SchoolPayment, YntraError};

#[uniffi::export]
pub async fn get_school_invoices(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied to invoices".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at, sync_status FROM school_invoices WHERE student_id = ?1").await?;
    let list = stmt.query_map(crate::params![&student_id], |row| {
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
            sync_status: row.get(9)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn save_school_invoice(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    title: String,
    amount: f64,
    due_date: String,
    status: String,
    paid_at: Option<String>,
) -> Result<SchoolInvoice, YntraError> {
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

    if !super::check_permission(&conn, &requester_user_id, "can_manage_finance").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage invoices".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let actual_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let invoice = SchoolInvoice {
        id: actual_id.clone(),
        workspace_id: workspace_id.clone(),
        student_id: student_id.clone(),
        title: title.clone(),
        amount,
        due_date: due_date.clone(),
        status: status.clone(),
        paid_at: paid_at.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO school_invoices (id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            &invoice.id,
            &invoice.workspace_id,
            &invoice.student_id,
            &invoice.title,
            &invoice.amount,
            &invoice.due_date,
            &invoice.status,
            &invoice.paid_at,
            &invoice.updated_at,
            &invoice.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(invoice)
}

#[uniffi::export]
pub async fn get_school_payments(
    requester_user_id: String,
    invoice_id: String,
) -> Result<Vec<SchoolPayment>, YntraError> {
    let conn = database::acquire_connection().await?;
    let student_id: String = conn.query_row(
        "SELECT student_id FROM school_invoices WHERE id = ?1",
        crate::params![&invoice_id],
        |row| row.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Invoice not found".to_string()))?;

    if !super::has_academic_access(&conn, &requester_user_id, &student_id).await? {
        return Err(YntraError::AuthError("Access denied to payments".to_string()));
    }
    let mut stmt = conn.prepare("SELECT id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at, sync_status FROM school_payments WHERE invoice_id = ?1").await?;
    let list = stmt.query_map(crate::params![&invoice_id], |row| {
        Ok(SchoolPayment {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            invoice_id: row.get(2)?,
            amount: row.get(3)?,
            payment_method: row.get(4)?,
            paid_at: row.get(5)?,
            updated_at: row.get(6)?,
            sync_status: row.get(7)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn record_school_payment(
    requester_user_id: String,
    workspace_id: String,
    invoice_id: String,
    amount: f64,
    payment_method: String,
    paid_at: String,
) -> Result<SchoolPayment, YntraError> {
    if amount <= 0.0 {
        return Err(YntraError::ValidationError("Payment amount must be greater than zero".to_string()));
    }

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let (invoice_ws, inv_amount, inv_status): (String, f64, String) = conn.query_row(
        "SELECT workspace_id, amount, status FROM school_invoices WHERE id = ?1",
        crate::params![&invoice_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).await.map_err(|_| YntraError::NotFoundError("Invoice not found".to_string()))?;

    if invoice_ws != workspace_id {
        return Err(YntraError::ValidationError("Invoice does not belong to the specified workspace".to_string()));
    }

    if inv_status == "paid" {
        return Err(YntraError::ValidationError("Invoice is already fully paid".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_finance").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage payments".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let payment_id = uuid::Uuid::new_v4().to_string();

    let payment = SchoolPayment {
        id: payment_id.clone(),
        workspace_id: workspace_id.clone(),
        invoice_id: invoice_id.clone(),
        amount,
        payment_method: payment_method.clone(),
        paid_at: paid_at.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.begin_transaction().await?;

    let res = async {
        // Fetch previous payments sum
        let prev_payments_sum: f64 = conn.query_row(
            "SELECT IFNULL(SUM(amount), 0.0) FROM school_payments WHERE invoice_id = ?1",
            crate::params![&payment.invoice_id],
            |row| row.get(0)
        ).await.unwrap_or(0.0);

        let total_paid = prev_payments_sum + amount;
        let new_status = if total_paid >= inv_amount { "paid" } else { "partially_paid" };

        // 1. Log payment
        conn.execute(
            "INSERT INTO school_payments (id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            crate::params![
                &payment.id,
                &payment.workspace_id,
                &payment.invoice_id,
                &payment.amount,
                &payment.payment_method,
                &payment.paid_at,
                &payment.updated_at,
                &payment.sync_status,
            ],
        ).await?;
        
        // 2. Update matching invoice status
        conn.execute(
            "UPDATE school_invoices SET status = ?1, paid_at = ?2, updated_at = ?3 WHERE id = ?4",
            crate::params![new_status, &payment.paid_at, &now_ms, &payment.invoice_id],
        ).await?;
        Ok::<(), YntraError>(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            Ok(payment)
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
    async fn test_save_school_invoice_and_scoping() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-fin-1'", ()).await;
        let _ = conn.execute("DELETE FROM student_parents WHERE student_id = 'student-fin-1'", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-fin-1'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-1'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-1'", ()).await;

        // Setup workspace & users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-fin-1', 'Fin WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fin-admin', 'ws-fin-1', 'admin@fin.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fin-parent', 'ws-fin-1', 'parent@fin.io', 'parent')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fin-stranger', 'ws-fin-1', 'stranger@fin.io', 'parent')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-fin-1', 'ws-fin-1', NULL, 'Billy', 'Kid', 'Grade 2', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_parents (student_id, parent_user_id) VALUES ('student-fin-1', 'u-fin-parent')", ()).await.unwrap();

        // 1. Save invoice (Should Succeed as Admin)
        let inv = save_school_invoice(
            "u-fin-admin".to_string(),
            None,
            "ws-fin-1".to_string(),
            "student-fin-1".to_string(),
            "Tuition fee".to_string(),
            1200.50,
            "2026-07-31".to_string(),
            "unpaid".to_string(),
            None,
        ).await.unwrap();

        assert_eq!(inv.title, "Tuition fee");
        assert_eq!(inv.amount, 1200.50);
        assert_eq!(inv.status, "unpaid");

        // 2. Fetch invoice as parent (academic access)
        let list = get_school_invoices("u-fin-parent".to_string(), "student-fin-1".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, inv.id);

        // 3. Fetch invoice as stranger (should fail)
        let res_stranger = get_school_invoices("u-fin-stranger".to_string(), "student-fin-1".to_string()).await;
        assert!(res_stranger.is_err());
        assert!(matches!(res_stranger.unwrap_err(), YntraError::AuthError(_)));

        // Cleanup
        conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-fin-1'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE student_id = 'student-fin-1'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-fin-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-1'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_record_school_payment_transaction_success() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("DELETE FROM school_payments WHERE workspace_id = 'ws-fin-2'", ()).await;
        let _ = conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-fin-2'", ()).await;
        let _ = conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-fin-2'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-2'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-2'", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-fin-2', 'Fin WS 2', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fin-admin-2', 'ws-fin-2', 'admin@fin.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-fin-2', 'ws-fin-2', NULL, 'Billy', 'Kid', 'Grade 2', 0)", ()).await.unwrap();

        // 1. Setup mock invoice
        conn.execute(
            "INSERT OR REPLACE INTO school_invoices (id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at, sync_status) VALUES ('inv-2', 'ws-fin-2', 'student-fin-2', 'Fee', 100.0, '2026-07-31', 'unpaid', NULL, 0, 'synced')",
            ()
        ).await.unwrap();

        // 2. Record payment (Should succeed)
        let pay = record_school_payment(
            "u-fin-admin-2".to_string(),
            "ws-fin-2".to_string(),
            "inv-2".to_string(),
            100.0,
            "Card".to_string(),
            "2026-07-05".to_string(),
        ).await.unwrap();

        assert_eq!(pay.amount, 100.0);
        assert_eq!(pay.payment_method, "Card");

        // Verify status in DB is paid
        let invoice_status: String = conn.query_row(
            "SELECT status FROM school_invoices WHERE id = 'inv-2'",
            (),
            |r| r.get(0)
        ).await.unwrap();
        assert_eq!(invoice_status, "paid");

        // Verify payment is in DB
        let payment_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM school_payments WHERE invoice_id = 'inv-2'",
            (),
            |r| r.get(0)
        ).await.unwrap();
        assert_eq!(payment_exists, 1);

        // Cleanup
        conn.execute("DELETE FROM school_payments WHERE workspace_id = 'ws-fin-2'", ()).await.unwrap();
        conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-fin-2'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-fin-2'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-2'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-2'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_record_school_payment_transaction_failure() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-3'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-3'", ()).await;

        // Enable foreign key checking for this test connection
        conn.execute("PRAGMA foreign_keys = ON;", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-fin-3', 'Fin WS 3', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-fin-admin-3', 'ws-fin-3', 'admin@fin.io', 'admin')", ()).await.unwrap();

        // Record payment with non-existent invoice ID (inv-nonexistent)
        // This should fail because of the foreign key constraint on invoice_id -> school_invoices(id)
        let res = record_school_payment(
            "u-fin-admin-3".to_string(),
            "ws-fin-3".to_string(),
            "inv-nonexistent".to_string(),
            100.0,
            "Card".to_string(),
            "2026-07-05".to_string(),
        ).await;

        assert!(res.is_err());

        // Disable foreign key checking again just to restore defaults
        conn.execute("PRAGMA foreign_keys = OFF;", ()).await.unwrap();

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-fin-3'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-fin-3'", ()).await.unwrap();
    }
}

