use crate::database;
use crate::observer::notify_observers;
use crate::{
    FeatureGateCheckResult, PaymentCheckoutSessionResult, PlatformInvoice, WorkspaceSubscription,
    YntraError,
};

/// Helper to resolve monthly seat price based on tier.
pub fn get_tier_seat_price(tier: &str) -> f64 {
    match tier.to_lowercase().as_str() {
        "enterprise" => 75.0,
        "pro" => 35.0,
        "starter" | _ => 15.0,
    }
}

/// Helper to get or insert default subscription for a workspace
async fn ensure_workspace_subscription(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<WorkspaceSubscription, YntraError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, tier, payment_platform, external_subscription_id, external_customer_id, \
             seats_allocated, seats_used, price_per_seat_monthly, currency, billing_cycle, status, \
             current_period_start, current_period_end, cancel_at_period_end, updated_at, sync_status \
             FROM workspace_subscriptions WHERE workspace_id = ?1",
        )
        .await?;

    let existing = stmt
        .query_map(crate::params![workspace_id], |row| {
            Ok(WorkspaceSubscription {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                tier: row.get(2)?,
                payment_platform: row.get(3)?,
                external_subscription_id: row.get(4)?,
                external_customer_id: row.get(5)?,
                seats_allocated: row.get::<i64>(6)? as u32,
                seats_used: row.get::<i64>(7)? as u32,
                price_per_seat_monthly: row.get(8)?,
                currency: row.get(9)?,
                billing_cycle: row.get(10)?,
                status: row.get(11)?,
                current_period_start: row.get(12)?,
                current_period_end: row.get(13)?,
                cancel_at_period_end: row.get::<i64>(14)? != 0,
                updated_at: row.get(15)?,
                sync_status: row.get(16)?,
            })
        })
        .await?
        .pop();

    if let Some(sub) = existing {
        return Ok(sub);
    }

    // Count current active users for seats_used
    let active_users_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE workspace_id = ?1",
            crate::params![workspace_id],
            |r| r.get::<i64>(0),
        )
        .await
        .map(|c| c as u32)
        .unwrap_or(1);

    let sub_id = format!("sub-{}", uuid::Uuid::new_v4());
    let now_ms = crate::infra::time::get_current_time_ms();
    let period_end_ms = now_ms + (30 * 86400 * 1000);
    let default_tier = "starter";
    let default_platform = "stripe";
    let default_price = get_tier_seat_price(default_tier);

    conn.execute(
        "INSERT INTO workspace_subscriptions (\
         id, workspace_id, tier, payment_platform, external_subscription_id, external_customer_id, \
         seats_allocated, seats_used, price_per_seat_monthly, currency, billing_cycle, status, \
         current_period_start, current_period_end, cancel_at_period_end, updated_at, sync_status\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'USD', 'monthly', 'active', ?10, ?11, 0, ?12, 'synced')",
        crate::params![
            &sub_id,
            workspace_id,
            default_tier,
            default_platform,
            &format!("sub_ext_{}", &sub_id[..8]),
            &format!("cus_ext_{}", &sub_id[..8]),
            active_users_count.max(5) as i64,
            active_users_count as i64,
            default_price,
            now_ms,
            period_end_ms,
            now_ms
        ],
    )
    .await?;

    Ok(WorkspaceSubscription {
        id: sub_id,
        workspace_id: workspace_id.to_string(),
        tier: default_tier.to_string(),
        payment_platform: default_platform.to_string(),
        external_subscription_id: Some(format!("sub_ext_{}", &workspace_id[..4])),
        external_customer_id: Some(format!("cus_ext_{}", &workspace_id[..4])),
        seats_allocated: active_users_count.max(5),
        seats_used: active_users_count,
        price_per_seat_monthly: default_price,
        currency: "USD".to_string(),
        billing_cycle: "monthly".to_string(),
        status: "active".to_string(),
        current_period_start: now_ms,
        current_period_end: period_end_ms,
        cancel_at_period_end: false,
        updated_at: now_ms,
        sync_status: "synced".to_string(),
    })
}

#[uniffi::export]
pub async fn get_workspace_subscription(
    requester_user_id: String,
    workspace_id: String,
) -> Result<WorkspaceSubscription, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    ensure_workspace_subscription(&conn, &workspace_id).await
}

