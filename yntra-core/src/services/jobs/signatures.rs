use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::MoveSignature;
use sha2::{Sha256, Digest};
use uuid::Uuid;

pub const DEFAULT_BOHAG_TERMS: &str = "Allkort & Bohag 2010 / Bohag 2020 Allmänna Bestämmelser för Bohagsettlement & Bohagsflyttning (Sveriges Åkeriföretag). Ansvarighet och försäkring i enlighet med Konsumentverket & Transportavtalet.";

pub fn get_regional_legal_terms(region: &str) -> &'static str {
    match region.to_uppercase().as_str() {
        "US" => "US DOT / FMCSA Carmack Amendment (49 U.S.C. § 14706) & STB Released Value Liability Terms. Bill of Lading contract & Valuation.",
        "UK" => "British Association of Removers (BAR) Model Terms & Conditions for Removal & Storage.",
        "EU" => "EU Consumer Rights Directive (2011/83/EU) & CMR Convention International Carriage Terms.",
        _ => DEFAULT_BOHAG_TERMS,
    }
}

async fn ensure_signature_audit_schema(conn: &database::DbConnection) -> Result<(), YntraError> {
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN ip_address TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN geolocation TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN device_fingerprint TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN terms_version TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN terms_hash TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE move_signatures ADD COLUMN signature_hash TEXT", ()).await;
    Ok(())
}

#[uniffi::export]
pub async fn save_job_signature(
    requester_user_id: String,
    job_id: String,
    signer_name: String,
    signature_data_base64: String,
) -> Result<(), YntraError> {
    save_job_signature_with_audit_trail(
        requester_user_id,
        job_id,
        signer_name,
        signature_data_base64,
        Some("127.0.0.1".to_string()),
        None,
        Some("YntraPlatform/Desktop/Mobile".to_string()),
        None,
    )
    .await
}

#[uniffi::export]
pub async fn save_job_signature_with_audit_trail(
    requester_user_id: String,
    job_id: String,
    signer_name: String,
    signature_data_base64: String,
    ip_address: Option<String>,
    geolocation: Option<String>,
    device_fingerprint: Option<String>,
    terms_version: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let id = Uuid::new_v4().to_string();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to sign".to_string(),
        ));
    }

    ensure_signature_audit_schema(&conn).await?;

    let (region_val,): (String,) = conn
        .query_row(
            "SELECT region FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
            |r| Ok((r.get(0).unwrap_or_else(|_| "SE".to_string()),)),
        )
        .await
        .unwrap_or_else(|_| ("SE".to_string(),));

    let legal_terms_text = get_regional_legal_terms(&region_val);
    let selected_terms_version = terms_version.unwrap_or_else(|| match region_val.to_uppercase().as_str() {
        "US" => "US DOT FMCSA Carmack 2024".to_string(),
        "UK" => "UK BAR Terms 2024".to_string(),
        "EU" => "EU CMR Consumer Terms 2024".to_string(),
        _ => "Bohag 2020".to_string(),
    });
    
    // Hash terms text
    let mut terms_hasher = Sha256::new();
    terms_hasher.update(format!("{}:{}", selected_terms_version, legal_terms_text).as_bytes());
    let terms_hash_bytes = terms_hasher.finalize();
    let terms_hash: String = terms_hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Compute signature manifest sha256 hash
    let mut sig_hasher = Sha256::new();
    let ip_str = ip_address.as_deref().unwrap_or("0.0.0.0");
    sig_hasher.update(format!("{}:{}:{}:{}:{}:{}", signer_name, signature_data_base64, now_ms, selected_terms_version, terms_hash, ip_str).as_bytes());
    let sig_hash_bytes = sig_hasher.finalize();
    let signature_hash: String = sig_hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    conn.execute(
        "INSERT OR REPLACE INTO move_signatures (id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at, sync_status, ip_address, geolocation, device_fingerprint, terms_version, terms_hash, signature_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9, ?10, ?11, ?12)",
        crate::params![
            id,
            auth.workspace_id,
            job_id,
            signer_name,
            signature_data_base64,
            now_ms,
            ip_address,
            geolocation,
            device_fingerprint,
            selected_terms_version,
            terms_hash,
            signature_hash
        ],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_job_signature(
    requester_user_id: String,
    job_id: String,
) -> Result<Option<MoveSignature>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    ensure_signature_audit_schema(&conn).await?;

    let res = conn.query_row(
        "SELECT id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at, sync_status, ip_address, geolocation, device_fingerprint, terms_version, terms_hash, signature_hash FROM move_signatures WHERE job_ticket_id = ?1",
        crate::params![&job_id],
        |row| {
            Ok(MoveSignature {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                signer_name: row.get(3)?,
                signature_data_base64: row.get(4)?,
                signed_at: row.get(5)?,
                sync_status: row.get(6)?,
                ip_address: row.get(7)?,
                geolocation: row.get(8)?,
                device_fingerprint: row.get(9)?,
                terms_version: row.get(10)?,
                terms_hash: row.get(11)?,
                signature_hash: row.get(12)?,
            })
        },
    ).await;

    match res {
        Ok(sig) => Ok(Some(sig)),
        Err(YntraError::NoRowsReturned) => Ok(None),
        Err(e) => Err(e),
    }
}

