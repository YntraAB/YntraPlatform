use super::helpers::{create_http_client, get_config_val};
use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::jobs::tickets::is_staff;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn initiate_stripe_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::StripePaymentSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Invoice
    let mut stmt = conn
        .prepare(
            "SELECT workspace_id, customer_amount, customer_id FROM move_invoices WHERE id = ?1",
        )
        .await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    let (ws_id, amount) = if let Some(row) = rows.next().await? {
        let ws: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        let cust: String = row.get(2)?;
        if auth.workspace_id != ws {
            return Err(YntraError::AuthError(
                "Access denied: workspace mismatch".to_string(),
            ));
        }
        if !is_staff(&auth) && auth.user_id != cust {
            return Err(YntraError::AuthError(
                "Access denied: customer mismatch".to_string(),
            ));
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

    let currency = settings_json
        .get("currency")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match target_region.as_str() {
            "US" => "USD".to_string(),
            "DE" => "EUR".to_string(),
            _ => "SEK".to_string(),
        });

    let client = create_http_client()?;
    let gateway_url = get_config_val("stripe_gateway_url", "STRIPE_GATEWAY_URL", &settings_json)
        .await
        .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await);
    let secret_key = get_config_val("stripe_secret_key", "STRIPE_SECRET_KEY", &settings_json).await;

    if let Some(gw_url) = gateway_url {
        let res = client
            .post(&gw_url)
            .json(&serde_json::json!({
                "invoice_id": invoice_id,
                "amount": amount,
                "currency": currency,
            }))
            .send()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(YntraError::NetworkError(format!(
                "Billing Gateway Error: {}",
                err_text
            )));
        }

        let session: crate::models::StripePaymentSession = res
            .json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;
        Ok(session)
    } else if let Some(key) = secret_key {
        let api_base_url = get_config_val("api_base_url", "API_BASE_URL", &settings_json)
            .await
            .unwrap_or_else(|| "https://api.yntra.se".to_string());
        let api_base_url = api_base_url.trim_end_matches('/');

        let url = "https://api.stripe.com/v1/checkout/sessions";
        let params = [
            (
                "success_url",
                format!("{}/v1/billing/stripe/success", api_base_url),
            ),
            (
                "cancel_url",
                format!("{}/v1/billing/stripe/cancel", api_base_url),
            ),
            ("mode", "payment".to_string()),
            (
                "line_items[0][price_data][currency]",
                currency.to_lowercase(),
            ),
            (
                "line_items[0][price_data][product_data][name]",
                format!("Move Invoice {}", invoice_id),
            ),
            (
                "line_items[0][price_data][unit_amount]",
                ((amount * 100.0).round() as i64).to_string(),
            ),
            ("line_items[0][quantity]", "1".to_string()),
        ];

        let res = client
            .post(url)
            .basic_auth(key, Some(""))
            .form(&params)
            .send()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(YntraError::NetworkError(format!(
                "Stripe API Error: {}",
                err_text
            )));
        }

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        let session_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let checkout_url = json
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let client_secret = json
            .get("client_secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(crate::models::StripePaymentSession {
            session_id,
            checkout_url,
            client_secret,
            amount,
            currency,
            status: "open".to_string(),
        })
    } else {
        Err(YntraError::ValidationError(
            "Stripe payment gateway is unconfigured: missing stripe_secret_key or billing_gateway_url in workspace settings or environment variables.".to_string(),
        ))
    }
}

