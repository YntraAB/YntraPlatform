use super::helpers::{create_http_client, get_config_val};
use super::invoices::calculate_eligible_labor_cost;
use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::jobs::tickets::is_staff;
use chrono::Datelike;

#[uniffi::export]
pub async fn get_rut_invoices(
    requester_user_id: String,
) -> Result<Vec<crate::models::RutInvoiceOverview>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can access Skatteverket RUT invoices".to_string(),
        ));
    }

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
        let metadata_json: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or_default();

        let raw_pnum = metadata_json
            .get("personal_number")
            .and_then(|v| v.as_str())
            .unwrap_or("MISSING_PERSONAL_NUMBER")
            .to_string();

        let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
            crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id)
                .unwrap_or_else(|_| "DECRYPTION_FAILED".to_string())
        } else {
            raw_pnum
        };

        let status = row.get::<String>(7).unwrap_or_else(|_| "ready".to_string());

        list.push(crate::models::RutInvoiceOverview {
            invoice_id: row.get::<String>(0)?,
            payment_date: row.get::<String>(1)?,
            rut_amount: row.get::<f64>(2)?,
            job_title: row.get::<String>(4)?,
            customer_name: row
                .get::<Option<String>>(5)
                .unwrap_or(None)
                .unwrap_or_else(|| "Kund".to_string()),
            customer_pnum,
            status,
        });
    }

    Ok(list)
}

