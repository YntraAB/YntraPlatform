use crate::database;
use crate::observer::notify_observers;
use crate::{TodoItem, YntraError};

#[uniffi::export]
pub async fn get_todos(workspace_id: String) -> Result<Vec<TodoItem>, YntraError> {
    let conn = database::acquire_connection().await?;

    let mut stmt = conn.prepare("SELECT id, text, completed, workspace_id, updated_at, sync_status FROM todos WHERE workspace_id = ?1").await?;

    let todos = stmt.query_map(crate::params![workspace_id], |row| {
        let completed_int: i32 = row.get(2)?;
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            completed: completed_int != 0,
            workspace_id: row.get(3)?,
            updated_at: row.get(4)?,
            sync_status: row.get(5)?,
        })
    }).await?;

    Ok(todos)
}

#[uniffi::export]
pub async fn add_todo(workspace_id: String, text: String) -> Result<TodoItem, YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let item = TodoItem {
        id: id.clone(),
        workspace_id,
        text,
        completed: false,
        updated_at: crate::infra::time::get_current_time_ms(),
        sync_status: "pending".to_string(),
    };

    let conn = database::acquire_connection().await?;

    conn.execute(
        "INSERT INTO todos (id, workspace_id, text, completed, updated_at, sync_status) VALUES (?1, ?2, ?3, 0, ?4, 'pending')",
        crate::params![&id, &item.workspace_id, &item.text, &item.updated_at],
    ).await?;

    notify_observers();
    Ok(item)
}

#[uniffi::export]
pub async fn toggle_todo(id: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE todos SET completed = NOT completed, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![&now_ms, &id],
    ).await?;

    notify_observers();
    Ok(())
}
