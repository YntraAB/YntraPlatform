use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;
use crate::WorkspaceUser;

#[uniffi::export]
pub async fn assign_vehicle_to_job(
    requester_user_id: String,
    job_id: String,
    assigned_vehicle_id: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
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

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign vehicles to jobs".to_string(),
        ));
    }

    // Verify vehicle belongs to workspace if assigned
    if let Some(ref vehicle_id) = assigned_vehicle_id {
        let (vehicle_ws,): (String,) = conn
            .query_row(
                "SELECT workspace_id FROM vehicles WHERE id = ?1",
                crate::params![vehicle_id],
                |r| Ok((r.get(0)?,)),
            )
            .await
            .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

        if auth.workspace_id != vehicle_ws {
            return Err(YntraError::AuthError(
                "Access denied: vehicle belongs to a different workspace".to_string(),
            ));
        }
    }

    conn.execute(
        "UPDATE job_tickets SET assigned_vehicle_id = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![assigned_vehicle_id, now_ms, job_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn add_crew_member(
    requester_user_id: String,
    job_id: String,
    user_id: String,
    role: String,
) -> Result<(), YntraError> {
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
            "Access denied: workspace mismatch on job".to_string(),
        ));
    }

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign crew members".to_string(),
        ));
    }

    // Verify user belongs to same workspace
    let (user_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&user_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("User not found".to_string()))?;

    if auth.workspace_id != user_ws {
        return Err(YntraError::AuthError(
            "Access denied: user belongs to a different workspace".to_string(),
        ));
    }

    conn.execute(
        "INSERT OR REPLACE INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, ?2, ?3)",
        crate::params![job_id, user_id, role],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn remove_crew_member(
    requester_user_id: String,
    job_id: String,
    user_id: String,
) -> Result<(), YntraError> {
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
            "Access denied: workspace mismatch on job".to_string(),
        ));
    }

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign crew members".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM job_crew WHERE job_ticket_id = ?1 AND user_id = ?2",
        crate::params![job_id, user_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_job_crew(
    requester_user_id: String,
    job_id: String,
) -> Result<Vec<WorkspaceUser>, YntraError> {
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

    let cipher = crate::infra::crypto::WorkspaceCipher::new(&auth.workspace_id)?;

    let mut stmt = conn.prepare(
        "SELECT u.id, u.workspace_id, u.email, u.full_name, u.phone, u.role, u.preferences, u.metadata, u.updated_at, u.sync_status 
         FROM users u 
         JOIN job_crew jc ON u.id = jc.user_id 
         WHERE jc.job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![&job_id], |row| {
            let id: String = row.get(0)?;
            let ws_id: Option<String> = row.get(1)?;
            let metadata_str: Option<String> = row.get(7)?;

            let is_self = id == requester_user_id;

            let mut siths_card_id = None;
            let mut nfc_badge_uid = None;
            let mut personal_number = None;
            let mut public_key = None;

            if let Some(ref m_str) = metadata_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                    public_key = meta_val
                        .get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            meta_val
                                .get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });

                    if auth.is_admin || is_self {
                        siths_card_id = meta_val
                            .get("siths_card_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        nfc_badge_uid = meta_val
                            .get("nfc_badge_uid")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let raw_pnum = meta_val
                            .get("personal_number")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if raw_pnum.is_some() {
                            personal_number = cipher.decrypt_opt(raw_pnum);
                        }
                    }
                }
            }

            Ok(WorkspaceUser {
                id,
                workspace_id: ws_id,
                email: row.get(2)?,
                full_name: row.get(3)?,
                phone: row.get(4)?,
                role: row.get(5)?,
                preferences: row.get(6)?,
                siths_card_id,
                nfc_badge_uid,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                personal_number,
                public_key,
            })
        })
        .await?;

    Ok(list)
}