#[uniffi::export]
pub async fn update_workspace_subscription(
    requester_user_id: String,
    workspace_id: String,
    tier: String,
    payment_platform: String,
    seats_allocated: u32,
    billing_cycle: String,
) -> Result<WorkspaceSubscription, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "admin" && auth.role != "platform_admin" {
        return Err(YntraError::AuthError(
            "Only workspace administrators can manage billing & subscription tiers.".to_string(),
        ));
    }

    let mut current_sub = ensure_workspace_subscription(&conn, &workspace_id).await?;

    let tier_norm = tier.to_lowercase();
    let platform_norm = payment_platform.to_lowercase();
    let price_per_seat = get_tier_seat_price(&tier_norm);
    let cycle_norm = if billing_cycle.to_lowercase() == "annual" {
        "annual"
    } else {
        "monthly"
    };

    let now_ms = crate::infra::time::get_current_time_ms();
    let new_seats = seats_allocated.max(current_sub.seats_used).max(1);

    conn.execute(
        "UPDATE workspace_subscriptions SET tier = ?1, payment_platform = ?2, seats_allocated = ?3, \
         price_per_seat_monthly = ?4, billing_cycle = ?5, updated_at = ?6, sync_status = 'pending' \
         WHERE workspace_id = ?7",
        crate::params![
            &tier_norm,
            &platform_norm,
            new_seats as i64,
            price_per_seat,
            cycle_norm,
            now_ms,
            &workspace_id
        ],
    )
    .await?;

    current_sub.tier = tier_norm;
    current_sub.payment_platform = platform_norm;
    current_sub.seats_allocated = new_seats;
    current_sub.price_per_seat_monthly = price_per_seat;
    current_sub.billing_cycle = cycle_norm.to_string();
    current_sub.updated_at = now_ms;

    let audit_id = format!("audit-{}", uuid::Uuid::new_v4());
    let _ = conn
        .execute(
            "INSERT INTO audit_logs (id, workspace_id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq) VALUES (?1, ?2, ?3, NULL, ?4, ?5, '0', '0', 0)",
            crate::params![
                &audit_id,
                &workspace_id,
                &requester_user_id,
                &format!(
                    "SUBSCRIPTION_UPDATE: tier={}, platform={}, seats={}",
                    current_sub.tier, current_sub.payment_platform, current_sub.seats_allocated
                ),
                now_ms
            ],
        )
        .await;

    notify_observers();

    Ok(current_sub)
}

#[uniffi::export]
pub async fn create_payment_checkout_session(
    requester_user_id: String,
    workspace_id: String,
    payment_platform: String,
    tier: String,
    seats_allocated: u32,
) -> Result<PaymentCheckoutSessionResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let platform_norm = payment_platform.to_lowercase();
    let tier_norm = tier.to_lowercase();
    let unit_price = get_tier_seat_price(&tier_norm);
    let total_amount = unit_price * (seats_allocated as f64);
    let session_id = format!(
        "cs_{}_{}",
        &platform_norm[..3],
        uuid::Uuid::new_v4().simple()
    );

    let (checkout_url, client_secret) = match platform_norm.as_str() {
        "revenuecat" => (
            format!(
                "https://app.revenuecat.com/subscribe/{}/{}",
                workspace_id, tier_norm
            ),
            Some(format!("rc_sk_{}", uuid::Uuid::new_v4().simple())),
        ),
        "chargebee" => (
            format!(
                "https://yntra.chargebee.com/hosted_pages/plans/{}/checkout",
                tier_norm
            ),
            Some(format!("cb_token_{}", uuid::Uuid::new_v4().simple())),
        ),
        "stripe" | _ => (
            format!("https://checkout.stripe.com/pay/{}#play", session_id),
            Some(format!("pi_secret_{}", uuid::Uuid::new_v4().simple())),
        ),
    };

    Ok(PaymentCheckoutSessionResult {
        session_id,
        payment_platform: platform_norm,
        checkout_url,
        client_secret,
        tier: tier_norm,
        seats_allocated,
        total_amount,
        currency: "USD".to_string(),
    })
}

#[uniffi::export]
pub async fn check_feature_tier_gate(
    requester_user_id: String,
    workspace_id: String,
    feature_key: String,
) -> Result<FeatureGateCheckResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let sub = ensure_workspace_subscription(&conn, &workspace_id).await?;
    let current_tier = sub.tier.to_lowercase();

    let (required_tier, is_allowed) = match feature_key.to_lowercase().as_str() {
        "siths_integration" | "custom_branding" | "unlimited_audit_history" => {
            ("enterprise", current_tier == "enterprise")
        }
        "advanced_analytics" | "rut_export" | "automated_scheduling" | "caldav_sync" => {
            ("pro", current_tier == "pro" || current_tier == "enterprise")
        }
        _ => ("starter", true),
    };

    let reason = if is_allowed {
        format!(
            "Feature '{}' is unlocked on your {} tier.",
            feature_key, current_tier
        )
    } else {
        format!(
            "Feature '{}' requires the {} tier. Your workspace is currently on the {} tier.",
            feature_key, required_tier, current_tier
        )
    };

    let upgrade_url = if !is_allowed {
        Some(format!(
            "yntra://settings/billing/upgrade?required={}",
            required_tier
        ))
    } else {
        None
    };

    Ok(FeatureGateCheckResult {
        allowed: is_allowed,
        required_tier: required_tier.to_string(),
        current_tier,
        reason,
        upgrade_url,
    })
}

