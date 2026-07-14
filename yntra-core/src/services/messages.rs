use crate::database;
use crate::{MessageItem, YntraError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, LazyLock};

static MESSAGE_STORES: LazyLock<Mutex<HashMap<String, Arc<crate::ZeroCopyMessageStore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_arch = "wasm32")]
fn get_message_store_path(workspace_id: &str) -> String {
    format!("yntra_zero_copy_messages_{}.db", workspace_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_message_store_path(workspace_id: &str) -> String {
    let path = if cfg!(test) {
        std::env::temp_dir()
            .join(format!("yntra_zero_copy_messages_{}_test.db", workspace_id))
            .to_string_lossy()
            .to_string()
    } else {
        crate::database::native::get_database_path(&format!("yntra_zero_copy_messages_{}.db", workspace_id))
    };
    if cfg!(test) {
        let _ = std::fs::remove_file(&path);
    }
    path
}

pub(crate) fn get_message_store(workspace_id: &str) -> Arc<crate::ZeroCopyMessageStore> {
    let mut stores = MESSAGE_STORES.lock().unwrap();
    stores
        .entry(workspace_id.to_string())
        .or_insert_with(|| {
            let path = get_message_store_path(workspace_id);
            Arc::new(crate::ZeroCopyMessageStore::new(path).expect("Failed to initialize ZeroCopyMessageStore for Messages"))
        })
        .clone()
}

#[uniffi::export]
pub async fn load_messages_from_opfs(workspace_id: String) -> Result<(), YntraError> {
    let store = get_message_store(&workspace_id);
    store.load_from_opfs().await?;
    Ok(())
}

#[uniffi::export]
pub async fn get_messages(
    requester_user_id: String,
    user_id: String,
) -> Result<Vec<MessageItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let target_ws: String = if requester_user_id == user_id {
        auth.workspace_id.clone()
    } else {
        conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&user_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Target user not found".to_string()))?
    };

    if auth.role != "platform_admin" && requester_user_id != user_id {
        if auth.role == "admin" {
            if auth.workspace_id != target_ws {
                return Err(YntraError::AuthError(
                    "Access denied: target user is in a different workspace".to_string(),
                ));
            }
        } else {
            return Err(YntraError::AuthError(
                "Access denied: cannot view messages of other users".to_string(),
            ));
        }
    }

    // Determine the teams user_id is in
    let mut team_stmt = conn
        .prepare("SELECT team_id FROM team_members WHERE user_id = ?1")
        .await?;
    let user_teams: Vec<String> = team_stmt
        .query_map(crate::params![&user_id], |r| r.get(0))
        .await?
        .into_iter()
        .collect();

    let store = get_message_store(&target_ws);
    let filtered = store.read_messages_filtered(target_ws, user_id, user_teams)?;

    // Sort by created_at ascending
    let mut list = filtered;
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    Ok(list)
}

#[uniffi::export]
pub async fn send_message(
    requester_user_id: String,
    workspace_id: String,
    sender_id: String,
    receiver_id: Option<String>,
    team_id: Option<String>,
    subject: String,
    body: String,
) -> Result<MessageItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_message_store(&workspace_id);
    let mut messages = store.read_all_messages()?;

    let id = uuid::Uuid::new_v4().to_string();
    let item = MessageItem {
        id: id.clone(),
        workspace_id,
        sender_id: Some(sender_id),
        receiver_id,
        target_team_id: team_id,
        subject: Some(subject),
        body: Some(body),
        is_read: false,
        created_at: crate::infra::time::get_current_time_ms().to_string(),
        updated_at: crate::infra::time::get_current_time_ms(),
        sync_status: "pending".to_string(),
    };

    messages.push(item.clone());
    store.write_messages(messages)?;

    // Notify observers so the UI updates reactively
    crate::infra::observer::set_last_modified_table("messages");
    crate::infra::observer::notify_observers();

    Ok(item)
}

