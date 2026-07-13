use crate::database;
use crate::observer::notify_observers;
use crate::{ReportItem, YntraError};

#[uniffi::export]
pub async fn get_reports(requester_user_id: String, anonymous_report_ids: Vec<String>) -> Result<Vec<ReportItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    let (query, params) = if auth.role == "platform_admin" {
        (
            "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports ORDER BY created_at DESC".to_string(),
            vec![],
        )
    } else if auth.role == "admin" {
        (
            "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports WHERE workspace_id = ?1 ORDER BY created_at DESC".to_string(),
            vec![ws_id],
        )
    } else {
        if anonymous_report_ids.is_empty() {
            (
                "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports WHERE user_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC".to_string(),
                vec![requester_user_id, ws_id],
            )
        } else {
            let placeholders: Vec<String> = (0..anonymous_report_ids.len()).map(|i| format!("?{}", i + 3)).collect();
            let query = format!(
                "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports WHERE (user_id = ?1 OR (id IN ({}) AND is_anonymous = 1)) AND workspace_id = ?2 ORDER BY created_at DESC",
                placeholders.join(", ")
            );
            let mut p = vec![requester_user_id, ws_id];
            p.extend(anonymous_report_ids);
            (query, p)
        }
    };

    let mut stmt = conn.prepare(&query).await?;

    let mut ciphers: std::collections::HashMap<String, crate::infra::crypto::WorkspaceCipher> = std::collections::HashMap::new();

    let list = stmt.query_map(crate::rusqlite::params_from_iter(params), move |row| {
        let is_anon_int: i32 = row.get(4)?;
        let ws_id: String = row.get(1)?;
        let raw_content: String = row.get(5)?;
        
        if !ciphers.contains_key(&ws_id) {
            let c = crate::infra::crypto::WorkspaceCipher::new(&ws_id)?;
            ciphers.insert(ws_id.clone(), c);
        }
        let cipher = ciphers.get(&ws_id).unwrap();
        let decrypted = cipher.decrypt(&raw_content).unwrap_or(raw_content);
        
        let user_id: String = if is_anon_int != 0 {
            "anonymous".to_string()
        } else {
            row.get(2)?
        };

        Ok(ReportItem {
            id: row.get(0)?,
            workspace_id: ws_id,
            user_id,
            type_name: row.get(3)?,
            is_anonymous: is_anon_int != 0,
            content: decrypted,
            status: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_report(
    workspace_id: String,
    user_id: String,
    report_type: String,
    is_anonymous: bool,
    subject: String,
    description: String,
    date_of_incident: String,
) -> Result<ReportItem, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let content_json = serde_json::json!({
        "subject": subject,
        "description": description,
        "date_of_incident": date_of_incident,
    })
    .to_string();

    let encrypted_content = crate::infra::crypto::encrypt_field(&content_json, &workspace_id)?;
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    let db_user_id = if is_anonymous {
        let anon_user_id = format!("anonymous_{}", workspace_id);
        conn.execute(
            "INSERT OR IGNORE INTO users (id, workspace_id, email, role, preferences, updated_at, sync_status)
             VALUES (?1, ?2, 'anonymous@yntra.io', 'anonymous', '{}', ?3, 'synced')",
            crate::params![&anon_user_id, &workspace_id, &now_ms],
        ).await?;
        anon_user_id
    } else {
        user_id.clone()
    };

    let item = ReportItem {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        user_id: if is_anonymous { "anonymous".to_string() } else { user_id.clone() },
        type_name: report_type.clone(),
        is_anonymous,
        content: encrypted_content.clone(),
        status: "pending".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO reports (id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, 'pending')",
        crate::params![
            &item.id,
            &item.workspace_id,
            &db_user_id,
            &item.type_name,
            if item.is_anonymous { 1 } else { 0 },
            &item.content,
            &item.created_at,
            &item.updated_at
        ],
    ).await?;

    let mut stmt =
        conn.prepare("SELECT id FROM users WHERE role = 'platform_admin' OR (role = 'admin' AND workspace_id = ?1)").await?;
    let admin_ids_iter = stmt.query_map(crate::params![&workspace_id], |row| row.get::<String>(0)).await?;
    let mut admin_ids = Vec::new();
    for admin_id in admin_ids_iter {
        admin_ids.push(admin_id);
    }

    for admin_id in admin_ids {
        let msg_id = uuid::Uuid::new_v4().to_string();
        let msg_created_at = crate::infra::time::get_current_datetime_str();
        let subject = format!("New Report Logged: {}", report_type);
        let body = format!(
            "A new report of type '{}' has been submitted: '{}'. Please review.",
            report_type, subject
        );

        let sender_id_param = if is_anonymous { None } else { Some(user_id.clone()) };
        let _ = conn.execute(
            "INSERT INTO messages (id, workspace_id, sender_id, receiver_id, target_team_id, subject, body, is_read, created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, 0, ?7, ?8, 'pending')",
            crate::params![
                &msg_id,
                &item.workspace_id,
                &sender_id_param,
                &admin_id,
                &subject,
                &body,
                &msg_created_at,
                &now_ms
            ],
        ).await;
    }

    notify_observers();
    Ok(item)
}

#[uniffi::export]
pub async fn update_report_status(requester_user_id: String, report_id: String, status: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "admin" && auth.role != "platform_admin" {
        return Err(YntraError::AuthError("Access denied: only admins can update report status".to_string()));
    }

    let report_ws: String = conn.query_row(
        "SELECT workspace_id FROM reports WHERE id = ?1",
        crate::params![&report_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Report not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != report_ws {
        return Err(YntraError::AuthError("Access denied: report belongs to a different workspace".to_string()));
    }

    conn.execute(
        "UPDATE reports SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![status, now_ms, report_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_report_scoping_and_anonymity() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Clear existing dirty state from previous failed runs
        let _ = conn.execute("DELETE FROM reports WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await;
        let _ = conn.execute("DELETE FROM messages WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('ws-rep-1', 'ws-rep-2')", ()).await;

        // 1. Setup workspace structure and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rep-1', 'Rep WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rep-2', 'Rep WS 2', '[]', '{}')", ()).await.unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rep-padmin', 'ws-rep-1', 'padmin@rep.io', 'platform_admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rep-admin1', 'ws-rep-1', 'admin1@rep.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rep-user1', 'ws-rep-1', 'user1@rep.io', 'user')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-rep-user2', 'ws-rep-2', 'user2@rep.io', 'user')", ()).await.unwrap();

        // Initialize key/salt for decrypting reports
        crate::infra::crypto::set_session_key("rep-test-session-key".to_string().into_bytes());

        // 2. Add reports
        let r1 = add_report(
            "ws-rep-1".to_string(),
            "u-rep-user1".to_string(),
            "whistleblow".to_string(),
            false, // not anonymous
            "Subject 1".to_string(),
            "Desc 1".to_string(),
            "2026-07-05".to_string(),
        ).await.unwrap();

        let r2 = add_report(
            "ws-rep-1".to_string(),
            "u-rep-user1".to_string(),
            "whistleblow".to_string(),
            true, // anonymous
            "Subject 2".to_string(),
            "Desc 2".to_string(),
            "2026-07-05".to_string(),
        ).await.unwrap();

        // Verify dummy user has 'anonymous' role
        let dummy_role: String = conn.query_row(
            "SELECT role FROM users WHERE id = 'anonymous_ws-rep-1'",
            (),
            |row| row.get(0),
        ).await.unwrap();
        assert_eq!(dummy_role, "anonymous");

        let r3 = add_report(
            "ws-rep-2".to_string(),
            "u-rep-user2".to_string(),
            "compliance".to_string(),
            false,
            "Subject 3".to_string(),
            "Desc 3".to_string(),
            "2026-07-05".to_string(),
        ).await.unwrap();

        // 3. Verify get_reports scoping:

        // A. Standard user 1 (should only see their own, and for r2 (anonymous), the author is still "anonymous" to them or they see it)
        // Wait, standard user query: WHERE user_id = requester_user_id.
        // Wait, r2 is anonymous, but in database user_id = 'u-rep-user1'. So they see both r1 and r2!
        let list_user = get_reports("u-rep-user1".to_string(), vec![r2.id.clone()]).await.unwrap();
        assert_eq!(list_user.len(), 2);
        // r2 is anonymous, so its returned user_id is masked to "anonymous"
        let anon_rep = list_user.iter().find(|r| r.id == r2.id).unwrap();
        assert_eq!(anon_rep.user_id, "anonymous");
        assert!(anon_rep.content.contains("Desc 2")); // Decrypted successfully

        let non_anon_rep = list_user.iter().find(|r| r.id == r1.id).unwrap();
        assert_eq!(non_anon_rep.user_id, "u-rep-user1");

        // B. Admin 1 (should see all reports in ws-rep-1, meaning r1 and r2, but not r3)
        let list_admin = get_reports("u-rep-admin1".to_string(), vec![]).await.unwrap();
        assert_eq!(list_admin.len(), 2);
        assert!(list_admin.iter().any(|r| r.id == r1.id));
        assert!(list_admin.iter().any(|r| r.id == r2.id));
        assert!(!list_admin.iter().any(|r| r.id == r3.id));

        // C. Platform admin (should see all reports across workspaces: r1, r2, r3)
        let list_padmin = get_reports("u-rep-padmin".to_string(), vec![]).await.unwrap();
        assert_eq!(list_padmin.len(), 3);

        // Cleanup
        conn.execute("DELETE FROM reports WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await.unwrap();
        conn.execute("DELETE FROM messages WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-rep-1', 'ws-rep-2')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-rep-1', 'ws-rep-2')", ()).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }

    #[tokio::test]
    async fn test_report_idor_prevention() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = "ws-rep-idor";
        let user1_id = "u-rep-user-1";
        let user2_id = "u-rep-user-2";

        // Setup workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'IDOR WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'user1@rep.io', 'user')", crate::params![user1_id, ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'user2@rep.io', 'user')", crate::params![user2_id, ws_id]).await.unwrap();

        crate::infra::crypto::set_session_key("idor-test-session".to_string().into_bytes());

        // User 2 logs a standard non-anonymous report
        let rep_user2 = add_report(
            ws_id.to_string(),
            user2_id.to_string(),
            "compliance".to_string(),
            false, // non-anonymous
            "User 2 Report".to_string(),
            "Sensitive user 2 details".to_string(),
            "2026-07-05".to_string(),
        ).await.unwrap();

        // User 1 tries to retrieve User 2's report by specifying its UUID in anonymous_report_ids
        let retrieved = get_reports(user1_id.to_string(), vec![rep_user2.id.clone()]).await.unwrap();

        // The returned list must NOT contain User 2's non-anonymous report
        assert!(retrieved.is_empty() || !retrieved.iter().any(|r| r.id == rep_user2.id));

        // Clean up
        conn.execute("DELETE FROM reports WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM messages WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
        crate::infra::crypto::clear_session_key();
    }
}
