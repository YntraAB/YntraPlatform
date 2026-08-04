use crate::database;
use crate::{TodoItem, YntraError};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static TODO_STORES: LazyLock<Mutex<HashMap<String, Arc<crate::ZeroCopyStore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_arch = "wasm32")]
fn get_todo_store_path(workspace_id: &str) -> String {
    format!("yntra_zero_copy_todos_{}.db", workspace_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_todo_store_path(workspace_id: &str) -> String {
    #[cfg(test)]
    {
        let path = std::env::temp_dir()
            .join(format!("yntra_zero_copy_todos_{}_test.db", workspace_id))
            .to_string_lossy()
            .to_string();
        let _ = std::fs::remove_file(&path);
        path
    }
    #[cfg(not(test))]
    {
        crate::database::native::get_database_path(&format!(
            "yntra_zero_copy_todos_{}.db",
            workspace_id
        ))
    }
}

pub fn get_todo_store(workspace_id: &str) -> Arc<crate::ZeroCopyStore> {
    let mut stores = TODO_STORES.lock().unwrap_or_else(|e| e.into_inner());
    stores
        .entry(workspace_id.to_string())
        .or_insert_with(|| {
            let path = get_todo_store_path(workspace_id);
            Arc::new(
                crate::ZeroCopyStore::new(path)
                    .expect("Failed to initialize ZeroCopyStore for Todos"),
            )
        })
        .clone()
}

pub async fn load_todos_from_opfs_internal(workspace_id: &str) -> Result<(), YntraError> {
    let store = get_todo_store(workspace_id);
    store.load_from_opfs().await?;
    Ok(())
}

#[uniffi::export]
pub async fn load_todos_from_opfs(workspace_id: String) -> Result<(), YntraError> {
    load_todos_from_opfs_internal(&workspace_id).await
}

#[uniffi::export]
pub async fn get_todos(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<TodoItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_todo_store(&workspace_id);
    let filtered = store.read_todos_by_workspace(workspace_id)?;
    Ok(filtered)
}

#[uniffi::export]
pub async fn add_todo(
    requester_user_id: String,
    workspace_id: String,
    text: String,
) -> Result<TodoItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_todo_store(&workspace_id);
    let id = uuid::Uuid::new_v4().to_string();
    let item = TodoItem {
        id: id.clone(),
        workspace_id,
        text,
        completed: false,
        updated_at: crate::infra::time::get_current_time_ms(),
        sync_status: "pending".to_string(),
    };

    store.upsert_todo(item.clone())?;
    Ok(item)
}

#[uniffi::export]
pub async fn toggle_todo(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let workspace_id = auth.workspace_id.clone();
    let store = get_todo_store(&workspace_id);

    let mut todo = store
        .read_todo_zero_copy(id.clone())?
        .ok_or_else(|| YntraError::NotFoundError("Todo not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != todo.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    todo.completed = !todo.completed;
    todo.updated_at = crate::infra::time::get_current_time_ms();
    todo.sync_status = "pending".to_string();

    store.upsert_todo(todo)?;
    Ok(())
}

#[uniffi::export]
pub async fn get_todos_rkyv(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<u8>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_todo_store(&workspace_id);
    let bytes = store.get_rkyv_bytes()?;
    Ok(bytes)
}

#[uniffi::export]
pub async fn get_todo_by_id(
    requester_user_id: String,
    workspace_id: String,
    todo_id: String,
) -> Result<Option<TodoItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_todo_store(&workspace_id);
    store.read_todo_zero_copy(todo_id)
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

        // Clear store first to be safe
        let _ = get_todo_store("ws-todo-test").write_todos(Vec::new());

        // 1. Add todo
        let ws_id = "ws-todo-test";
        let todo = add_todo(
            "u-todo-user".to_string(),
            ws_id.to_string(),
            "Verify tests pass".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(todo.text, "Verify tests pass");
        assert_eq!(todo.completed, false);
        assert_eq!(todo.workspace_id, ws_id);

        // 2. Get todos and assert it contains our added todo
        let list = get_todos("u-todo-user".to_string(), ws_id.to_string())
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, todo.id);
        assert_eq!(list[0].text, "Verify tests pass");
        assert_eq!(list[0].completed, false);

        // 3. Toggle todo
        toggle_todo("u-todo-user".to_string(), todo.id.clone())
            .await
            .unwrap();

        // 4. Retrieve again and verify completed = true
        let list_updated = get_todos("u-todo-user".to_string(), ws_id.to_string())
            .await
            .unwrap();
        assert_eq!(list_updated.len(), 1);
        assert_eq!(list_updated[0].completed, true);

        // Cleanup
        let _ = get_todo_store(ws_id).write_todos(Vec::new());
        let _ = conn
            .execute(
                "DELETE FROM users WHERE workspace_id = ?1",
                crate::params![ws_id],
            )
            .await;
        let _ = conn
            .execute(
                "DELETE FROM workspaces WHERE id = ?1",
                crate::params![ws_id],
            )
            .await;
    }
}