#[uniffi::export]
pub async fn export_skatteverket_claims_with_options(
    requester_user_id: String,
    invoice_ids: Vec<String>,
    format_type: String,
    allow_partial: bool,
) -> Result<crate::models::SkatteverketExportManifest, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can export Skatteverket RUT claims".to_string(),
        ));
    }

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

    let total_requested = invoice_ids.len() as i32;
    let mut exported_count = 0;
    let mut omitted_count = 0;
    let mut total_exported_amount = 0.0;
    let mut total_omitted_amount = 0.0;
    let mut omitted_claims = Vec::new();
    let mut omitted_warnings = Vec::new();

    let is_csv = format_type.to_lowercase() == "csv";
    let mut csv_rows = String::new();
    let mut xml = String::new();

    if !is_csv {
        xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.push_str("<BegaranFil xmlns=\"http://xmls.skatteverket.se/se/skatteverket/us/omr/rotrut/begaran/6.0\">\n");
    }

    for inv_id in &invoice_ids {
        let mut stmt = conn.prepare(
            "SELECT i.id, i.invoice_date, i.rut_deduction, q.base_price, q.stairs_surcharge, i.customer_id, q.job_ticket_id, q.distance_fee, q.packing_supplies_fee, i.additional_charges
             FROM move_invoices i
             JOIN move_quotes q ON i.quote_id = q.id
             WHERE i.id = ?1 AND i.workspace_id = ?2"
        ).await?;

        let mut rows = stmt
            .query(crate::params![inv_id, &auth.workspace_id])
            .await?;
        if let Some(row) = rows.next().await? {
            let inv_date = row.get::<String>(1)?;
            let rut_deduction = row.get::<f64>(2)?;
            let base_price = row.get::<f64>(3)?;
            let stairs_surcharge = row.get::<f64>(4)?;
            let customer_id = row.get::<String>(5)?;
            let job_ticket_id = row.get::<String>(6)?;
            let distance_fee = row.get::<f64>(7)?;
            let packing_supplies_fee = row.get::<f64>(8)?;
            let additional_charges = row.get::<Option<f64>>(9)?.unwrap_or(0.0);

            let mut user_stmt = conn
                .prepare("SELECT metadata FROM users WHERE id = ?1")
                .await?;
            let mut user_rows = user_stmt.query(crate::params![&customer_id]).await?;
            let raw_pnum = if let Some(user_row) = user_rows.next().await? {
                let metadata_str = user_row.get::<String>(0)?;
                let metadata_json: serde_json::Value =
                    serde_json::from_str(&metadata_str).unwrap_or_default();
                if let Some(p) = metadata_json
                    .get("personal_number")
                    .and_then(|v| v.as_str())
                {
                    p.to_string()
                } else {
                    let reason = format!(
                        "Customer '{}' has no personal_number in metadata",
                        customer_id
                    );
                    let warn = format!("Invoice '{}' omitted: {}", inv_id, reason);
                    tracing::warn!("{}", warn);
                    omitted_warnings.push(warn);
                    omitted_count += 1;
                    total_omitted_amount += rut_deduction;
                    omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                        invoice_id: inv_id.clone(),
                        customer_id: customer_id.clone(),
                        reason,
                        omitted_rut_amount: rut_deduction,
                    });
                    if !is_csv {
                        xml.push_str(&format!(
                            "  <!-- WARNING: Invoice '{}' omitted -->\n",
                            inv_id
                        ));
                    }
                    continue;
                }
            } else {
                let reason = format!("Customer '{}' not found in workspace", customer_id);
                let warn = format!("Invoice '{}' omitted: {}", inv_id, reason);
                tracing::warn!("{}", warn);
                omitted_warnings.push(warn);
                omitted_count += 1;
                total_omitted_amount += rut_deduction;
                omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                    invoice_id: inv_id.clone(),
                    customer_id: customer_id.clone(),
                    reason,
                    omitted_rut_amount: rut_deduction,
                });
                if !is_csv {
                    xml.push_str(&format!(
                        "  <!-- WARNING: Invoice '{}' omitted -->\n",
                        inv_id
                    ));
                }
                continue;
            };

            let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
                if let Ok(decrypted) =
                    crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id)
                {
                    decrypted
                } else {
                    let reason = format!(
                        "Personal number decryption failed for customer '{}'",
                        customer_id
                    );
                    let warn = format!("Invoice '{}' omitted: {}", inv_id, reason);
                    tracing::warn!("{}", warn);
                    omitted_warnings.push(warn);
                    omitted_count += 1;
                    total_omitted_amount += rut_deduction;
                    omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                        invoice_id: inv_id.clone(),
                        customer_id: customer_id.clone(),
                        reason,
                        omitted_rut_amount: rut_deduction,
                    });
                    if !is_csv {
                        xml.push_str(&format!(
                            "  <!-- WARNING: Invoice '{}' omitted -->\n",
                            inv_id
                        ));
                    }
                    continue;
                }
            } else {
                raw_pnum
            };

            let current_year = chrono::Utc::now().year();
            let normalized_pnum = match crate::services::clients::normalize_swedish_pnum(
                &customer_pnum,
                current_year,
            ) {
                Some(p) => p,
                None => {
                    let reason = format!(
                        "Personal number '{}' failed Luhn checksum validation",
                        customer_pnum
                    );
                    let warn = format!("Invoice '{}' omitted: {}", inv_id, reason);
                    tracing::warn!("{}", warn);
                    omitted_warnings.push(warn);
                    omitted_count += 1;
                    total_omitted_amount += rut_deduction;
                    omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                        invoice_id: inv_id.clone(),
                        customer_id: customer_id.clone(),
                        reason,
                        omitted_rut_amount: rut_deduction,
                    });
                    if !is_csv {
                        xml.push_str(&format!(
                            "  <!-- WARNING: Invoice '{}' omitted -->\n",
                            inv_id
                        ));
                    }
                    continue;
                }
            };

            let eligible_labor =
                calculate_eligible_labor_cost(&conn, &job_ticket_id, base_price, &settings_json)
                    .await?;
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
                drop(inv_rows);
                drop(inv_stmt);

                let hours_per_m3 = crate::services::workspaces::get_setting_f64(
                    &settings_json,
                    "moving_hours_per_m3",
                );
                let minimum_hours = crate::services::workspaces::get_setting_f64(
                    &settings_json,
                    "moving_minimum_hours",
                );
                if total_volume > 0.0 {
                    (total_volume * hours_per_m3).max(minimum_hours).round() as i64
                } else {
                    let hourly_rate = crate::services::workspaces::get_setting_f64(
                        &settings_json,
                        "moving_hourly_rate_per_mover",
                    );
                    (eligible_labor / hourly_rate).max(minimum_hours).round() as i64
                }
            } else {
                let hourly_rate = crate::services::workspaces::get_setting_f64(
                    &settings_json,
                    "moving_hourly_rate_per_mover",
                );
                let minimum_hours = crate::services::workspaces::get_setting_f64(
                    &settings_json,
                    "moving_minimum_hours",
                );
                (eligible_labor / hourly_rate).max(minimum_hours).round() as i64
            };

            if is_csv {
                csv_rows.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    inv_id,
                    org_number,
                    normalized_pnum,
                    inv_date,
                    labor_cost.round() as i64,
                    rut_deduction.round() as i64,
                    hours,
                    hours
                ));
            } else {
                xml.push_str("  <Arende>\n");
                xml.push_str(&format!(
                    "    <UtforareOrgNr>{}</UtforareOrgNr>\n",
                    org_number
                ));
                xml.push_str(&format!(
                    "    <KoparePersnr>{}</KoparePersnr>\n",
                    normalized_pnum
                ));
                xml.push_str(&format!(
                    "    <BetalningsDatum>{}</BetalningsDatum>\n",
                    inv_date
                ));
                xml.push_str(&format!(
                    "    <Arbetskostnad>{}</Arbetskostnad>\n",
                    labor_cost.round() as i64
                ));
                xml.push_str(&format!(
                    "    <BegartBelopp>{}</BegartBelopp>\n",
                    rut_deduction.round() as i64
                ));
                xml.push_str(&format!("    <ArbetadeTimmar>{}</ArbetadeTimmar>\n", hours));
                xml.push_str(&format!(
                    "    <Materialkostnad>{}</Materialkostnad>\n",
                    packing_supplies_fee.round() as i64
                ));
                xml.push_str(&format!(
                    "    <OvrigKostnad>{}</OvrigKostnad>\n",
                    (distance_fee + additional_charges).round() as i64
                ));
                xml.push_str("    <RutArbete>\n");
                xml.push_str(&format!("      <Flyttjanster>{}</Flyttjanster>\n", hours));
                xml.push_str("    </RutArbete>\n");
                xml.push_str("  </Arende>\n");
            }

            exported_count += 1;
            total_exported_amount += rut_deduction;
        } else {
            let reason = format!("Invoice '{}' not found in workspace", inv_id);
            let warn = format!("Invoice '{}' omitted: {}", inv_id, reason);
            tracing::warn!("{}", warn);
            omitted_warnings.push(warn);
            omitted_count += 1;
            omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                invoice_id: inv_id.clone(),
                customer_id: "unknown".to_string(),
                reason,
                omitted_rut_amount: 0.0,
            });
            if !is_csv {
                xml.push_str(&format!(
                    "  <!-- WARNING: Invoice '{}' omitted -->\n",
                    inv_id
                ));
            }
        }
    }

    if !allow_partial && omitted_count > 0 {
        let omitted_ids: Vec<String> = omitted_claims
            .iter()
            .map(|c| c.invoice_id.clone())
            .collect();
        return Err(YntraError::ValidationError(format!(
            "Skatteverket export aborted: {} of {} invoice(s) omitted (invoices: [{}]), risking {:.2} SEK in uncollected RUT tax deductions. Fix customer personal numbers or allow partial export.",
            omitted_count,
            total_requested,
            omitted_ids.join(", "),
            total_omitted_amount
        )));
    }

    if exported_count == 0 {
        return Err(YntraError::ValidationError(
            "No claims in export batch have a valid customer personal number".to_string(),
        ));
    }

    let payload = if is_csv {
        let mut csv = String::new();
        for warn in &omitted_warnings {
            csv.push_str(&format!("# WARNING: {}\n", warn));
        }
        csv.push_str("InvoiceID,OrgNr,KoparePersnr,BetalningsDatum,Arbetskostnad,BegartBelopp,ArbetadeTimmar,FlyttjansterHours\n");
        csv.push_str(&csv_rows);
        csv
    } else {
        xml.push_str("</BegaranFil>\n");
        xml
    };

    Ok(crate::models::SkatteverketExportManifest {
        payload,
        format_type: if is_csv {
            "csv".to_string()
        } else {
            "xml".to_string()
        },
        total_requested,
        exported_count,
        omitted_count,
        total_exported_amount,
        total_omitted_amount,
        omitted_claims,
    })
}

