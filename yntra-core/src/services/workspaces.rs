use crate::database;
use crate::observer::notify_observers;
use crate::{Workspace, YntraError};

#[uniffi::export]
pub async fn get_workspace(requester_user_id: String) -> Result<Workspace, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare("SELECT id, name, modules_active, settings, brand_color, logo_url, block_settings, updated_at, sync_status FROM workspaces WHERE id = ?1").await?;

    let mut rows = stmt.query(crate::params![&auth.workspace_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            modules_active: row.get(2)?,
            settings: row.get(3)?,
            brand_color: row.get(4)?,
            logo_url: row.get(5)?,
            block_settings: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    } else {
        Err(YntraError::NotFoundError(format!(
            "Workspace '{}' not found",
            auth.workspace_id
        )))
    }
}

#[uniffi::export]
pub async fn update_workspace_modules(
    requester_user_id: String,
    workspace_id: String,
    modules_json: String,
) -> Result<(), YntraError> {
    crate::infra::auth::validate_id(&workspace_id, "Workspace ID")?;
    tracing::info!(
        "update_workspace_modules FFI called: requester_user_id={}, workspace_id={}, modules_json={}",
        requester_user_id,
        workspace_id,
        modules_json
    );
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let modules_val: serde_json::Value = serde_json::from_str(&modules_json).unwrap_or_default();
    let reset_roles = modules_val
        .get("reset_roles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let now_ms = crate::infra::time::get_current_time_ms();
    let res = conn.execute(
        "UPDATE workspaces SET modules_active = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![modules_json, now_ms, workspace_id],
    ).await;
    tracing::debug!("UPDATE workspaces query execution result: {:?}", res);
    res?;

    if reset_roles {
        let default_settings = get_default_settings_for_modules(&modules_json);
        conn.execute(
            "UPDATE workspaces SET settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![default_settings, now_ms, &workspace_id],
        ).await?;
    }

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_workspaces(requester_user_id: String) -> Result<Vec<Workspace>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (sql, params) = if auth.role == "platform_admin" {
        (
            "SELECT id, name, modules_active, settings, brand_color, logo_url, block_settings, updated_at, sync_status FROM workspaces",
            crate::params![],
        )
    } else {
        (
            "SELECT id, name, modules_active, settings, brand_color, logo_url, block_settings, updated_at, sync_status FROM workspaces WHERE id = ?1",
            crate::params![&auth.workspace_id],
        )
    };

    let mut stmt = conn.prepare(sql).await?;

    let list = stmt
        .query_map(params, |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                modules_active: row.get(2)?,
                settings: row.get(3)?,
                brand_color: row.get(4)?,
                logo_url: row.get(5)?,
                block_settings: row.get(6)?,
                updated_at: row.get(7)?,
                sync_status: row.get(8)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_workspace_template_type(
    requester_user_id: String,
    workspace_id: String,
) -> Result<crate::WorkspaceTemplateType, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }
    let modules_json: Option<String> = conn
        .query_row(
            "SELECT modules_active FROM workspaces WHERE id = ?1",
            crate::params![workspace_id],
            |row| row.get(0),
        )
        .await
        .ok();

    if let Some(modules_json) = modules_json {
        let modules_val: serde_json::Value =
            serde_json::from_str(&modules_json).unwrap_or_default();
        let is_school = modules_val
            .get("school")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || modules_val
                .get("academics")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("attendance")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("finance")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("library")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("timetable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let is_assistance = modules_val
            .get("assistance")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || modules_val
                .get("journals")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || modules_val
                .get("medications")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let is_moving_company = modules_val
            .get("moving_company")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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
    let is_assistance = modules_val
        .get("assistance")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || modules_val
            .get("journals")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || modules_val
            .get("medications")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    let is_moving_company = modules_val
        .get("moving_company")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_school = modules_val
        .get("school")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || modules_val
            .get("academics")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || modules_val
            .get("attendance")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || modules_val
            .get("finance")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || modules_val
            .get("library")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || modules_val
            .get("timetable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    let care_subtype = modules_val
        .get("care_subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("aldreomsorg");
    let locale = modules_val
        .get("locale")
        .and_then(|v| v.as_str())
        .unwrap_or("se");
    let is_scandi =
        locale.starts_with("se") || locale.starts_with("no") || locale.starts_with("dk");

    let mut settings_map = serde_json::Map::new();

    if is_moving_company {
        let moving_roles = super::role_templates::get_moving_company_roles(is_scandi);
        settings_map.insert("roles".to_string(), moving_roles);

        // Seed operational defaults from definitions and regional defaults
        let defs = get_workspace_settings_definitions();
        for def in defs {
            let val = match def.value_type.as_str() {
                "currency" | "number" | "percentage" | "multiplier" => {
                    if let Ok(n) = def.default_value.parse::<f64>() {
                        serde_json::json!(n)
                    } else {
                        serde_json::Value::String(def.default_value)
                    }
                }
                "boolean" => {
                    if let Ok(b) = def.default_value.parse::<bool>() {
                        serde_json::Value::Bool(b)
                    } else {
                        serde_json::Value::String(def.default_value)
                    }
                }
                _ => serde_json::Value::String(def.default_value),
            };
            settings_map.insert(def.key, val);
        }

        settings_map.insert(
            "target_region".to_string(),
            serde_json::Value::String(if is_scandi {
                "SE".to_string()
            } else {
                "US".to_string()
            }),
        );
        settings_map.insert(
            "currency".to_string(),
            serde_json::Value::String(if is_scandi {
                "SEK".to_string()
            } else {
                "USD".to_string()
            }),
        );
        settings_map.insert(
            "use_rut_deduction".to_string(),
            serde_json::Value::Bool(is_scandi),
        );
        settings_map.insert(
            "annual_rut_limit_per_person".to_string(),
            serde_json::json!(75000.0),
        );
    } else if is_assistance {
        let care_roles = super::role_templates::get_care_roles(care_subtype, is_scandi);
        settings_map.insert("roles".to_string(), care_roles);
    } else if is_school {
        let school_roles = super::role_templates::get_school_roles(is_scandi);
        settings_map.insert("roles".to_string(), school_roles);
    }

    serde_json::to_string(&serde_json::Value::Object(settings_map))
        .unwrap_or_else(|_| "{}".to_string())
}

#[uniffi::export]
pub async fn create_workspace_via_hub(
    requester_user_id: String,
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
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" {
        return Err(YntraError::AuthError(
            "Access denied: platform administrator privileges required".to_string(),
        ));
    }

    conn.begin_transaction().await?;

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

            crate::services::users::ensure_user_role_signature(&conn, &user_id, "admin", &ws_id).await?;
        }
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.rollback().await;
            // Clean up keyring key to avoid orphaned private key on rollback
            let private_key_setting = format!("creator_private_key_{}", ws_id);
            let _ = crate::infra::crypto::set_local_secret(&private_key_setting, "").await;
            Err(e)
        }
    }
}

#[uniffi::export]
pub async fn delete_workspace_via_hub(
    requester_user_id: String,
    workspace_id: String,
) -> Result<(), YntraError> {
    crate::infra::auth::validate_id(&workspace_id, "Workspace ID")?;
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" {
        return Err(YntraError::AuthError(
            "Access denied: platform administrator privileges required".to_string(),
        ));
    }

    conn.begin_transaction().await?;

    let res = async {
        // Delete invitations
        conn.execute("DELETE FROM invitations WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;


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

        // Delete messages & time reports & reports & todos & entities
        conn.execute("DELETE FROM messages WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM time_reports WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM reports WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM todos WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;
        conn.execute("DELETE FROM entities WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;



        // Delete all users in that workspace
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![&workspace_id]).await?;

        // Delete the workspace itself
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![&workspace_id]).await?;
        Ok(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            // Clean up all keyring credentials associated with this workspace on successful deletion
            let _ = crate::infra::crypto::set_local_secret(
                &format!("creator_private_key_{}", workspace_id),
                "",
            )
            .await;
            let _ = crate::infra::crypto::set_local_secret(
                &format!("workspace_public_key_{}", workspace_id),
                "",
            )
            .await;
            let _ = crate::infra::crypto::set_local_secret(
                &format!("workspace_key_{}", workspace_id),
                "",
            )
            .await;
            let _ = crate::infra::crypto::set_local_secret(
                &format!("workspace_auth_epoch_{}", workspace_id),
                "",
            )
            .await;
            notify_observers();
            Ok(())
        }
        Err(e) => {
            let _ = conn.rollback().await;
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
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE workspaces SET name = ?1, brand_color = ?2, logo_url = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![name, brand_color, logo_url, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_workspace_settings(
    requester_user_id: String,
    workspace_id: String,
    settings_json: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE workspaces SET settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![settings_json, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn set_workspace_zk_proof_policy(
    requester_user_id: String,
    workspace_id: String,
    require_zk_proofs: bool,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let existing_settings: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());

    let mut val: serde_json::Value = serde_json::from_str(&existing_settings)
        .unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = val.as_object_mut() {
        obj.insert("require_zk_proofs".to_string(), serde_json::Value::Bool(require_zk_proofs));
        obj.insert("bypass_zk_proofs".to_string(), serde_json::Value::Bool(!require_zk_proofs));
    }

    let updated_json = val.to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE workspaces SET settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![updated_json, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn set_workspace_p2p_isolation_policy(
    requester_user_id: String,
    workspace_id: String,
    disable_p2p_mesh: bool,
    enforce_hipaa_ferpa_dlp: bool,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let existing_settings: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());

    let mut val: serde_json::Value = serde_json::from_str(&existing_settings)
        .unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = val.as_object_mut() {
        obj.insert("disable_p2p_mesh".to_string(), serde_json::Value::Bool(disable_p2p_mesh));
        obj.insert("enforce_hipaa_ferpa_dlp".to_string(), serde_json::Value::Bool(enforce_hipaa_ferpa_dlp));
        obj.insert("compliance_mode".to_string(), serde_json::Value::String(
            if disable_p2p_mesh { "StrictServerOnly".to_string() } else { "AuditedLocalP2P".to_string() }
        ));
    }

    let updated_json = val.to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE workspaces SET settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![updated_json, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_workspace_block_settings(
    requester_user_id: String,
    workspace_id: String,
    block_settings_json: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE workspaces SET block_settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![block_settings_json, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub fn get_workspace_settings_definitions() -> Vec<crate::models::SettingDefinition> {
    vec![
        // 0. AI & Guardrail Settings (BYOK)
        crate::models::SettingDefinition {
            key: "ai_provider".to_string(),
            label: "AI Provider Strategy / AI-leverantörsstrategi".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "select".to_string(),
            default_value: "local_ast".to_string(),
            tooltip: "Choose between Local AST Engine or Cloud BYOK providers (OpenAI, Anthropic, Gemini).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_api_key".to_string(),
            label: "Workspace BYOK API Key / BYOK API-nyckel".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "password".to_string(),
            default_value: "".to_string(),
            tooltip: "Cloud provider API key (sk-...) used for all employees in this workspace.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_model_name".to_string(),
            label: "Model Name / Modellnamn".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "string".to_string(),
            default_value: "gpt-4o-mini".to_string(),
            tooltip: "Selected LLM model identifier (e.g. gpt-4o-mini, claude-3-5-sonnet, gemini-1.5-flash).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_guardrails_enabled".to_string(),
            label: "AI Guardrails Active / AI-skyddsbarriärer aktiva".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "boolean".to_string(),
            default_value: "true".to_string(),
            tooltip: "Enforce deterministic execution limits and automated risk bounds on AI actions.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_max_allowed_risk".to_string(),
            label: "Maximum Risk Tolerance / Maximal risktolerans".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "select".to_string(),
            default_value: "medium".to_string(),
            tooltip: "Maximum risk level permitted for automated AI actions (low, medium, high).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_require_human_approval_above_hours".to_string(),
            label: "Human Approval Hours Limit / Gräns för manuellt godkännande (timmar)".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "number".to_string(),
            default_value: "8.0".to_string(),
            tooltip: "Shifts logged exceeding this number of hours require mandatory human attestation.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "ai_min_auto_approve_confidence".to_string(),
            label: "Minimum Auto-Approve Confidence / Minsta tillförlitlighet för autogodkännande".to_string(),
            category: "AI & Guardrail Settings".to_string(),
            value_type: "number".to_string(),
            default_value: "0.90".to_string(),
            tooltip: "Confidence threshold required before allowing automated AutoApprove actions.".to_string(),
        },

        // 1. Staircase Multipliers
        crate::models::SettingDefinition {
            key: "mult_spiral_staircase".to_string(),
            label: "Spiral Staircase Multiplier / Spiraltrapp-multiplikator".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.5".to_string(),
            tooltip: "Multiplies standard floor carrying fees for spiral staircases due to tight turning radius and increased physical exertion. Example: 1.5x increases 300 kr/floor to 450 kr/floor. Set to 1.0 for 0% surcharge.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "mult_narrow_staircase".to_string(),
            label: "Narrow Staircase Multiplier / Trång trapp-multiplikator".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.3".to_string(),
            tooltip: "Applies to narrow stairwells (< 1.1m width) requiring careful tilt maneuver. Set to 1.25 or 1.0 if your firm waives narrow staircase fees.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "mult_outdoor_staircase".to_string(),
            label: "Outdoor Staircase Multiplier / Utomhustrapp-multiplikator".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.2".to_string(),
            tooltip: "Surcharge multiplier for exposed outdoor stone/metal staircases subject to weather or steep slope.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_stairs_surcharge_per_floor".to_string(),
            label: "Per-Floor Stair Surcharge / Trappavgift per våning".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "300.0".to_string(),
            tooltip: "Base cost in SEK added per floor when no working elevator is available. Multiplied by staircase type multiplier if applicable.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_small_elevator".to_string(),
            label: "Small Elevator Constraint Fee / Trång hiss-tillägg".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "500.0".to_string(),
            tooltip: "Flat surcharge when elevator dimensions (< 4-person capacity) force movers to carry bulky furniture via stairs instead.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_long_carry_per_meter".to_string(),
            label: "Long Carry Fee per Meter / Långbäring per meter".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "40.0".to_string(),
            tooltip: "Surcharge per meter for carrying distance exceeding 20m from parking spot to building entrance. Set to 0.0 for free long carries.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_no_parking_zone".to_string(),
            label: "No Parking / Permit Fee / Parkeringstillståndstillägg".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "400.0".to_string(),
            tooltip: "Administrative fee for securing municipal parking permits or hazard risk allowance in strict loading zones.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_shuttle_truck_needed".to_string(),
            label: "Shuttle Van Fee / Omlastningsbil (Shuttle)".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "1800.0".to_string(),
            tooltip: "Fee charged when large 18t truck cannot access location (e.g. Gamla Stan archways), requiring secondary shuttle van transfer.".to_string(),
        },

        // 2. Temporal & Shift Multipliers
        crate::models::SettingDefinition {
            key: "moving_weekend_multiplier".to_string(),
            label: "Weekend Multiplier / Helgmultiplikator".to_string(),
            category: "Temporal & Shift Multipliers".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.25".to_string(),
            tooltip: "Labor rate multiplier applied for Saturday/Sunday relocations. Set to 1.0 if your moving firm charges flat weekend rates.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_peak_season_multiplier".to_string(),
            label: "Peak Season / Month-End Surge Rate / Månadsskiftes-multiplikator".to_string(),
            category: "Temporal & Shift Multipliers".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.15".to_string(),
            tooltip: "Surge multiplier applied during high-demand month-end date windows (25th to 3rd of each month).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_overtime_multiplier".to_string(),
            label: "After-Hours Overtime Rate / Övertids-multiplikator".to_string(),
            category: "Temporal & Shift Multipliers".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "1.5".to_string(),
            tooltip: "Overtime rate multiplier applied to evening shifts starting after 18:00 or exceeding 8 daily hours.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_holiday_multiplier".to_string(),
            label: "Public Holiday Rate / Röd dag-multiplikator".to_string(),
            category: "Temporal & Shift Multipliers".to_string(),
            value_type: "multiplier".to_string(),
            default_value: "2.0".to_string(),
            tooltip: "Rate multiplier for official national holidays / red days (*röda dagar*).".to_string(),
        },

        // 3. Specialty Item Surcharges
        crate::models::SettingDefinition {
            key: "surcharge_piano".to_string(),
            label: "Piano / Flygel Heavy Lifting Fee".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "1500.0".to_string(),
            tooltip: "Flat surcharge for upright pianos, grand pianos (*flygel*), organs, or heavy musical instruments.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_safe".to_string(),
            label: "Safe / Kassaskåp Heavy Vault Fee".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "2000.0".to_string(),
            tooltip: "Flat surcharge for gun safes, fireproof vaults (*kassaskåp*), or heavy machinery > 150 kg.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_jacuzzi".to_string(),
            label: "Jacuzzi / Spabad / Sauna Fee".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "2500.0".to_string(),
            tooltip: "Flat surcharge for hot tubs, jacuzzis, outdoor spas (*spabad*), or saunas.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_fragile".to_string(),
            label: "Fine Art / Fragile Care Fee / Konsthanteringstillägg".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "500.0".to_string(),
            tooltip: "Handling fee for delicate paintings, sculptures, marble tops, or crystal chandeliers requiring custom crating.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_server_rack".to_string(),
            label: "B2B Server Rack / IT Equipment Fee".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "3000.0".to_string(),
            tooltip: "B2B commercial fee for moving heavy IT server racks, battery UPS banks, or sensitive data center gear.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_fitness_equipment".to_string(),
            label: "Heavy Fitness Equipment Fee / Träningsredskapstilägg".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "800.0".to_string(),
            tooltip: "Surcharge for heavy treadmills (*löpband*), rowing machines, or commercial gym multi-stations.".to_string(),
        },

        // 4. Billing, Tariffs & Deposit Gates
        crate::models::SettingDefinition {
            key: "moving_pricing_model".to_string(),
            label: "Core Pricing Model / Prissättningsmodell".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "select".to_string(),
            default_value: "volume".to_string(),
            tooltip: "Choose between 'volume' (fixed rate per m³) or 'hourly' (crew size x hourly labor rate + vehicle fee).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_hourly_rate_per_mover".to_string(),
            label: "Hourly Rate per Mover / Timpris per flyttkarl".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "400.0".to_string(),
            tooltip: "Hourly rate charged per active mover on duty.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_hourly_rate_vehicle".to_string(),
            label: "Hourly Vehicle Fee / Timpris per flyttbil".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "400.0".to_string(),
            tooltip: "Hourly rate charged for the moving truck/van.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_minimum_hours".to_string(),
            label: "Minimum Billable Hours / Minimidebitering (timmar)".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "number".to_string(),
            default_value: "3.0".to_string(),
            tooltip: "Minimum billable hours threshold (e.g. 3.0 hours minimum) applied to hourly moving jobs.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_deposit_percent".to_string(),
            label: "Non-Refundable Deposit % / Handpenning (%)".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "percentage".to_string(),
            default_value: "20.0".to_string(),
            tooltip: "Percentage of quote total required as a non-refundable deposit upon customer quote acceptance.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_payment_due_days".to_string(),
            label: "Payment Due Terms (Days) / Betalningsvillkor (Dagar)".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "number".to_string(),
            default_value: "30.0".to_string(),
            tooltip: "Number of days from invoice generation date until payment is due (e.g. 30, 14, 7, or 0 days upon job completion).".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_base_rate_per_m3".to_string(),
            label: "Base Rate per m³ / Grundpris per m³".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "500.0".to_string(),
            tooltip: "Base volumetric tariff rate charged per cubic meter under volume pricing model.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_distance_fee_flat".to_string(),
            label: "Base Distance Fee / Grundavgift sträcka".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "800.0".to_string(),
            tooltip: "Flat base distance/transport fee for relocations within the local radius.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_packing_supplies_fee_per_m3".to_string(),
            label: "Packing Supplies Fee per m³ / Emballageavgift per m³".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "100.0".to_string(),
            tooltip: "Estimated packing materials supply fee per cubic meter when no itemized supplies are logged.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_hourly_rate".to_string(),
            label: "Combined Hourly Rate / Sammanlagt timpris".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "1200.0".to_string(),
            tooltip: "Flat combined hourly labor and vehicle rate when explicit mover/vehicle breakdown is disabled.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_default_crew_size".to_string(),
            label: "Default Crew Size / Standard lagstorlek".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "number".to_string(),
            default_value: "2.0".to_string(),
            tooltip: "Default active crew size assigned to jobs when unassigned.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_hours_per_m3".to_string(),
            label: "Labor Hours per m³ / Arbetstimmar per m³".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "number".to_string(),
            default_value: "0.15".to_string(),
            tooltip: "Estimated labor hours required per m³ of move volume.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_local_radius_km".to_string(),
            label: "Local Service Radius (km) / Lokalområdesradie".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "number".to_string(),
            default_value: "30.0".to_string(),
            tooltip: "Radius in kilometers covered by the flat distance base fee before distance per-km charges apply.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "moving_per_km_rate".to_string(),
            label: "Extra Distance Rate per km / Drivmedel/Km-pris".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "15.0".to_string(),
            tooltip: "Per kilometer surcharge for travel distance beyond the local service radius.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_marble_glass".to_string(),
            label: "Marble / Glass Heavy Slab Fee / Marmor & Glasskiva-tillägg".to_string(),
            category: "Specialty Item Surcharges".to_string(),
            value_type: "currency".to_string(),
            default_value: "600.0".to_string(),
            tooltip: "Surcharge for fragile marble tops or large glass dining tabletops requiring blanket/crate protection.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "surcharge_crane_hoist".to_string(),
            label: "Crane Hoist Equipment Fee / Kran/Lyft-utrustningstillägg".to_string(),
            category: "Staircase & Architectural Access".to_string(),
            value_type: "currency".to_string(),
            default_value: "3500.0".to_string(),
            tooltip: "Equipment rental surcharge when external balcony crane/hoist is required.".to_string(),
        },
        crate::models::SettingDefinition {
            key: "annual_rut_limit_per_person".to_string(),
            label: "Annual Individual RUT Tax Limit / Årligt RUT-avdragstak".to_string(),
            category: "Billing & Deposit Gates".to_string(),
            value_type: "currency".to_string(),
            default_value: "75000.0".to_string(),
            tooltip: "Maximum annual Swedish RUT tax deduction cap per person under Skatteverket rules.".to_string(),
        },
    ]
}

pub fn get_setting_f64(settings_json: &serde_json::Value, key: &str) -> f64 {
    if let Some(v) = settings_json.get(key) {
        if let Some(n) = v.as_f64() {
            return n;
        }
        if let Some(s) = v.as_str() {
            if let Ok(n) = s.parse::<f64>() {
                return n;
            }
        }
    }
    for def in get_workspace_settings_definitions() {
        if def.key == key {
            if let Ok(n) = def.default_value.parse::<f64>() {
                return n;
            }
        }
    }
    0.0
}

pub fn get_setting_str(settings_json: &serde_json::Value, key: &str, fallback_str: &str) -> String {
    if let Some(s) = settings_json.get(key).and_then(|v| v.as_str()) {
        return s.to_string();
    }
    for def in get_workspace_settings_definitions() {
        if def.key == key {
            return def.default_value;
        }
    }
    fallback_str.to_string()
}

#[uniffi::export]
pub async fn create_workspace_invitation(
    requester_user_id: String,
    workspace_id: String,
    email: String,
    full_name: String,
    role: String,
) -> Result<crate::models::WorkspaceInvitation, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: only administrators can issue invitations".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let code = format!(
        "INV-{}",
        &uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
    );
    let now_ms = crate::infra::time::get_current_time_ms();
    let invitation = crate::models::WorkspaceInvitation {
        code: code.clone(),
        workspace_id: workspace_id.clone(),
        email: email.trim().to_string(),
        full_name: full_name.trim().to_string(),
        role: role.clone(),
        activated: false,
        metadata: serde_json::json!({
            "invited_by": requester_user_id,
            "created_at_ms": now_ms,
            "expires_at_ms": now_ms + (7 * 24 * 60 * 60 * 1000)
        })
        .to_string(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO invitations (code, workspace_id, email, full_name, role, activated, metadata, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, 'pending')",
        crate::params![
            &invitation.code,
            &invitation.workspace_id,
            &invitation.email,
            &invitation.full_name,
            &invitation.role,
            &invitation.metadata,
            &invitation.updated_at
        ],
    ).await?;

    notify_observers();
    Ok(invitation)
}

#[uniffi::export]
pub async fn get_workspace_invitations(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<crate::models::WorkspaceInvitation>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare("SELECT code, workspace_id, email, full_name, role, activated, metadata, updated_at, sync_status FROM invitations WHERE workspace_id = ?1 ORDER BY updated_at DESC").await?;
    let mut rows = stmt.query(crate::params![&workspace_id]).await?;

    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        let act_num: i64 = row.get(5)?;
        list.push(crate::models::WorkspaceInvitation {
            code: row.get(0)?,
            workspace_id: row.get(1)?,
            email: row.get(2)?,
            full_name: row.get(3)?,
            role: row.get(4)?,
            activated: act_num != 0,
            metadata: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        });
    }

    Ok(list)
}

#[uniffi::export]
pub async fn revoke_workspace_invitation(
    requester_user_id: String,
    workspace_id: String,
    code: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM invitations WHERE code = ?1 AND workspace_id = ?2",
        crate::params![&code, &workspace_id],
    )
    .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn complete_workspace_onboarding(
    requester_user_id: String,
    workspace_id: String,
    onboarding_data_json: String,
) -> Result<Workspace, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required to complete onboarding".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let payload: serde_json::Value = serde_json::from_str(&onboarding_data_json)
        .map_err(|e| YntraError::DbError(format!("Invalid onboarding JSON: {}", e)))?;

    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("My Workspace")
        .to_string();
    let modules_json = payload
        .get("modules_active")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let brand_color = payload
        .get("brand_color")
        .and_then(|v| v.as_str())
        .unwrap_or("hsl(217.2, 91.2%, 59.8%)")
        .to_string();

    let raw_settings: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());

    let mut current_settings: serde_json::Value =
        serde_json::from_str(&raw_settings).unwrap_or(serde_json::json!({}));

    current_settings["onboarding_completed"] = serde_json::Value::Bool(true);
    current_settings["onboarding_completed_at"] =
        serde_json::Value::Number(crate::infra::time::get_current_time_ms().into());
    if let Some(org_type) = payload.get("org_type").and_then(|v| v.as_str()) {
        current_settings["org_type"] = serde_json::Value::String(org_type.to_string());
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let settings_str = current_settings.to_string();

    conn.execute(
        "UPDATE workspaces SET name = ?1, modules_active = ?2, settings = ?3, brand_color = ?4, updated_at = ?5, sync_status = 'pending' WHERE id = ?6",
        crate::params![&name, &modules_json, &settings_str, &brand_color, now_ms, &workspace_id],
    ).await?;

    crate::infra::auth::invalidate_auth_context_cache_for_workspace(&workspace_id);
    notify_observers();

    get_workspace(requester_user_id).await
}

#[uniffi::export]
pub async fn update_user_workspace_role(
    requester_user_id: String,
    workspace_id: String,
    target_user_id: String,
    new_role: String,
) -> Result<crate::WorkspaceUser, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: only administrators can change user roles".to_string(),
        ));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let valid_roles = [
        "platform_admin",
        "admin",
        "manager",
        "member",
        "user",
        "viewer",
        "client",
        "student",
        "parent",
    ];
    if !valid_roles.contains(&new_role.as_str()) {
        return Err(YntraError::AuthError(format!(
            "Invalid role specified: {}",
            new_role
        )));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE users SET role = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3 AND workspace_id = ?4",
        crate::params![&new_role, now_ms, &target_user_id, &workspace_id],
    ).await?;

    crate::services::users::ensure_user_role_signature(
        &conn,
        &target_user_id,
        &new_role,
        &workspace_id,
    )
    .await?;

    notify_observers();

    let user = conn.query_row(
        "SELECT id, workspace_id, email, full_name, phone, role, preferences, metadata, updated_at, sync_status FROM users WHERE id = ?1",
        crate::params![&target_user_id],
        |row| {
            let meta_str: Option<String> = row.get(7)?;
            let mut siths_card_id = None;
            let mut nfc_badge_uid = None;
            let mut personal_number = None;
            let mut public_key = None;
            if let Some(ref m) = meta_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m) {
                    siths_card_id = meta_val.get("siths_card_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    nfc_badge_uid = meta_val.get("nfc_badge_uid").and_then(|v| v.as_str()).map(|s| s.to_string());
                    personal_number = meta_val.get("personal_number").and_then(|v| v.as_str()).map(|s| s.to_string());
                    public_key = meta_val.get("public_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }

            Ok(crate::WorkspaceUser {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                email: row.get(2)?,
                full_name: row.get(3)?,
                phone: row.get(4)?,
                role: row.get(5)?,
                preferences: row.get(6)?,
                siths_card_id,
                nfc_badge_uid,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                personal_number,
                public_key,
            })
        },
    ).await?;

    Ok(user)
}

#[uniffi::export]
pub async fn get_workspace_role_permissions(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<crate::models::WorkspaceRolePermission>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    Ok(vec![
        crate::models::WorkspaceRolePermission {
            role_id: "admin".to_string(),
            role_name: "Administrator".to_string(),
            description:
                "Full access to workspace settings, user management, RBAC, and audit logs."
                    .to_string(),
            permissions_json: serde_json::json!([
                "manage_workspace",
                "manage_users",
                "manage_roles",
                "view_audit_logs",
                "export_audit_logs",
                "manage_billing",
                "edit_content",
                "view_content"
            ])
            .to_string(),
            is_custom: false,
        },
        crate::models::WorkspaceRolePermission {
            role_id: "manager".to_string(),
            role_name: "Operations Manager".to_string(),
            description:
                "Operational access for scheduling, team dispatch, notes, and staff management."
                    .to_string(),
            permissions_json: serde_json::json!([
                "manage_users",
                "edit_content",
                "view_content",
                "dispatch_teams",
                "manage_schedules"
            ])
            .to_string(),
            is_custom: false,
        },
        crate::models::WorkspaceRolePermission {
            role_id: "member".to_string(),
            role_name: "Standard Member".to_string(),
            description:
                "Standard daily operational features (messaging, daily notes, time reporting)."
                    .to_string(),
            permissions_json: serde_json::json!(["edit_content", "view_content", "time_reporting"])
                .to_string(),
            is_custom: false,
        },
        crate::models::WorkspaceRolePermission {
            role_id: "viewer".to_string(),
            role_name: "Read-Only Viewer".to_string(),
            description: "Read-only access to schedules, team notes, and directory.".to_string(),
            permissions_json: serde_json::json!(["view_content"]).to_string(),
            is_custom: false,
        },
    ])
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct RolePermissionRule {
    pub role_id: String,
    pub role_label: String,
    pub can_view_audit_logs: bool,
    pub can_export_data: bool,
    pub can_manage_users: bool,
    pub can_edit_settings: bool,
}

#[uniffi::export]
pub async fn get_workspace_rbac_matrix(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<RolePermissionRule>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    Ok(vec![
        RolePermissionRule {
            role_id: "platform_admin".to_string(),
            role_label: "Platform Admin".to_string(),
            can_view_audit_logs: true,
            can_export_data: true,
            can_manage_users: true,
            can_edit_settings: true,
        },
        RolePermissionRule {
            role_id: "admin".to_string(),
            role_label: "Workspace Administrator".to_string(),
            can_view_audit_logs: true,
            can_export_data: true,
            can_manage_users: true,
            can_edit_settings: true,
        },
        RolePermissionRule {
            role_id: "manager".to_string(),
            role_label: "Team Manager".to_string(),
            can_view_audit_logs: false,
            can_export_data: true,
            can_manage_users: false,
            can_edit_settings: false,
        },
        RolePermissionRule {
            role_id: "field_technician".to_string(),
            role_label: "Field Specialist / Technician".to_string(),
            can_view_audit_logs: false,
            can_export_data: false,
            can_manage_users: false,
            can_edit_settings: false,
        },
        RolePermissionRule {
            role_id: "member".to_string(),
            role_label: "Standard Team Member".to_string(),
            can_view_audit_logs: false,
            can_export_data: false,
            can_manage_users: false,
            can_edit_settings: false,
        },
    ])
}

#[uniffi::export]
pub async fn export_workspace_full_data_json(
    requester_user_id: String,
    workspace_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.role != "admin" {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }

    let ws: Workspace = get_workspace(requester_user_id.clone()).await?;

    // Query users
    let mut user_stmt = conn
        .prepare("SELECT id, email, full_name, role FROM users WHERE workspace_id = ?1")
        .await?;
    let users: Vec<serde_json::Value> = user_stmt
        .query_map(crate::params![&workspace_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<String>(0)?,
                "email": r.get::<String>(1)?,
                "full_name": r.get::<Option<String>>(2)?,
                "role": r.get::<String>(3)?,
            }))
        })
        .await?;

    // Query time reports
    let mut tr_stmt = conn.prepare("SELECT id, user_id, date, hours, note, status FROM time_reports WHERE workspace_id = ?1").await?;
    let time_reports: Vec<serde_json::Value> = tr_stmt
        .query_map(crate::params![&workspace_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<String>(0)?,
                "user_id": r.get::<String>(1)?,
                "date": r.get::<String>(2)?,
                "hours": r.get::<f64>(3)?,
                "note": r.get::<Option<String>>(4)?,
                "status": r.get::<String>(5)?,
            }))
        })
        .await?;

    // Query notes
    let mut notes_stmt = conn
        .prepare("SELECT id, subject, content, created_at FROM notes WHERE workspace_id = ?1")
        .await?;
    let notes: Vec<serde_json::Value> = notes_stmt
        .query_map(crate::params![&workspace_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<String>(0)?,
                "subject": r.get::<String>(1)?,
                "content": r.get::<String>(2)?,
                "created_at": r.get::<String>(3)?,
            }))
        })
        .await?;

    // Query audit logs
    let audit_logs = crate::services::audit::get_audit_logs(requester_user_id.clone())
        .await
        .unwrap_or_default();

    let export_bundle = serde_json::json!({
        "export_info": {
            "exported_at": crate::infra::time::get_current_time_ms(),
            "exported_by": requester_user_id,
            "workspace_id": workspace_id,
            "compliance_standard": "GDPR_ARTICLE_20_DATA_PORTABILITY"
        },
        "workspace": ws,
        "users": users,
        "time_reports": time_reports,
        "notes": notes,
        "audit_logs": audit_logs
    });

    serde_json::to_string_pretty(&export_bundle)
        .map_err(|e| YntraError::DbError(format!("Failed to format workspace export JSON: {}", e)))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_moving_company_default_settings_seeding() {
        let modules_json = r#"{"moving_company": true, "locale": "se"}"#;
        let default_settings_json = get_default_settings_for_modules(modules_json);
        let settings_val: serde_json::Value = serde_json::from_str(&default_settings_json).unwrap();

        assert!(settings_val.get("roles").is_some());
        assert_eq!(
            settings_val
                .get("moving_pricing_model")
                .and_then(|v| v.as_str()),
            Some("volume")
        );
        assert_eq!(
            settings_val
                .get("moving_base_rate_per_m3")
                .and_then(|v| v.as_f64()),
            Some(500.0)
        );
        assert_eq!(
            settings_val
                .get("moving_hourly_rate_per_mover")
                .and_then(|v| v.as_f64()),
            Some(400.0)
        );
        assert_eq!(
            settings_val.get("surcharge_piano").and_then(|v| v.as_f64()),
            Some(1500.0)
        );
        assert_eq!(
            settings_val
                .get("moving_deposit_percent")
                .and_then(|v| v.as_f64()),
            Some(20.0)
        );
        assert_eq!(
            settings_val.get("target_region").and_then(|v| v.as_str()),
            Some("SE")
        );
        assert_eq!(
            settings_val.get("currency").and_then(|v| v.as_str()),
            Some("SEK")
        );
        assert_eq!(
            settings_val
                .get("use_rut_deduction")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_workspaces_tenant_isolation() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        let ws_id1 = format!("ws-iso-1-{}", uuid::Uuid::new_v4());
        let ws_id2 = format!("ws-iso-2-{}", uuid::Uuid::new_v4());

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Workspace 1', '{}', '{}')", crate::params![&ws_id1]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Workspace 2', '{}', '{}')", crate::params![&ws_id2]).await.unwrap();

        // Setup test users
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('ws-user-admin', ?1, 'wsadmin@yntra.se', 'admin')", crate::params![&ws_id1]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('ws-user-padmin', ?1, 'wspadmin@yntra.se', 'platform_admin')", crate::params![&ws_id1]).await.unwrap();

        // Querying as standard admin user (should only see workspace-1)
        let list1 = get_workspaces("ws-user-admin".to_string()).await.unwrap();
        assert!(list1.iter().any(|w| w.id == ws_id1));
        assert!(!list1.iter().any(|w| w.id == ws_id2));

        // Querying as platform admin (should see all workspaces)
        let list2 = get_workspaces("ws-user-padmin".to_string()).await.unwrap();
        assert!(list2.iter().any(|w| w.id == ws_id1));
        assert!(list2.iter().any(|w| w.id == ws_id2));

        // Clean up
        conn.execute(
            "DELETE FROM users WHERE id IN ('ws-user-admin', 'ws-user-padmin')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id IN (?1, ?2)",
            crate::params![ws_id1, ws_id2],
        )
        .await
        .unwrap();
    }

    #[test]
    fn test_workspace_settings_definitions() {
        let defs = get_workspace_settings_definitions();
        assert!(!defs.is_empty());
        assert!(
            defs.iter()
                .any(|d| d.key == "mult_narrow_staircase" && !d.tooltip.is_empty())
        );
        assert!(
            defs.iter()
                .any(|d| d.key == "moving_weekend_multiplier" && !d.tooltip.is_empty())
        );
        assert!(
            defs.iter()
                .any(|d| d.key == "surcharge_piano" && !d.tooltip.is_empty())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_workspace_onboarding_invitation_rbac_flow() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-onb-test-{}", uuid::Uuid::new_v4());
        let admin_uid = format!("u-admin-{}", uuid::Uuid::new_v4());
        let member_uid = format!("u-member-{}", uuid::Uuid::new_v4());

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Initial WS', '{}', '{}')", crate::params![&ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'admin@ws.io', 'admin')", crate::params![&admin_uid, &ws_id]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'member@ws.io', 'member')", crate::params![&member_uid, &ws_id]).await.unwrap();

        crate::services::users::ensure_user_role_signature(&conn, &admin_uid, "admin", &ws_id)
            .await
            .unwrap();
        crate::services::users::ensure_user_role_signature(&conn, &member_uid, "member", &ws_id)
            .await
            .unwrap();

        // 1. Test Onboarding completion
        let onboarding_payload = serde_json::json!({
            "name": "Configured Workspace",
            "org_type": "care",
            "brand_color": "hsl(210, 80%, 50%)",
            "modules_active": {"messaging": true, "scheduling": true}
        })
        .to_string();

        let updated_ws =
            complete_workspace_onboarding(admin_uid.clone(), ws_id.clone(), onboarding_payload)
                .await
                .unwrap();
        assert_eq!(updated_ws.name, "Configured Workspace");
        assert!(updated_ws.settings.contains("onboarding_completed"));

        // 2. Test Team Invitation Creation & Revocation
        let inv = create_workspace_invitation(
            admin_uid.clone(),
            ws_id.clone(),
            "invitee@ws.io".to_string(),
            "Invited User".to_string(),
            "manager".to_string(),
        )
        .await
        .unwrap();
        assert!(inv.code.starts_with("INV-"));

        let inv_list = get_workspace_invitations(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert!(inv_list.iter().any(|i| i.code == inv.code));

        revoke_workspace_invitation(admin_uid.clone(), ws_id.clone(), inv.code.clone())
            .await
            .unwrap();
        let inv_list_after = get_workspace_invitations(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert!(!inv_list_after.iter().any(|i| i.code == inv.code));

        // 3. Test RBAC Role Update
        crate::services::users::ensure_user_role_signature(&conn, &admin_uid, "admin", &ws_id)
            .await
            .unwrap();
        let updated_user = update_user_workspace_role(
            admin_uid.clone(),
            ws_id.clone(),
            member_uid.clone(),
            "manager".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(updated_user.role, "manager");

        let roles_perm = get_workspace_role_permissions(admin_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert!(!roles_perm.is_empty());

        // 4. Test Audit Export
        let csv_export =
            crate::services::audit::export_audit_logs_csv(admin_uid.clone(), None, None, None)
                .await
                .unwrap();
        assert!(csv_export.contains("action_type"));

        let json_export =
            crate::services::audit::export_audit_logs_json(admin_uid.clone(), None, None, None)
                .await
                .unwrap();
        assert!(json_export.contains("export_metadata"));

        // Clean up
        conn.execute(
            "DELETE FROM users WHERE id IN (?1, ?2)",
            crate::params![admin_uid, member_uid],
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id = ?1",
            crate::params![ws_id],
        )
        .await
        .unwrap();
    }
}
