use crate::database;
use crate::observer::notify_observers;
use crate::{DynamicEntity, YntraError};

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FieldPermissionPolicy {
    pub role: String,
    pub read_allowed_fields: Vec<String>,
    pub write_allowed_fields: Vec<String>,
}

pub fn filter_entity_data_for_role(
    data_json: &str,
    role: &str,
    policies: &[FieldPermissionPolicy],
) -> String {
    if role == "platform_admin" || role == "admin" {
        return data_json.to_string();
    }

    let policy = policies.iter().find(|p| p.role.eq_ignore_ascii_case(role));
    let allowed_read = match policy {
        Some(p) => &p.read_allowed_fields,
        None => return data_json.to_string(),
    };

    if allowed_read.contains(&"*".to_string()) {
        return data_json.to_string();
    }

    let Ok(val) = serde_json::from_str::<serde_json::Value>(data_json) else {
        return data_json.to_string();
    };

    let Some(obj) = val.as_object() else {
        return data_json.to_string();
    };

    let mut filtered_map = serde_json::Map::new();
    for (k, v) in obj {
        if allowed_read.contains(k) {
            filtered_map.insert(k.clone(), v.clone());
        }
    }

    serde_json::to_string(&filtered_map).unwrap_or_else(|_| data_json.to_string())
}

pub fn validate_entity_data_write_permissions(
    data_json: &str,
    role: &str,
    policies: &[FieldPermissionPolicy],
) -> Result<(), YntraError> {
    if role == "platform_admin" || role == "admin" {
        return Ok(());
    }

    let policy = policies.iter().find(|p| p.role.eq_ignore_ascii_case(role));
    let allowed_write = match policy {
        Some(p) => &p.write_allowed_fields,
        None => return Ok(()),
    };

    if allowed_write.contains(&"*".to_string()) {
        return Ok(());
    }

    let Ok(val) = serde_json::from_str::<serde_json::Value>(data_json) else {
        return Ok(());
    };

    let Some(obj) = val.as_object() else {
        return Ok(());
    };

    for (k, _) in obj {
        if !allowed_write.contains(k) {
            return Err(YntraError::AuthError(format!(
                "Field-level write permission denied for field '{}' under role '{}'",
                k, role
            )));
        }
    }

    Ok(())
}

pub async fn get_field_permission_policies_internal(
    conn: &database::DbConnection,
    block_id: &str,
) -> Vec<FieldPermissionPolicy> {
    let ui_config_str: Option<String> = conn
        .query_row(
            "SELECT ui_config FROM blocks WHERE id = ?1",
            crate::params![block_id],
            |r| r.get(0),
        )
        .await
        .ok()
        .flatten();

    if let Some(cfg) = ui_config_str {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cfg) {
            if let Some(perms) = val.get("field_permissions") {
                if let Ok(policies) =
                    serde_json::from_value::<Vec<FieldPermissionPolicy>>(perms.clone())
                {
                    return policies;
                }
            }
        }
    }
    Vec::new()
}

#[uniffi::export]
pub async fn get_field_permission_policies(
    requester_user_id: String,
    block_id: String,
) -> Result<Vec<FieldPermissionPolicy>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let policies = get_field_permission_policies_internal(&conn, &block_id).await;
    Ok(policies)
}

#[uniffi::export]
pub async fn get_dynamic_entities(
    requester_user_id: String,
    workspace_id: String,
    block_id: String,
) -> Result<Vec<DynamicEntity>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let policies = get_field_permission_policies_internal(&conn, &block_id).await;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status FROM entities WHERE workspace_id = ?1 AND block_id = ?2")
        .await?;

    let entities = stmt
        .query_map(crate::params![workspace_id, block_id], |row| {
            let raw_data: String = row.get(4)?;
            let filtered_data = filter_entity_data_for_role(&raw_data, &auth.role, &policies);
            Ok(DynamicEntity {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                block_id: row.get(2)?,
                entity_type: row.get(3)?,
                data: filtered_data,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                sync_status: row.get(7)?,
            })
        })
        .await?;

    Ok(entities)
}

#[derive(serde::Deserialize, Debug)]
struct FieldDefinition {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    required: bool,
}

