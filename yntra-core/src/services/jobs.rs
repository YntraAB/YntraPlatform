#![allow(unused)]

use crate::database;
use crate::infra::observer::notify_observers;
use crate::{JobTicket, MoveInventoryItem, MoveQuote, MoveSignature, WorkspaceUser, YntraError};
use uuid::Uuid;

fn is_staff(auth: &crate::AuthContext) -> bool {
    auth.role == "platform_admin"
        || auth.role == "admin"
        || auth.role == "assistant"
        || auth.role == "workspace_admin"
}

fn validate_job_status(status: &str) -> Result<(), YntraError> {
    match status {
        "pending" | "assigned" | "in_progress" | "completed" | "cancelled" => Ok(()),
        _ => Err(YntraError::ValidationError(format!(
            "Invalid job ticket status: {}",
            status
        ))),
    }
}

#[uniffi::export]
pub async fn get_job_tickets(requester_user_id: String) -> Result<Vec<JobTicket>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id FROM job_tickets WHERE workspace_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![auth.workspace_id], |row| {
            Ok(JobTicket {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                location_address: row.get(4)?,
                priority: row.get(5)?,
                status: row.get(6)?,
                assigned_user_id: row.get(7)?,
                scheduled_date: row.get(8)?,
                checklist_json: row.get(9)?,
                completion_report: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                sync_status: row.get(13)?,
                origin_address: row.get(14)?,
                destination_address: row.get(15)?,
                origin_floor: row.get(16)?,
                destination_floor: row.get(17)?,
                origin_has_elevator: row.get::<bool>(18)?,
                destination_has_elevator: row.get::<bool>(19)?,
                origin_parking_permit_needed: row.get::<bool>(20)?,
                destination_parking_permit_needed: row.get::<bool>(21)?,
                assigned_vehicle_id: row.get::<Option<String>>(22)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn create_job_ticket(
    requester_user_id: String,
    workspace_id: String,
    title: String,
    description: String,
    location_address: String,
    priority: String,
    assigned_user_id: Option<String>,
    scheduled_date: String,
    checklist_json: String,
    origin_address: Option<String>,
    destination_address: Option<String>,
    origin_floor: i32,
    destination_floor: i32,
    origin_has_elevator: bool,
    destination_has_elevator: bool,
    origin_parking_permit_needed: bool,
    destination_parking_permit_needed: bool,
) -> Result<JobTicket, YntraError> {
    let id = Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();

    let status = if assigned_user_id.is_some() {
        "assigned".to_string()
    } else {
        "pending".to_string()
    };

    let job = JobTicket {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        title,
        description,
        location_address,
        priority,
        status,
        assigned_user_id,
        scheduled_date,
        checklist_json,
        completion_report: None,
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
        origin_address,
        destination_address,
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        origin_parking_permit_needed,
        destination_parking_permit_needed,
        assigned_vehicle_id: None,
    };

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: requester belongs to a different workspace".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
        crate::params![
            job.id,
            job.workspace_id,
            job.title,
            job.description,
            job.location_address,
            job.priority,
            job.status,
            job.assigned_user_id,
            job.scheduled_date,
            job.checklist_json,
            job.completion_report,
            job.created_at,
            job.updated_at,
            job.sync_status,
            job.origin_address,
            job.destination_address,
            job.origin_floor,
            job.destination_floor,
            job.origin_has_elevator,
            job.destination_has_elevator,
            job.origin_parking_permit_needed,
            job.destination_parking_permit_needed,
            job.assigned_vehicle_id
        ],
    ).await?;
    Ok(job)
}

#[uniffi::export]
pub async fn update_job_status(
    requester_user_id: String,
    job_id: String,
    status: String,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    validate_job_status(&status)?;

    let (job_ws, assigned_uid): (String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, assigned_user_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let is_assigned_worker = assigned_uid.as_ref() == Some(&auth.user_id);
    if !is_staff(&auth) && !is_assigned_worker {
        return Err(YntraError::AuthError(
            "Access denied: only staff or the assigned worker can update job status".to_string(),
        ));
    }

    conn.execute(
        "UPDATE job_tickets SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![status, now_ms, job_id],
    ).await?;
    Ok(())
}

#[uniffi::export]
pub async fn schedule_job_ticket(
    requester_user_id: String,
    job_id: String,
    scheduled_date: String,
    assigned_user_id: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, job_title): (String, String) = conn
        .query_row(
            "SELECT workspace_id, title FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
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
            "Access denied: only staff can schedule jobs".to_string(),
        ));
    }

    // Update job ticket: set scheduled_date, assigned_user_id, status to "assigned"
    conn.execute(
        "UPDATE job_tickets SET scheduled_date = ?1, assigned_user_id = ?2, status = 'assigned', updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![&scheduled_date, &assigned_user_id, now_ms, &job_id],
    ).await?;

    // Create or update calendar event for this job ticket
    let event_id: Option<String> = conn
        .query_row(
            "SELECT id FROM events WHERE metadata LIKE ?1 AND workspace_id = ?2",
            crate::params![format!("%\"job_ticket_id\":\"{}\"%", job_id), &job_ws],
            |r| Ok(r.get(0)?),
        )
        .await
        .ok();

    let start_time = format!("{} 09:00", scheduled_date);
    let end_time = format!("{} 17:00", scheduled_date);
    let metadata = format!("{{\"job_ticket_id\":\"{}\"}}", job_id);

    if let Some(id) = event_id {
        conn.execute(
            "UPDATE events SET start_time = ?1, end_time = ?2, assignee_id = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
            crate::params![&start_time, &end_time, &assigned_user_id, now_ms, &id],
        ).await?;
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO events (id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status) VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
            crate::params![
                &id,
                &job_ws,
                &assigned_user_id,
                format!("Flytt: {}", job_title),
                &start_time,
                &end_time,
                &metadata,
                &now_ms
            ],
        ).await?;
    }

    notify_observers();
    Ok(())
}

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