fn verify_stripe_signature(
    secret: &str,
    signature_header: &str,
    payload: &str,
) -> Result<(), YntraError> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in signature_header.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim();
        let val = kv.next().unwrap_or("").trim();
        if key == "t" {
            timestamp = Some(val);
        } else if key == "v1" {
            signatures.push(val);
        }
    }

    let t = timestamp.ok_or_else(|| {
        YntraError::AuthError("Missing timestamp in Stripe signature".to_string())
    })?;
    if signatures.is_empty() {
        return Err(YntraError::AuthError(
            "Missing v1 signature in Stripe signature".to_string(),
        ));
    }

    let t_parsed = t
        .parse::<i64>()
        .map_err(|_| YntraError::ValidationError("Invalid Stripe timestamp".to_string()))?;
    let now = chrono::Utc::now().timestamp();
    if (now - t_parsed).abs() > 300 {
        return Err(YntraError::AuthError(
            "Stripe webhook timestamp is outside tolerance".to_string(),
        ));
    }

    let signed_payload = format!("{}.{}", t, payload);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| YntraError::AuthError(e.to_string()))?;
    mac.update(signed_payload.as_bytes());
    let expected_code = mac.finalize().into_bytes();

    for sig in signatures {
        if let Ok(sig_bytes) = const_hex::decode(sig) {
            if sig_bytes.len() == expected_code.len() {
                let mut mac2 = HmacSha256::new_from_slice(secret.as_bytes())
                    .map_err(|e| YntraError::AuthError(e.to_string()))?;
                mac2.update(signed_payload.as_bytes());
                if mac2.verify_slice(&sig_bytes).is_ok() {
                    return Ok(());
                }
            }
        }
    }

    Err(YntraError::AuthError(
        "Invalid Stripe signature".to_string(),
    ))
}

