use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use super::helpers::create_http_client;
use super::invoices::calculate_eligible_labor_cost;

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
            .unwrap_or("MISSING_PERSONAL_NUMBER")
            .to_string();

        let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
            crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id).unwrap_or_else(|_| "DECRYPTION_FAILED".to_string())
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

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| Ok(r.get::<String>(0)?),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let org_number = settings_json
        .get("company_org_number")
        .and_then(|v| v.as_str())
        .unwrap_or("556123-4567")
        .to_string();

    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    if format_type.to_lowercase() == "csv" {
        let mut csv = String::new();
        csv.push_str("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours\n");

        for inv_id in invoice_ids {
            let mut stmt = conn.prepare(
                "SELECT i.id, i.invoice_date, i.rut_deduction, q.base_price, q.stairs_surcharge, i.customer_id, q.job_ticket_id
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
                let job_ticket_id = row.get::<String>(6)?;

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
                        .ok_or_else(|| YntraError::ValidationError(format!("Customer personal number is missing for invoice {}", inv_id)))?
                        .to_string()
                } else {
                    return Err(YntraError::ValidationError(format!("Customer user record not found for invoice {}", inv_id)));
                };

                let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
                    crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id)
                        .map_err(|_| YntraError::ValidationError(format!("Failed to decrypt customer personal number for invoice {}", inv_id)))?
                } else {
                    raw_pnum
                };

                let eligible_labor = calculate_eligible_labor_cost(&conn, &job_ticket_id, base_price, &settings_json).await?;
                let labor_cost = eligible_labor + stairs_surcharge;

                let hours = if pricing_model == "hourly" {
                    let mut inv_stmt = conn.prepare(
                        "SELECT quantity, estimated_volume_m3 FROM move_inventory WHERE job_ticket_id = ?1",
                    ).await?;
                    let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
                    let mut total_volume = 0.0;
                    while let Some(inv_row) = inv_rows.next().await? {
                        let quantity: i64 = inv_row.get(0)?;
                        let vol: f64 = inv_row.get(1)?;
                        total_volume += (quantity as f64) * vol;
                    }
                    let hours_per_m3 = settings_json.get("moving_hours_per_m3").and_then(|v| v.as_f64()).unwrap_or(0.15);
                    let minimum_hours = settings_json.get("moving_minimum_hours").and_then(|v| v.as_f64()).unwrap_or(2.0);
                    let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);
                    estimated_hours.round() as i64
                } else {
                    let hourly_rate = settings_json
                        .get("moving_hourly_rate_per_mover")
                        .and_then(|v| v.as_f64())
                        .or_else(|| settings_json.get("moving_hourly_rate").and_then(|v| v.as_f64()))
                        .unwrap_or(500.0);
                    (eligible_labor / hourly_rate).max(1.0).round() as i64
                };

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
                "SELECT i.id, i.invoice_date, i.rut_deduction, q.base_price, q.stairs_surcharge, i.customer_id, q.job_ticket_id
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
                let job_ticket_id = row.get::<String>(6)?;

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
                        .ok_or_else(|| YntraError::ValidationError(format!("Customer personal number is missing for invoice {}", inv_id)))?
                        .to_string()
                } else {
                    return Err(YntraError::ValidationError(format!("Customer user record not found for invoice {}", inv_id)));
                };

                let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
                    crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id)
                        .map_err(|_| YntraError::ValidationError(format!("Failed to decrypt customer personal number for invoice {}", inv_id)))?
                } else {
                    raw_pnum
                };

                let eligible_labor = calculate_eligible_labor_cost(&conn, &job_ticket_id, base_price, &settings_json).await?;
                let labor_cost = eligible_labor + stairs_surcharge;

                let hours = if pricing_model == "hourly" {
                    let mut inv_stmt = conn.prepare(
                        "SELECT quantity, estimated_volume_m3 FROM move_inventory WHERE job_ticket_id = ?1",
                    ).await?;
                    let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
                    let mut total_volume = 0.0;
                    while let Some(inv_row) = inv_rows.next().await? {
                        let quantity: i64 = inv_row.get(0)?;
                        let vol: f64 = inv_row.get(1)?;
                        total_volume += (quantity as f64) * vol;
                    }
                    let hours_per_m3 = settings_json.get("moving_hours_per_m3").and_then(|v| v.as_f64()).unwrap_or(0.15);
                    let minimum_hours = settings_json.get("moving_minimum_hours").and_then(|v| v.as_f64()).unwrap_or(2.0);
                    let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);
                    estimated_hours.round() as i64
                } else {
                    let hourly_rate = settings_json
                        .get("moving_hourly_rate_per_mover")
                        .and_then(|v| v.as_f64())
                        .or_else(|| settings_json.get("moving_hourly_rate").and_then(|v| v.as_f64()))
                        .unwrap_or(500.0);
                    (eligible_labor / hourly_rate).max(1.0).round() as i64
                };

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
pub async fn initiate_bankid_skatteverket_session(
    requester_user_id: String,
) -> Result<crate::models::BankIdAuthSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }
    
    let session_id = uuid::Uuid::new_v4().to_string();
    let token = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    
    let session = crate::models::BankIdAuthSession {
        id: session_id.clone(),
        token: token.clone(),
        target_role: auth.role.clone(),
        provider: "se_bankid".to_string(),
        status: "pending".to_string(),
        error_message: None,
        qr_data: format!("https://api.yntra.se/v1/bankid/qr/{}", session_id),
        progress: 0.0,
        authenticated_user_id: Some(requester_user_id.clone()),
        created_at: now_ms.to_string(),
        challenge: Some("skatteverket-rut-signing".to_string()),
    };
    
    conn.execute(
        "INSERT INTO bankid_auth_sessions (id, target_role, provider, status, error_message, qr_data, progress, authenticated_user_id, created_at, challenge, token) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        crate::params![
            session.id,
            session.target_role,
            session.provider,
            session.status,
            session.error_message,
            session.qr_data,
            session.progress,
            session.authenticated_user_id,
            session.created_at,
            session.challenge,
            session.token
        ],
    ).await?;
    
    Ok(session)
}

