use crate::JobTicket;
use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use uuid::Uuid;

pub fn is_staff(auth: &crate::AuthContext) -> bool {
    if auth.is_admin {
        return true;
    }
    let r = auth.role.to_lowercase();
    r == "platform_admin"
        || r == "admin"
        || r == "assistant"
        || r == "workspace_admin"
        || r == "manager"
        || r == "staff"
        || r == "dispatch"
        || r == "field_worker"
        || r == "mover"
        || r == "driver"
        || r.starts_with("role-move-")
        || r.starts_with("role-flytt-")
        || r.starts_with("role-care-")
        || r.starts_with("role-school-")
}

pub fn is_field_mover_or_driver(auth: &crate::AuthContext) -> bool {
    let r = auth.role.to_lowercase();
    r == "mover"
        || r == "driver"
        || r == "role-move-mover"
        || r == "role-move-driver"
        || r == "role-flytt-arbetare"
        || r == "role-flytt-chauffor"
        || r.contains("mover")
        || r.contains("driver")
        || r.contains("chauffor")
        || r.contains("arbetare")
}

pub fn is_management_staff(auth: &crate::AuthContext) -> bool {
    if auth.is_admin {
        return true;
    }
    let r = auth.role.to_lowercase();
    r == "platform_admin"
        || r == "admin"
        || r == "assistant"
        || r == "workspace_admin"
        || r == "manager"
        || r == "role-move-coordinator"
        || r == "role-move-admin"
        || r == "role-flytt-ledare"
        || r == "role-flytt-koordinator"
}

fn validate_job_status(status: &str) -> Result<(), YntraError> {
    match status {
        "pending" | "quote_requested" | "assigned" | "in_progress" | "completed" | "cancelled" => {
            Ok(())
        }
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

    if auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees 
         FROM job_tickets 
         WHERE workspace_id = ?1
           AND (?2 = 1 OR assigned_user_id = ?3 OR EXISTS (
               SELECT 1 FROM job_crew WHERE job_ticket_id = job_tickets.id AND user_id = ?3
           ))",
    ).await?;

    let is_staff_val = if is_management_staff(&auth) {
        1i64
    } else {
        0i64
    };

    let list = stmt
        .query_map(
            crate::params![auth.workspace_id, is_staff_val, auth.user_id],
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
                    route_stops_json: row.get::<Option<String>>(23)?,
                    long_carry_meters: row.get::<i64>(24)? as i32,
                    toll_fees: row.get::<f64>(25)?,
                })
            },
        )
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
        route_stops_json: Some("[]".to_string()),
        long_carry_meters: 0,
        toll_fees: 0.0,
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
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, 0, 0.0)",
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
            job.assigned_vehicle_id,
            job.route_stops_json
        ],
    ).await?;
    Ok(job)
}

#[uniffi::export]
pub async fn update_job_moving_surcharges(
    requester_user_id: String,
    job_id: String,
    long_carry_meters: i32,
    toll_fees: f64,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let job_ws: String = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| r.get(0),
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

    conn.execute(
        "UPDATE job_tickets SET long_carry_meters = ?1, toll_fees = ?2, updated_at = ?3 WHERE id = ?4",
        crate::params![long_carry_meters, toll_fees, now_ms, &job_id],
    ).await?;

    // If quote exists, re-calculate it to reflect the new surcharges immediately
    let quote_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0)
        > 0;

    if quote_exists {
        // Run quote recalculation
        drop(conn);
        crate::services::jobs::moves::calculate_and_save_move_quote(requester_user_id, job_id)
            .await?;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_job_status(
    requester_user_id: String,
    job_id: String,
    status: String,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();

    let mut conn = database::acquire_connection().await?;
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

    if status == "in_progress"
        || status == "in_transit"
        || status == "assigned"
        || status == "scheduled"
    {
        let assigned_v_id: Option<String> = conn
            .query_row(
                "SELECT assigned_vehicle_id FROM job_tickets WHERE id = ?1",
                crate::params![&job_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(None);

        if let Some(vid) = assigned_v_id {
            drop(conn);
            super::crew::validate_vehicle_dispatch_capacity(
                requester_user_id.clone(),
                job_id.clone(),
                vid,
            )
            .await?;
            conn = database::acquire_connection().await?;
        }
    }

    conn.execute(
        "UPDATE job_tickets SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![&status, now_ms, job_id],
    ).await?;

    if status == "in_progress" {
        let customer_id = conn
            .query_row(
                "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
                crate::params![&job_ws],
                |r| r.get::<String>(0),
            )
            .await
            .unwrap_or_else(|_| "client-1".to_string());

        let _ = crate::services::jobs::notifications::send_external_notification(
            requester_user_id.clone(),
            job_ws,
            customer_id,
            "arrival_reminder".to_string(),
            None,
        )
        .await;
    }

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
    let mut conn = database::acquire_connection().await?;
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

    let assigned_v_id: Option<String> = conn
        .query_row(
            "SELECT assigned_vehicle_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(None);

    if let Some(vid) = assigned_v_id {
        drop(conn);
        super::crew::validate_vehicle_dispatch_capacity(
            requester_user_id.clone(),
            job_id.clone(),
            vid,
        )
        .await?;
        conn = database::acquire_connection().await?;
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

    let customer_id = conn
        .query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
            crate::params![&job_ws],
            |r| r.get::<String>(0),
        )
        .await
        .unwrap_or_else(|_| "client-1".to_string());

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id.clone(),
        job_ws,
        customer_id,
        "job_completion".to_string(),
        None,
    )
    .await;

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
pub async fn update_route_stops(
    requester_user_id: String,
    job_id: String,
    stops: Vec<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let job_ws: String = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| r.get(0),
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
            "Access denied: only staff can modify route stops".to_string(),
        ));
    }

    let stops_json =
        serde_json::to_string(&stops).map_err(|e| YntraError::SerializationError(e.to_string()))?;

    conn.execute(
        "UPDATE job_tickets SET route_stops_json = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![stops_json, now_ms, job_id],
    ).await?;

    notify_observers();
    Ok(())
}

async fn optimize_job_route_inner(
    requester_user_id: String,
    job_id: String,
) -> Result<Vec<String>, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Load the job ticket
    let job: JobTicket = conn
        .query_row(
            "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees FROM job_tickets WHERE id = ?1",
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
                    route_stops_json: row.get::<Option<String>>(23)?,
                    long_carry_meters: row.get::<i64>(24)? as i32,
                    toll_fees: row.get::<f64>(25)?,
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

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can optimize route".to_string(),
        ));
    }

    let origin = job
        .origin_address
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| job.location_address.clone());
    let destination = job
        .destination_address
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| job.location_address.clone());

    let stops_str = job
        .route_stops_json
        .clone()
        .unwrap_or_else(|| "[]".to_string());
    let stops: Vec<String> = serde_json::from_str(&stops_str).unwrap_or_default();

    if stops.is_empty() {
        return Ok(Vec::new());
    }

    let optimized_stops =
        super::routing::optimize_route(&auth.workspace_id, &origin, &destination, &stops).await;

    let optimized_json = serde_json::to_string(&optimized_stops)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    conn.execute(
        "UPDATE job_tickets SET route_stops_json = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![optimized_json, now_ms, job_id],
    ).await?;

    notify_observers();
    Ok(optimized_stops)
}

