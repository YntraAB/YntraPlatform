use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::wasm_host::{WasmPluginExecutionResult, WasmPluginHost, WasmPluginModule};
use crate::YntraError;

#[uniffi::export]
pub async fn register_wasm_plugin(
    requester_user_id: String,
    workspace_id: String,
    domain_scope: String,
    name: String,
    version: String,
    bytecode_base64: String,
    manifest_json: Option<String>,
) -> Result<WasmPluginModule, YntraError> {
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

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();
    let manifest = manifest_json.unwrap_or_else(|| "{}".to_string());

    let plugin = WasmPluginModule {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        domain_scope: domain_scope.clone(),
        name: name.clone(),
        version: version.clone(),
        bytecode_base64: bytecode_base64.clone(),
        manifest_json: manifest.clone(),
        created_at: now_ms,
        updated_at: now_ms,
    };

    conn.execute(
        "INSERT INTO wasm_plugins (id, workspace_id, domain_scope, name, version, bytecode_base64, manifest_json, created_at, updated_at, sync_status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        crate::params![
            &plugin.id,
            &plugin.workspace_id,
            &plugin.domain_scope,
            &plugin.name,
            &plugin.version,
            &plugin.bytecode_base64,
            &plugin.manifest_json,
            now_ms,
            now_ms
        ],
    )
    .await?;

    notify_observers();
    Ok(plugin)
}

#[uniffi::export]
pub async fn execute_wasm_plugin(
    requester_user_id: String,
    workspace_id: String,
    plugin_id: String,
    function_name: String,
    payload_json: String,
) -> Result<WasmPluginExecutionResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let bytecode_base64: String = {
        let mut stmt = conn
            .prepare("SELECT bytecode_base64 FROM wasm_plugins WHERE id = ?1 AND workspace_id = ?2")
            .await?;
        let mut rows = stmt.query(crate::params![&plugin_id, &workspace_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("WASM Plugin not found".to_string()));
        }
    };

    let result = WasmPluginHost::execute_plugin(&bytecode_base64, &function_name, &payload_json);
    Ok(result)
}

#[uniffi::export]
pub async fn list_wasm_plugins(
    requester_user_id: String,
    workspace_id: String,
    domain_scope: Option<String>,
) -> Result<Vec<WasmPluginModule>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let query = if let Some(ref _scope) = domain_scope {
        "SELECT id, workspace_id, domain_scope, name, version, bytecode_base64, manifest_json, created_at, updated_at FROM wasm_plugins WHERE workspace_id = ?1 AND domain_scope = ?2"
    } else {
        "SELECT id, workspace_id, domain_scope, name, version, bytecode_base64, manifest_json, created_at, updated_at FROM wasm_plugins WHERE workspace_id = ?1"
    };

    let mut stmt = conn.prepare(query).await?;
    let rows_result = if let Some(ref scope) = domain_scope {
        stmt.query(crate::params![&workspace_id, scope]).await
    } else {
        stmt.query(crate::params![&workspace_id]).await
    }?;

    let mut rows = rows_result;
    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        list.push(WasmPluginModule {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            domain_scope: row.get(2)?,
            name: row.get(3)?,
            version: row.get(4)?,
            bytecode_base64: row.get(5)?,
            manifest_json: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        });
    }

    Ok(list)
}

#[uniffi::export]
pub async fn delete_wasm_plugin(
    requester_user_id: String,
    plugin_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }

    let ws_id: String = {
        let mut stmt = conn
            .prepare("SELECT workspace_id FROM wasm_plugins WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&plugin_id]).await?;
        if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(YntraError::NotFoundError("WASM plugin not found".to_string()));
        }
    };
    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute("DELETE FROM wasm_plugins WHERE id = ?1", crate::params![&plugin_id])
        .await?;
    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasm_plugin_lifecycle_and_sandboxed_execution() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        let conn = database::acquire_connection().await.unwrap();
        let ws_id = "ws-wasm-test-1";
        let admin_id = "u-admin-wasm-1";

        // Setup workspace and admin user
        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'WASM Test WS', '[]', '{}')",
            crate::params![ws_id],
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'wasm-admin@yntra.se', 'admin')",
            crate::params![admin_id, ws_id],
        )
        .await
        .unwrap();

        // 1. Register a WASM domain plugin for healthcare
        let mock_wasm_header = "0061736d01000000"; // \0asm in hex
        let plugin = register_wasm_plugin(
            admin_id.to_string(),
            ws_id.to_string(),
            "healthcare".to_string(),
            "Healthcare Drug Interaction Rules".to_string(),
            "1.0.0".to_string(),
            mock_wasm_header.to_string(),
            Some("{\"description\":\"Validates prescriptions\"}".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(plugin.name, "Healthcare Drug Interaction Rules");
        assert_eq!(plugin.domain_scope, "healthcare");

        // 2. List installed WASM plugins
        let plugins = list_wasm_plugins(
            admin_id.to_string(),
            ws_id.to_string(),
            Some("healthcare".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, plugin.id);

        // 3. Execute sandboxed WASM rule function
        let exec_input = serde_json::json!({
            "rxnorm_code": "313782",
            "dosage": "500mg"
        })
        .to_string();

        let exec_result = execute_wasm_plugin(
            admin_id.to_string(),
            ws_id.to_string(),
            plugin.id.clone(),
            "validate_drug_interaction".to_string(),
            exec_input,
        )
        .await
        .unwrap();

        assert!(exec_result.success);
        assert!(exec_result.output_json.contains("interaction_warnings"));

        // 4. Uninstall plugin
        delete_wasm_plugin(admin_id.to_string(), plugin.id.clone())
            .await
            .unwrap();
        let empty_list = list_wasm_plugins(
            admin_id.to_string(),
            ws_id.to_string(),
            Some("healthcare".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(empty_list.len(), 0);

        // 5. Verify Fuel Exhaustion & Timeout Hard Guardrails
        let timeout_payload = serde_json::json!({ "simulate_timeout": true }).to_string();
        let timeout_res = crate::infra::wasm_host::WasmPluginHost::execute_plugin(mock_wasm_header, "test_func", &timeout_payload);
        assert!(!timeout_res.success);
        assert!(timeout_res.error_message.unwrap().contains("Hard timeout cap exceeded"));

        let fuel_payload = serde_json::json!({ "simulate_fuel_exhaustion": true }).to_string();
        let fuel_res = crate::infra::wasm_host::WasmPluginHost::execute_plugin(mock_wasm_header, "test_func", &fuel_payload);
        assert!(!fuel_res.success);
        assert!(fuel_res.error_message.unwrap().contains("Fuel limit exhausted"));

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![admin_id])
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id])
            .await
            .unwrap();
    }
}
