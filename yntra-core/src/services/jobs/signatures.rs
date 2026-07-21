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
            created_at INTEGER NOT NULL
        )",
        (),
    ).await?;
    Ok(())
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

    let (ws_id, title, origin_addr): (String, String, String) = conn
        .query_row(
            "SELECT workspace_id, title, location_address FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let val_opt = valuation_option.to_lowercase();
    let (val_code, val_premium) = if val_opt == "full_value_protection" || val_opt == "full" {
        ("full_value_protection".to_string(), (declared_value * 0.01).max(50.0))
    } else {
        ("released_value_060".to_string(), 0.0)
    };

    let est_weight_lbs = 3300.0;
    let bol_num = format!("BOL-US-{}", Uuid::new_v4().simple());
    let legal_terms = get_regional_legal_terms("US").to_string();
    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "DELETE FROM bill_of_ladings WHERE job_ticket_id = ?1",
        crate::params![&job_id],
    ).await?;

    conn.execute(
        "INSERT INTO bill_of_ladings (bol_number, workspace_id, job_ticket_id, carrier_name, shipper_name, origin_address, destination_address, valuation_option, valuation_declared_amount, valuation_deductible, valuation_premium, total_estimated_weight_lbs, legal_terms, customer_signature_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        crate::params![
            &bol_num,
            &ws_id,
            &job_id,
            &carrier_name,
            &title,
            &origin_addr,
            &origin_addr,
            &val_code,
            declared_value,
            deductible,
            val_premium,
            est_weight_lbs,
            &legal_terms,
            Option::<String>::None,
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(crate::models::BillOfLading {
        bol_number: bol_num,
        job_ticket_id: job_id,
        carrier_name,
        shipper_name: title,
        origin_address: origin_addr.clone(),
        destination_address: origin_addr,
        valuation_option: val_code,
        valuation_declared_amount: declared_value,
        valuation_deductible: deductible,
        valuation_premium: val_premium,
        total_estimated_weight_lbs: est_weight_lbs,
        legal_terms,
        customer_signature_hash: None,
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn get_bill_of_lading(
    job_id: String,
) -> Result<Option<crate::models::BillOfLading>, YntraError> {
    let conn = database::acquire_connection().await?;
    ensure_bol_schema(&conn).await?;

    let res = conn.query_row(
        "SELECT bol_number, job_ticket_id, carrier_name, shipper_name, origin_address, destination_address, valuation_option, valuation_declared_amount, valuation_deductible, valuation_premium, total_estimated_weight_lbs, legal_terms, customer_signature_hash, created_at FROM bill_of_ladings WHERE job_ticket_id = ?1",
        crate::params![&job_id],
        |r| Ok(crate::models::BillOfLading {
            bol_number: r.get(0)?,
            job_ticket_id: r.get(1)?,
            carrier_name: r.get(2)?,
            shipper_name: r.get(3)?,
            origin_address: r.get(4)?,
            destination_address: r.get(5)?,
            valuation_option: r.get(6)?,
            valuation_declared_amount: r.get(7)?,
            valuation_deductible: r.get(8)?,
            valuation_premium: r.get(9)?,
            total_estimated_weight_lbs: r.get(10)?,
            legal_terms: r.get(11)?,
            customer_signature_hash: r.get(12)?,
            created_at: r.get(13)?,
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

        // 2. Full Value Protection
        let bol_full = generate_bill_of_lading("u-bol-staff".to_string(), "job-bol-1".to_string(), "North American Moving Corp".to_string(), "full_value_protection".to_string(), 25000.0, 250.0).await.unwrap();
        assert_eq!(bol_full.valuation_option, "full_value_protection");
        assert_eq!(bol_full.valuation_declared_amount, 25000.0);
        assert_eq!(bol_full.valuation_premium, 250.0); // 1% of 25000 = 250

        // 3. Fetch BOL
        let fetched = get_bill_of_lading("job-bol-1".to_string()).await.unwrap().unwrap();
        assert_eq!(fetched.valuation_option, "full_value_protection");

        conn.execute("DELETE FROM bill_of_ladings WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-bol-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-bol-test'", ()).await.unwrap();
    }
}
