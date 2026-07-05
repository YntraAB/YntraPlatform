use crate::{JobTicket, YntraError};
use uuid::Uuid;
use crate::database;
use crate::infra::observer::notify_observers;
use crate::{MoveInventoryItem, MoveQuote};

#[uniffi::export]
pub async fn get_job_tickets(requester_user_id: String) -> Result<Vec<JobTicket>, YntraError> {
    let conn = database::acquire_connection().await?;
    let requester_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed FROM job_tickets WHERE workspace_id = ?1",
    ).await?;

    let list = stmt.query_map(crate::params![requester_ws], |row| {
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
            origin_has_elevator: row.get::<i32>(18)? != 0,
            destination_has_elevator: row.get::<i32>(19)? != 0,
            origin_parking_permit_needed: row.get::<i32>(20)? != 0,
            destination_parking_permit_needed: row.get::<i32>(21)? != 0,
        })
    }).await?;

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
        
    let job = JobTicket {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        title,
        description,
        location_address,
        priority,
        status: "assigned".to_string(),
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
    };

    let conn = database::acquire_connection().await?;
    let requester_ws: String = conn.query_row(
        "SELECT workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::AuthError("Requester user not found".to_string()))?;

    if requester_ws != workspace_id {
        return Err(YntraError::AuthError("Access denied: requester belongs to a different workspace".to_string()));
    }

    conn.execute(
        "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
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
            job.destination_parking_permit_needed
        ],
    ).await?;

    notify_observers();
    Ok(job)
}

#[uniffi::export]
pub async fn update_job_status(requester_user_id: String, job_id: String, status: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE job_tickets SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![status, now_ms, job_id],
    ).await?;

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
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE job_tickets SET checklist_json = ?1, completion_report = ?2, status = 'completed', updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![checklist_json, completion_report, now_ms, job_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_move_inventory(requester_user_id: String, job_ticket_id: String) -> Result<Vec<MoveInventoryItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_ticket_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;

    let list = stmt.query_map(crate::params![job_ticket_id], |row| {
        Ok(MoveInventoryItem {
            id: row.get(0)?,
            job_ticket_id: row.get(1)?,
            item_category: row.get(2)?,
            item_name: row.get(3)?,
            quantity: row.get(4)?,
            estimated_volume_m3: row.get(5)?,
            handling_notes: row.get(6)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_move_inventory_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_category: String,
    item_name: String,
    quantity: i32,
    estimated_volume_m3: f64,
    handling_notes: Option<String>,
) -> Result<MoveInventoryItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_ticket_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let item = MoveInventoryItem {
        id: id.clone(),
        job_ticket_id: job_ticket_id.clone(),
        item_category,
        item_name,
        quantity,
        estimated_volume_m3,
        handling_notes,
    };

    conn.execute(
        "INSERT INTO move_inventory (id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        crate::params![
            item.id,
            item.job_ticket_id,
            item.item_category,
            item.item_name,
            item.quantity,
            item.estimated_volume_m3,
            item.handling_notes
        ],
    ).await?;

    notify_observers();
    Ok(item)
}

#[uniffi::export]
pub async fn get_move_quote(requester_user_id: String, job_ticket_id: String) -> Result<Option<MoveQuote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_ticket_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at FROM move_quotes WHERE job_ticket_id = ?1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(MoveQuote {
            id: row.get(0)?,
            job_ticket_id: row.get(1)?,
            base_price: row.get(2)?,
            distance_fee: row.get(3)?,
            stairs_surcharge: row.get(4)?,
            packing_supplies_fee: row.get(5)?,
            total_price: row.get(6)?,
            status: row.get(7)?,
            accepted_at: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn create_or_update_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
    base_price: f64,
    distance_fee: f64,
    stairs_surcharge: f64,
    packing_supplies_fee: f64,
    status: String,
) -> Result<MoveQuote, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT workspace_id FROM job_tickets WHERE id = ?1",
        crate::params![&job_ticket_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let total_price = base_price + distance_fee + stairs_surcharge + packing_supplies_fee;
    
    let quote = MoveQuote {
        id: id.clone(),
        job_ticket_id: job_ticket_id.clone(),
        base_price,
        distance_fee,
        stairs_surcharge,
        packing_supplies_fee,
        total_price,
        status: status.clone(),
        accepted_at: None,
    };

    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;

    let res = async {
        // Delete existing quote for the job first
        let _ = conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![job_ticket_id]).await;

        conn.execute(
            "INSERT INTO move_quotes (id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            crate::params![
                quote.id,
                quote.job_ticket_id,
                quote.base_price,
                quote.distance_fee,
                quote.stairs_surcharge,
                quote.packing_supplies_fee,
                quote.total_price,
                quote.status,
                quote.accepted_at
            ],
        ).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.execute("COMMIT", ()).await?;
            notify_observers();
            Ok(quote)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn accept_move_quote(requester_user_id: String, quote_id: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let job_ws: String = conn.query_row(
        "SELECT jt.workspace_id FROM move_quotes mq JOIN job_tickets jt ON mq.job_ticket_id = jt.id WHERE mq.id = ?1",
        crate::params![&quote_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1 WHERE id = ?2",
        crate::params![now_ms, quote_id],
    ).await?;

    notify_observers();
    Ok(())
}
