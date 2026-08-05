use crate::database;
use crate::YntraError;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MicroKernelFootprint {
    pub core_version: String,
    pub loaded_domain_plugins: Vec<String>,
    pub memory_footprint_kb: u64,
    pub active_extension_hooks: u32,
    pub kernel_status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DomainPluginManifest {
    pub domain_scope: String,
    pub display_name: String,
    pub version: String,
    pub is_active: bool,
    pub required_permissions: Vec<String>,
}

#[uniffi::export]
pub async fn get_micro_kernel_footprint(
    requester_user_id: String,
    workspace_id: String,
) -> Result<MicroKernelFootprint, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let active_modules = get_active_workspace_modules(&conn, &workspace_id).await?;
    let active_hooks = active_modules.len() as u32 * 4;

    Ok(MicroKernelFootprint {
        core_version: env!("CARGO_PKG_VERSION").to_string(),
        loaded_domain_plugins: active_modules,
        memory_footprint_kb: 4096 + (active_hooks as u64 * 128),
        active_extension_hooks: active_hooks,
        kernel_status: "optimal".to_string(),
    })
}

#[uniffi::export]
pub async fn list_available_domain_plugins(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<DomainPluginManifest>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let active_modules = get_active_workspace_modules(&conn, &workspace_id).await?;

    let all_plugins = vec![
        ("care", "Healthcare & Client Care", "1.4.0", vec!["medications.manage".to_string()]),
        ("school", "Education & Academics", "2.1.0", vec!["report_cards.publish".to_string()]),
        ("jobs", "Field Services & Dispatch", "1.8.0", vec!["jobs.dispatch".to_string()]),
        ("vehicles", "Fleet & Asset Management", "1.2.0", vec!["vehicles.track".to_string()]),
    ];

    let manifests = all_plugins
        .into_iter()
        .map(|(scope, name, ver, perms)| DomainPluginManifest {
            domain_scope: scope.to_string(),
            display_name: name.to_string(),
            version: ver.to_string(),
            is_active: active_modules.contains(&scope.to_string()),
            required_permissions: perms,
        })
        .collect();

    Ok(manifests)
}

#[uniffi::export]
pub async fn activate_domain_plugin(
    requester_user_id: String,
    workspace_id: String,
    domain_scope: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.role != "admin" {
        return Err(YntraError::AuthError(
            "Access denied: only administrators can manage domain plugins".to_string(),
        ));
    }

    let mut active = get_active_workspace_modules(&conn, &workspace_id).await?;
    if !active.contains(&domain_scope) {
        active.push(domain_scope.clone());
        let json_val = serde_json::to_string(&active)
            .map_err(|e| YntraError::ValidationError(e.to_string()))?;

        conn.execute(
            "UPDATE workspaces SET modules_active = ?1 WHERE id = ?2",
            crate::params![json_val, &workspace_id],
        )
        .await?;

        crate::infra::observer::notify_observers();
    }

    Ok(true)
}

#[uniffi::export]
pub async fn deactivate_domain_plugin(
    requester_user_id: String,
    workspace_id: String,
    domain_scope: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.role != "admin" {
        return Err(YntraError::AuthError(
            "Access denied: only administrators can manage domain plugins".to_string(),
        ));
    }

    let mut active = get_active_workspace_modules(&conn, &workspace_id).await?;
    if active.contains(&domain_scope) {
        active.retain(|m| m != &domain_scope);
        let json_val = serde_json::to_string(&active)
            .map_err(|e| YntraError::ValidationError(e.to_string()))?;

        conn.execute(
            "UPDATE workspaces SET modules_active = ?1 WHERE id = ?2",
            crate::params![json_val, &workspace_id],
        )
        .await?;

        crate::infra::observer::notify_observers();
    }

    Ok(true)
}

pub async fn validate_domain_plugin_access(
    conn: &database::DbConnection,
    workspace_id: &str,
    domain_scope: &str,
) -> Result<(), YntraError> {
    let active = get_active_workspace_modules(conn, workspace_id).await?;
    if !active.contains(&domain_scope.to_string()) {
        return Err(YntraError::ModuleDisabledError(format!(
            "Domain plugin '{}' is not active for this workspace micro-kernel",
            domain_scope
        )));
    }
    Ok(())
}

async fn get_active_workspace_modules(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<Vec<String>, YntraError> {
    let raw_json: Option<String> = conn
        .query_row(
            "SELECT modules_active FROM workspaces WHERE id = ?1",
            crate::params![workspace_id],
            |r| r.get(0),
        )
        .await
        .ok();

    if let Some(json_str) = raw_json {
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(&json_str) {
            return Ok(vec);
        }
    }

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_micro_kernel_plugin_lifecycle_and_footprint() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        let conn = database::acquire_connection().await.unwrap();
        let ws_id = "ws-kernel-test";
        let admin_id = "u-kernel-admin";

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Kernel WS', '[]', '{}')",
            crate::params![ws_id],
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'kernel@yntra.se', 'admin')",
            crate::params![admin_id, ws_id],
        )
        .await
        .unwrap();

        // 1. Initial Footprint check
        let footprint1 = get_micro_kernel_footprint(admin_id.to_string(), ws_id.to_string()).await.unwrap();
        assert_eq!(footprint1.loaded_domain_plugins.len(), 0);

        // 2. Activate Care plugin
        let ok_care = activate_domain_plugin(admin_id.to_string(), ws_id.to_string(), "care".to_string()).await.unwrap();
        assert!(ok_care);

        // 3. Verify Care activation and access guardrail
        validate_domain_plugin_access(&conn, ws_id, "care").await.unwrap();
        assert!(validate_domain_plugin_access(&conn, ws_id, "school").await.is_err());

        // 4. Deactivate Care plugin
        let ok_deact = deactivate_domain_plugin(admin_id.to_string(), ws_id.to_string(), "care".to_string()).await.unwrap();
        assert!(ok_deact);
        assert!(validate_domain_plugin_access(&conn, ws_id, "care").await.is_err());

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![admin_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }
}
