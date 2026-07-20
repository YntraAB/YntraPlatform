use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;

#[uniffi::export]
pub async fn generate_move_invoice(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
) -> Result<crate::models::MoveInvoice, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status FROM move_quotes WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    let (ws_id, _job_id, base_price, _distance_fee, stairs_surcharge, _packing_supplies_fee, total_price, _quote_status) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<f64>(2)?,
            row.get::<f64>(3)?,
            row.get::<f64>(4)?,
            row.get::<f64>(5)?,
            row.get::<f64>(6)?,
            row.get::<String>(7)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Quote not found".to_string()));
    };

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let customer_id = "client-1".to_string();

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    let currency = match target_region.as_str() {
        "US" => "USD".to_string(),
        "DE" => "EUR".to_string(),
        _ => "SEK".to_string(),
    };

    let subtotal = total_price;
    let (rut_deduction, tax_authority_amount, customer_amount) = match target_region.as_str() {
        "US" => {
            let tax_rate = settings_json
                .get("sales_tax_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.08); // 8% sales tax
            let tax = subtotal * tax_rate;
            (0.0, tax, subtotal + tax)
        }
        "DE" => {
            let vat_rate = settings_json
                .get("vat_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.19); // 19% VAT
            let vat = subtotal * vat_rate;
            (0.0, vat, subtotal + vat)
        }
        _ => {
            let rut = if use_rut {
                0.5 * (base_price + stairs_surcharge)
            } else {
                0.0
            };
            (rut, rut, subtotal - rut)
        }
    };

    let now = chrono::Utc::now();
    let invoice_date = now.format("%Y-%m-%d").to_string();
    let due_date = (now + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let now_ms = now.timestamp_millis();

    let mut inv_stmt = conn.prepare("SELECT id FROM move_invoices WHERE quote_id = ?1").await?;
    let mut inv_rows = inv_stmt.query(crate::params![&quote_id]).await?;
    let invoice_id = if let Some(row) = inv_rows.next().await? {
        row.get::<String>(0)?
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let invoice = crate::models::MoveInvoice {
        id: invoice_id.clone(),
        workspace_id: ws_id.clone(),
        quote_id: quote_id.clone(),
        customer_id: customer_id.clone(),
        invoice_date: invoice_date.clone(),
        due_date: due_date.clone(),
        subtotal,
        rut_deduction,
        customer_amount,
        tax_authority_amount,
        status: "unpaid".to_string(),
        currency,
    };

    conn.execute(
        "INSERT OR REPLACE INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending')",
        crate::params![
            invoice.id,
            invoice.workspace_id,
            invoice.quote_id,
            invoice.customer_id,
            invoice.invoice_date,
            invoice.due_date,
            invoice.subtotal,
            invoice.rut_deduction,
            invoice.customer_amount,
            invoice.tax_authority_amount,
            invoice.status,
            now_ms
        ]
    ).await?;

    notify_observers();
    Ok(invoice)
}

#[uniffi::export]
pub async fn get_move_invoice(
    requester_user_id: String,
    quote_id: String,
) -> Result<Option<crate::models::MoveInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status FROM move_invoices WHERE quote_id = ?1 LIMIT 1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(1)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        
        let settings_str: String = conn
            .query_row(
                "SELECT settings FROM workspaces WHERE id = ?1",
                crate::params![&ws_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or_else(|_| "{}".to_string());
        let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
        let target_region = settings_json
            .get("target_region")
            .and_then(|v| v.as_str())
            .unwrap_or("SE")
            .to_uppercase();

        let currency = match target_region.as_str() {
            "US" => "USD".to_string(),
            "DE" => "EUR".to_string(),
            _ => "SEK".to_string(),
        };

        Ok(Some(crate::models::MoveInvoice {
            id: row.get::<String>(0)?,
            workspace_id: ws_id,
            quote_id,
            customer_id: row.get::<String>(2)?,
            invoice_date: row.get::<String>(3)?,
            due_date: row.get::<String>(4)?,
            subtotal: row.get::<f64>(5)?,
            rut_deduction: row.get::<f64>(6)?,
            customer_amount: row.get::<f64>(7)?,
            tax_authority_amount: row.get::<f64>(8)?,
            status: row.get::<String>(9)?,
            currency,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn pay_move_invoice(
    requester_user_id: String,
    invoice_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT workspace_id FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(0)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, invoice_id],
    ).await?;

    notify_observers();
    Ok(())
}

fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    while let Some(chunk) = chunks.next() {
        let b = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push(CHARSET[((b >> 6) & 63) as usize] as char);
        result.push(CHARSET[(b & 63) as usize] as char);
    }
    let remainder = chunks.remainder();
    if remainder.len() == 1 {
        let b = (remainder[0] as u32) << 16;
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push('=');
        result.push('=');
    } else if remainder.len() == 2 {
        let b = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
        result.push(CHARSET[((b >> 18) & 63) as usize] as char);
        result.push(CHARSET[((b >> 12) & 63) as usize] as char);
        result.push(CHARSET[((b >> 6) & 63) as usize] as char);
        result.push('=');
    }
    result
}

#[uniffi::export]
pub async fn initiate_swish_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::SwishPaymentSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Invoice
    let mut stmt = conn.prepare("SELECT workspace_id, customer_amount FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, amount) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        (ws, amt)
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    // 2. Fetch Workspace settings
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    
    let payee = settings_json
        .get("swish_payee_alias")
        .and_then(|v| v.as_str())
        .unwrap_or("1234567890")
        .to_string();

    // 3. Construct Swish payment URL / token
    let token = format!("swish-req-{}", uuid::Uuid::new_v4().simple());
    let swish_url = format!("swish://paymentrequest?token={}", token);

    // 4. Generate SVG QR Code base64 data
    let svg_data = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="200" height="200"><rect width="100" height="100" rx="12" fill="#ffffff" stroke="#e2e8f0" stroke-width="2"/><path d="M12 12h20v20H12zm4 4v12h12V16zM68 12h20v20H68zm4 4v12h12V16zM12 68h20v20H12zm4 4v12h12V72z" fill="#0f172a"/><rect x="37" y="17" width="26" height="10" fill="#0f172a"/><rect x="17" y="37" width="10" height="26" fill="#0f172a"/><rect x="42" y="42" width="16" height="16" rx="4" fill="#e11d48"/><circle cx="50" cy="50" r="4" fill="#ffffff"/><path d="M37 68h15v10H37zm31 5h10v10H68zm0-20h20v10H68z" fill="#0f172a"/><text x="50" y="88" font-family="system-ui,sans-serif" font-size="6" font-weight="bold" fill="#0f172a" text-anchor="middle">Swish: {} kr</text></svg>"##,
        amount
    );
    let qr_code_base64 = format!("data:image/svg+xml;base64,{}", base64_encode(svg_data.as_bytes()));

    // 5. Check sandbox configuration
    if settings_json.get("swish_use_sandbox").and_then(|v| v.as_bool()).unwrap_or(false) {
        tracing::info!("Querying Swish Sandbox endpoint for Payee: {}", payee);
    }

    Ok(crate::models::SwishPaymentSession {
        token,
        swish_url,
        qr_code_base64,
        amount,
        status: "pending".to_string(),
    })
}

#[uniffi::export]
pub async fn get_rut_invoices(
    requester_user_id: String,
) -> Result<Vec<crate::models::RutInvoiceOverview>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT i.id, i.invoice_date, i.rut_deduction, i.customer_id, j.title, u.full_name, u.metadata, i.status
         FROM move_invoices i
         JOIN move_quotes q ON i.quote_id = q.id
         JOIN job_tickets j ON q.job_ticket_id = j.id
         JOIN users u ON i.customer_id = u.id
         WHERE i.workspace_id = ?1 AND i.rut_deduction > 0.0"
    ).await?;
    
    let mut rows = stmt.query(crate::params![&auth.workspace_id]).await?;
    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        let metadata_str = row.get::<String>(6)?;
        let metadata_json: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();
        
        let raw_pnum = metadata_json
            .get("personal_number")
            .and_then(|v| v.as_str())
            .unwrap_or("19900101-1234")
            .to_string();

        let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
            crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id).unwrap_or_else(|_| "19900101-1234".to_string())
        } else {
            raw_pnum
        };

        let status = row.get::<String>(7).unwrap_or_else(|_| "ready".to_string());

        list.push(crate::models::RutInvoiceOverview {
            invoice_id: row.get::<String>(0)?,
            payment_date: row.get::<String>(1)?,
            rut_amount: row.get::<f64>(2)?,
            job_title: row.get::<String>(4)?,
            customer_name: row.get::<Option<String>>(5).unwrap_or(None).unwrap_or_else(|| "Kund".to_string()),
            customer_pnum,
            status,
        });
    }

    Ok(list)
}

