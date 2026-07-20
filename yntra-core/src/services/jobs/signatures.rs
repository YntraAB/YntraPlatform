use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::MoveSignature;
use uuid::Uuid;

#[uniffi::export]
pub async fn save_job_signature(
    requester_user_id: String,
    job_id: String,
    signer_name: String,
    signature_data_base64: String,
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

    // Clients or staff can sign the job ticket
    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to sign".to_string(),
        ));
    }

    conn.execute(
        "INSERT OR REPLACE INTO move_signatures (id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending')",
        crate::params![id, auth.workspace_id, job_id, signer_name, signature_data_base64, now_ms],
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

    let res = conn.query_row(
        "SELECT id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at, sync_status FROM move_signatures WHERE job_ticket_id = ?1",
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
            })
        },
    ).await;

    match res {
        Ok(sig) => Ok(Some(sig)),
        Err(YntraError::NoRowsReturned) => Ok(None),
        Err(e) => Err(e),
    }
}