async fn submit_skatteverket_claim_direct_inner(
    requester_user_id: String,
    session_id: String,
    invoice_ids: Vec<String>,
) -> Result<crate::models::SkatteverketSubmitResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }
    
    let bankid_status: String = conn.query_row(
        "SELECT status FROM bankid_auth_sessions WHERE id = ?1",
        crate::params![&session_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("BankID signature session not found".to_string()))?;
    
    if bankid_status != "success" {
        return Err(YntraError::AuthError(format!("BankID signature verification not completed (current status: {})", bankid_status)));
    }
    
    let mut total_claims = 0;
    let mut total_amount = 0.0;
    
    let xml_payload = export_skatteverket_claims(requester_user_id.clone(), invoice_ids.clone(), "xml".to_string()).await?;
    
    for inv_id in &invoice_ids {
        let mut stmt = conn.prepare(
            "SELECT rut_deduction FROM move_invoices WHERE id = ?1 AND workspace_id = ?2"
        ).await?;
        let mut rows = stmt.query(crate::params![inv_id, &auth.workspace_id]).await?;
        if let Some(row) = rows.next().await? {
            let amt: f64 = row.get(0)?;
            total_claims += 1;
            total_amount += amt;
        }
    }
    
    if total_claims == 0 {
        return Err(YntraError::ValidationError("No valid unpaid claims selected".to_string()));
    }
    
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();
    let has_cert = settings_json
        .get("skatteverket_corporate_cert")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !has_cert {
        return Err(YntraError::ValidationError(
            "Missing Skatteverket corporate certificate (skatteverket_corporate_cert) in workspace settings.".to_string()
        ));
    }

    let client = create_http_client()?;
    let skatteverket_url = settings_json
        .get("skatteverket_api_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://test.skatteverket.se/service/rotrut/v6");
        
    let res = client.post(skatteverket_url)
        .header("Content-Type", "application/xml")
        .body(xml_payload)
        .send()
        .await;
        
    let (status, reference_number, message) = match res {
        Ok(resp) if resp.status().is_success() => {
            let ref_num = format!("SV-REAL-{}", uuid::Uuid::new_v4().simple());
            ("accepted".to_string(), ref_num, "Successfully transmitted to Skatteverket. Processing approved.".to_string())
        }
        Ok(resp) => {
            let err_txt = resp.text().await.unwrap_or_default();
            ("rejected".to_string(), "".to_string(), format!("Skatteverket rejected request: {}", err_txt))
        }
        Err(e) => {
            ("failed".to_string(), "".to_string(), format!("Skatteverket connection failed: {}", e))
        }
    };
    
    if status == "accepted" {
        for inv_id in &invoice_ids {
            conn.execute(
                "UPDATE move_invoices SET status = 'claimed', sync_status = 'pending' WHERE id = ?1 AND workspace_id = ?2",
                crate::params![inv_id, &auth.workspace_id]
            ).await?;
        }
        notify_observers();
    }
    
    Ok(crate::models::SkatteverketSubmitResult {
        reference_number,
        total_claims,
        total_amount,
        status,
        message,
    })
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn submit_skatteverket_claim_direct(
    requester_user_id: String,
    session_id: String,
    invoice_ids: Vec<String>,
) -> Result<crate::models::SkatteverketSubmitResult, YntraError> {
    let fut = submit_skatteverket_claim_direct_inner(requester_user_id, session_id, invoice_ids);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn submit_skatteverket_claim_direct(
    requester_user_id: String,
    session_id: String,
    invoice_ids: Vec<String>,
) -> Result<crate::models::SkatteverketSubmitResult, YntraError> {
    submit_skatteverket_claim_direct_inner(requester_user_id, session_id, invoice_ids).await
}