#[uniffi::export]
#[cfg(target_arch = "wasm32")]
pub async fn optimize_job_route(
    requester_user_id: String,
    job_id: String,
) -> Result<Vec<String>, YntraError> {
    let fut = optimize_job_route_inner(requester_user_id, job_id);
    crate::database::wasm::SendFuture::new(fut).await
}

#[uniffi::export]
#[cfg(not(target_arch = "wasm32"))]
pub async fn optimize_job_route(
    requester_user_id: String,
    job_id: String,
) -> Result<Vec<String>, YntraError> {
    optimize_job_route_inner(requester_user_id, job_id).await
}

#[uniffi::export]
pub async fn get_mover_field_sheet_manifest(
    requester_user_id: String,
    job_id: String,
) -> Result<crate::models::MoverFieldSheetManifest, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (
        job_ws,
        title,
        description,
        scheduled_date,
        status,
        origin_address,
        destination_address,
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        assigned_vehicle_id,
        route_stops_json,
        long_carry_meters,
    ): (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i32,
        i32,
        bool,
        bool,
        Option<String>,
        Option<String>,
        i32,
    ) = conn
        .query_row(
            "SELECT workspace_id, title, description, scheduled_date, status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, assigned_vehicle_id, route_stops_json, long_carry_meters FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get::<i64>(7)? as i32,
                    r.get::<i64>(8)? as i32,
                    r.get::<bool>(9)?,
                    r.get::<bool>(10)?,
                    r.get(11)?,
                    r.get(12)?,
                    r.get::<i64>(13)? as i32,
                ))
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Verify assigned mover access or management staff
    let is_assigned: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM job_tickets WHERE id = ?1 AND (assigned_user_id = ?2 OR EXISTS (SELECT 1 FROM job_crew WHERE job_ticket_id = ?1 AND user_id = ?2))",
            crate::params![&job_id, &auth.user_id],
            |r| Ok(r.get::<i64>(0)? > 0),
        )
        .await
        .unwrap_or(false);

    if !is_management_staff(&auth) && !is_assigned {
        return Err(YntraError::AuthError(
            "Access denied: unassigned mover field sheet".to_string(),
        ));
    }

    // Load Item Manifest (Read-Only)
    let inventory_items =
        super::moves::get_move_inventory(requester_user_id.clone(), job_id.clone())
            .await
            .unwrap_or_default();

    // Load Assigned Crew Names
    let mut crew_stmt = conn.prepare("SELECT u.full_name FROM job_crew j JOIN users u ON j.user_id = u.id WHERE j.job_ticket_id = ?1").await?;
    let mut crew_rows = crew_stmt.query(crate::params![&job_id]).await?;
    let mut assigned_crew_names = Vec::new();
    while let Some(row) = crew_rows.next().await? {
        let name: String = row
            .get::<Option<String>>(0)?
            .unwrap_or_else(|| "Mover".to_string());
        assigned_crew_names.push(name);
    }

    // Load Assigned Vehicle License Plate
    let mut assigned_vehicle_plate = None;
    if let Some(vid) = assigned_vehicle_id {
        if let Ok(plate) = conn
            .query_row(
                "SELECT license_plate FROM vehicles WHERE id = ?1",
                crate::params![&vid],
                |r| r.get::<String>(0),
            )
            .await
        {
            assigned_vehicle_plate = Some(plate);
        }
    }

    Ok(crate::models::MoverFieldSheetManifest {
        job_id,
        title,
        description,
        scheduled_date,
        status,
        origin_address,
        destination_address,
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        inventory_items,
        assigned_crew_names,
        assigned_vehicle_plate,
        route_stops_json,
        long_carry_meters,
    })
}
