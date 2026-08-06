use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::JobPackagingItem;

#[uniffi::export]
pub async fn get_job_packaging_items(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<JobPackagingItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

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
        "SELECT id, workspace_id, job_ticket_id, item_name, quantity, returned_quantity, is_leased, price_per_unit, (quantity * price_per_unit), created_at, updated_at FROM job_packaging_items WHERE job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id], |row| {
            Ok(JobPackagingItem {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                item_name: row.get(3)?,
                quantity: row.get::<i64>(4)? as i32,
                returned_quantity: row.get::<i64>(5)? as i32,
                is_leased: row.get::<i64>(6)? != 0,
                price_per_unit: row.get(7)?,
                sync_status: "synced".to_string(),
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_job_packaging_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_name: String,
    quantity_delivered: i32,
    unit_price_sek: f64,
    is_rented: bool,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

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

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO job_packaging_items (id, workspace_id, job_ticket_id, item_name, quantity, returned_quantity, is_leased, price_per_unit, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?8)",
        crate::params![
            id,
            job_ws,
            job_ticket_id,
            item_name,
            quantity_delivered as i64,
            if is_rented { 1i64 } else { 0i64 },
            unit_price_sek,
            now_ms,
        ],
    ).await?;

    let _ = super::pricing::calculate_and_save_move_quote(
        requester_user_id.clone(),
        job_ticket_id.clone(),
    )
    .await;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_job_packaging_item_returned(
    requester_user_id: String,
    item_id: String,
    quantity_returned: i32,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE job_packaging_items SET returned_quantity = ?1, updated_at = ?2 WHERE id = ?3",
        crate::params![quantity_returned as i64, now_ms, item_id],
    )
    .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn remove_job_packaging_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM job_packaging_items WHERE id = ?1",
        crate::params![item_id],
    )
    .await?;

    let _ = super::pricing::calculate_and_save_move_quote(
        requester_user_id.clone(),
        job_ticket_id.clone(),
    )
    .await;

    notify_observers();
    Ok(())
}