async fn ensure_bol_schema(conn: &database::DbConnection) -> Result<(), YntraError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS bill_of_ladings (
            bol_number TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            carrier_name TEXT NOT NULL,
            shipper_name TEXT NOT NULL,
            origin_address TEXT NOT NULL,
            destination_address TEXT NOT NULL,
            valuation_option TEXT NOT NULL,
            valuation_declared_amount REAL NOT NULL,
            valuation_deductible REAL NOT NULL,
            valuation_premium REAL NOT NULL,
            total_estimated_weight_lbs REAL NOT NULL,
            legal_terms TEXT NOT NULL,
            customer_signature_hash TEXT,
            created_at INTEGER NOT NULL,
            origin_signature_hash TEXT,
            destination_signature_hash TEXT,
            signed_origin_at INTEGER,
            signed_destination_at INTEGER,
            document_tamper_hash TEXT,
            inventory_manifest_json TEXT,
            carrier_dot_number TEXT
        )",
        (),
    ).await?;

    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN origin_signature_hash TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN destination_signature_hash TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN signed_origin_at INTEGER", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN signed_destination_at INTEGER", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN document_tamper_hash TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN inventory_manifest_json TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE bill_of_ladings ADD COLUMN carrier_dot_number TEXT", ()).await;
    Ok(())
}

pub fn calculate_bol_tamper_hash(
    bol_number: &str,
    job_ticket_id: &str,
    shipper_name: &str,
    origin_address: &str,
    destination_address: &str,
    valuation_option: &str,
    valuation_declared_amount: f64,
    total_estimated_weight_lbs: f64,
    inventory_manifest_json: &str,
    origin_signature_hash: Option<&str>,
    destination_signature_hash: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{:.2}:{:.2}:{}:{}:{}",
        bol_number,
        job_ticket_id,
        shipper_name,
        origin_address,
        destination_address,
        valuation_option,
        valuation_declared_amount,
        total_estimated_weight_lbs,
        inventory_manifest_json,
        origin_signature_hash.unwrap_or("none"),
        destination_signature_hash.unwrap_or("none")
    );
    hasher.update(payload.as_bytes());
    let hash_bytes = hasher.finalize();
    hash_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[uniffi::export]