#[uniffi::export]
pub async fn export_skatteverket_claims(
    requester_user_id: String,
    invoice_ids: Vec<String>,
    format_type: String,
) -> Result<String, YntraError> {
    let manifest =
        export_skatteverket_claims_with_options(requester_user_id, invoice_ids, format_type, true)
            .await?;
    Ok(manifest.payload)
}

#[uniffi::export]
pub async fn export_skatteverket_claims_strict(
    requester_user_id: String,
    invoice_ids: Vec<String>,
    format_type: String,
) -> Result<String, YntraError> {
    let manifest =
        export_skatteverket_claims_with_options(requester_user_id, invoice_ids, format_type, false)
            .await?;
    Ok(manifest.payload)
}

#[uniffi::export]
pub async fn export_skatteverket_claims_detailed(
    requester_user_id: String,
    invoice_ids: Vec<String>,
    format_type: String,
) -> Result<crate::models::SkatteverketExportManifest, YntraError> {
    export_skatteverket_claims_with_options(requester_user_id, invoice_ids, format_type, true).await
}

#[uniffi::export]
pub async fn validate_skatteverket_claim_batch(
    requester_user_id: String,
    invoice_ids: Vec<String>,
) -> Result<crate::models::SkatteverketBatchValidationResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can validate Skatteverket claim batches".to_string(),
        ));
    }

    let total_requested = invoice_ids.len() as i32;
    let mut valid_count = 0;
    let mut total_valid_amount = 0.0;
    let mut total_omitted_amount = 0.0;
    let mut omitted_claims = Vec::new();

    let current_year = chrono::Utc::now().year();

    for inv_id in &invoice_ids {
        let inv_info: Option<(String, f64)> = conn
            .query_row(
                "SELECT customer_id, rut_deduction FROM move_invoices WHERE id = ?1 AND workspace_id = ?2",
                crate::params![inv_id, &auth.workspace_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .await
            .ok();

        let (customer_id, rut_deduction) = match inv_info {
            Some((cid, rut)) => (cid, rut),
            None => {
                omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                    invoice_id: inv_id.clone(),
                    customer_id: "unknown".to_string(),
                    reason: format!("Invoice '{}' not found in workspace", inv_id),
                    omitted_rut_amount: 0.0,
                });
                continue;
            }
        };

        let raw_pnum: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![&customer_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(None)
            .and_then(|meta_str: String| {
                serde_json::from_str::<serde_json::Value>(&meta_str)
                    .ok()
                    .and_then(|v| {
                        v.get("personal_number")
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string())
                    })
            });

        let raw_pnum = match raw_pnum {
            Some(p) => p,
            None => {
                total_omitted_amount += rut_deduction;
                omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                    invoice_id: inv_id.clone(),
                    customer_id: customer_id.clone(),
                    reason: format!(
                        "Customer '{}' has no personal_number in metadata",
                        customer_id
                    ),
                    omitted_rut_amount: rut_deduction,
                });
                continue;
            }
        };

        let customer_pnum = if raw_pnum.starts_with("enc:") || raw_pnum.len() > 30 {
            match crate::infra::crypto::decrypt_field(&raw_pnum, &auth.workspace_id) {
                Ok(decrypted) => decrypted,
                Err(_) => {
                    total_omitted_amount += rut_deduction;
                    omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                        invoice_id: inv_id.clone(),
                        customer_id: customer_id.clone(),
                        reason: "Personal number decryption failed".to_string(),
                        omitted_rut_amount: rut_deduction,
                    });
                    continue;
                }
            }
        } else {
            raw_pnum
        };

        if crate::services::clients::normalize_swedish_pnum(&customer_pnum, current_year).is_some()
        {
            valid_count += 1;
            total_valid_amount += rut_deduction;
        } else {
            total_omitted_amount += rut_deduction;
            omitted_claims.push(crate::models::SkatteverketOmittedClaim {
                invoice_id: inv_id.clone(),
                customer_id: customer_id.clone(),
                reason: format!(
                    "Personal number '{}' failed Luhn checksum or format validation",
                    customer_pnum
                ),
                omitted_rut_amount: rut_deduction,
            });
        }
    }

    Ok(crate::models::SkatteverketBatchValidationResult {
        total_requested,
        valid_count,
        omitted_count: omitted_claims.len() as i32,
        total_valid_amount,
        total_omitted_amount,
        omitted_claims,
    })
}

