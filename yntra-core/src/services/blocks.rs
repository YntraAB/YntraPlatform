use crate::database;
use crate::{BlockItem, YntraError};

#[uniffi::export]
pub async fn get_blocks(requester_user_id: String) -> Result<Vec<BlockItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt =
        conn.prepare("SELECT id, name, description, icon, category, dependencies, fields_schema, navigation_items, ui_config FROM blocks").await?;

    let list = stmt
        .query_map((), |row| {
            Ok(BlockItem {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                icon: row.get(3)?,
                category: row.get(4)?,
                dependencies: row.get(5)?,
                fields_schema: row.get(6)?,
                navigation_items: row.get(7)?,
                ui_config: row.get(8)?,
            })
        })
        .await?;

    if auth.role == "platform_admin" {
        Ok(list)
    } else {
        let modules_active_str: String = conn
            .query_row(
                "SELECT modules_active FROM workspaces WHERE id = ?1",
                crate::params![auth.workspace_id],
                |r| r.get(0),
            )
            .await?;

        let active_ids: Vec<String> = serde_json::from_str(&modules_active_str).unwrap_or_default();
        let filtered = list
            .into_iter()
            .filter(|b| active_ids.contains(&b.id))
            .collect();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_blocks() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        // Setup mock user & workspace
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-test-blocks', 'Test WS', '[\"test-block-id\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('user-test-blocks', 'ws-test-blocks', 'test@blocks.io', 'platform_admin')", ()).await.unwrap();

        // Write a block
        conn.execute(
            "INSERT OR REPLACE INTO blocks (id, name, description, icon, category, dependencies, created_at)
             VALUES ('test-block-id', 'Test Block Name', 'Test Description', 'icon-test', 'test-category', '[]', '2026-07-05T22:50:00Z')",
            ()
        ).await.unwrap();

        let list = get_blocks("user-test-blocks".to_string()).await.unwrap();
        let found = list.iter().find(|b| b.id == "test-block-id");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "Test Block Name");
        assert_eq!(found.icon, "icon-test");

        // Clean up
        conn.execute("DELETE FROM blocks WHERE id = 'test-block-id'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE id = 'user-test-blocks'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-test-blocks'", ())
            .await
            .unwrap();
    }
}
