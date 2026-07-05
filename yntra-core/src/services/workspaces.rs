use crate::database;
use crate::observer::notify_observers;
use crate::{Workspace, YntraError};

#[uniffi::export]
pub async fn get_workspace() -> Result<Workspace, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare("SELECT id, name, modules_active, settings, brand_color, logo_url, block_settings FROM workspaces LIMIT 1").await?;

    let mut rows = stmt.query(()).await?;
    if let Some(row) = rows.next().await? {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            modules_active: row.get(2)?,
            settings: row.get(3)?,
            brand_color: row.get(4)?,
            logo_url: row.get(5)?,
            block_settings: row.get(6)?,
        })
    } else {
        Err(YntraError::NotFoundError("No workspace found".to_string()))
    }
}

#[uniffi::export]
pub async fn update_workspace_modules(requester_user_id: String, workspace_id: String, modules_json: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let modules_val: serde_json::Value = serde_json::from_str(&modules_json).unwrap_or_default();
    let reset_roles = modules_val.get("reset_roles").and_then(|v| v.as_bool()).unwrap_or(false);

    conn.execute(
        "UPDATE workspaces SET modules_active = ?1 WHERE id = ?2",
        crate::params![modules_json, workspace_id],
    ).await?;

    if reset_roles {
        let default_settings = get_default_settings_for_modules(&modules_json);
        conn.execute(
            "UPDATE workspaces SET settings = ?1 WHERE id = ?2",
            crate::params![default_settings, workspace_id],
        ).await?;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_workspaces() -> Result<Vec<Workspace>, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare("SELECT id, name, modules_active, settings, brand_color, logo_url, block_settings FROM workspaces").await?;

    let list = stmt.query_map((), |row| {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            modules_active: row.get(2)?,
            settings: row.get(3)?,
            brand_color: row.get(4)?,
            logo_url: row.get(5)?,
            block_settings: row.get(6)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_workspace_template_type(workspace_id: String) -> Result<crate::WorkspaceTemplateType, YntraError> {
    let conn = database::acquire_connection().await?;
    let modules_json: Option<String> = conn.query_row(
        "SELECT modules_active FROM workspaces WHERE id = ?1",
        crate::params![workspace_id],
        |row| row.get(0)
    ).await.ok();

    if let Some(modules_json) = modules_json {
        let modules_val: serde_json::Value = serde_json::from_str(&modules_json).unwrap_or_default();
        let is_school = modules_val.get("school").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("academics").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("attendance").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("finance").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("library").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("timetable").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_assistance = modules_val.get("assistance").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("journals").and_then(|v| v.as_bool()).unwrap_or(false)
            || modules_val.get("medications").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_moving_company = modules_val.get("moving_company").and_then(|v| v.as_bool()).unwrap_or(false);

        if is_school {
            Ok(crate::WorkspaceTemplateType::School)
        } else if is_moving_company {
            Ok(crate::WorkspaceTemplateType::MovingCompany)
        } else if is_assistance {
            Ok(crate::WorkspaceTemplateType::Care)
        } else {
            Ok(crate::WorkspaceTemplateType::General)
        }
    } else {
        Ok(crate::WorkspaceTemplateType::General)
    }
}

fn get_default_settings_for_modules(modules_json: &str) -> String {
    let modules_val: serde_json::Value = serde_json::from_str(modules_json).unwrap_or_default();
    let is_school = modules_val.get("school").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("academics").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("attendance").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("finance").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("library").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("timetable").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_assistance = modules_val.get("assistance").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("journals").and_then(|v| v.as_bool()).unwrap_or(false)
        || modules_val.get("medications").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_moving_company = modules_val.get("moving_company").and_then(|v| v.as_bool()).unwrap_or(false);
    let care_subtype = modules_val.get("care_subtype").and_then(|v| v.as_str()).unwrap_or("aldreomsorg");
    let locale = modules_val.get("locale").and_then(|v| v.as_str()).unwrap_or("se");
    let is_scandi = locale.starts_with("se") || locale.starts_with("no") || locale.starts_with("dk");

    let mut settings_map = serde_json::Map::new();
    
    if is_school {
        let school_roles = super::role_templates::get_school_roles(is_scandi);
        settings_map.insert("roles".to_string(), school_roles);
    } else if is_moving_company {
        let moving_roles = super::role_templates::get_moving_company_roles(is_scandi);
        settings_map.insert("roles".to_string(), moving_roles);
    } else if is_assistance {
        let care_roles = super::role_templates::get_care_roles(care_subtype, is_scandi);
        settings_map.insert("roles".to_string(), care_roles);
    }

    serde_json::to_string(&serde_json::Value::Object(settings_map)).unwrap_or_else(|_| "{}".to_string())
}

#[uniffi::export]
pub async fn create_workspace_via_hub(
    name: String,
    admin_email: String,
    modules_active: String,
) -> Result<(), YntraError> {
    let ws_id = uuid::Uuid::new_v4().to_string();
    let emails: Vec<&str> = admin_email
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let default_settings = get_default_settings_for_modules(&modules_active);

    let conn = database::acquire_connection().await?;

    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;

    let res = async {
        // 1. Insert Workspace
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings, brand_color, logo_url, block_settings) VALUES (?1, ?2, ?3, ?4, '#3b82f6', NULL, '{}')",
            crate::params![&ws_id, &name, &modules_active, &default_settings],
        ).await?;

        // 2. Insert Admin Users
        for email in emails {
            let user_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO users (id, workspace_id, email, full_name, role, preferences) VALUES (?1, ?2, ?3, 'Administrator', 'admin', '{}')",
                crate::params![&user_id, &ws_id, &email.to_string()],
            ).await?;
        }
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.execute("COMMIT", ()).await?;
            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn delete_workspace_via_hub(requester_user_id: String, workspace_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" {
        return Err(YntraError::AuthError("Access denied: platform administrator privileges required".to_string()));
    }

    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;

    let res = async {
        // Delete invitations
        conn.execute("DELETE FROM invitations WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete move inventory & move quotes (joined via job_tickets)
        conn.execute("DELETE FROM move_inventory WHERE job_ticket_id IN (SELECT id FROM job_tickets WHERE workspace_id = ?1)", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM move_quotes WHERE job_ticket_id IN (SELECT id FROM job_tickets WHERE workspace_id = ?1)", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM job_tickets WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete client medications & client journals (joined via clients)
        conn.execute("DELETE FROM client_medications WHERE client_id IN (SELECT id FROM clients WHERE workspace_id = ?1)", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM client_journals WHERE client_id IN (SELECT id FROM clients WHERE workspace_id = ?1)", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM clients WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete team members & events & notes (joined via teams or workspace_id)
        conn.execute("DELETE FROM team_members WHERE team_id IN (SELECT id FROM teams WHERE workspace_id = ?1) OR workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM events WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM notes WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM teams WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete messages & time reports & reports & todos
        conn.execute("DELETE FROM messages WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM time_reports WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM reports WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM todos WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete school child tables first
        conn.execute("DELETE FROM student_parents WHERE student_id IN (SELECT id FROM student_profiles WHERE workspace_id = ?1)", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM submissions WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM attendance_records WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM term_grades WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM report_cards WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM health_records WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM health_incidents WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM school_payments WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM school_invoices WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM library_books WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM timetable_slots WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM assignments WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete all users in that workspace
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete the workspace itself
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![&workspace_id]).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.execute("COMMIT", ()).await?;
            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn update_workspace_general(
    requester_user_id: String,
    workspace_id: String,
    name: String,
    brand_color: String,
    logo_url: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE workspaces SET name = ?1, brand_color = ?2, logo_url = ?3 WHERE id = ?4",
        crate::params![name, brand_color, logo_url, workspace_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_workspace_settings(requester_user_id: String, workspace_id: String, settings_json: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE workspaces SET settings = ?1 WHERE id = ?2",
        crate::params![settings_json, workspace_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_workspace_block_settings(requester_user_id: String, workspace_id: String, block_settings_json: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE workspaces SET block_settings = ?1 WHERE id = ?2",
        crate::params![block_settings_json, workspace_id],
    ).await?;

    notify_observers();
    Ok(())
}
