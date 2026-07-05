use crate::database;
use crate::{BlockItem, YntraError};

#[uniffi::export]
pub async fn get_blocks() -> Result<Vec<BlockItem>, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt =
        conn.prepare("SELECT id, name, description, icon, category, dependencies FROM blocks").await?;

    let list = stmt.query_map((), |row| {
        Ok(BlockItem {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            icon: row.get(3)?,
            category: row.get(4)?,
            dependencies: row.get(5)?,
        })
    }).await?;

    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_blocks() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        // Write a block
        conn.execute(
            "INSERT OR REPLACE INTO blocks (id, name, description, icon, category, dependencies, created_at)
             VALUES ('test-block-id', 'Test Block Name', 'Test Description', 'icon-test', 'test-category', '[]', '2026-07-05T22:50:00Z')",
            ()
        ).await.unwrap();

        let list = get_blocks().await.unwrap();
        let found = list.iter().find(|b| b.id == "test-block-id");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "Test Block Name");
        assert_eq!(found.icon, "icon-test");

        // Clean up
        conn.execute("DELETE FROM blocks WHERE id = 'test-block-id'", ()).await.unwrap();
    }
}
