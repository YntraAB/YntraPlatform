use crate::{database, YntraError};

pub mod students;
pub mod academics;
pub mod attendance;
pub mod scheduling;
pub mod health;
pub mod finance;
pub mod library;

pub use students::*;
pub use academics::*;
pub use attendance::*;
pub use scheduling::*;
pub use health::*;
pub use finance::*;
pub use library::*;

pub(crate) async fn check_permission(
    conn: &database::DbConnection,
    user_id: &str,
    permission: &str,
) -> Result<bool, YntraError> {
    let row: Option<(String, Option<String>)> = conn.query_row(
        "SELECT role, workspace_id FROM users WHERE id = ?1",
        crate::params![user_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.ok();

    let (role, ws_id) = match row {
        Some((r, Some(w))) => (r, w),
        _ => return Ok(false),
    };

    if role == "platform_admin" || role == "admin" {
        return Ok(true);
    }

    let ws_settings: String = conn.query_row(
        "SELECT settings FROM workspaces WHERE id = ?1",
        crate::params![&ws_id],
        |r| r.get(0)
    ).await.unwrap_or_default();

    let settings_val: serde_json::Value = serde_json::from_str(&ws_settings).unwrap_or_default();
    if let Some(roles) = settings_val.get("roles").and_then(|r| r.as_array()) {
        for role_val in roles {
            if role_val.get("id").and_then(|i| i.as_str()) == Some(&role) {
                if let Some(perms) = role_val.get("permissions") {
                    return Ok(perms.get(permission).and_then(|p| p.as_bool()).unwrap_or(false));
                }
            }
        }
    }

    if role.contains("rektor") || role.contains("principal") {
        return Ok(true);
    }
    if role.contains("skoterska") || role.contains("nurse") {
        return Ok(permission == "can_access_health_records" || permission == "can_submit_reports");
    }

    Ok(false)
}

pub(crate) async fn has_health_access(
    conn: &database::DbConnection,
    requester_user_id: &str,
    student_id: &str,
) -> Result<bool, YntraError> {
    if check_permission(conn, requester_user_id, "can_access_health_records").await? {
        return Ok(true);
    }

    let student_user_id: Option<String> = conn.query_row(
        "SELECT user_id FROM student_profiles WHERE id = ?1",
        crate::params![student_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if let Some(uid) = student_user_id {
        if uid == requester_user_id {
            return Ok(true);
        }
    }

    let is_parent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2",
        crate::params![student_id, requester_user_id],
        |r| r.get(0)
    ).await.unwrap_or(0);

    if is_parent > 0 {
        return Ok(true);
    }

    Ok(false)
}

pub(crate) async fn has_academic_access(
    conn: &database::DbConnection,
    requester_user_id: &str,
    student_id: &str,
) -> Result<bool, YntraError> {
    if check_permission(conn, requester_user_id, "can_manage_grades").await? {
        return Ok(true);
    }

    let student_user_id: Option<String> = conn.query_row(
        "SELECT user_id FROM student_profiles WHERE id = ?1",
        crate::params![student_id],
        |r| r.get(0)
    ).await.ok().flatten();

    if let Some(uid) = student_user_id {
        if uid == requester_user_id {
            return Ok(true);
        }
    }

    let is_parent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2",
        crate::params![student_id, requester_user_id],
        |r| r.get(0)
    ).await.unwrap_or(0);

    if is_parent > 0 {
        return Ok(true);
    }

    Ok(false)
}