pub async fn generate_bill_of_lading(
    requester_user_id: String,
    job_id: String,
    carrier_name: String,
    valuation_option: String,
    declared_value: f64,
    deductible: f64,
) -> Result<crate::models::BillOfLading, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    ensure_bol_schema(&conn).await?;

    let (ws_id, title, loc_addr, orig_opt, dest_opt): (String, String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, title, location_address, origin_address, destination_address FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

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
    let carrier_dot = settings_json.get("usdot_number").and_then(|v| v.as_str()).map(|s| s.to_string());

    let origin_address = orig_opt.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| loc_addr.clone());
    let destination_address = dest_opt.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| loc_addr.clone());

    let val_opt = valuation_option.to_lowercase();
    let (val_code, val_premium) = if val_opt == "full_value_protection" || val_opt == "full" {
        ("full_value_protection".to_string(), (declared_value * 0.01).max(50.0))
    } else {
        ("released_value_060".to_string(), 0.0)
    };

    let est_weight_lbs = match crate::services::jobs::moves::get_move_inventory_summary(requester_user_id.clone(), job_id.clone()).await {
        Ok(summary) if summary.total_weight_lbs > 0.0 => summary.total_weight_lbs,
        _ => 0.0,
    };

    let manifest_json = match crate::services::jobs::moves::get_inventory_scan_manifest(requester_user_id.clone(), job_id.clone()).await {
        Ok(m) => serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()),
        Err(_) => "{}".to_string(),
    };

    ensure_signature_audit_schema(&conn).await?;
    let customer_signature_hash: Option<String> = conn
        .query_row(
            "SELECT signature_hash FROM move_signatures WHERE job_ticket_id = ?1 ORDER BY signed_at DESC LIMIT 1",
            crate::params![&job_id],
            |r| r.get(0),
        )
        .await
        .ok();

    let bol_num = format!("BOL-US-{}", Uuid::new_v4().simple());
    let legal_terms = get_regional_legal_terms("US").to_string();
    let now_ms = crate::infra::time::get_current_time_ms();

    let tamper_hash = calculate_bol_tamper_hash(
        &bol_num,
        &job_id,
        &title,
        &origin_address,
        &destination_address,
        &val_code,
        declared_value,
        est_weight_lbs,
        &manifest_json,
        customer_signature_hash.as_deref(),
        None,
    );

    conn.execute(
        "DELETE FROM bill_of_ladings WHERE job_ticket_id = ?1",
        crate::params![&job_id],
    ).await?;

    conn.execute(
        "INSERT INTO bill_of_ladings (bol_number, workspace_id, job_ticket_id, carrier_name, shipper_name, origin_address, destination_address, valuation_option, valuation_declared_amount, valuation_deductible, valuation_premium, total_estimated_weight_lbs, legal_terms, customer_signature_hash, created_at, origin_signature_hash, destination_signature_hash, signed_origin_at, signed_destination_at, document_tamper_hash, inventory_manifest_json, carrier_dot_number) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
        crate::params![
            &bol_num,
            &ws_id,
            &job_id,
            &carrier_name,
            &title,
            &origin_address,
            &destination_address,
            &val_code,
            declared_value,
            deductible,
            val_premium,
            est_weight_lbs,
            &legal_terms,
            &customer_signature_hash,
            now_ms,
            &customer_signature_hash,
            Option::<String>::None,
            if customer_signature_hash.is_some() { Some(now_ms) } else { None },
            Option::<i64>::None,
            &tamper_hash,
            &manifest_json,
            &carrier_dot,
        ],
    ).await?;

    let _ = crate::services::audit::log_action_with_conn(
        &conn,
        requester_user_id,
        None,
        format!("bol_generated_{}", bol_num),
    ).await;

    notify_observers();

    Ok(crate::models::BillOfLading {
        bol_number: bol_num,
        workspace_id: ws_id,
        job_ticket_id: job_id,
        carrier_name,
        shipper_name: title,
        origin_address,
        destination_address,
        valuation_option: val_code,
        valuation_declared_amount: declared_value,
        valuation_deductible: deductible,
        valuation_premium: val_premium,
        total_estimated_weight_lbs: est_weight_lbs,
        legal_terms,
        customer_signature_hash: customer_signature_hash.clone(),
        created_at: now_ms,
        origin_signature_hash: customer_signature_hash,
        destination_signature_hash: None,
        signed_origin_at: Some(now_ms),
        signed_destination_at: None,
        document_tamper_hash: tamper_hash,
        inventory_manifest_json: manifest_json,
        carrier_dot_number: carrier_dot,
    })
}

