#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{SchoolInvoice, SchoolPayment, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_school_invoices(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        if !super::has_academic_access_wasm(&store, &requester_user_id, &student_id) {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
        let list: Vec<SchoolInvoice> = store.school_invoices.iter().filter(|i| i.student_id == student_id).cloned().collect();
        Ok(list)
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_finance").await? {
            return Err(YntraError::AuthError("Access denied: cannot manage invoices".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.school_invoices.retain(|i| i.id != invoice.id);
        store.school_invoices.push(invoice.clone());
    }

    notify_observers();
    Ok(invoice)
}

#[uniffi::export]
pub async fn get_school_payments(
    requester_user_id: String,
    invoice_id: String,
) -> Result<Vec<SchoolPayment>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let invoice = store.school_invoices.iter().find(|i| i.id == invoice_id)
            .ok_or_else(|| YntraError::NotFoundError("Invoice not found".to_string()))?;
        if !super::has_academic_access_wasm(&store, &requester_user_id, &invoice.student_id) {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
        let list: Vec<SchoolPayment> = store.school_payments.iter().filter(|p| p.invoice_id == invoice_id).cloned().collect();
        Ok(list)
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_finance").await? {
            return Err(YntraError::AuthError("Access denied: cannot manage payments".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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
        
        // 2. Update matching invoice to paid
        conn.execute(
            "UPDATE school_invoices SET status = 'paid', paid_at = ?1, updated_at = ?2 WHERE id = ?3",
            crate::params![&payment.paid_at, &now_ms, &payment.invoice_id],
        ).await?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        // 1. Log payment
        store.school_payments.push(payment.clone());
        // 2. Update invoice status
        if let Some(inv) = store.school_invoices.iter_mut().find(|i| i.id == invoice_id) {
            inv.status = "paid".to_string();
            inv.paid_at = Some(paid_at.clone());
            inv.updated_at = now_ms;
        }
    }

    notify_observers();
    Ok(payment)
}
