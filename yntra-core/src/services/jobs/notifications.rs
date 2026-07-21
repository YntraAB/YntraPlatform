use crate::database;
use crate::infra::errors::YntraError;

async fn send_external_notification_inner(
    requester_user_id: String,
    workspace_id: String,
    customer_id: String,
    trigger_type: String,
    custom_minutes: Option<u32>,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Query customer contact information
    let (customer_email, customer_phone, customer_name): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT email, phone, full_name FROM users WHERE id = ?1",
            crate::params![&customer_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Customer not found".to_string()))?;

    // Query workspace settings
    let settings_json: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    
    let settings: serde_json::Value = serde_json::from_str(&settings_json)
        .unwrap_or(serde_json::json!({}));

    // Setup Notification Content Templates
    let (subject, body_email, body_sms) = match trigger_type.as_str() {
        "booking_confirmation" => (
            "Bokningsbekräftelse - Din flytt är bokad!".to_string(),
            format!(
                "Hej {},\n\nVi är glada att meddela att din flytt har bokats! Du hittar all information om din flytt i portalen.\n\nMed vänliga hälsningar,\nYntra Platform Team",
                customer_name.as_deref().unwrap_or("Kund")
            ),
            "Hej! Din flytt är nu bokad. Vi ses på flyttdagen! Mvh Yntra".to_string(),
        ),
        "invoice_created" => (
            "Din faktura är tillgänglig".to_string(),
            format!(
                "Hej {},\n\nDin faktura för flytten har nu skapats och finns tillgänglig i portalen. Vänligen betala innan förfallodagen.\n\nMed vänliga hälsningar,\nYntra Platform Team",
                customer_name.as_deref().unwrap_or("Kund")
            ),
            "Hej! Din flyttfaktura har skapats och finns tillgänglig för betalning i portalen. Mvh Yntra".to_string(),
        ),
        "invoice_paid" => (
            "Tack för din betalning!".to_string(),
            format!(
                "Hej {},\n\nVi har tagit emot din betalning för fakturan. Tack för att du flyttade med oss!\n\nMed vänliga hälsningar,\nYntra Platform Team",
                customer_name.as_deref().unwrap_or("Kund")
            ),
            "Hej! Vi har tagit emot din betalning. Tack för att du flyttade med oss! Mvh Yntra".to_string(),
        ),
        "arrival_reminder" => {
            let mins = custom_minutes.unwrap_or(15);
            (
                "Ditt flytteam är på väg!".to_string(),
                format!(
                    "Hej {},\n\nDitt flytteam är nu på väg och beräknas anlända om cirka {} minuter.\n\nMed vänliga hälsningar,\nYntra Platform Team",
                    customer_name.as_deref().unwrap_or("Kund"),
                    mins
                ),
                format!(
                    "Hej! Ditt flytteam är på väg och anländer om ca {} minuter. Mvh Yntra",
                    mins
                ),
            )
        },
        "job_completion" => (
            "Tack för att du valde oss!".to_string(),
            format!(
                "Hej {},\n\nDin flytt är nu slutförd. Vi hoppas att du är nöjd med vår service! Lämna gärna ett omdöme i portalen.\n\nMed vänliga hälsningar,\nYntra Platform Team",
                customer_name.as_deref().unwrap_or("Kund")
            ),
            "Hej! Din flytt är slutförd. Tack för att du valde oss! Lämna gärna ett omdöme i portalen. Mvh Yntra".to_string(),
        ),
        _ => return Err(YntraError::ValidationError(format!("Unknown trigger type: {}", trigger_type))),
    };

    let twilio_sid = settings.get("twilio_sid").and_then(|v| v.as_str());
    let twilio_token = settings.get("twilio_token").and_then(|v| v.as_str());
    let twilio_from = settings.get("twilio_from_number").and_then(|v| v.as_str()).unwrap_or("+1234567890");

    let sendgrid_key = settings.get("sendgrid_api_key").and_then(|v| v.as_str());
    let sendgrid_from = settings.get("sendgrid_from_email").and_then(|v| v.as_str()).unwrap_or("no-reply@yntra.se");

    let mut sms_sent = false;
    let mut email_sent = false;

    // Send SMS via Twilio API if configured
    if let (Some(sid), Some(token), Some(ref phone)) = (twilio_sid, twilio_token, customer_phone.as_ref()) {
        let client = reqwest::Client::new();
        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", sid);
        let params = [
            ("To", phone.as_str()),
            ("From", twilio_from),
            ("Body", &body_sms),
        ];
        let response = client
            .post(&url)
            .basic_auth(sid, Some(token))
            .form(&params)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    sms_sent = true;
                    tracing::info!("Twilio SMS successfully sent to {}", phone);
                } else {
                    let status = resp.status();
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::error!("Twilio SMS failed with status {}: {}", status, err_text);
                }
            }
            Err(e) => tracing::error!("Twilio SMS request error: {:?}", e),
        }
    } else {
        // Fallback Mock Log
        if let Some(ref phone) = customer_phone {
            tracing::info!(
                "[MOCK TWILIO SMS] To: {}, From: {}, Body: {}",
                phone, twilio_from, body_sms
            );
            sms_sent = true;
        }
    }

    // Send Email via SendGrid API if configured
    if let (Some(key), ref email) = (sendgrid_key, &customer_email) {
        let client = reqwest::Client::new();
        let url = "https://api.sendgrid.com/v3/mail/send";
        
        let payload = serde_json::json!({
            "personalizations": [{
                "to": [{ "email": email }]
            }],
            "from": { "email": sendgrid_from },
            "subject": subject,
            "content": [{
                "type": "text/plain",
                "value": body_email
            }]
        });

        let response = client
            .post(url)
            .bearer_auth(key)
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    email_sent = true;
                    tracing::info!("SendGrid Email successfully sent to {}", email);
                } else {
                    let status = resp.status();
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::error!("SendGrid Email failed with status {}: {}", status, err_text);
                }
            }
            Err(e) => tracing::error!("SendGrid Email request error: {:?}", e),
        }
    } else {
        // Fallback Mock Log
        tracing::info!(
            "[MOCK SENDGRID EMAIL] To: {}, From: {}, Subject: {}, Body: {}",
            customer_email, sendgrid_from, subject, body_email
        );
        email_sent = true;
    }

    Ok(sms_sent || email_sent)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn send_external_notification(
    requester_user_id: String,
    workspace_id: String,
    customer_id: String,
    trigger_type: String,
    custom_minutes: Option<u32>,
) -> Result<bool, YntraError> {
    let fut = send_external_notification_inner(requester_user_id, workspace_id, customer_id, trigger_type, custom_minutes);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn send_external_notification(
    requester_user_id: String,
    workspace_id: String,
    customer_id: String,
    trigger_type: String,
    custom_minutes: Option<u32>,
) -> Result<bool, YntraError> {
    send_external_notification_inner(requester_user_id, workspace_id, customer_id, trigger_type, custom_minutes).await
}