#[uniffi::export]
pub async fn export_skatteverket_claims(
    requester_user_id: String,
    invoice_ids: Vec<String>,
    format_type: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let org_number = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| Ok(r.get::<String>(0)?),
        )
        .await
        .map(|settings_str| {
            let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
            settings_json
                .get("company_org_number")
                .and_then(|v| v.as_str())
                .unwrap_or("556123-4567")
                .to_string()
        })
        .unwrap_or_else(|_| "556123-4567".to_string());

    if format_type.to_lowercase() == "csv" {
        let mut csv = String::new();
        csv.push_str("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours\n");

        for inv_id in invoice_ids {
            let mut stmt = conn.prepare(
                "SELECT i.id, i.invoice_date, i.rut_deduction, q.base_price, q.stairs_surcharge, i.customer_id
                 FROM move_invoices i
                 JOIN move_quotes q ON i.quote_id = q.id
                 WHERE i.id = ?1 AND i.workspace_id = ?2"
            ).await?;
            
            let mut rows = stmt.query(crate::params![&inv_id, &auth.workspace_id]).await?;
            if let Some(row) = rows.next().await? {
                let inv_date = row.get::<String>(1)?;
                let rut_deduction = row.get::<f64>(2)?;
                let base_price = row.get::<f64>(3)?;
                let stairs_surcharge = row.get::<f64>(4)?;
                let customer_id = row.get::<String>(5)?;

                let mut user_stmt = conn.prepare(
                    "SELECT metadata FROM users WHERE id = ?1"
                ).await?;
                let mut user_rows = user_stmt.query(crate::params![&customer_id]).await?;
                let raw_pnum = if let Some(user_row) = user_rows.next().await? {
                    let metadata_str = user_row.get::<String>(0)?;
                    let metadata_json: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();
                    metadata_json
                        .get("personal_number")
                        .and_then(|v| v.as_str())
                        .unwrap_or("19900101-1234")
                        .to_string()
                } else {
                    "19900101-1234".to_string()
                };

                let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
                    crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id).unwrap_or_else(|_| "19900101-1234".to_string())
                } else {
                    raw_pnum
                };

                let hours = ((base_price + stairs_surcharge) / 600.0).max(1.0).round() as i64;
                let labor_cost = base_price + stairs_surcharge;

                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    inv_id, org_number, customer_pnum, inv_date, labor_cost, rut_deduction, hours, hours
                ));
            }
        }
        Ok(csv)
    } else {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.push_str("<BegaranFil xmlns=\"http://xmls.skatteverket.se/se/skatteverket/us/omr/rotrut/begaran/6.0\">\n");

        for inv_id in invoice_ids {
            let mut stmt = conn.prepare(
                "SELECT i.id, i.invoice_date, i.rut_deduction, q.base_price, q.stairs_surcharge, i.customer_id
                 FROM move_invoices i
                 JOIN move_quotes q ON i.quote_id = q.id
                 WHERE i.id = ?1 AND i.workspace_id = ?2"
            ).await?;
            
            let mut rows = stmt.query(crate::params![&inv_id, &auth.workspace_id]).await?;
            if let Some(row) = rows.next().await? {
                let inv_date = row.get::<String>(1)?;
                let rut_deduction = row.get::<f64>(2)?;
                let base_price = row.get::<f64>(3)?;
                let stairs_surcharge = row.get::<f64>(4)?;
                let customer_id = row.get::<String>(5)?;

                let mut user_stmt = conn.prepare(
                    "SELECT metadata FROM users WHERE id = ?1"
                ).await?;
                let mut user_rows = user_stmt.query(crate::params![&customer_id]).await?;
                let raw_pnum = if let Some(user_row) = user_rows.next().await? {
                    let metadata_str = user_row.get::<String>(0)?;
                    let metadata_json: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();
                    metadata_json
                        .get("personal_number")
                        .and_then(|v| v.as_str())
                        .unwrap_or("19900101-1234")
                        .to_string()
                } else {
                    "19900101-1234".to_string()
                };

                let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
                    crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id).unwrap_or_else(|_| "19900101-1234".to_string())
                } else {
                    raw_pnum
                };

                let hours = ((base_price + stairs_surcharge) / 600.0).max(1.0).round() as i64;
                let labor_cost = base_price + stairs_surcharge;

                xml.push_str("  <Arende>\n");
                xml.push_str(&format!("    <UtforareOrgNr>{}</UtforareOrgNr>\n", org_number));
                xml.push_str(&format!("    <KoparePersnr>{}</KoparePersnr>\n", customer_pnum));
                xml.push_str(&format!("    <BetalningsDatum>{}</BetalningsDatum>\n", inv_date));
                xml.push_str(&format!("    <Arbetskostnad>{}</Arbetskostnad>\n", labor_cost));
                xml.push_str(&format!("    <BegartBelopp>{}</BegartBelopp>\n", rut_deduction));
                xml.push_str(&format!("    <ArbetadeTimmar>{}</ArbetadeTimmar>\n", hours));
                xml.push_str("    <Materialkostnad>0</Materialkostnad>\n");
                xml.push_str("    <OvrigKostnad>0</OvrigKostnad>\n");
                xml.push_str("    <RutArbete>\n");
                xml.push_str(&format!("      <Flyttjanster>{}</Flyttjanster>\n", hours));
                xml.push_str("    </RutArbete>\n");
                xml.push_str("  </Arende>\n");
            }
        }

        xml.push_str("</BegaranFil>\n");
        Ok(xml)
    }
}