fn urlencode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[uniffi::export]
pub async fn get_directions_url(
    requester_user_id: String,
    job_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let job: JobTicket = conn
        .query_row(
            "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |row| {
                Ok(JobTicket {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    location_address: row.get(4)?,
                    priority: row.get(5)?,
                    status: row.get(6)?,
                    assigned_user_id: row.get(7)?,
                    scheduled_date: row.get(8)?,
                    checklist_json: row.get(9)?,
                    completion_report: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    sync_status: row.get(13)?,
                    origin_address: row.get(14)?,
                    destination_address: row.get(15)?,
                    origin_floor: row.get(16)?,
                    destination_floor: row.get(17)?,
                    origin_has_elevator: row.get::<bool>(18)?,
                    destination_has_elevator: row.get::<bool>(19)?,
                    origin_parking_permit_needed: row.get::<bool>(20)?,
                    destination_parking_permit_needed: row.get::<bool>(21)?,
                    assigned_vehicle_id: row.get::<Option<String>>(22)?,
                })
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let origin = job.origin_address.filter(|s| !s.trim().is_empty());
    let dest = job.destination_address.filter(|s| !s.trim().is_empty())
        .unwrap_or(job.location_address);

    let url = match origin {
        Some(org) => format!(
            "https://www.google.com/maps/dir/?api=1&origin={}&destination={}",
            urlencode(&org),
            urlencode(&dest)
        ),
        None => format!(
            "https://www.google.com/maps/dir/?api=1&destination={}",
            urlencode(&dest)
        ),
    };

    Ok(url)
}

#[uniffi::export]
pub async fn submit_job_completion(
    requester_user_id: String,
    job_id: String,
    checklist_json: String,
    completion_report: String,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, assigned_uid): (String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, assigned_user_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let is_assigned_worker = assigned_uid.as_ref() == Some(&auth.user_id);
    if !is_staff(&auth) && !is_assigned_worker {
        return Err(YntraError::AuthError(
            "Access denied: only staff or the assigned worker can submit job completion"
                .to_string(),
        ));
    }

    conn.execute(
        "UPDATE job_tickets SET checklist_json = ?1, completion_report = ?2, status = 'completed', updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![checklist_json, completion_report, now_ms, job_id],
    ).await?;
    Ok(())
}

#[uniffi::export]
pub async fn get_job_tickets_rkyv(requester_user_id: String) -> Result<Vec<u8>, YntraError> {
    let tickets = get_job_tickets(requester_user_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&tickets)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[uniffi::export]
pub async fn get_move_inventory(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<MoveInventoryItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id], |row| {
            Ok(MoveInventoryItem {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                item_category: row.get(3)?,
                item_name: row.get(4)?,
                quantity: row.get::<i64>(5)? as i32,
                estimated_volume_m3: row.get(6)?,
                handling_notes: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<MoveQuote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(MoveQuote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            base_price: row.get::<f64>(3)? as i64,
            distance_fee: row.get::<f64>(4)? as i64,
            stairs_surcharge: row.get::<f64>(5)? as i64,
            packing_supplies_fee: row.get::<f64>(6)? as i64,
            total_price: row.get::<f64>(7)? as i64,
            status: row.get(8)?,
            accepted_at: row.get(9)?,
            updated_at: row.get(10)?,
            sync_status: row.get(11)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn accept_move_quote(
    requester_user_id: String,
    quote_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (quote_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![now_ms, now_ms, quote_id],
    ).await?;

    Ok(())
}

#[uniffi::export]
pub async fn create_move_inventory_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_category: String,
    item_name: String,
    quantity: i32,
    estimated_volume_m3: f64,
    handling_notes: Option<String>,
) -> Result<(), YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Verify workspace scoping of the job ticket
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            id,
            job_ws,
            job_ticket_id,
            item_category,
            item_name,
            quantity as i64,
            estimated_volume_m3,
            handling_notes,
            now_ms
        ],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_move_inventory_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_inventory WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Inventory item not found".to_string()))?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM move_inventory WHERE id = ?1",
        crate::params![item_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn calculate_and_save_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Job Ticket details
    let mut stmt = conn.prepare(
        "SELECT workspace_id, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator FROM job_tickets WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let (job_ws, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<i64>(1)? as i32,
            row.get::<i64>(2)? as i32,
            row.get::<bool>(3)?,
            row.get::<bool>(4)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Job not found".to_string()));
    };

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // 2. Fetch inventory items and calculate volume
    let mut inv_stmt = conn.prepare(
        "SELECT quantity, estimated_volume_m3 FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;
    let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
    let mut total_volume = 0.0;
    while let Some(row) = inv_rows.next().await? {
        let quantity: i64 = row.get(0)?;
        let vol: f64 = row.get(1)?;
        total_volume += (quantity as f64) * vol;
    }

    // 3. Quoting Calculations:
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&job_ws],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let base_rate_per_m3 = settings_json
        .get("moving_base_rate_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(500.0);
    let distance_fee_flat = settings_json
        .get("moving_distance_fee_flat")
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    let stairs_surcharge_per_floor = settings_json
        .get("moving_stairs_surcharge_per_floor")
        .and_then(|v| v.as_f64())
        .unwrap_or(300.0);
    let packing_supplies_fee_per_m3 = settings_json
        .get("moving_packing_supplies_fee_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);

    // Base hourly/labor rate = total volume * base_rate_per_m3
    let base_price = (total_volume * base_rate_per_m3) as i64;
    // Flat distance rate
    let distance_fee = distance_fee_flat as i64;
    // Stairs surcharge (stairs_surcharge_per_floor per floor if no elevator)
    let mut stairs_surcharge = 0;
    if !origin_has_elevator && origin_floor > 0 {
        stairs_surcharge += (origin_floor as i64) * (stairs_surcharge_per_floor as i64);
    }
    if !destination_has_elevator && destination_floor > 0 {
        stairs_surcharge += (destination_floor as i64) * (stairs_surcharge_per_floor as i64);
    }
    // Packing supplies fee = total volume * packing_supplies_fee_per_m3
    let packing_supplies_fee = (total_volume * packing_supplies_fee_per_m3) as i64;
    let total_price = base_price + distance_fee + stairs_surcharge + packing_supplies_fee;

    let now_ms = chrono::Utc::now().timestamp_millis();
    
    // Check if quote exists to keep its status, default to "sent"
    let mut quote_stmt = conn.prepare(
        "SELECT id, status FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let (quote_id, quote_status) = if let Some(row) = q_rows.next().await? {
        (row.get::<String>(0)?, row.get::<String>(1)?)
    } else {
        (uuid::Uuid::new_v4().to_string(), "sent".to_string())
    };

    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        crate::params![
            quote_id,
            job_ws,
            job_ticket_id,
            base_price as f64,
            distance_fee as f64,
            stairs_surcharge as f64,
            packing_supplies_fee as f64,
            total_price as f64,
            quote_status,
            now_ms
        ],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn generate_move_invoice(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
) -> Result<crate::models::MoveInvoice, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status FROM move_quotes WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    let (ws_id, job_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, _quote_status) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<f64>(2)?,
            row.get::<f64>(3)?,
            row.get::<f64>(4)?,
            row.get::<f64>(5)?,
            row.get::<f64>(6)?,
            row.get::<String>(7)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Quote not found".to_string()));
    };

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let customer_id = "client-1".to_string();

    let subtotal = total_price;
    let rut_deduction = if use_rut {
        0.5 * (base_price + stairs_surcharge)
    } else {
        0.0
    };
    let customer_amount = subtotal - rut_deduction;
    let tax_authority_amount = rut_deduction;

    let now = chrono::Utc::now();
    let invoice_date = now.format("%Y-%m-%d").to_string();
    let due_date = (now + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let now_ms = now.timestamp_millis();

    let mut inv_stmt = conn.prepare("SELECT id FROM move_invoices WHERE quote_id = ?1").await?;
    let mut inv_rows = inv_stmt.query(crate::params![&quote_id]).await?;
    let invoice_id = if let Some(row) = inv_rows.next().await? {
        row.get::<String>(0)?
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let invoice = crate::models::MoveInvoice {
        id: invoice_id.clone(),
        workspace_id: ws_id.clone(),
        quote_id: quote_id.clone(),
        customer_id: customer_id.clone(),
        invoice_date: invoice_date.clone(),
        due_date: due_date.clone(),
        subtotal,
        rut_deduction,
        customer_amount,
        tax_authority_amount,
        status: "unpaid".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending')",
        crate::params![
            invoice.id,
            invoice.workspace_id,
            invoice.quote_id,
            invoice.customer_id,
            invoice.invoice_date,
            invoice.due_date,
            invoice.subtotal,
            invoice.rut_deduction,
            invoice.customer_amount,
            invoice.tax_authority_amount,
            invoice.status,
            now_ms
        ]
    ).await?;

    notify_observers();
    Ok(invoice)
}

#[uniffi::export]
pub async fn get_move_invoice(
    requester_user_id: String,
    quote_id: String,
) -> Result<Option<crate::models::MoveInvoice>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status FROM move_invoices WHERE quote_id = ?1 LIMIT 1",
    ).await?;
    let mut rows = stmt.query(crate::params![&quote_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(1)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
        Ok(Some(crate::models::MoveInvoice {
            id: row.get::<String>(0)?,
            workspace_id: ws_id,
            quote_id,
            customer_id: row.get::<String>(2)?,
            invoice_date: row.get::<String>(3)?,
            due_date: row.get::<String>(4)?,
            subtotal: row.get::<f64>(5)?,
            rut_deduction: row.get::<f64>(6)?,
            customer_amount: row.get::<f64>(7)?,
            tax_authority_amount: row.get::<f64>(8)?,
            status: row.get::<String>(9)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn pay_move_invoice(
    requester_user_id: String,
    invoice_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT workspace_id FROM move_invoices WHERE id = ?1").await?;
    let mut rows = stmt.query(crate::params![&invoice_id]).await?;
    if let Some(row) = rows.next().await? {
        let ws_id = row.get::<String>(0)?;
        if auth.workspace_id != ws_id {
            return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
        }
    } else {
        return Err(YntraError::NotFoundError("Invoice not found".to_string()));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_invoices SET status = 'paid', updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![now_ms, invoice_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_job_tickets_workspace_scoping() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Workspaces and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-job-1', 'Job WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-job-2', 'Job WS 2', '[]', '{}')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-job-user1', 'ws-job-1', 'u1@job.io', 'user')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-job-user2', 'ws-job-2', 'u2@job.io', 'user')", ()).await.unwrap();

        // Create a job in ws-job-1
        let job1 = create_job_ticket(
            "u-job-user1".to_string(),
            "ws-job-1".to_string(),
            "Move office chair".to_string(),
            "Heavy chair".to_string(),
            "123 Main St".to_string(),
            "high".to_string(),
            Some("u-job-user1".to_string()),
            "2026-07-05".to_string(),
            "[]".to_string(),
            None,
            None,
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

        // Retrieve tickets as user 1 (should see job1)
        let list1 = get_job_tickets("u-job-user1".to_string()).await.unwrap();
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].id, job1.id);

        // Retrieve tickets as user 1 with rkyv
        let bytes = get_job_tickets_rkyv("u-job-user1".to_string())
            .await
            .unwrap();
        let rkyv_list: Vec<JobTicket> =
            rkyv::from_bytes::<Vec<JobTicket>, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(rkyv_list.len(), 1);
        assert_eq!(rkyv_list[0].id, job1.id);

        // Retrieve tickets as user 2 (should see 0, since ws-job-2 has no jobs)
        let list2 = get_job_tickets("u-job-user2".to_string()).await.unwrap();
        assert_eq!(list2.len(), 0);

        // Cleanup
        conn.execute(
            "DELETE FROM job_tickets WHERE workspace_id IN ('ws-job-1', 'ws-job-2')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM users WHERE workspace_id IN ('ws-job-1', 'ws-job-2')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id IN ('ws-job-1', 'ws-job-2')",
            (),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_move_operations() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace, staff user (role = 'admin')
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-move-test', 'Move Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-move-staff', 'ws-move-test', 'staff@move.io', 'admin')", ()).await.unwrap();

        // Create a job ticket: 3rd floor, no elevator at origin
        let job = create_job_ticket(
            "u-move-staff".to_string(),
            "ws-move-test".to_string(),
            "Move Sofa and Boxes".to_string(),
            "Client moving".to_string(),
            "Origin St 5".to_string(),
            "medium".to_string(),
            None,
            "2026-08-10".to_string(),
            "[]".to_string(),
            Some("Origin St 5".to_string()),
            Some("Dest St 10".to_string()),
            3,      // origin floor
            1,      // destination floor
            false,  // origin elevator
            true,   // destination elevator
            false,
            false,
        )
        .await
        .unwrap();

        // 1. Add Sofa (Furniture) - Qty 1, Vol 1.5
        create_move_inventory_item(
            "u-move-staff".to_string(),
            job.id.clone(),
            "Furniture".to_string(),
            "Sofa".to_string(),
            1,
            1.5,
            Some("Leather sofa".to_string()),
        )
        .await
        .unwrap();

        // 2. Add Books (Boxes) - Qty 5, Vol 0.1
        create_move_inventory_item(
            "u-move-staff".to_string(),
            job.id.clone(),
            "Boxes".to_string(),
            "Books".to_string(),
            5,
            0.1,
            None,
        )
        .await
        .unwrap();

        // Verify inventory
        let inv = get_move_inventory("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap();
        assert_eq!(inv.len(), 2);
        
        let sofa_item = inv.iter().find(|i| i.item_name == "Sofa").unwrap();
        let books_item = inv.iter().find(|i| i.item_name == "Books").unwrap();
        assert_eq!(sofa_item.quantity, 1);
        assert_eq!(books_item.quantity, 5);

        // 3. Calculate Quote
        calculate_and_save_move_quote("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap();

        let quote_opt = get_move_quote("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap();
        assert!(quote_opt.is_some());
        let q = quote_opt.unwrap();
        
        // Calculations verification:
        // Volume = 1.5 * 1 + 0.1 * 5 = 2.0 m3
        // Base Price = 2.0 * 500 = 1000 SEK
        // Distance Fee = 800 SEK
        // Stairs Surcharge = 3 floors * 300 SEK (since origin has no elevator, dest has elevator so 0 surcharge) = 900 SEK
        // Packing supplies fee = 2.0 * 100 = 200 SEK
        // Total = 1000 + 800 + 900 + 200 = 2900 SEK
        assert_eq!(q.base_price, 1000);
        assert_eq!(q.distance_fee, 800);
        assert_eq!(q.stairs_surcharge, 900);
        assert_eq!(q.packing_supplies_fee, 200);
        assert_eq!(q.total_price, 2900);

        // 4. Delete the Books item
        delete_move_inventory_item("u-move-staff".to_string(), books_item.id.clone())
            .await
            .unwrap();

        // Verify inventory count decreased
        let inv_after = get_move_inventory("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap();
        assert_eq!(inv_after.len(), 1);
        assert_eq!(inv_after[0].item_name, "Sofa");

        // 5. Recalculate Quote (Volume drops to 1.5 m3)
        // Base Price = 1.5 * 500 = 750 SEK
        // Distance Fee = 800 SEK
        // Stairs Surcharge = 900 SEK
        // Packing supplies = 1.5 * 100 = 150 SEK
        // Total = 750 + 800 + 900 + 150 = 2600 SEK
        calculate_and_save_move_quote("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap();

        let q_updated = get_move_quote("u-move-staff".to_string(), job.id.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(q_updated.base_price, 750);
        assert_eq!(q_updated.total_price, 2600);

        // Cleanup
        conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-move-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-move-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduling_and_sync() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace, staff user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sync-test', 'Sync Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sync-staff', 'ws-sync-test', 'staff@sync.io', 'admin')", ()).await.unwrap();

        // Create job ticket
        let job = create_job_ticket(
            "u-sync-staff".to_string(),
            "ws-sync-test".to_string(),
            "Office Relocation".to_string(),
            "Large office relocation".to_string(),
            "Main St 1".to_string(),
            "high".to_string(),
            None,
            "".to_string(), // Unscheduled
            "[]".to_string(),
            None,
            None,
            0,
            0,
            true,
            true,
            false,
            false,
        )
        .await
        .unwrap();

        // Verify initial state
        assert_eq!(job.scheduled_date, "");
        assert_eq!(job.status, "pending");

        // 1. Schedule the job ticket
        schedule_job_ticket(
            "u-sync-staff".to_string(),
            job.id.clone(),
            "2026-08-15".to_string(),
            Some("u-sync-staff".to_string()),
        )
        .await
        .unwrap();

        // Verify job ticket updated
        let updated_job: JobTicket = conn
            .query_row(
                "SELECT scheduled_date, status, assigned_user_id FROM job_tickets WHERE id = ?1",
                crate::params![&job.id],
                |r| Ok(JobTicket {
                    id: job.id.clone(),
                    workspace_id: "ws-sync-test".to_string(),
                    title: "".to_string(),
                    description: "".to_string(),
                    location_address: "".to_string(),
                    priority: "".to_string(),
                    status: r.get(1)?,
                    assigned_user_id: r.get(2)?,
                    scheduled_date: r.get(0)?,
                    checklist_json: "[]".to_string(),
                    completion_report: None,
                    created_at: "".to_string(),
                    updated_at: 0,
                    sync_status: "pending".to_string(),
                    origin_address: None,
                    destination_address: None,
                    origin_floor: 0,
                    destination_floor: 0,
                    origin_has_elevator: false,
                    destination_has_elevator: false,
                    origin_parking_permit_needed: false,
                    destination_parking_permit_needed: false,
                    assigned_vehicle_id: None,
                }),
            )
            .await
            .unwrap();
        
        assert_eq!(updated_job.scheduled_date, "2026-08-15");
        assert_eq!(updated_job.status, "assigned");
        assert_eq!(updated_job.assigned_user_id, Some("u-sync-staff".to_string()));

        // Verify calendar event created
        let (event_id, event_start, event_assignee): (String, String, Option<String>) = conn
            .query_row(
                "SELECT id, start_time, assignee_id FROM events WHERE metadata LIKE ?1",
                crate::params![format!("%\"job_ticket_id\":\"{}\"%", job.id)],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .await
            .unwrap();

        assert_eq!(event_start, "2026-08-15 09:00");
        assert_eq!(event_assignee, Some("u-sync-staff".to_string()));

        // 2. Drag & Drop update (calls update_event_time)
        crate::services::teams::update_event_time(
            "u-sync-staff".to_string(),
            event_id.clone(),
            "2026-08-20 10:00".to_string(),
            "2026-08-20 18:00".to_string(),
        )
        .await
        .unwrap();

        // Verify job ticket updated to new date YYYY-MM-DD
        let res_date: String = conn
            .query_row(
                "SELECT scheduled_date FROM job_tickets WHERE id = ?1",
                crate::params![&job.id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(res_date, "2026-08-20");

        // 3. Delete event
        crate::services::teams::delete_event("u-sync-staff".to_string(), event_id.clone())
            .await
            .unwrap();

        // Verify job ticket reset to unscheduled
        let (res_date_2, res_status, res_assignee): (String, String, Option<String>) = conn
            .query_row(
                "SELECT scheduled_date, status, assigned_user_id FROM job_tickets WHERE id = ?1",
                crate::params![&job.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .await
            .unwrap();

        assert_eq!(res_date_2, "");
        assert_eq!(res_status, "pending");
        assert_eq!(res_assignee, None);

        // Cleanup
        conn.execute("DELETE FROM events WHERE id = ?1", crate::params![&event_id]).await.ok();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-sync-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-sync-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_invoice_and_rut_calculations() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace, staff/client user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-inv-test', 'Invoice Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-inv-staff', 'ws-inv-test', 'staff@inv.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-inv-client', 'ws-inv-test', 'client@inv.io', 'client')", ()).await.unwrap();

        // Create job ticket
        let job = create_job_ticket(
            "u-inv-staff".to_string(),
            "ws-inv-test".to_string(),
            "RUT Relocation".to_string(),
            "Move with tax deductions".to_string(),
            "Main St 1".to_string(),
            "medium".to_string(),
            Some("u-inv-client".to_string()),
            "2026-09-01".to_string(),
            "[]".to_string(),
            None,
            None,
            0,
            0,
            true,
            true,
            false,
            false,
        )
        .await
        .unwrap();

        // Setup a mock quote
        let quote_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES (?1, 'ws-inv-test', ?2, 2000.0, 800.0, 600.0, 400.0, 3800.0, 'sent', 123456)",
            crate::params![&quote_id, &job.id],
        ).await.unwrap();

        // 1. Generate invoice with RUT deduction enabled
        let invoice = generate_move_invoice(
            "u-inv-staff".to_string(),
            quote_id.clone(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(invoice.subtotal, 3800.0);
        assert_eq!(invoice.rut_deduction, 1300.0);
        assert_eq!(invoice.customer_amount, 2500.0);
        assert_eq!(invoice.tax_authority_amount, 1300.0);
        assert_eq!(invoice.status, "unpaid");

        // 2. Fetch the invoice
        let fetched_invoice = get_move_invoice(
            "u-inv-staff".to_string(),
            quote_id.clone(),
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(fetched_invoice.id, invoice.id);
        assert_eq!(fetched_invoice.rut_deduction, 1300.0);
        assert_eq!(fetched_invoice.status, "unpaid");

        // 3. Pay the invoice
        pay_move_invoice(
            "u-inv-staff".to_string(),
            invoice.id.clone(),
        )
        .await
        .unwrap();

        // Verify status updated in database
        let status_res: String = conn.query_row(
            "SELECT status FROM move_invoices WHERE id = ?1",
            crate::params![&invoice.id],
            |r| r.get(0),
        ).await.unwrap();
        assert_eq!(status_res, "paid");

        // Cleanup
        conn.execute("DELETE FROM move_invoices WHERE quote_id = ?1", crate::params![&quote_id]).await.ok();
        conn.execute("DELETE FROM move_quotes WHERE id = ?1", crate::params![&quote_id]).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-inv-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-inv-client'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-inv-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_configurable_pricing_calculations() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace with custom pricing settings
        let settings = r#"{"moving_base_rate_per_m3":600.0,"moving_distance_fee_flat":1000.0,"moving_stairs_surcharge_per_floor":400.0,"moving_packing_supplies_fee_per_m3":150.0}"#;
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-price-test', 'Price Test WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-price-staff', 'ws-price-test', 'staff@price.io', 'admin')", ()).await.unwrap();

        // Create job ticket: 2nd floor, no elevator at origin
        let job = create_job_ticket(
            "u-price-staff".to_string(),
            "ws-price-test".to_string(),
            "Move Desk".to_string(),
            "Custom pricing check".to_string(),
            "Origin St 50".to_string(),
            "medium".to_string(),
            None,
            "2026-08-15".to_string(),
            "[]".to_string(),
            Some("Origin St 50".to_string()),
            Some("Dest St 100".to_string()),
            2,      // origin floor
            0,      // destination floor
            false,  // origin elevator
            true,   // destination elevator
            false,
            false,
        )
        .await
        .unwrap();

        // Add 1 Desk - Qty 1, Vol 1.0 m3
        create_move_inventory_item(
            "u-price-staff".to_string(),
            job.id.clone(),
            "Furniture".to_string(),
            "Desk".to_string(),
            1,
            1.0,
            None,
        )
        .await
        .unwrap();

        // Calculate Quote
        calculate_and_save_move_quote("u-price-staff".to_string(), job.id.clone())
            .await
            .unwrap();

        let quote_opt = get_move_quote("u-price-staff".to_string(), job.id.clone())
            .await
            .unwrap();
        assert!(quote_opt.is_some());
        let q = quote_opt.unwrap();

        // Custom Calculations verification:
        // Volume = 1.0 m3
        // Base Price = 1.0 * 600.0 = 600 SEK
        // Distance Fee = 1000 SEK
        // Stairs Surcharge = 2 floors * 400 SEK = 800 SEK
        // Packing supplies fee = 1.0 * 150.0 = 150 SEK
        // Total = 600 + 1000 + 800 + 150 = 2550 SEK
        assert_eq!(q.base_price, 600);
        assert_eq!(q.distance_fee, 1000);
        assert_eq!(q.stairs_surcharge, 800);
        assert_eq!(q.packing_supplies_fee, 150);
        assert_eq!(q.total_price, 2550);

        // Cleanup
        conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-price-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-price-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_multi_mover_crew_assignment() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        crate::infra::crypto::set_session_key("test-session-key-for-crew-tests".to_string().into_bytes(), "ws-crew-test".to_string());
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-crew-test', 'Crew Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff1', 'ws-crew-test', 'lead@crew.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff2', 'ws-crew-test', 'mover1@crew.io', 'mover')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-staff3', 'ws-crew-test', 'mover2@crew.io', 'mover')", ()).await.unwrap();

        // Create job ticket
        let job = create_job_ticket(
            "u-crew-staff1".to_string(),
            "ws-crew-test".to_string(),
            "Relocate Piano".to_string(),
            "Heavy lift move".to_string(),
            "Piano St 1".to_string(),
            "high".to_string(),
            None,
            "2026-08-12".to_string(),
            "[]".to_string(),
            None,
            None,
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

        // 1. Assign crew members
        add_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff2".to_string(), "driver".to_string()).await.unwrap();
        add_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff3".to_string(), "helper".to_string()).await.unwrap();

        // 2. Fetch crew members
        let crew = get_job_crew("u-crew-staff1".to_string(), job.id.clone()).await.unwrap();
        assert_eq!(crew.len(), 2);
        assert!(crew.iter().any(|u| u.id == "u-crew-staff2"));
        assert!(crew.iter().any(|u| u.id == "u-crew-staff3"));

        // 3. Remove a crew member
        remove_crew_member("u-crew-staff1".to_string(), job.id.clone(), "u-crew-staff2".to_string()).await.unwrap();

        // Verify updated crew list
        let crew_after = get_job_crew("u-crew-staff1".to_string(), job.id.clone()).await.unwrap();
        assert_eq!(crew_after.len(), 1);
        assert_eq!(crew_after[0].id, "u-crew-staff3");

        // Cleanup
        conn.execute("DELETE FROM job_crew WHERE job_ticket_id = ?1", crate::params![&job.id]).await.ok();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-crew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-crew-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_digital_signature_capture() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sig-test', 'Sig Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sig-staff', 'ws-sig-test', 'staff@sig.io', 'admin')", ()).await.unwrap();

        // Create job ticket
        let job = create_job_ticket(
            "u-sig-staff".to_string(),
            "ws-sig-test".to_string(),
            "Cabinet relocation".to_string(),
            "Delicate office cabinets".to_string(),
            "Cabinet Road 10".to_string(),
            "medium".to_string(),
            None,
            "2026-08-14".to_string(),
            "[]".to_string(),
            None,
            None,
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

        // 1. Initially verify no signature exists
        let sig_opt = get_job_signature("u-sig-staff".to_string(), job.id.clone()).await.unwrap();
        assert!(sig_opt.is_none());

        // 2. Save signature
        let mock_signature = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADIA...";
        save_job_signature("u-sig-staff".to_string(), job.id.clone(), "John Doe (Customer)".to_string(), mock_signature.to_string()).await.unwrap();

        // 3. Retrieve and verify signature details
        let sig_opt_2 = get_job_signature("u-sig-staff".to_string(), job.id.clone()).await.unwrap();
        assert!(sig_opt_2.is_some());
        let sig = sig_opt_2.unwrap();
        assert_eq!(sig.signer_name, "John Doe (Customer)");
        assert_eq!(sig.signature_data_base64, mock_signature);
        assert_eq!(sig.job_ticket_id, job.id);
        assert_eq!(sig.workspace_id, "ws-sig-test");

        // Cleanup
        conn.execute("DELETE FROM move_signatures WHERE job_ticket_id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![&job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sig-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-sig-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_gps_routing_urls() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-test', 'GPS Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-staff', 'ws-gps-test', 'staff@gps.io', 'admin')", ()).await.unwrap();

        // 1. Create job ticket with both origin and destination addresses
        let job1 = create_job_ticket(
            "u-gps-staff".to_string(),
            "ws-gps-test".to_string(),
            "Cabinet relocation".to_string(),
            "Delicate office cabinets".to_string(),
            "Dest Road 10".to_string(),
            "medium".to_string(),
            None,
            "2026-08-14".to_string(),
            "[]".to_string(),
            Some("Origin St 1".to_string()),
            Some("Dest St 5".to_string()),
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

        // Verify routing URL with both origin and destination
        let url1 = get_directions_url("u-gps-staff".to_string(), job1.id.clone()).await.unwrap();
        assert_eq!(url1, "https://www.google.com/maps/dir/?api=1&origin=Origin%20St%201&destination=Dest%20St%205");

        // 2. Create job ticket with destination only (relying on fallback to location_address)
        let job2 = create_job_ticket(
            "u-gps-staff".to_string(),
            "ws-gps-test".to_string(),
            "Cabinet relocation".to_string(),
            "Delicate office cabinets".to_string(),
            "Location St 20".to_string(),
            "medium".to_string(),
            None,
            "2026-08-14".to_string(),
            "[]".to_string(),
            None,
            None,
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .await
        .unwrap();

        let url2 = get_directions_url("u-gps-staff".to_string(), job2.id.clone()).await.unwrap();
        assert_eq!(url2, "https://www.google.com/maps/dir/?api=1&destination=Location%20St%2020");

        // Cleanup
        conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-gps-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-gps-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-gps-test'", ()).await.unwrap();
    }
}
