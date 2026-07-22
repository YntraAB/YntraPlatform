pub mod compliance;
pub mod routing;
pub mod telemetry;

#[cfg(test)]
mod tests;

pub use compliance::*;
pub use routing::*;
pub use telemetry::*;

use crate::database;
use crate::infra::observer::notify_observers;
use crate::{MoveVehicle, YntraError};
use uuid::Uuid;

#[uniffi::export]
pub async fn get_vehicles(requester_user_id: String) -> Result<Vec<MoveVehicle>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, name, license_plate, capacity_m3, status, updated_at, sync_status, latitude, longitude, last_ping, gps_device_id FROM vehicles WHERE workspace_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![auth.workspace_id], |row| {
            Ok(MoveVehicle {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                license_plate: row.get(3)?,
                capacity_m3: row.get(4)?,
                status: row.get(5)?,
                updated_at: row.get(6)?,
                sync_status: row.get(7)?,
                latitude: row.get(8)?,
                longitude: row.get(9)?,
                last_ping: row.get(10)?,
                gps_device_id: row.get(11)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn create_vehicle(
    requester_user_id: String,
    name: String,
    license_plate: String,
    capacity_m3: f64,
    gps_device_id: Option<String>,
) -> Result<MoveVehicle, YntraError> {
    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: only staff can manage vehicles".to_string(),
        ));
    }

    let vehicle = MoveVehicle {
        id: id.clone(),
        workspace_id: auth.workspace_id.clone(),
        name,
        license_plate,
        capacity_m3,
        status: "active".to_string(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
        latitude: None,
        longitude: None,
        last_ping: None,
        gps_device_id,
    };

    conn.execute(
        "INSERT INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, status, updated_at, sync_status, latitude, longitude, last_ping, gps_device_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        crate::params![
            vehicle.id,
            vehicle.workspace_id,
            vehicle.name,
            vehicle.license_plate,
            vehicle.capacity_m3,
            vehicle.status,
            vehicle.updated_at,
            vehicle.sync_status,
            vehicle.latitude,
            vehicle.longitude,
            vehicle.last_ping,
            vehicle.gps_device_id
        ],
    ).await?;

    notify_observers();
    Ok(vehicle)
}

#[uniffi::export]
pub async fn delete_vehicle(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" || auth.role == "client" {
        return Err(YntraError::AuthError(
            "Access denied: only staff can manage vehicles".to_string(),
        ));
    }

    let (ws_id,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM vehicles WHERE id = ?1",
        crate::params![vehicle_id],
    ).await?;

    notify_observers();
    Ok(())
}
