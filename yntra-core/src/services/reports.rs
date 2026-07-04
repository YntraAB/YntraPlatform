#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{ReportItem, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::{MessageItem, wasm_store};

#[uniffi::export]
pub async fn get_reports(requester_user_id: String) -> Result<Vec<ReportItem>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let user_role: String = conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "user".to_string());

        let is_admin = user_role == "admin" || user_role == "platform_admin";

        let query = if is_admin {
            "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports ORDER BY created_at DESC"
        } else {
            "SELECT id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status FROM reports WHERE user_id = ?1 ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(query).await?;

        let params: Vec<String> = if is_admin { vec![] } else { vec![requester_user_id] };

        let list = stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
            let is_anon_int: i32 = row.get(4)?;
            let ws_id: String = row.get(1)?;
            let raw_content: String = row.get(5)?;
            let decrypted = crate::infra::crypto::decrypt_field(&raw_content, &ws_id);
            
            Ok(ReportItem {
                id: row.get(0)?,
                workspace_id: ws_id,
                user_id: row.get(2)?,
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id);
        let is_admin = requester.map(|u| u.role == "admin" || u.role == "platform_admin").unwrap_or(false);

        let reports_list = if is_admin {
            store.reports.clone()
        } else {
            store
                .reports
                .iter()
                .filter(|r| r.user_id == requester_user_id)
                .cloned()
                .collect()
        };

        let decrypted_list: Vec<ReportItem> = reports_list
            .into_iter()
            .map(|mut r| {
                r.content = crate::infra::crypto::decrypt_field(&r.content, &r.workspace_id);
                r
            })
            .collect();
        Ok(decrypted_list)
    }
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

    let encrypted_content = crate::infra::crypto::encrypt_field(&content_json, &workspace_id);
    let now_ms = crate::infra::time::get_current_time_ms();
    let item = ReportItem {
        id: id.clone(),
        workspace_id,
        user_id: user_id.clone(),
        type_name: report_type.clone(),
        is_anonymous,
        content: encrypted_content.clone(),
        status: "pending".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO reports (id, workspace_id, user_id, type, is_anonymous, content, status, created_at, updated_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, 'pending')",
            crate::params![
                &item.id,
                &item.workspace_id,
                &item.user_id,
                &item.type_name,
                if item.is_anonymous { 1 } else { 0 },
                &item.content,
                &item.created_at,
                &item.updated_at
            ],
        ).await?;

        let mut stmt =
            conn.prepare("SELECT id FROM users WHERE role IN ('admin', 'platform_admin')").await?;
        let admin_ids_iter = stmt.query_map((), |row| row.get::<String>(0)).await?;
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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.reports.push(item.clone());

        let admin_ids: Vec<String> = store
            .users
            .iter()
            .filter(|u| u.role == "admin" || u.role == "platform_admin")
            .map(|u| u.id.clone())
            .collect();

        for admin_id in admin_ids {
            let msg_id = uuid::Uuid::new_v4().to_string();
            let subject = format!("New Report Logged: {}", report_type);
            let body = format!(
                "A new report of type '{}' has been submitted: '{}'. Please review.",
                report_type, subject
            );
            let sender_id_param = if is_anonymous { None } else { Some(user_id.clone()) };
            store.messages.push(MessageItem {
                id: msg_id,
                workspace_id: item.workspace_id.clone(),
                sender_id: sender_id_param,
                receiver_id: Some(admin_id),
                target_team_id: None,
                subject: Some(subject),
                body: Some(body),
                is_read: false,
                created_at: crate::infra::time::get_current_datetime_str(),
                updated_at: now_ms,
                sync_status: "pending".to_string(),
            });
        }

        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn update_report_status(requester_user_id: String, report_id: String, status: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let user_role: String = conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "user".to_string());

        if user_role != "admin" && user_role != "platform_admin" {
            return Err(YntraError::AuthError("Access denied: only admins can update report status".to_string()));
        }

        conn.execute(
            "UPDATE reports SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![status, now_ms, report_id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id);
        let is_admin = requester.map(|u| u.role == "admin" || u.role == "platform_admin").unwrap_or(false);

        if !is_admin {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }

        if let Some(r) = store.reports.iter_mut().find(|r| r.id == report_id) {
            r.status = status;
            r.updated_at = now_ms;
            r.sync_status = "pending".to_string();
        }
        notify_observers();
        Ok(())
    }
}
