use crate::database;
use crate::observer::notify_observers;
use crate::{DynamicEntity, YntraError};

#[uniffi::export]
pub async fn get_dynamic_entities(
    requester_user_id: String,
    workspace_id: String,
    block_id: String,
) -> Result<Vec<DynamicEntity>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status FROM entities WHERE workspace_id = ?1 AND block_id = ?2")
        .await?;

    let entities = stmt
        .query_map(crate::params![workspace_id, block_id], |row| {
            Ok(DynamicEntity {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                block_id: row.get(2)?,
                entity_type: row.get(3)?,
                data: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                sync_status: row.get(7)?,
            })
        })
        .await?;

    Ok(entities)
}

#[uniffi::export]
pub async fn save_dynamic_entity(
    requester_user_id: String,
    entity: DynamicEntity,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != entity.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let created_at = if entity.created_at == 0 { now_ms } else { entity.created_at };

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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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
    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
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

        save_dynamic_entity("u-dyn-user".to_string(), entity).await.unwrap();

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
        delete_dynamic_entity("u-dyn-user".to_string(), "entity-1".to_string()).await.unwrap();

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
        conn.execute("DELETE FROM entities WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
        conn.execute("DELETE FROM blocks WHERE id = 'block-dyn-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-dyn-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-dyn-test'", ()).await.unwrap();
    }
}
