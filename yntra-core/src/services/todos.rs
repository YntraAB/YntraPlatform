use crate::database;
use crate::observer::notify_observers;
use crate::{TodoItem, YntraError};

#[uniffi::export]
pub async fn get_todos(requester_user_id: String, workspace_id: String) -> Result<Vec<TodoItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

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
pub async fn add_todo(requester_user_id: String, workspace_id: String, text: String) -> Result<TodoItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let item = TodoItem {
        id: id.clone(),
        workspace_id,
        text,
        completed: false,
        updated_at: crate::infra::time::get_current_time_ms(),
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO todos (id, workspace_id, text, completed, updated_at, sync_status) VALUES (?1, ?2, ?3, 0, ?4, 'pending')",
        crate::params![&id, &item.workspace_id, &item.text, &item.updated_at],
    ).await?;

    notify_observers();
    Ok(item)
}

#[uniffi::export]
pub async fn toggle_todo(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    
    let todo_ws: String = conn.query_row(
        "SELECT workspace_id FROM todos WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Todo not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != todo_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "UPDATE todos SET completed = NOT completed, updated_at = ?1, sync_status = 'pending' WHERE id = ?2",
        crate::params![&now_ms, &id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_todo_crud_flow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup a test user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-todo-test', 'Todo WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-todo-user', 'ws-todo-test', 'todo@user.com', 'user')", ()).await.unwrap();

        // 1. Add todo
        let ws_id = "ws-todo-test";
        let todo = add_todo("u-todo-user".to_string(), ws_id.to_string(), "Verify tests pass".to_string()).await.unwrap();
        assert_eq!(todo.text, "Verify tests pass");
        assert_eq!(todo.completed, false);
        assert_eq!(todo.workspace_id, ws_id);

        // 2. Get todos and assert it contains our added todo
        let list = get_todos("u-todo-user".to_string(), ws_id.to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, todo.id);
        assert_eq!(list[0].text, "Verify tests pass");
        assert_eq!(list[0].completed, false);

        // 3. Toggle todo
        toggle_todo("u-todo-user".to_string(), todo.id.clone()).await.unwrap();

        // 4. Retrieve again and verify completed = true
        let list_updated = get_todos("u-todo-user".to_string(), ws_id.to_string()).await.unwrap();
        assert_eq!(list_updated.len(), 1);
        assert_eq!(list_updated[0].completed, true);

        // Cleanup
        conn.execute("DELETE FROM todos WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
    }
}
