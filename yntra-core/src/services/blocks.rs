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
