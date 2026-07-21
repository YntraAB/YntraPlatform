use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::DamageInspection;
use uuid::Uuid;

#[uniffi::export]
pub async fn record_damage_inspection(
    requester_user_id: String,
    job_ticket_id: String,
    item_inventory_id: Option<String>,
    item_name: String,
    damage_type: String,
    severity: String,
    annotations: Option<String>,
    photo_url: Option<String>,
) -> Result<DamageInspection, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (workspace_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied: insufficient permissions".to_string()));
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let inspection = DamageInspection {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        job_ticket_id: job_ticket_id.clone(),
        item_inventory_id: item_inventory_id.clone(),
        item_name: item_name.clone(),
        damage_type: damage_type.clone(),
        severity: severity.clone(),
        annotations: annotations.clone(),
        photo_url: photo_url.clone(),
        timestamp_ms: now_ms,
        inspector_user_id: requester_user_id.clone(),
        client_acknowledged: false,
        client_signature_svg: None,
        created_at: now_ms,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO damage_inspections (id, workspace_id, job_ticket_id, item_inventory_id, item_name, damage_type, severity, annotations, photo_url, timestamp_ms, inspector_user_id, client_acknowledged, client_signature_svg, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        crate::params![
            inspection.id,
            inspection.workspace_id,
            inspection.job_ticket_id,
            inspection.item_inventory_id,
            inspection.item_name,
            inspection.damage_type,
            inspection.severity,
            inspection.annotations,
            inspection.photo_url,
            inspection.timestamp_ms,
            inspection.inspector_user_id,
            inspection.client_acknowledged as i32,
            inspection.client_signature_svg,
            inspection.created_at,
            inspection.updated_at,
            inspection.sync_status
        ],
    ).await?;

    notify_observers();
    Ok(inspection)
}

#[uniffi::export]
pub async fn get_job_damage_inspections(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<DamageInspection>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (workspace_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_inventory_id, item_name, damage_type, severity, annotations, photo_url, timestamp_ms, inspector_user_id, client_acknowledged, client_signature_svg, created_at, updated_at, sync_status FROM damage_inspections WHERE job_ticket_id = ?1 ORDER BY timestamp_ms DESC"
    ).await?;

    let list = stmt
        .query_map(crate::params![&job_ticket_id], |r| {
            Ok(DamageInspection {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                job_ticket_id: r.get(2)?,
                item_inventory_id: r.get(3)?,
                item_name: r.get(4)?,
                damage_type: r.get(5)?,
                severity: r.get(6)?,
                annotations: r.get(7)?,
                photo_url: r.get(8)?,
                timestamp_ms: r.get(9)?,
                inspector_user_id: r.get(10)?,
                client_acknowledged: r.get::<i32>(11)? != 0,
                client_signature_svg: r.get(12)?,
                created_at: r.get(13)?,
                updated_at: r.get(14)?,
                sync_status: r.get(15)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn acknowledge_damage_inspection_by_client(
    requester_user_id: String,
    inspection_id: String,
    signature_svg: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM damage_inspections WHERE id = ?1",
            crate::params![&inspection_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Inspection record not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE damage_inspections SET client_acknowledged = 1, client_signature_svg = ?1, updated_at = ?2 WHERE id = ?3",
        crate::params![signature_svg, now_ms, &inspection_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_damage_inspection(
    requester_user_id: String,
    inspection_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM damage_inspections WHERE id = ?1",
            crate::params![&inspection_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Inspection record not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError("Access denied: only staff can delete inspections".to_string()));
    }

    conn.execute(
        "DELETE FROM damage_inspections WHERE id = ?1",
        crate::params![&inspection_id],
    ).await?;

    notify_observers();
    Ok(())
}