#[uniffi::export]
pub async fn generate_automated_invoice(
    requester_user_id: String,
    workspace_id: String,
) -> Result<PlatformInvoice, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "admin" && auth.role != "platform_admin" {
        return Err(YntraError::AuthError(
            "Only workspace administrators can generate automated invoices.".to_string(),
        ));
    }

    let sub = ensure_workspace_subscription(&conn, &workspace_id).await?;

    let inv_seq: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM platform_invoices WHERE workspace_id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0)
        + 1;

    let invoice_id = format!("inv-{}", uuid::Uuid::new_v4());
    let inv_num = format!("INV-YNTRA-{:05}", inv_seq);
    let now_ms = crate::infra::time::get_current_time_ms();
    let period_start = sub.current_period_start;
    let period_end = sub.current_period_end;

    let annual_mult = if sub.billing_cycle == "annual" {
        0.85
    } else {
        1.0
    };
    let amount_due = sub.price_per_seat_monthly * (sub.seats_allocated as f64) * annual_mult;
    let pdf_url = format!("yntra://invoices/download/{}.pdf", invoice_id);

    conn.execute(
        "INSERT INTO platform_invoices (\
         id, workspace_id, subscription_id, invoice_number, payment_platform, amount_due, amount_paid, \
         currency, seat_count, period_start, period_end, status, pdf_download_url, created_at, sync_status\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'paid', ?12, ?13, 'synced')",
        crate::params![
            &invoice_id,
            &workspace_id,
            &sub.id,
            &inv_num,
            &sub.payment_platform,
            amount_due,
            amount_due,
            &sub.currency,
            sub.seats_allocated as i64,
            period_start,
            period_end,
            &pdf_url,
            now_ms
        ],
    )
    .await?;

    let audit_id = format!("audit-{}", uuid::Uuid::new_v4());
    let _ = conn
        .execute(
            "INSERT INTO audit_logs (id, workspace_id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq) VALUES (?1, ?2, ?3, NULL, ?4, ?5, '0', '0', 0)",
            crate::params![
                &audit_id,
                &workspace_id,
                &requester_user_id,
                &format!("INVOICE_GENERATED: number={}, amount=${:.2}", inv_num, amount_due),
                now_ms
            ],
        )
        .await;

    notify_observers();

    Ok(PlatformInvoice {
        id: invoice_id,
        workspace_id,
        subscription_id: sub.id,
        invoice_number: inv_num,
        payment_platform: sub.payment_platform,
        amount_due,
        amount_paid: amount_due,
        currency: sub.currency,
        seat_count: sub.seats_allocated,
        period_start,
        period_end,
        status: "paid".to_string(),
        pdf_download_url: Some(pdf_url),
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn get_platform_invoices(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<PlatformInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, subscription_id, invoice_number, payment_platform, \
             amount_due, amount_paid, currency, seat_count, period_start, period_end, status, \
             pdf_download_url, created_at FROM platform_invoices WHERE workspace_id = ?1 ORDER BY created_at DESC",
        )
        .await?;

    let invoices = stmt
        .query_map(crate::params![&workspace_id], |row| {
            Ok(PlatformInvoice {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                subscription_id: row.get(2)?,
                invoice_number: row.get(3)?,
                payment_platform: row.get(4)?,
                amount_due: row.get(5)?,
                amount_paid: row.get(6)?,
                currency: row.get(7)?,
                seat_count: row.get::<i64>(8)? as u32,
                period_start: row.get(9)?,
                period_end: row.get(10)?,
                status: row.get(11)?,
                pdf_download_url: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .await?;

    Ok(invoices)
}

#[uniffi::export]
pub async fn export_platform_invoice_pdf(
    requester_user_id: String,
    workspace_id: String,
    invoice_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let invoices = get_platform_invoices(requester_user_id, workspace_id.clone()).await?;
    let inv = invoices
        .into_iter()
        .find(|i| i.id == invoice_id)
        .ok_or_else(|| YntraError::NotFoundError(format!("Invoice {} not found", invoice_id)))?;

    let pdf_content = format!(
        "%PDF-1.4 YNTRA PLATFORM AUTOMATED INVOICE\n\
         =========================================\n\
         Invoice Number: {}\n\
         Workspace ID:   {}\n\
         Payment Engine: {}\n\
         Period:         {} to {}\n\
         Allocated Seats:{}\n\
         Total Paid:     {:.2} {}\n\
         Status:         {}\n\
         Date Generated: {}\n\
         =========================================\n",
        inv.invoice_number,
        inv.workspace_id,
        inv.payment_platform.to_uppercase(),
        inv.period_start,
        inv.period_end,
        inv.seat_count,
        inv.amount_paid,
        inv.currency,
        inv.status.to_uppercase(),
        inv.created_at
    );

    Ok(pdf_content)
}

#[uniffi::export]
pub async fn export_platform_invoice_json(
    requester_user_id: String,
    workspace_id: String,
    invoice_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let invoices = get_platform_invoices(requester_user_id, workspace_id).await?;
    let inv = invoices
        .into_iter()
        .find(|i| i.id == invoice_id)
        .ok_or_else(|| YntraError::NotFoundError(format!("Invoice {} not found", invoice_id)))?;

    serde_json::to_string_pretty(&inv).map_err(|e| YntraError::DbError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_billing_subscription_engine_flow() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-bill-{}", uuid::Uuid::new_v4());
        let admin_uid = format!("u-admin-bill-{}", uuid::Uuid::new_v4());

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Billing WS', '{}', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'admin@billing.io', 'admin')",
            crate::params![&admin_uid, &ws_id],
        )
        .await
        .unwrap();

        crate::services::users::ensure_user_role_signature(&conn, &admin_uid, "admin", &ws_id)
            .await
            .unwrap();

        // 1. Get initial subscription (defaults to starter)
        let sub = get_workspace_subscription(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert_eq!(sub.tier, "starter");
        assert_eq!(sub.payment_platform, "stripe");

        // 2. Test feature gate on starter tier
        let gate_siths = check_feature_tier_gate(
            admin_uid.clone(),
            ws_id.clone(),
            "siths_integration".to_string(),
        )
        .await
        .unwrap();
        assert!(!gate_siths.allowed);
        assert_eq!(gate_siths.required_tier, "enterprise");

        // 3. Upgrade subscription to Pro tier with 10 seats via RevenueCat
        let updated = update_workspace_subscription(
            admin_uid.clone(),
            ws_id.clone(),
            "pro".to_string(),
            "revenuecat".to_string(),
            10,
            "annual".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(updated.tier, "pro");
        assert_eq!(updated.payment_platform, "revenuecat");
        assert_eq!(updated.seats_allocated, 10);
        assert_eq!(updated.price_per_seat_monthly, 35.0);

        // 4. Test feature gate on Pro tier
        let gate_analytics = check_feature_tier_gate(
            admin_uid.clone(),
            ws_id.clone(),
            "advanced_analytics".to_string(),
        )
        .await
        .unwrap();
        assert!(gate_analytics.allowed);

        // 5. Create checkout session for Enterprise tier via Chargebee
        let checkout = create_payment_checkout_session(
            admin_uid.clone(),
            ws_id.clone(),
            "chargebee".to_string(),
            "enterprise".to_string(),
            15,
        )
        .await
        .unwrap();
        assert_eq!(checkout.payment_platform, "chargebee");
        assert_eq!(checkout.tier, "enterprise");
        assert_eq!(checkout.total_amount, 75.0 * 15.0);

        // 6. Generate automated invoice
        let inv = generate_automated_invoice(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert_eq!(inv.seat_count, 10);
        assert!(inv.amount_paid > 0.0);
        assert_eq!(inv.status, "paid");

        // 7. Retrieve invoices list
        let inv_list = get_platform_invoices(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert!(!inv_list.is_empty());
        assert_eq!(inv_list[0].id, inv.id);

        // 8. Export invoice PDF & JSON
        let pdf = export_platform_invoice_pdf(admin_uid.clone(), ws_id.clone(), inv.id.clone())
            .await
            .unwrap();
        assert!(pdf.contains("YNTRA PLATFORM AUTOMATED INVOICE"));

        let json = export_platform_invoice_json(admin_uid.clone(), ws_id.clone(), inv.id)
            .await
            .unwrap();
        assert!(json.contains("invoice_number"));
    }
}
