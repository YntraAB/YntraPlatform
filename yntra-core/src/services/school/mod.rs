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
    let auth = match crate::AuthContext::authorize(conn, user_id).await {
        Ok(a) => a,
        Err(_) => return Ok(false),
    };


    if auth.role == "platform_admin" || auth.role == "admin" {
        return Ok(true);
    }

    let settings_val: serde_json::Value = auth.workspace_settings
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    if let Some(roles) = settings_val.get("roles").and_then(|r| r.as_array()) {
        for role_val in roles {
            if role_val.get("id").and_then(|i| i.as_str()) == Some(&auth.role) {
                if let Some(perms) = role_val.get("permissions") {
                    return Ok(perms.get(permission).and_then(|p| p.as_bool()).unwrap_or(false));
                }
            }
        }
    }

    if auth.role == "rektor" || auth.role == "principal" || auth.role == "principal_head" {
        return Ok(true);
    }
    let r = auth.role.as_str();
    if r.contains("nurse")
        || r.contains("skoterska")
        || r.contains("sköterska")
        || r.contains("helsesykepleier")
        || r.contains("helsesøster")
        || r.contains("sundhedsplejerske")
        || r.contains("terveydenhoitaja")
        || r.contains("kouluterveydenhoitaja")
        || r.contains("hoitaja")
    {
        return Ok(permission == "can_access_health_records" || permission == "can_submit_reports");
    }

    Ok(false)
}

pub(crate) async fn has_health_access(
    conn: &database::DbConnection,
    requester_user_id: &str,
    student_id: &str,
) -> Result<bool, YntraError> {
    let auth = match crate::AuthContext::authorize(conn, requester_user_id).await {
        Ok(a) => a,
        Err(_) => return Ok(false),
    };

    let student_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![student_id],
        |r| r.get(0)
    ).await.ok().flatten();

    let student_ws = match student_ws_id {
        Some(ws) => ws,
        None => return Ok(false),
    };

    if auth.role == "platform_admin" {
        return Ok(true);
    }

    if auth.workspace_id != student_ws {
        return Ok(false);
    }

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
    let auth = match crate::AuthContext::authorize(conn, requester_user_id).await {
        Ok(a) => a,
        Err(_) => return Ok(false),
    };

    let student_ws_id: Option<String> = conn.query_row(
        "SELECT workspace_id FROM student_profiles WHERE id = ?1",
        crate::params![student_id],
        |r| r.get(0)
    ).await.ok().flatten();

    let student_ws = match student_ws_id {
        Some(ws) => ws,
        None => return Ok(false),
    };

    if auth.role == "platform_admin" {
        return Ok(true);
    }

    if auth.workspace_id != student_ws {
        return Ok(false);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_check_permission_admin_and_overrides() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup mock workspaces and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-school-1', 'School WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-admin', 'ws-school-1', 'admin@sch.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-rektor', 'ws-school-1', 'rektor@sch.io', 'principal_head')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-nurse', 'ws-school-1', 'nurse@sch.io', 'nurse')", ()).await.unwrap();

        // 1. Admin should have any permission
        assert!(check_permission(&conn, "u-school-admin", "any_random_permission").await.unwrap());

        // 2. Rektor / Principal override
        assert!(check_permission(&conn, "u-school-rektor", "can_view_dashboard").await.unwrap());

        // 3. Nurse overrides
        assert!(check_permission(&conn, "u-school-nurse", "can_access_health_records").await.unwrap());
        assert!(check_permission(&conn, "u-school-nurse", "can_submit_reports").await.unwrap());
        assert!(!check_permission(&conn, "u-school-nurse", "can_manage_grades").await.unwrap());

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-school-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-school-1'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_check_permission_role_from_settings() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Workspaces with custom roles settings JSON
        let custom_settings = r#"{
            "roles": [
                {
                    "id": "teacher",
                    "permissions": {
                        "can_manage_grades": true,
                        "can_access_health_records": false
                    }
                }
            ]
        }"#;

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-school-2', 'School WS 2', '[]', ?1)",
            crate::params![custom_settings],
        ).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-teacher', 'ws-school-2', 'teacher@sch.io', 'teacher')", ()).await.unwrap();

        // Teacher permission check
        assert!(check_permission(&conn, "u-school-teacher", "can_manage_grades").await.unwrap());
        assert!(!check_permission(&conn, "u-school-teacher", "can_access_health_records").await.unwrap());

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-school-2'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-school-2'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_has_health_access_scenarios() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-school-3', 'School WS 3', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-nurse-3', 'ws-school-3', 'nurse3@sch.io', 'nurse')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-student-3', 'ws-school-3', 'student3@sch.io', 'student')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-parent-3', 'ws-school-3', 'parent3@sch.io', 'parent')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-stranger-3', 'ws-school-3', 'stranger3@sch.io', 'teacher')", ()).await.unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('student-prof-3', 'ws-school-3', 'u-school-student-3', 'Bob', 'Smith', 'Grade 5', 0)",
            (),
        ).await.unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id) VALUES ('student-prof-3', 'u-school-parent-3')",
            (),
        ).await.unwrap();

        // 1. Nurse has access
        assert!(has_health_access(&conn, "u-school-nurse-3", "student-prof-3").await.unwrap());

        // 2. Student has access to self
        assert!(has_health_access(&conn, "u-school-student-3", "student-prof-3").await.unwrap());

        // 3. Parent has access to child
        assert!(has_health_access(&conn, "u-school-parent-3", "student-prof-3").await.unwrap());

        // 4. Stranger (unauthorized teacher) has no access
        assert!(!has_health_access(&conn, "u-school-stranger-3", "student-prof-3").await.unwrap());

        // Cleanup
        conn.execute("DELETE FROM student_parents WHERE student_id = 'student-prof-3'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE id = 'student-prof-3'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-school-3'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-school-3'", ()).await.unwrap();
    }
}