#[uniffi::export]
pub async fn sign_bill_of_lading_phase(
    requester_user_id: String,
    job_id: String,
    phase: String,
    signer_name: String,
    signature_data_base64: String,
) -> Result<crate::models::BillOfLading, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    ensure_bol_schema(&conn).await?;

    let existing = get_bill_of_lading(job_id.clone()).await?
        .ok_or_else(|| YntraError::NotFoundError("Bill of lading does not exist yet".to_string()))?;

    if auth.workspace_id != existing.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    save_job_signature_with_audit_trail(
        requester_user_id.clone(),
        job_id.clone(),
        signer_name,
        signature_data_base64,
        Some("127.0.0.1".to_string()),
        None,
        Some("YntraPlatform/PhaseSigner".to_string()),
        Some(existing.legal_terms.clone()),
    ).await?;

    let sig = get_job_signature(requester_user_id.clone(), job_id.clone()).await?
        .ok_or_else(|| YntraError::ValidationError("Failed to retrieve signature after save".to_string()))?;

    let is_dest = phase.to_lowercase().contains("dest") || phase.to_lowercase().contains("deliver");
    let (origin_sig, origin_at, dest_sig, dest_at): (Option<String>, Option<i64>, Option<String>, Option<i64>) = if is_dest {
        (
            existing.origin_signature_hash.clone(),
            existing.signed_origin_at,
            sig.signature_hash.clone(),
            Some(now_ms),
        )
    } else {
        (
            sig.signature_hash.clone(),
            Some(now_ms),
            existing.destination_signature_hash.clone(),
            existing.signed_destination_at,
        )
    };

    let tamper_hash = calculate_bol_tamper_hash(
        &existing.bol_number,
        &existing.job_ticket_id,
        &existing.shipper_name,
        &existing.origin_address,
        &existing.destination_address,
        &existing.valuation_option,
        existing.valuation_declared_amount,
        existing.total_estimated_weight_lbs,
        &existing.inventory_manifest_json,
        origin_sig.as_deref(),
        dest_sig.as_deref(),
    );

    conn.execute(
        "UPDATE bill_of_ladings SET origin_signature_hash = ?1, signed_origin_at = ?2, destination_signature_hash = ?3, signed_destination_at = ?4, customer_signature_hash = ?5, document_tamper_hash = ?6 WHERE job_ticket_id = ?7",
        crate::params![
            &origin_sig,
            origin_at,
            &dest_sig,
            dest_at,
            dest_sig.as_ref().or(origin_sig.as_ref()),
            &tamper_hash,
            &job_id,
        ],
    ).await?;

    if is_dest {
        conn.execute(
            "UPDATE job_tickets SET status = 'completed', updated_at = ?1, sync_status = 'pending' WHERE id = ?2 AND status != 'completed'",
            crate::params![now_ms, &job_id],
        ).await.ok();
    }

    let _ = crate::services::audit::log_action_with_conn(
        &conn,
        requester_user_id,
        None,
        format!("bol_signed_phase_{}", phase),
    ).await;

    notify_observers();

    get_bill_of_lading(job_id).await?.ok_or_else(|| YntraError::NotFoundError("Failed to reload BOL".to_string()))
}

