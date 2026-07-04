#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{Workspace, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::{wasm_store, WorkspaceUser};

#[uniffi::export]
pub async fn get_workspace() -> Result<Workspace, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.workspace.clone())
    }
}

#[uniffi::export]
pub async fn update_workspace_modules(workspace_id: String, modules_json: String) -> Result<(), YntraError> {
    let modules_val: serde_json::Value = serde_json::from_str(&modules_json).unwrap_or_default();
    let reset_roles = modules_val.get("reset_roles").and_then(|v| v.as_bool()).unwrap_or(false);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if store.workspace.id == workspace_id {
            store.workspace.modules_active = modules_json.clone();
            if reset_roles {
                let default_settings = get_default_settings_for_modules(&modules_json);
                store.workspace.settings = default_settings;
            }
        }
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn get_workspaces() -> Result<Vec<Workspace>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(vec![store.workspace.clone()])
    }
}

#[uniffi::export]
pub async fn get_workspace_template_type(workspace_id: String) -> Result<crate::WorkspaceTemplateType, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        if store.workspace.id == workspace_id {
            let modules_val: serde_json::Value = serde_json::from_str(&store.workspace.modules_active).unwrap_or_default();
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

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

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.workspace = Workspace {
            id: ws_id,
            name,
            modules_active,
            settings: default_settings,
            brand_color: "#3b82f6".to_string(),
            logo_url: None,
            block_settings: "{}".to_string(),
        };
        store.users = emails
            .into_iter()
            .map(|email| {
                let user_id = uuid::Uuid::new_v4().to_string();
                let now_ms = crate::infra::time::get_current_time_ms();
                WorkspaceUser {
                    id: user_id,
                    workspace_id: Some(store.workspace.id.clone()),
                    email: email.to_string(),
                    full_name: Some("Administrator".to_string()),
                    phone: None,
                    role: "admin".to_string(),
                    preferences: "{}".to_string(),
                    siths_card_id: None,
                    nfc_badge_uid: None,
                    updated_at: now_ms,
                    sync_status: "pending".to_string(),
                    personal_number: None,
                }
            })
            .collect();
        notify_observers();
        Ok(())
    }
}


#[uniffi::export]
pub async fn delete_workspace_via_hub(workspace_id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        // Delete all users in that workspace
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete the workspace itself
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![&workspace_id]).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = workspace_id;
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn update_workspace_general(
    workspace_id: String,
    name: String,
    brand_color: String,
    logo_url: Option<String>,
) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE workspaces SET name = ?1, brand_color = ?2, logo_url = ?3 WHERE id = ?4",
            crate::params![name, brand_color, logo_url, workspace_id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if store.workspace.id == workspace_id {
            store.workspace.name = name;
            store.workspace.brand_color = brand_color;
            store.workspace.logo_url = logo_url;
        }
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn update_workspace_settings(workspace_id: String, settings_json: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE workspaces SET settings = ?1 WHERE id = ?2",
            crate::params![settings_json, workspace_id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if store.workspace.id == workspace_id {
            store.workspace.settings = settings_json;
        }
        notify_observers();
        Ok(())
    }
}

#[uniffi::export]
pub async fn update_workspace_block_settings(workspace_id: String, block_settings_json: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE workspaces SET block_settings = ?1 WHERE id = ?2",
            crate::params![block_settings_json, workspace_id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if store.workspace.id == workspace_id {
            store.workspace.block_settings = block_settings_json;
        }
        notify_observers();
        Ok(())
    }
}
