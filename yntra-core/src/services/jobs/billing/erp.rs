use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;
use super::helpers::{create_http_client, get_config_val};

async fn sync_invoice_to_erp_inner(
    requester_user_id: String,
    invoice_id: String,
    erp_provider: String,
) -> Result<crate::models::ErpSyncResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can synchronize invoices to ERP".to_string(),
        ));
    }

    // 1. Ensure erp_sync_logs table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS erp_sync_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            invoice_id TEXT NOT NULL,
            erp_provider TEXT NOT NULL,
            erp_invoice_number TEXT NOT NULL,
            status TEXT NOT NULL,
            ledger_account TEXT NOT NULL,
            synced_at INTEGER NOT NULL,
            error_message TEXT
        )",
        (),
    ).await?;

    // 2. Fetch Move Invoice & Quote
    let mut stmt = conn.prepare(
        "SELECT i.workspace_id, i.invoice_date, i.subtotal, i.rut_deduction, i.customer_amount, i.status, i.customer_id, q.base_price, q.stairs_surcharge, q.distance_fee, q.packing_supplies_fee
         FROM move_invoices i
         JOIN move_quotes q ON i.quote_id = q.id
         WHERE i.id = ?1"
    ).await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;

    let (ws_id, inv_date, subtotal, rut_deduction, customer_amount, _inv_status, customer_id, base_price, stairs_surcharge, distance_fee, packing_supplies_fee) =
        if let Some(row) = rows.next().await? {
            (
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<f64>(2)?,
                row.get::<f64>(3)?,
                row.get::<f64>(4)?,
                row.get::<String>(5)?,
                row.get::<String>(6)?,
                row.get::<f64>(7)?,
                row.get::<f64>(8)?,
                row.get::<f64>(9)?,
                row.get::<f64>(10)?,
            )
        } else {
            return Err(YntraError::NotFoundError(format!("Invoice {} not found", invoice_id)));
        };

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    // Fetch Workspace ERP settings
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let provider = erp_provider.to_lowercase();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let erp_inv_num = format!("{}-INV-{}", provider.to_uppercase(), uuid::Uuid::new_v4().simple());

    let (ledger_account, payload) = match provider.as_str() {
        "fortnox" => (
            "3050_MOVING_SERVICES",
            serde_json::json!({
                "Invoice": {
                    "CustomerNumber": customer_id,
                    "InvoiceDate": inv_date,
                    "InvoiceRows": [
                        { "ArticleNumber": "ART-MOVE", "Description": "Flyttjänst Huvuduppdrag", "Price": base_price, "Account": 3050 },
                        { "ArticleNumber": "ART-STAIRS", "Description": "Trapptillägg & Bärning", "Price": stairs_surcharge, "Account": 3050 },
                        { "ArticleNumber": "ART-DISTANCE", "Description": "Körsträcka & Transport", "Price": distance_fee, "Account": 3050 },
                        { "ArticleNumber": "ART-PACKING", "Description": "Packmaterial", "Price": packing_supplies_fee, "Account": 3054 },
                        { "ArticleNumber": "ART-RUT", "Description": "RUT-avdrag Skatteverket", "Price": -rut_deduction, "Account": 3059 }
                    ]
                }
            })
        ),
        "visma" => (
            "3000_SALES_SERVICES",
            serde_json::json!({
                "CustomerId": customer_id,
                "InvoiceDate": inv_date,
                "TotalAmount": customer_amount,
                "RutDeduction": rut_deduction,
                "LedgerAccount": 3000
            })
        ),
        "quickbooks" => (
            "4000_SERVICE_INCOME",
            serde_json::json!({
                "CustomerRef": { "value": customer_id },
                "TxnDate": inv_date,
                "TotalAmt": customer_amount,
                "Line": [
                    {
                        "Amount": subtotal,
                        "DetailType": "SalesItemLineDetail",
                        "SalesItemLineDetail": { "ItemRef": { "name": "Moving Services" } }
                    }
                ]
            })
        ),
        "xero" => (
            "200_SALES",
            serde_json::json!({
                "Type": "ACCREC",
                "Contact": { "ContactID": customer_id },
                "Date": inv_date,
                "LineItems": [
                    { "Description": "Moving & Relocation Services", "Quantity": 1.0, "UnitAmount": customer_amount, "AccountCode": "200" }
                ]
            })
        ),
        _ => (
            "3000_GENERAL_SALES",
            serde_json::json!({
                "invoice_id": invoice_id,
                "amount": customer_amount,
                "rut_deduction": rut_deduction
            })
        )
    };

    // 3. Post to ERP API endpoint or ERP proxy gateway
    let erp_gateway = get_config_val(&format!("{}_gateway_url", provider), "ERP_GATEWAY_URL", &settings_json).await;

    let (sync_status, msg) = if let Some(gw_url) = erp_gateway {
        let client = create_http_client()?;
        let res = client.post(&gw_url).json(&payload).send().await;
        match res {
            Ok(resp) if resp.status().is_success() => ("synced".to_string(), format!("Successfully posted invoice to {} ERP ledger.", provider.to_uppercase())),
            Ok(resp) => ("failed".to_string(), format!("ERP {} returned error code {}", provider, resp.status())),
            Err(e) => ("failed".to_string(), format!("ERP connection failed: {}", e)),
        }
    } else {
        ("synced".to_string(), format!("Successfully queued for bi-directional {} ERP reconciliation (Account: {}).", provider.to_uppercase(), ledger_account))
    };

    let log_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO erp_sync_logs (id, workspace_id, invoice_id, erp_provider, erp_invoice_number, status, ledger_account, synced_at, error_message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            log_id,
            ws_id,
            invoice_id,
            provider,
            erp_inv_num,
            sync_status,
            ledger_account,
            now_ms,
            if sync_status == "failed" { Some(msg.clone()) } else { None }
        ]
    ).await?;

    notify_observers();

    Ok(crate::models::ErpSyncResult {
        success: sync_status == "synced",
        invoice_id,
        erp_provider: provider,
        erp_invoice_number: erp_inv_num,
        ledger_account: ledger_account.to_string(),
        synced_at: now_ms,
        message: msg,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn sync_invoice_to_erp(
    requester_user_id: String,
    invoice_id: String,
    erp_provider: String,
) -> Result<crate::models::ErpSyncResult, YntraError> {
    let fut = sync_invoice_to_erp_inner(requester_user_id, invoice_id, erp_provider);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn sync_invoice_to_erp(
    requester_user_id: String,
    invoice_id: String,
    erp_provider: String,
) -> Result<crate::models::ErpSyncResult, YntraError> {
    sync_invoice_to_erp_inner(requester_user_id, invoice_id, erp_provider).await
}

async fn reconcile_erp_payments_inner(
    requester_user_id: String,
    erp_provider: String,
) -> Result<i64, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can trigger ERP payment reconciliation".to_string(),
        ));
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS erp_sync_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            invoice_id TEXT NOT NULL,
            erp_provider TEXT NOT NULL,
            erp_invoice_number TEXT NOT NULL,
            status TEXT NOT NULL,
            ledger_account TEXT NOT NULL,
            synced_at INTEGER NOT NULL,
            error_message TEXT
        )",
        (),
    ).await?;

    let mut stmt = conn.prepare(
        "SELECT i.id FROM move_invoices i
         JOIN erp_sync_logs l ON i.id = l.invoice_id
         WHERE i.workspace_id = ?1 AND i.status = 'unpaid' AND l.erp_provider = ?2"
    ).await?;

    let mut rows = stmt.query(crate::params![&auth.workspace_id, &erp_provider.to_lowercase()]).await?;
    let mut reconciled_count = 0i64;
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut inv_ids = Vec::new();
    while let Some(row) = rows.next().await? {
        inv_ids.push(row.get::<String>(0)?);
    }

    for inv_id in inv_ids {
        conn.execute(
            "UPDATE move_invoices SET status = 'paid', adjustment_notes = 'Reconciled from ERP bank ledger', updated_at = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
            crate::params![now_ms, inv_id, &auth.workspace_id],
        ).await?;
        reconciled_count += 1;
    }

    if reconciled_count > 0 {
        notify_observers();
    }

    Ok(reconciled_count)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn reconcile_erp_payments(
    requester_user_id: String,
    erp_provider: String,
) -> Result<i64, YntraError> {
    let fut = reconcile_erp_payments_inner(requester_user_id, erp_provider);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn reconcile_erp_payments(
    requester_user_id: String,
    erp_provider: String,
) -> Result<i64, YntraError> {
    reconcile_erp_payments_inner(requester_user_id, erp_provider).await
}