#[uniffi::export]
pub async fn validate_bill_of_lading_fmcsa_compliance(
    requester_user_id: String,
    job_id: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let bol = get_bill_of_lading(job_id.clone()).await?
        .ok_or_else(|| YntraError::NotFoundError(format!("No Bill of Lading found for job {}", job_id)))?;

    if bol.origin_signature_hash.is_none() {
        return Err(YntraError::ValidationError(
            "US DOT FMCSA Compliance Error: Origin pick-up signature is missing from Bill of Lading.".to_string(),
        ));
    }

    if bol.destination_signature_hash.is_none() {
        return Err(YntraError::ValidationError(
            "US DOT FMCSA Compliance Error: Destination delivery signature is missing. 49 CFR § 375.505 requires destination sign-off to validate cargo condition upon delivery.".to_string(),
        ));
    }

    let calculated_tamper_hash = calculate_bol_tamper_hash(
        &bol.bol_number,
        &bol.job_ticket_id,
        &bol.shipper_name,
        &bol.origin_address,
        &bol.destination_address,
        &bol.valuation_option,
        bol.valuation_declared_amount,
        bol.total_estimated_weight_lbs,
        &bol.inventory_manifest_json,
        bol.origin_signature_hash.as_deref(),
        bol.destination_signature_hash.as_deref(),
    );

    if bol.document_tamper_hash != calculated_tamper_hash {
        return Err(YntraError::ValidationError(
            "Security Integrity Violation: Bill of Lading tamper hash mismatch. Document may have been modified after signing.".to_string(),
        ));
    }

    Ok(true)
}

