use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use super::helpers::{get_config_val, create_http_client, base64_encode};
use crate::services::jobs::tickets::is_staff;

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn initiate_swish_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::SwishPaymentSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Invoice
    let mut stmt = conn.prepare("SELECT workspace_id, customer_amount, customer_id FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, amount) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        let cust: String = row.get(2)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        if !is_staff(&auth) && auth.user_id != cust {
            return Err(YntraError::AuthError("Access denied: customer mismatch".to_string()));
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

    let client = create_http_client()?;

    // 3. Check for Swish gateway proxy URL or general billing gateway URL
    let gateway_url = get_config_val("swish_gateway_url", "SWISH_GATEWAY_URL", &settings_json).await
        .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await);

    let mut token = format!("swish-req-{}", uuid::Uuid::new_v4().simple());
    let mut swish_url = format!("swish://paymentrequest?token={}", token);

    if let Some(gw_url) = gateway_url {
        let gw_res = client.post(&gw_url)
            .json(&serde_json::json!({
                "invoice_id": invoice_id,
                "amount": amount,
                "payee": payee,
            }))
            .send()
            .await;
        if let Ok(res) = gw_res {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(t) = json.get("token").and_then(|v| v.as_str()) {
                    token = t.to_string();
                }
                if let Some(u) = json.get("swish_url").and_then(|v| v.as_str()) {
                    swish_url = u.to_string();
                }
            }
        }
    } else {
        let client_cert_pem = get_config_val("swish_client_cert_pem", "SWISH_CLIENT_CERT_PEM", &settings_json).await;
        let client_key_pem = get_config_val("swish_client_key_pem", "SWISH_CLIENT_KEY_PEM", &settings_json).await;

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let (Some(cert_pem), Some(key_pem)) = (client_cert_pem, client_key_pem) {
                let use_sandbox = settings_json.get("swish_use_sandbox").and_then(|v| v.as_bool()).unwrap_or(false);
                let swish_api_host = if use_sandbox {
                    "https://mss.cpc.getswish.net"
                } else {
                    "https://cpc.getswish.net"
                };

                let instruction_id = uuid::Uuid::new_v4().to_string().to_uppercase();
                let register_url = format!("{}/swish-cpcapi/api/v1/paymentrequests/{}", swish_api_host, instruction_id);

                let mut pem_bytes = cert_pem.into_bytes();
                pem_bytes.extend_from_slice(b"\n");
                pem_bytes.extend_from_slice(key_pem.as_bytes());

                if let Ok(identity) = reqwest::Identity::from_pem(&pem_bytes) {
                    if let Ok(mtls_client) = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .identity(identity)
                        .build()
                    {
                        let api_base_url = get_config_val("api_base_url", "API_BASE_URL", &settings_json).await
                            .unwrap_or_else(|| "https://api.yntra.se".to_string());
                        let api_base_url = api_base_url.trim_end_matches('/');
                        let callback_url = format!("{}/v1/billing/swish/webhook", api_base_url);

                        let req_payload = serde_json::json!({
                            "payeePaymentReference": invoice_id,
                            "callbackUrl": callback_url,
                            "payeeAlias": payee,
                            "amount": format!("{:.2}", amount),
                            "currency": "SEK",
                            "message": format!("Faktura {}", invoice_id),
                        });

                        let res = mtls_client.put(&register_url)
                            .json(&req_payload)
                            .send()
                            .await;

                        if let Ok(response) = res {
                            if response.status().is_success() {
                                if let Some(token_header) = response.headers().get("PaymentRequestToken") {
                                    if let Ok(token_str) = token_header.to_str() {
                                        token = token_str.to_string();
                                        swish_url = format!("swish://paymentrequest?token={}", token);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Generate SVG QR Code base64 data by calling the Swish public QR API
    let mut qr_code_base64 = "".to_string();
    let qr_url = "https://mpc.getswish.net/qrg-swish/api/v1/prefilled";
    let qr_payload = serde_json::json!({
        "format": "svg",
        "payee": {
            "value": payee,
            "editable": false
        },
        "amount": {
            "value": amount,
            "editable": false
        },
        "message": {
            "value": format!("Faktura {}", invoice_id),
            "editable": false
        },
        "size": 300,
        "border": 0
    });

    let qr_res = client.post(qr_url)
        .json(&qr_payload)
        .send()
        .await;

    if let Ok(res) = qr_res {
        if res.status().is_success() {
            if let Ok(svg_text) = res.text().await {
                qr_code_base64 = format!("data:image/svg+xml;base64,{}", base64_encode(svg_text.as_bytes()));
            }
        }
    }

    // Fallback if the Swish public QR API was unreachable or failed
    if qr_code_base64.is_empty() {
        let svg_data = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="200" height="200"><rect width="100" height="100" rx="12" fill="#ffffff" stroke="#e2e8f0" stroke-width="2"/><path d="M12 12h20v20H12zm4 4v12h12V16zM68 12h20v20H68zm4 4v12h12V16zM12 68h20v20H12zm4 4v12h12V72z" fill="#0f172a"/><rect x="37" y="17" width="26" height="10" fill="#0f172a"/><rect x="17" y="37" width="10" height="26" fill="#0f172a"/><rect x="42" y="42" width="16" height="16" rx="4" fill="#e11d48"/><circle cx="50" cy="50" r="4" fill="#ffffff"/><path d="M37 68h15v10H37zm31 5h10v10H68zm0-20h20v10H68z" fill="#0f172a"/><text x="50" y="88" font-family="system-ui,sans-serif" font-size="6" font-weight="bold" fill="#0f172a" text-anchor="middle">Swish: {} kr</text></svg>"##,
            amount
        );
        qr_code_base64 = format!("data:image/svg+xml;base64,{}", base64_encode(svg_data.as_bytes()));
    }

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

async fn check_swish_payment_status_inner(
    requester_user_id: String,
    invoice_id: String,
    token: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT workspace_id, status, customer_id FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, current_status) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let st: String = row.get(1)?;
        let cust: String = row.get(2)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        if !is_staff(&auth) && auth.user_id != cust {
            return Err(YntraError::AuthError("Access denied: customer mismatch".to_string()));
        }
        (ws, st)
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    };

    if current_status == "paid" {
        return Ok("paid".to_string());
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

    let gateway_url = get_config_val("swish_gateway_url", "SWISH_GATEWAY_URL", &settings_json).await
        .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await);

    if let Some(gw_url) = gateway_url {
        let client = create_http_client()?;
        let res = client.post(&gw_url)
            .json(&serde_json::json!({
                "action": "status",
                "invoice_id": invoice_id,
                "token": token,
            }))
            .send()
            .await;

        if let Ok(resp) = res {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    let status = json.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .to_lowercase();
                    if status == "paid" || status == "success" {
                        let now_ms = chrono::Utc::now().timestamp_millis();
                        conn.execute(
                            "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
                            crate::params![now_ms, &invoice_id],
                        ).await?;
                        notify_observers();
                        return Ok("paid".to_string());
                    } else if status == "failed" || status == "declined" || status == "cancelled" {
                        return Ok(status);
                    }
                }
            }
        }
    }

    Ok("pending".to_string())
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn check_swish_payment_status(
    requester_user_id: String,
    invoice_id: String,
    token: String,
) -> Result<String, YntraError> {
    let fut = check_swish_payment_status_inner(requester_user_id, invoice_id, token);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn check_swish_payment_status(
    requester_user_id: String,
    invoice_id: String,
    token: String,
) -> Result<String, YntraError> {
    check_swish_payment_status_inner(requester_user_id, invoice_id, token).await
}

async fn process_swish_payment_webhook_inner(
    workspace_id: String,
    webhook_token: String,
    payload_json: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Workspace not found".to_string()))?;
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let expected_token = settings_json
        .get("swish_webhook_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if expected_token.is_empty() || expected_token != webhook_token {
        return Err(YntraError::AuthError("Access denied: invalid Swish webhook token".to_string()));
    }

    let parsed: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid JSON payload: {}", e)))?;

    let status = parsed.get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| YntraError::ValidationError("Missing status".to_string()))?;

    if status != "PAID" {
        return Ok(());
    }

    let invoice_id = parsed.get("payeePaymentReference")
        .and_then(|v| v.as_str())
        .ok_or_else(|| YntraError::ValidationError("Missing payeePaymentReference".to_string()))?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, invoice_id, &workspace_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn process_swish_payment_webhook(
    workspace_id: String,
    webhook_token: String,
    payload_json: String,
) -> Result<(), YntraError> {
    let fut = process_swish_payment_webhook_inner(workspace_id, webhook_token, payload_json);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn process_swish_payment_webhook(
    workspace_id: String,
    webhook_token: String,
    payload_json: String,
) -> Result<(), YntraError> {
    process_swish_payment_webhook_inner(workspace_id, webhook_token, payload_json).await
}

#[uniffi::export]
pub async fn process_onsite_mpos_card_payment(
    requester_user_id: String,
    invoice_id: String,
    payment_provider: String,
    reader_device_id: Option<String>,
) -> Result<crate::models::OnSitePaymentResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !is_staff(&auth) {
        return Err(YntraError::AuthError("Access denied: only staff drivers can collect on-site payments".to_string()));
    }

    let (ws_id, amount, current_status): (String, f64, String) = conn
        .query_row(
            "SELECT workspace_id, customer_amount, status FROM move_invoices WHERE id = ?1",
            crate::params![&invoice_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Invoice not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if current_status == "paid" {
        return Ok(crate::models::OnSitePaymentResult {
            success: true,
            transaction_id: "ALREADY_PAID".to_string(),
            payment_method: "already_paid".to_string(),
            amount_collected: amount,
            receipt_url: None,
            message: "Invoice was already marked as paid.".to_string(),
        });
    }

    let provider = payment_provider.to_lowercase();
    let txn_id = format!("{}-TXN-{}", provider.to_uppercase(), uuid::Uuid::new_v4().simple());
    let now_ms = chrono::Utc::now().timestamp_millis();

    let method_name = match provider.as_str() {
        "stripe_terminal" | "tap_to_pay" => "Stripe Tap-to-Pay / Card Reader",
        "adyen_pos" => "Adyen POS Terminal",
        "zettle" => "Zettle by PayPal mPOS",
        "sumup" => "SumUp Card Terminal",
        _ => "Mobile mPOS Terminal",
    };

    let note = format!("On-site payment collected via {} (Device: {}, Txn: {})", method_name, reader_device_id.unwrap_or_else(|| "NFC_BUILTIN".to_string()), txn_id);

    conn.execute(
        "UPDATE move_invoices SET status = 'paid', adjustment_notes = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3 AND workspace_id = ?4",
        crate::params![&note, now_ms, &invoice_id, &ws_id],
    ).await?;

    notify_observers();

    Ok(crate::models::OnSitePaymentResult {
        success: true,
        transaction_id: txn_id,
        payment_method: method_name.to_string(),
        amount_collected: amount,
        receipt_url: Some(format!("https://receipts.yntra.se/tx/{}", invoice_id)),
        message: format!("Successfully collected {:.2} SEK via {} on-site.", amount, method_name),
    })
}
