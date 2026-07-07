use crate::{JobTicket, YntraError};
use uuid::Uuid;
use crate::database;
use crate::infra::observer::notify_observers;
use crate::{MoveInventoryItem, MoveQuote};

#[uniffi::export]
pub async fn get_job_tickets(requester_user_id: String) -> Result<Vec<JobTicket>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, sync_status, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed FROM job_tickets WHERE workspace_id = ?1",
    ).await?;

    let list = stmt.query_map(crate::params![auth.workspace_id], |row| {
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
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.workspace_id != workspace_id {
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

    if auth.role == "client" {
        return Err(YntraError::AuthError("Access denied: clients cannot create or edit quotes".to_string()));
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

    conn.begin_transaction().await?;

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
            conn.commit().await?;
            notify_observers();
            Ok(quote)
        }
        Err(e) => {
            let _ = conn.rollback().await;
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
            None, None, 0, 0, false, false, false, false,
        ).await.unwrap();

        // Retrieve tickets as user 1 (should see job1)
        let list1 = get_job_tickets("u-job-user1".to_string()).await.unwrap();
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].id, job1.id);

        // Retrieve tickets as user 2 (should see 0, since ws-job-2 has no jobs)
        let list2 = get_job_tickets("u-job-user2".to_string()).await.unwrap();
        assert_eq!(list2.len(), 0);

        // Cleanup
        conn.execute("DELETE FROM job_tickets WHERE workspace_id IN ('ws-job-1', 'ws-job-2')", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-job-1', 'ws-job-2')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-job-1', 'ws-job-2')", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_move_quote_lifecycle() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-job-q', 'Job Q WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-job-q-user', 'ws-job-q', 'uq@job.io', 'user')", ()).await.unwrap();

        // Create job
        let job = create_job_ticket(
            "u-job-q-user".to_string(),
            "ws-job-q".to_string(),
            "Move piano".to_string(),
            "Grand piano".to_string(),
            "123 Piano Rd".to_string(),
            "medium".to_string(),
            None,
            "2026-07-05".to_string(),
            "[]".to_string(),
            None, None, 0, 0, false, false, false, false,
        ).await.unwrap();

        // Generate quote
        let quote = create_or_update_move_quote(
            "u-job-q-user".to_string(),
            job.id.clone(),
            200.0, // base
            50.0,  // distance
            100.0, // stairs
            20.0,  // supplies
            "pending".to_string(),
        ).await.unwrap();

        assert_eq!(quote.base_price, 200.0);
        assert_eq!(quote.total_price, 370.0); // 200 + 50 + 100 + 20
        assert_eq!(quote.status, "pending");

        // Accept quote
        let accept_res = accept_move_quote("u-job-q-user".to_string(), quote.id.clone()).await;
        assert!(accept_res.is_ok());

        // Get quote and verify status is accepted
        let retrieved = get_move_quote("u-job-q-user".to_string(), job.id.clone()).await.unwrap();
        assert!(retrieved.is_some());
        let q = retrieved.unwrap();
        assert_eq!(q.status, "accepted");
        assert!(q.accepted_at.is_some());

        // Cleanup
        conn.execute("DELETE FROM move_quotes WHERE job_ticket_id = ?1", crate::params![job.id]).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = ?1", crate::params![job.id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-job-q'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-job-q'", ()).await.unwrap();
    }
}