#[uniffi::export]
pub async fn initiate_bankid_skatteverket_session(
    requester_user_id: String,
) -> Result<crate::models::BankIdAuthSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can initiate Skatteverket BankID sessions".to_string(),
        ));
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
    let api_base_url = get_config_val("api_base_url", "API_BASE_URL", &settings_json)
        .await
        .unwrap_or_else(|| "https://api.yntra.se".to_string());
    let api_base_url = api_base_url.trim_end_matches('/');

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
        qr_data: format!("{}/v1/bankid/qr/{}", api_base_url, session_id),
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

pub fn extract_skatteverket_receipt_reference(body_text: &str) -> String {
    let trimmed = body_text.trim();
    if trimmed.is_empty() {
        return format!("SV-REF-{}", uuid::Uuid::new_v4().simple());
    }

    // 1. Check XML tag patterns
    let xml_tags = [
        "Mottagningsreferens",
        "mottagningsreferens",
        "Journalnummer",
        "journalnummer",
        "Referensnummer",
        "referensnummer",
        "Receipt",
        "receipt",
        "BegaranId",
        "begaranId",
    ];

    for tag in xml_tags {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);
        if let Some(start) = trimmed.find(&open_tag) {
            let value_start = start + open_tag.len();
            if let Some(end) = trimmed[value_start..].find(&close_tag) {
                let val = trimmed[value_start..value_start + end].trim();
                if !val.is_empty() {
                    return val.to_string();
                }
            }
        }
    }

    // 2. Check JSON keys
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let json_keys = [
            "mottagningsreferens",
            "Mottagningsreferens",
            "reference_number",
            "referenceNumber",
            "journalnummer",
            "Journalnummer",
            "receipt_id",
            "receiptId",
            "begaran_id",
            "begaranId",
        ];
        for key in json_keys {
            if let Some(val_str) = json_val.get(key).and_then(|v| v.as_str()) {
                let val = val_str.trim();
                if !val.is_empty() {
                    return val.to_string();
                }
            }
            if let Some(val_num) = json_val.get(key).and_then(|v| v.as_i64()) {
                return val_num.to_string();
            }
        }
    }

    // 3. Fallback: plain reference string
    if !trimmed.contains('<') && !trimmed.contains('{') && trimmed.len() <= 64 {
        return trimmed.to_string();
    }

    format!("SV-REF-{}", uuid::Uuid::new_v4().simple())
}

