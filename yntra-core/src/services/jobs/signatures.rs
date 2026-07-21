use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::MoveSignature;
use sha2::{Sha256, Digest};
use uuid::Uuid;

pub const DEFAULT_BOHAG_TERMS: &str = "Allkort & Bohag 2010 / Bohag 2020 Allmänna Bestämmelser för Bohagsettlement & Bohagsflyttning (Sveriges Åkeriföretag). Ansvarighet och försäkring i enlighet med Konsumentverket & Transportavtalet.";

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
        Some("Bohag 2020".to_string()),
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

    let selected_terms_version = terms_version.unwrap_or_else(|| "Bohag 2020".to_string());
    
    // Hash terms text
    let mut terms_hasher = Sha256::new();
    terms_hasher.update(format!("{}:{}", selected_terms_version, DEFAULT_BOHAG_TERMS).as_bytes());
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