fn validate_entity_data(data_json: &str, schema_json: &str) -> Result<(), YntraError> {
    let fields: Vec<FieldDefinition> = serde_json::from_str(schema_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid fields_schema JSON: {}", e)))?;

    let data_val: serde_json::Value = serde_json::from_str(data_json).map_err(|e| {
        YntraError::ValidationError(format!("Invalid dynamic entity data JSON: {}", e))
    })?;

    let data_map = data_val.as_object().ok_or_else(|| {
        YntraError::ValidationError("Entity data must be a JSON object".to_string())
    })?;

    for field in fields {
        let value_opt = data_map.get(&field.name);
        match value_opt {
            Some(value) => {
                if value.is_null() {
                    if field.required {
                        return Err(YntraError::ValidationError(format!(
                            "Field '{}' is required but is null",
                            field.name
                        )));
                    }
                } else {
                    match field.field_type.as_str() {
                        "text" | "select" => {
                            if !value.is_string() {
                                return Err(YntraError::ValidationError(format!(
                                    "Field '{}' expects text, but got {:?}",
                                    field.name, value
                                )));
                            }
                        }
                        "number" => {
                            if !value.is_number() {
                                return Err(YntraError::ValidationError(format!(
                                    "Field '{}' expects number, but got {:?}",
                                    field.name, value
                                )));
                            }
                        }
                        "boolean" => {
                            if !value.is_boolean() {
                                return Err(YntraError::ValidationError(format!(
                                    "Field '{}' expects boolean, but got {:?}",
                                    field.name, value
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {
                if field.required {
                    return Err(YntraError::ValidationError(format!(
                        "Field '{}' is required but missing",
                        field.name
                    )));
                }
            }
        }
    }

    Ok(())
}

#[uniffi::export]
pub async fn save_dynamic_entity(
    requester_user_id: String,
    entity: DynamicEntity,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != entity.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }
    let policies = get_field_permission_policies_internal(&conn, &entity.block_id).await;
    validate_entity_data_write_permissions(&entity.data, &auth.role, &policies)?;

    // Retrieve fields_schema for the block to perform validation
    let block_schema: Option<String> = conn
        .query_row(
            "SELECT fields_schema FROM blocks WHERE id = ?1",
            crate::params![&entity.block_id],
            |r| r.get(0),
        )
        .await
        .ok()
        .flatten();

    if let Some(schema_str) = block_schema {
        if !schema_str.trim().is_empty() {
            validate_entity_data(&entity.data, &schema_str)?;
        }
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let created_at = if entity.created_at == 0 {
        now_ms
    } else {
        entity.created_at
    };

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')",
        crate::params![
            &entity.id,
            &entity.workspace_id,
            &entity.block_id,
            &entity.entity_type,
            &entity.data,
            &created_at,
            &now_ms
        ],
    )
    .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_dynamic_entity(
    requester_user_id: String,
    id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let entity_ws: String = conn
        .query_row(
            "SELECT workspace_id FROM entities WHERE id = ?1",
            crate::params![&id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Entity not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != entity_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute("DELETE FROM entities WHERE id = ?1", crate::params![&id])
        .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn update_block_schema(
    requester_user_id: String,
    block_id: String,
    fields_schema: String,
    ui_config: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" {
        return Err(YntraError::AuthError(
            "Access denied: platform administrator privileges required".to_string(),
        ));
    }

    conn.execute(
        "UPDATE blocks SET fields_schema = ?1, ui_config = ?2 WHERE id = ?3",
        crate::params![&fields_schema, &ui_config, &block_id],
    )
    .await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_dynamic_entity_crud() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup a test workspace & user
        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-dyn-test', 'Dynamic WS', '[]', '{}')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-dyn-user', 'ws-dyn-test', 'dyn@user.com', 'user')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO blocks (id, name, icon, category, created_at) VALUES ('block-dyn-test', 'Dynamic Block', 'Layout', 'Operations', '2026-07-08T00:00:00Z')",
            (),
        )
        .await
        .unwrap();

        // 1. Save dynamic entity
        let entity = DynamicEntity {
            id: "entity-1".to_string(),
            workspace_id: "ws-dyn-test".to_string(),
            block_id: "block-dyn-test".to_string(),
            entity_type: "vehicle".to_string(),
            data: r#"{"name": "Truck A", "capacity": 500}"#.to_string(),
            created_at: 0,
            updated_at: 0,
            sync_status: "pending".to_string(),
        };

        save_dynamic_entity("u-dyn-user".to_string(), entity)
            .await
            .unwrap();

        // 2. Get dynamic entities
        let list = get_dynamic_entities(
            "u-dyn-user".to_string(),
            "ws-dyn-test".to_string(),
            "block-dyn-test".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "entity-1");
        assert_eq!(list[0].entity_type, "vehicle");
        assert!(list[0].data.contains("Truck A"));

        // 3. Delete dynamic entity
        delete_dynamic_entity("u-dyn-user".to_string(), "entity-1".to_string())
            .await
            .unwrap();

        // 4. Verify deleted
        let list_after = get_dynamic_entities(
            "u-dyn-user".to_string(),
            "ws-dyn-test".to_string(),
            "block-dyn-test".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(list_after.len(), 0);

        // Cleanup
        conn.execute(
            "DELETE FROM entities WHERE workspace_id = 'ws-dyn-test'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM blocks WHERE id = 'block-dyn-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dyn-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-dyn-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_dynamic_entity_schema_validation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup a test workspace & user
        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-val-test', 'Val WS', '[]', '{}')",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-val-user', 'ws-val-test', 'val@user.com', 'user')",
            (),
        )
        .await
        .unwrap();

        // 1. Create a block with a defined schema
        // Schema requires: "name" (text), "age" (number, optional), "is_active" (boolean, required)
        let fields_schema = r#"[
            {"name": "name", "type": "text", "required": true},
            {"name": "age", "type": "number", "required": false},
            {"name": "is_active", "type": "boolean", "required": true}
        ]"#;

        conn.execute(
            "INSERT OR REPLACE INTO blocks (id, name, icon, category, fields_schema, created_at)
             VALUES ('block-val-test', 'Val Block', 'Shield', 'Security', ?1, '2026-07-08T00:00:00Z')",
            crate::params![fields_schema],
        )
        .await
        .unwrap();

        // Case A: Valid entity data should save successfully
        let valid_entity = DynamicEntity {
            id: "val-ent-ok".to_string(),
            workspace_id: "ws-val-test".to_string(),
            block_id: "block-val-test".to_string(),
            entity_type: "person".to_string(),
            data: r#"{"name": "Alice", "age": 30, "is_active": true}"#.to_string(),
            created_at: 0,
            updated_at: 0,
            sync_status: "pending".to_string(),
        };
        assert!(
            save_dynamic_entity("u-val-user".to_string(), valid_entity)
                .await
                .is_ok()
        );

        // Case B: Missing required field ("is_active") should fail
        let missing_required = DynamicEntity {
            id: "val-ent-fail1".to_string(),
            workspace_id: "ws-val-test".to_string(),
            block_id: "block-val-test".to_string(),
            entity_type: "person".to_string(),
            data: r#"{"name": "Bob"}"#.to_string(),
            created_at: 0,
            updated_at: 0,
            sync_status: "pending".to_string(),
        };
        let res = save_dynamic_entity("u-val-user".to_string(), missing_required).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        // Case C: Mismatching field type ("age" expects number, got text) should fail
        let type_mismatch = DynamicEntity {
            id: "val-ent-fail2".to_string(),
            workspace_id: "ws-val-test".to_string(),
            block_id: "block-val-test".to_string(),
            entity_type: "person".to_string(),
            data: r#"{"name": "Charlie", "age": "thirty", "is_active": false}"#.to_string(),
            created_at: 0,
            updated_at: 0,
            sync_status: "pending".to_string(),
        };
        let res = save_dynamic_entity("u-val-user".to_string(), type_mismatch).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        // Case D: Invalid JSON format should fail
        let invalid_json = DynamicEntity {
            id: "val-ent-fail3".to_string(),
            workspace_id: "ws-val-test".to_string(),
            block_id: "block-val-test".to_string(),
            entity_type: "person".to_string(),
            data: r#"{"name": "Charlie", "#.to_string(),
            created_at: 0,
            updated_at: 0,
            sync_status: "pending".to_string(),
        };
        let res = save_dynamic_entity("u-val-user".to_string(), invalid_json).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        // Cleanup
        let _ = conn
            .execute(
                "DELETE FROM entities WHERE workspace_id = 'ws-val-test'",
                (),
            )
            .await;
        let _ = conn
            .execute("DELETE FROM blocks WHERE id = 'block-val-test'", ())
            .await;
        let _ = conn
            .execute("DELETE FROM users WHERE workspace_id = 'ws-val-test'", ())
            .await;
        let _ = conn
            .execute("DELETE FROM workspaces WHERE id = 'ws-val-test'", ())
            .await;
    }

    #[test]
    fn test_field_level_rbac_filtering() {
        let policies = vec![FieldPermissionPolicy {
            role: "external_contractor".to_string(),
            read_allowed_fields: vec![
                "id".to_string(),
                "task_name".to_string(),
                "status".to_string(),
            ],
            write_allowed_fields: vec!["task_name".to_string(), "status".to_string()],
        }];

        let raw_data = r#"{"id": "t-1", "task_name": "Fix HVAC", "status": "open", "internal_margin": 450.0, "ssn": "123-45"}"#;

        // Admin sees 100% of fields
        let admin_filtered = filter_entity_data_for_role(raw_data, "admin", &policies);
        assert!(admin_filtered.contains("internal_margin"));
        assert!(admin_filtered.contains("ssn"));

        // Contractor only sees allowed fields
        let contractor_filtered =
            filter_entity_data_for_role(raw_data, "external_contractor", &policies);
        assert!(contractor_filtered.contains("task_name"));
        assert!(!contractor_filtered.contains("internal_margin"));
        assert!(!contractor_filtered.contains("ssn"));

        // Write validation: writing allowed field passes
        let ok_write = validate_entity_data_write_permissions(
            r#"{"task_name": "Fix HVAC", "status": "completed"}"#,
            "external_contractor",
            &policies,
        );
        assert!(ok_write.is_ok());

        // Write validation: writing forbidden field fails
        let forbidden_write = validate_entity_data_write_permissions(
            r#"{"task_name": "Fix HVAC", "internal_margin": 999.0}"#,
            "external_contractor",
            &policies,
        );
        assert!(forbidden_write.is_err());
    }
}
