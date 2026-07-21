use crate::database;
use crate::infra::errors::YntraError;
use super::helpers::{get_config_val, create_http_client, base64_encode};
use crate::services::jobs::tickets::is_staff;

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
pub async fn initiate_adyen_payment(
    requester_user_id: String,
    invoice_id: String,
) -> Result<crate::models::AdyenPaymentSession, YntraError> {
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
    let gateway_url = get_config_val("adyen_gateway_url", "ADYEN_GATEWAY_URL", &settings_json).await
        .or(get_config_val("billing_gateway_url", "BILLING_GATEWAY_URL", &settings_json).await);
    let api_key = get_config_val("adyen_api_key", "ADYEN_API_KEY", &settings_json).await;
    let merchant_account = get_config_val("adyen_merchant_account", "ADYEN_MERCHANT_ACCOUNT", &settings_json).await
        .unwrap_or_else(|| "MyMerchantAccount".to_string());

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

        let session: crate::models::AdyenPaymentSession = res.json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;
        Ok(session)
    } else if let Some(key) = api_key {
        let api_base_url = get_config_val("api_base_url", "API_BASE_URL", &settings_json).await
            .unwrap_or_else(|| "https://api.yntra.se".to_string());
        let api_base_url = api_base_url.trim_end_matches('/');

        let url = if let Some(custom_url) = get_config_val("adyen_checkout_url", "ADYEN_CHECKOUT_URL", &settings_json).await {
            custom_url
        } else {
            let adyen_env = get_config_val("adyen_environment", "ADYEN_ENVIRONMENT", &settings_json).await
                .unwrap_or_else(|| "test".to_string());
            if adyen_env.to_lowercase() == "live" || adyen_env.to_lowercase() == "production" {
                if let Some(prefix) = get_config_val("adyen_live_prefix", "ADYEN_LIVE_PREFIX", &settings_json).await {
                    format!("https://{}-checkout-live.adyenpayments.com/checkout/v70/sessions", prefix)
                } else {
                    return Err(YntraError::ValidationError("Adyen live prefix (ADYEN_LIVE_PREFIX) is required for live environment payments.".to_string()));
                }
            } else {
                "https://checkout-test.adyen.com/v70/sessions".to_string()
            }
        };
        let res = client.post(&url)
            .header("x-API-key", key)
            .json(&serde_json::json!({
                "amount": {
                    "currency": currency,
                    "value": (amount * 100.0).round() as i64,
                },
                "reference": invoice_id,
                "merchantAccount": merchant_account,
                "returnUrl": format!("{}/v1/billing/adyen/callback", api_base_url),
            }))
            .send()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(YntraError::NetworkError(format!("Adyen API Error: {}", err_text)));
        }

        let json: serde_json::Value = res.json()
            .await
            .map_err(|e| YntraError::NetworkError(e.to_string()))?;

        let session_id = json.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let session_data = json.get("sessionData").and_then(|v| v.as_str()).unwrap_or_default().to_string();

        Ok(crate::models::AdyenPaymentSession {
            session_id,
            session_data,
            amount,
            currency,
            status: "pending".to_string(),
        })
    } else {
        // Fallback mock
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
}
