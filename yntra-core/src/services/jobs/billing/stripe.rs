use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use super::helpers::{get_config_val, create_http_client};

type HmacSha256 = Hmac<Sha256>;

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
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
    let gateway_url = get_config_val("stripe_gateway_url", "STRIPE_GATEWAY_URL", &settings_json).await
        .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await);
    let secret_key = get_config_val("stripe_secret_key", "STRIPE_SECRET_KEY", &settings_json).await;

    if let Some(gw_url) = gateway_url {
        let res = client.post(&gw_url)
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
            return Err(YntraError::NetworkError(format!("Billing Gateway Error: {}", err_text)));
        }

        let session: crate::models::StripePaymentSession = res.json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;
        Ok(session)
    } else if let Some(key) = secret_key {
        let api_base_url = get_config_val("api_base_url", "API_BASE_URL", &settings_json).await
            .unwrap_or_else(|| "https://api.yntra.se".to_string());
        let api_base_url = api_base_url.trim_end_matches('/');

        let url = "https://api.stripe.com/v1/checkout/sessions";
        let params = [
            ("success_url", format!("{}/v1/billing/stripe/success", api_base_url)),
            ("cancel_url", format!("{}/v1/billing/stripe/cancel", api_base_url)),
            ("mode", "payment".to_string()),
            ("line_items[0][price_data][currency]", currency.to_lowercase()),
            ("line_items[0][price_data][product_data][name]", format!("Move Invoice {}", invoice_id)),
            ("line_items[0][price_data][unit_amount]", ((amount * 100.0).round() as i64).to_string()),
            ("line_items[0][quantity]", "1".to_string()),
        ];
        
        let res = client.post(url)
            .basic_auth(key, Some(""))
            .form(&params)
            .send()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(YntraError::NetworkError(format!("Stripe API Error: {}", err_text)));
        }

        let json: serde_json::Value = res.json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        let session_id = json.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let checkout_url = json.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let client_secret = json.get("client_secret").and_then(|v| v.as_str()).map(|s| s.to_string());

        Ok(crate::models::StripePaymentSession {
            session_id,
            checkout_url,
            client_secret,
            amount,
            currency,
            status: "open".to_string(),
        })
    } else {
        // Fallback mock
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

    let t = timestamp.ok_or_else(|| YntraError::AuthError("Missing timestamp in Stripe signature".to_string()))?;
    if signatures.is_empty() {
        return Err(YntraError::AuthError("Missing v1 signature in Stripe signature".to_string()));
    }

    let t_parsed = t.parse::<i64>().map_err(|_| YntraError::ValidationError("Invalid Stripe timestamp".to_string()))?;
    let now = chrono::Utc::now().timestamp();
    if (now - t_parsed).abs() > 300 {
        return Err(YntraError::AuthError("Stripe webhook timestamp is outside tolerance".to_string()));
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

    Err(YntraError::AuthError("Invalid Stripe signature".to_string()))
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
        return Err(YntraError::AuthError("Stripe webhook signing secret is not configured".to_string()));
    }

    verify_stripe_signature(secret, &signature_header, &payload_json)?;

    let parsed: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid JSON payload: {}", e)))?;

    let event_type = parsed.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| YntraError::ValidationError("Missing event type".to_string()))?;

    if event_type == "checkout.session.completed" || event_type == "payment_intent.succeeded" {
        let data_obj = parsed.get("data")
            .and_then(|d| d.get("object"))
            .ok_or_else(|| YntraError::ValidationError("Missing data object".to_string()))?;

        let invoice_id = data_obj.get("client_reference_id")
            .and_then(|v| v.as_str())
            .or_else(|| data_obj.get("metadata").and_then(|m| m.get("invoice_id")).and_then(|v| v.as_str()))
            .ok_or_else(|| YntraError::ValidationError("Missing invoice identification".to_string()))?;

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