#[uniffi::export]
pub async fn get_bill_of_lading(
    job_id: String,
) -> Result<Option<crate::models::BillOfLading>, YntraError> {
    let conn = database::acquire_connection().await?;
    ensure_bol_schema(&conn).await?;

    let res = conn.query_row(
        "SELECT bol_number, workspace_id, job_ticket_id, carrier_name, shipper_name, origin_address, destination_address, valuation_option, valuation_declared_amount, valuation_deductible, valuation_premium, total_estimated_weight_lbs, legal_terms, customer_signature_hash, created_at, origin_signature_hash, destination_signature_hash, signed_origin_at, signed_destination_at, document_tamper_hash, inventory_manifest_json, carrier_dot_number FROM bill_of_ladings WHERE job_ticket_id = ?1",
        crate::params![&job_id],
        |r| Ok(crate::models::BillOfLading {
            bol_number: r.get(0)?,
            workspace_id: r.get(1)?,
            job_ticket_id: r.get(2)?,
            carrier_name: r.get(3)?,
            shipper_name: r.get(4)?,
            origin_address: r.get(5)?,
            destination_address: r.get(6)?,
            valuation_option: r.get(7)?,
            valuation_declared_amount: r.get(8)?,
            valuation_deductible: r.get(9)?,
            valuation_premium: r.get(10)?,
            total_estimated_weight_lbs: r.get(11)?,
            legal_terms: r.get(12)?,
            customer_signature_hash: r.get::<Option<String>>(13)?,
            created_at: r.get(14)?,
            origin_signature_hash: r.get::<Option<String>>(15)?,
            destination_signature_hash: r.get::<Option<String>>(16)?,
            signed_origin_at: r.get::<Option<i64>>(17)?,
            signed_destination_at: r.get::<Option<i64>>(18)?,
            document_tamper_hash: r.get::<Option<String>>(19)?.unwrap_or_default(),
            inventory_manifest_json: r.get::<Option<String>>(20)?.unwrap_or_else(|| "{}".to_string()),
            carrier_dot_number: r.get::<Option<String>>(21)?,
        }),
    ).await;

    match res {
        Ok(bol) => Ok(Some(bol)),
        Err(YntraError::NoRowsReturned) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_region_legal_terms() {
        assert!(get_regional_legal_terms("US").contains("FMCSA"));
        assert!(get_regional_legal_terms("UK").contains("British Association of Removers"));
        assert!(get_regional_legal_terms("EU").contains("EU Consumer Rights"));
        assert!(get_regional_legal_terms("SE").contains("Bohag 2020"));
        assert!(get_regional_legal_terms("GLOBAL").contains("Bohag 2020"));
    }

    #[tokio::test]
    async fn test_bill_of_lading_and_valuation_workflow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-bol-test', 'BOL WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-bol-staff', 'ws-bol-test', 'staff@fleet.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, scheduled_date, checklist_json, created_at, updated_at, sync_status) VALUES ('job-bol-1', 'ws-bol-test', 'Job BOL', 'Desc', '123 Main St', 'normal', 'scheduled', 0, 0, 1, 1, '2026-08-01', '[]', 1700000000000, 1700000000000, 'synced')", ()).await.unwrap();

        // 1. Released Value Protection ($0.60/lb)
        let bol_rel = generate_bill_of_lading("u-bol-staff".to_string(), "job-bol-1".to_string(), "North American Moving Corp".to_string(), "released_value_060".to_string(), 0.0, 0.0).await.unwrap();
        assert_eq!(bol_rel.valuation_option, "released_value_060");
        assert_eq!(bol_rel.valuation_premium, 0.0);
        assert!(bol_rel.legal_terms.contains("Carmack"));
        assert_eq!(bol_rel.origin_address, "123 Main St");
        assert_eq!(bol_rel.destination_address, "123 Main St");

        // 2. Save signature and generate full value protection BOL with signature linking
        save_job_signature_with_audit_trail(
            "u-bol-staff".to_string(),
            "job-bol-1".to_string(),
            "John Customer".to_string(),
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
            Some("192.168.1.100".to_string()),
            None,
            Some("MobileDevice".to_string()),
            Some("US DOT FMCSA Carmack 2024".to_string()),
        ).await.unwrap();

        let bol_full = generate_bill_of_lading("u-bol-staff".to_string(), "job-bol-1".to_string(), "North American Moving Corp".to_string(), "full_value_protection".to_string(), 25000.0, 250.0).await.unwrap();
        assert_eq!(bol_full.valuation_option, "full_value_protection");
        assert_eq!(bol_full.valuation_declared_amount, 25000.0);
        assert_eq!(bol_full.valuation_premium, 250.0); // 1% of 25000 = 250
        assert!(bol_full.customer_signature_hash.is_some());

        // 2.5 FMCSA Compliance check before destination sign-off fails due to missing delivery signature
        let fmcsa_fail = validate_bill_of_lading_fmcsa_compliance("u-bol-staff".to_string(), "job-bol-1".to_string()).await;
        assert!(fmcsa_fail.is_err());
        assert!(fmcsa_fail.unwrap_err().to_string().contains("Destination delivery signature is missing"));

        // 3. Phase-2 Destination Sign-off
        let bol_dest = sign_bill_of_lading_phase(
            "u-bol-staff".to_string(),
            "job-bol-1".to_string(),
            "destination".to_string(),
            "Jane Customer".to_string(),
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
        ).await.unwrap();
        assert!(bol_dest.origin_signature_hash.is_some());
        assert!(bol_dest.destination_signature_hash.is_some());
        assert!(bol_dest.signed_destination_at.is_some());
        assert!(!bol_dest.document_tamper_hash.is_empty());

        // 3.5 FMCSA Compliance check after destination delivery sign-off succeeds
        let fmcsa_pass = validate_bill_of_lading_fmcsa_compliance("u-bol-staff".to_string(), "job-bol-1".to_string()).await;
        assert!(fmcsa_pass.is_ok());
        assert!(fmcsa_pass.unwrap());

        // 4. Fetch BOL
        let fetched = get_bill_of_lading("job-bol-1".to_string()).await.unwrap().unwrap();
        assert_eq!(fetched.valuation_option, "full_value_protection");
        assert!(fetched.customer_signature_hash.is_some());
        assert!(fetched.destination_signature_hash.is_some());
        assert_eq!(fetched.document_tamper_hash, bol_dest.document_tamper_hash);

        conn.execute("DELETE FROM move_signatures WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM bill_of_ladings WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-bol-test'", ()).await.unwrap();
    }
}