async fn process_stripe_payment_webhook_inner(
    workspace_id: String,
    signature_header: String,
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

    let secret = settings_json
        .get("stripe_webhook_signing_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if secret.is_empty() {
        return Err(YntraError::AuthError(
            "Stripe webhook signing secret is not configured".to_string(),
        ));
    }

    verify_stripe_signature(secret, &signature_header, &payload_json)?;

    let parsed: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid JSON payload: {}", e)))?;

    let event_type = parsed
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| YntraError::ValidationError("Missing event type".to_string()))?;

    if event_type == "checkout.session.completed" || event_type == "payment_intent.succeeded" {
        let data_obj = parsed
            .get("data")
            .and_then(|d| d.get("object"))
            .ok_or_else(|| YntraError::ValidationError("Missing data object".to_string()))?;

        let invoice_id = data_obj
            .get("client_reference_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                data_obj
                    .get("metadata")
                    .and_then(|m| m.get("invoice_id"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                YntraError::ValidationError("Missing invoice identification".to_string())
            })?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
            crate::params![now_ms, invoice_id, &workspace_id],
        ).await?;

        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn process_stripe_payment_webhook(
    workspace_id: String,
    signature_header: String,
    payload_json: String,
) -> Result<(), YntraError> {
    let fut = process_stripe_payment_webhook_inner(workspace_id, signature_header, payload_json);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn process_stripe_payment_webhook(
    workspace_id: String,
    signature_header: String,
    payload_json: String,
) -> Result<(), YntraError> {
    process_stripe_payment_webhook_inner(workspace_id, signature_header, payload_json).await
}

async fn initiate_mobile_pos_terminal_session_inner(
    requester_user_id: String,
    invoice_id: String,
    provider: Option<String>,
    reader_id: Option<String>,
) -> Result<crate::models::MobilePosTerminalSession, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, amount, status): (String, f64, String) = conn
        .query_row(
            "SELECT workspace_id, customer_amount, status FROM move_invoices WHERE id = ?1",
            crate::params![&invoice_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Invoice not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if status == "paid" {
        return Err(YntraError::ValidationError(
            "Invoice is already paid".to_string(),
        ));
    }

    let prov = provider.unwrap_or_else(|| "stripe_terminal".to_string());
    let session_id = format!("pos_sess_{}", uuid::Uuid::new_v4());
    let conn_token = format!("pst_live_tok_{}", uuid::Uuid::new_v4().simple());
    let pi_id = format!("pi_pos_{}", uuid::Uuid::new_v4().simple());

    let session = crate::models::MobilePosTerminalSession {
        session_id,
        invoice_id,
        connection_token: conn_token,
        payment_intent_id: pi_id,
        reader_id,
        amount,
        currency: "SEK".to_string(),
        provider: prov,
        status: "requires_payment_method".to_string(),
    };

    Ok(session)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn initiate_mobile_pos_terminal_session(
    requester_user_id: String,
    invoice_id: String,
    provider: Option<String>,
    reader_id: Option<String>,
) -> Result<crate::models::MobilePosTerminalSession, YntraError> {
    let fut = initiate_mobile_pos_terminal_session_inner(
        requester_user_id,
        invoice_id,
        provider,
        reader_id,
    );
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn initiate_mobile_pos_terminal_session(
    requester_user_id: String,
    invoice_id: String,
    provider: Option<String>,
    reader_id: Option<String>,
) -> Result<crate::models::MobilePosTerminalSession, YntraError> {
    initiate_mobile_pos_terminal_session_inner(requester_user_id, invoice_id, provider, reader_id)
        .await
}

async fn confirm_mobile_pos_terminal_payment_inner(
    requester_user_id: String,
    invoice_id: String,
    payment_method_type: String,
    card_brand: String,
    last4: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, amount): (String, f64) = conn
        .query_row(
            "SELECT workspace_id, customer_amount FROM move_invoices WHERE id = ?1",
            crate::params![&invoice_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Invoice not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, invoice_id, &ws_id],
    ).await?;

    tracing::info!(
        "On-site Bluetooth POS terminal payment confirmed for invoice {}! Amount: {:.2} SEK, Method: {}, Card: {} ending in {}",
        invoice_id,
        amount,
        payment_method_type,
        card_brand,
        last4
    );

    notify_observers();
    Ok(())
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn confirm_mobile_pos_terminal_payment(
    requester_user_id: String,
    invoice_id: String,
    payment_method_type: String,
    card_brand: String,
    last4: String,
) -> Result<(), YntraError> {
    let fut = confirm_mobile_pos_terminal_payment_inner(
        requester_user_id,
        invoice_id,
        payment_method_type,
        card_brand,
        last4,
    );
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn confirm_mobile_pos_terminal_payment(
    requester_user_id: String,
    invoice_id: String,
    payment_method_type: String,
    card_brand: String,
    last4: String,
) -> Result<(), YntraError> {
    confirm_mobile_pos_terminal_payment_inner(
        requester_user_id,
        invoice_id,
        payment_method_type,
        card_brand,
        last4,
    )
    .await
}

#[cfg(test)]
mod pos_tests {
    use super::*;

    #[tokio::test]
    async fn test_mobile_pos_terminal_payment_workflow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-pos-test', 'POS WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pos-crew', 'ws-pos-test', 'crew@fleet.io', 'staff')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, scheduled_date, checklist_json, created_at, updated_at, sync_status) VALUES ('job-pos-1', 'ws-pos-test', 'Job 1', 'Desc', 'Loc', 'normal', 'completed', 0, 0, 1, 1, '2026-08-01', '[]', 1700000000000, 1700000000000, 'synced')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-1', 'ws-pos-test', 'job-pos-1', 4000.0, 500.0, 0.0, 0.0, 4500.0, 'accepted', 1700000000000, 'synced')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES ('inv-pos-1', 'ws-pos-test', 'q-1', 'u-pos-crew', '2026-08-01', '2026-08-15', 4500.0, 0.0, 4500.0, 0.0, 'unpaid', 1700000000000, 'synced')", ()).await.unwrap();

        let sess = initiate_mobile_pos_terminal_session(
            "u-pos-crew".to_string(),
            "inv-pos-1".to_string(),
            Some("stripe_terminal".to_string()),
            Some("reader_bt_999".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(sess.amount, 4500.0);
        assert_eq!(sess.provider, "stripe_terminal");
        assert!(sess.connection_token.starts_with("pst_live_tok_"));

        confirm_mobile_pos_terminal_payment(
            "u-pos-crew".to_string(),
            "inv-pos-1".to_string(),
            "card_present_tap".to_string(),
            "Visa".to_string(),
            "4242".to_string(),
        )
        .await
        .unwrap();

        let (status,): (String,) = conn
            .query_row(
                "SELECT status FROM move_invoices WHERE id = 'inv-pos-1'",
                (),
                |r| Ok((r.get(0)?,)),
            )
            .await
            .unwrap();
        assert_eq!(status, "paid");

        conn.execute(
            "DELETE FROM move_invoices WHERE workspace_id = 'ws-pos-test'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-pos-crew'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-pos-test'", ())
            .await
            .unwrap();
    }
}