async fn submit_skatteverket_claim_direct_inner(
    requester_user_id: String,
    session_id: String,
    invoice_ids: Vec<String>,
) -> Result<crate::models::SkatteverketSubmitResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can submit Skatteverket claims".to_string(),
        ));
    }

    let bankid_status: String = conn
        .query_row(
            "SELECT status FROM bankid_auth_sessions WHERE id = ?1",
            crate::params![&session_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::AuthError("BankID signature session not found".to_string()))?;

    if bankid_status != "success" {
        return Err(YntraError::AuthError(format!(
            "BankID signature verification not completed (current status: {})",
            bankid_status
        )));
    }

    let manifest = export_skatteverket_claims_with_options(
        requester_user_id.clone(),
        invoice_ids.clone(),
        "xml".to_string(),
        false,
    )
    .await?;
    let xml_payload = manifest.payload;
    let total_claims = manifest.exported_count;
    let total_amount = manifest.total_exported_amount;

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

    let res = client
        .post(skatteverket_url)
        .header("Content-Type", "application/xml")
        .body(xml_payload)
        .send()
        .await;

    let (status, reference_number, message) = match res {
        Ok(resp) if resp.status().is_success() => {
            let body_txt = resp.text().await.unwrap_or_default();
            let ref_num = extract_skatteverket_receipt_reference(&body_txt);
            (
                "accepted".to_string(),
                ref_num,
                "Successfully transmitted to Skatteverket. Processing approved.".to_string(),
            )
        }
        Ok(resp) => {
            let err_txt = resp.text().await.unwrap_or_default();
            (
                "rejected".to_string(),
                "".to_string(),
                format!("Skatteverket rejected request: {}", err_txt),
            )
        }
        Err(e) => (
            "failed".to_string(),
            "".to_string(),
            format!("Skatteverket connection failed: {}", e),
        ),
    };

    if status == "accepted" {
        for inv_id in &invoice_ids {
            conn.execute(
                "UPDATE move_invoices SET status = 'claimed', adjustment_notes = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
                crate::params![&format!("Skatteverket Receipt Ref: {}", reference_number), inv_id, &auth.workspace_id]
            ).await?;
        }
        notify_observers();
    } else if status == "rejected" || status == "failed" {
        for inv_id in &invoice_ids {
            conn.execute(
                "UPDATE move_invoices SET status = 'rut_rejected', adjustment_notes = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
                crate::params![&message, inv_id, &auth.workspace_id]
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