#[uniffi::export]
pub async fn mark_message_read(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let workspace_id = auth.workspace_id.clone();
    let store = get_message_store(&workspace_id);
    let mut messages = store.read_all_messages()?;

    let mut found_idx = None;
    for (idx, msg) in messages.iter().enumerate() {
        if msg.id == id {
            found_idx = Some(idx);
            break;
        }
    }

    let idx =
        found_idx.ok_or_else(|| YntraError::NotFoundError("Message not found".to_string()))?;
    let msg = &messages[idx];

    if auth.role != "platform_admin" && auth.workspace_id != msg.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let is_recipient = msg.receiver_id.as_deref() == Some(&requester_user_id);
    let mut is_team_member = false;
    if let Some(ref tid) = msg.target_team_id {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
                crate::params![tid, &requester_user_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);
        is_team_member = count > 0;
    }

    if auth.role != "platform_admin" && auth.role != "admin" && !is_recipient && !is_team_member {
        return Err(YntraError::AuthError(
            "Access denied: you are not the recipient of this message".to_string(),
        ));
    }

    // Mutate the message
    messages[idx].is_read = true;
    messages[idx].updated_at = now_ms;
    messages[idx].sync_status = "pending".to_string();

    store.write_messages(messages)?;

    // Notify observers so the UI updates reactively
    crate::infra::observer::set_last_modified_table("messages");
    crate::infra::observer::notify_observers();

    Ok(())
}

#[uniffi::export]
pub async fn get_messages_rkyv(
    requester_user_id: String,
    user_id: String,
) -> Result<Vec<u8>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let target_ws: String = if requester_user_id == user_id {
        auth.workspace_id.clone()
    } else {
        conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&user_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Target user not found".to_string()))?
    };

    if auth.role != "platform_admin" && requester_user_id != user_id {
        if auth.role == "admin" {
            if auth.workspace_id != target_ws {
                return Err(YntraError::AuthError(
                    "Access denied: target user is in a different workspace".to_string(),
                ));
            }
        } else {
            return Err(YntraError::AuthError(
                "Access denied: cannot view messages of other users".to_string(),
            ));
        }
    }

    // Since get_messages filters, we can just fetch and serialize the dynamic subset
    let messages = get_messages(requester_user_id, user_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&messages)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}

#[uniffi::export]
pub async fn get_message_by_id(
    requester_user_id: String,
    workspace_id: String,
    message_id: String,
) -> Result<Option<MessageItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let store = get_message_store(&workspace_id);
    store.read_message_zero_copy(message_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_messages_authorization() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Clear message store first to be safe
        let _ = get_message_store("workspace-1").write_messages(Vec::new());

        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                crate::database::native::block_on(async move {
                    if let Ok(c) = database::acquire_connection().await {
                        let _ = get_message_store("workspace-1").write_messages(Vec::new());
                        let _ = c.execute("DELETE FROM users WHERE id IN ('test-msg-user-1', 'test-msg-user-2')", ()).await;
                    }
                });
            }
        }
        let _cleanup = Cleanup;

        // Insert two test users
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-msg-user-1', 'workspace-1', 'msg1@yntra.io', 'assistant')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-msg-user-2', 'workspace-1', 'msg2@yntra.io', 'assistant')", ()).await.unwrap();

        // Verify user 1 can get their own messages
        let res1 = get_messages("test-msg-user-1".to_string(), "test-msg-user-1".to_string()).await;
        assert!(res1.is_ok());

        // Verify user 1 can get their own messages with rkyv
        let bytes1 =
            get_messages_rkyv("test-msg-user-1".to_string(), "test-msg-user-1".to_string()).await;
        assert!(bytes1.is_ok());
        let rkyv_msgs: Vec<MessageItem> =
            rkyv::from_bytes::<Vec<MessageItem>, rkyv::rancor::Error>(&bytes1.unwrap()).unwrap();
        assert_eq!(rkyv_msgs.len(), res1.unwrap().len());

        // Verify user 1 cannot get user 2's messages
        let res2 = get_messages("test-msg-user-1".to_string(), "test-msg-user-2".to_string()).await;
        assert!(res2.is_err());
        assert!(matches!(res2.unwrap_err(), YntraError::AuthError(_)));
    }
}