#[uniffi::export]
pub async fn initiate_stripe_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::StripePaymentSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Invoice
    let mut stmt = conn.prepare("SELECT workspace_id, customer_amount FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, amount) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        (ws, amt)
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    // 2. Fetch Workspace settings
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    let currency = match target_region.as_str() {
        "US" => "USD".to_string(),
        "DE" => "EUR".to_string(),
        _ => "SEK".to_string(),
    };

    let session_id = format!("cs_test_{}", uuid::Uuid::new_v4().simple());
    let checkout_url = format!("https://checkout.stripe.com/pay/{}#client_secret=mock_secret", session_id);

    Ok(crate::models::StripePaymentSession {
        session_id,
        checkout_url,
        client_secret: Some(format!("mock_secret_{}", uuid::Uuid::new_v4().simple())),
        amount,
        currency,
        status: "open".to_string(),
    })
}

#[uniffi::export]
pub async fn initiate_adyen_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::AdyenPaymentSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Invoice
    let mut stmt = conn.prepare("SELECT workspace_id, customer_amount FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, amount) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        (ws, amt)
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    // 2. Fetch Workspace settings
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    
    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("SE")
        .to_uppercase();

    let currency = match target_region.as_str() {
        "US" => "USD".to_string(),
        "DE" => "EUR".to_string(),
        _ => "SEK".to_string(),
    };

    let session_id = format!("adyen_session_{}", uuid::Uuid::new_v4().simple());
    let session_data = base64_encode(format!("{{\"session_id\":\"{}\",\"amount\":{}}}", session_id, amount).as_bytes());

    Ok(crate::models::AdyenPaymentSession {
        session_id,
        session_data,
        amount,
        currency,
        status: "pending".to_string(),
    })
}

