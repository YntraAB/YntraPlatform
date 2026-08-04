use crate::SchoolInvoice;
use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::school::auth::{verify_school_permission, verify_school_write_zkp};
use uuid::Uuid;

#[uniffi::export]
pub async fn get_school_invoices(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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

    let list = if role_lower == "student"
        || role_lower == "role-school-student"
        || role_lower == "parent"
        || role_lower == "role-school-parent"
    {
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
        })
        .await?
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
        })
        .await?
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
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
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_billing")?;

    conn.begin_transaction().await?;

    let invoice_opt: Option<(f64, String)> = conn
        .query_row(
            "SELECT amount, status FROM school_invoices WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&invoice_id, &workspace_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .ok();

    if let Some((amount, status)) = invoice_opt {
        if status == "paid" {
            let _ = conn.rollback().await;
            return Err(YntraError::ValidationError(
                "Invoice is already paid".to_string(),
            ));
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
