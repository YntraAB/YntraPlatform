mod academics;
mod auth;
mod billing;
mod conflicts;
mod health;
mod library;
mod profiles;
mod timetable;
mod attendance;

#[cfg(test)]
mod tests;

// Re-export public functions from sub-modules so they appear at services::school::*
pub use academics::*;
pub use auth::check_school_permission;
pub use billing::*;
pub use conflicts::get_school_conflicts;
pub use conflicts::resolve_school_conflict;
pub use conflicts::delete_school_conflict;
pub use health::*;
pub use library::*;
pub use profiles::*;
pub use timetable::*;
pub use attendance::*;

use crate::database;
use crate::infra::errors::YntraError;

#[uniffi::export]
pub async fn save_blob(
    requester_user_id: String,
    sha256: String,
    workspace_id: String,
    base64_data: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO local_blobs (sha256, workspace_id, data, created_at) VALUES (?1, ?2, ?3, ?4)",
        crate::params![sha256, workspace_id, base64_data, now_ms],
    )
    .await?;

    Ok(())
}

#[uniffi::export]
pub async fn get_blob(
    requester_user_id: String,
    sha256: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (data, ws_id): (String, String) = conn
        .query_row(
            "SELECT data, workspace_id FROM local_blobs WHERE sha256 = ?1",
            crate::params![sha256],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Blob not found: {}", sha256)))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    Ok(data)
}
