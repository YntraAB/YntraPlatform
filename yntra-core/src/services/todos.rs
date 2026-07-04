#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{TodoItem, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_todos() -> Result<Vec<TodoItem>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        let mut stmt = conn.prepare("SELECT id, text, completed, workspace_id, updated_at, sync_status FROM todos").await?;

        let todos = stmt.query_map((), |row| {
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.todos.clone())
    }
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "INSERT INTO todos (id, workspace_id, text, completed, updated_at, sync_status) VALUES (?1, ?2, ?3, 0, ?4, 'pending')",
            crate::params![&id, &item.workspace_id, &item.text, &item.updated_at],
        ).await?;

        notify_observers();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.todos.push(item.clone());
        notify_observers();
    }

    Ok(item)
}

#[uniffi::export]
pub async fn toggle_todo(id: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;

        conn.execute(
            "UPDATE todos SET completed = NOT completed, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
            crate::params![&now_ms, &id],
        ).await?;

        notify_observers();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        if let Some(todo) = store.todos.iter_mut().find(|t| t.id == id) {
            todo.completed = !todo.completed;
            todo.updated_at = now_ms;
            todo.sync_status = "pending".to_string();
        }
        notify_observers();
        Ok(())
    }
}
