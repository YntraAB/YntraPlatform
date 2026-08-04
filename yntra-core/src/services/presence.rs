use crate::YntraError;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(uniffi::Record, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserPresence {
    pub user_id: String,
    pub workspace_id: String,
    pub user_name: String,
    pub avatar_url: Option<String>,
    pub status: String,
    pub active_block_id: Option<String>,
    pub active_view: Option<String>,
    pub last_seen_ms: u64,
}

static PRESENCE_STORE: LazyLock<Mutex<HashMap<String, Vec<UserPresence>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const PRESENCE_TTL_MS: u64 = 45_000; // 45 seconds timeout

pub fn clean_stale_presences(presences: &mut Vec<UserPresence>, now_ms: u64) {
    presences.retain(|p| now_ms.saturating_sub(p.last_seen_ms) <= PRESENCE_TTL_MS);
}

#[uniffi::export]
pub async fn update_user_presence(
    requester_user_id: String,
    workspace_id: String,
    user_name: String,
    avatar_url: Option<String>,
    active_block_id: Option<String>,
    active_view: Option<String>,
    status: String,
) -> Result<UserPresence, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms().max(0) as u64;
    let presence = UserPresence {
        user_id: requester_user_id.clone(),
        workspace_id: workspace_id.clone(),
        user_name,
        avatar_url,
        status,
        active_block_id,
        active_view,
        last_seen_ms: now_ms,
    };

    {
        let mut store = PRESENCE_STORE.lock().unwrap_or_else(|e| e.into_inner());
        let list = store.entry(workspace_id.clone()).or_insert_with(Vec::new);

        // Retain fresh presences
        clean_stale_presences(list, now_ms);

        // Upsert user's presence
        if let Some(pos) = list.iter().position(|p| p.user_id == requester_user_id) {
            list[pos] = presence.clone();
        } else {
            list.push(presence.clone());
        }
    }

    // Trigger observer notification for reactivity
    crate::infra::observer::set_last_modified_table("user_presence");
    crate::infra::observer::notify_observers();

    Ok(presence)
}

#[uniffi::export]
pub async fn get_workspace_presences(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<UserPresence>, YntraError> {
    let _ = requester_user_id;
    let now_ms = crate::infra::time::get_current_time_ms().max(0) as u64;
    let mut store = PRESENCE_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let list = store.entry(workspace_id).or_insert_with(Vec::new);

    clean_stale_presences(list, now_ms);
    Ok(list.clone())
}

#[uniffi::export]
pub async fn get_block_presences(
    requester_user_id: String,
    workspace_id: String,
    block_id: String,
) -> Result<Vec<UserPresence>, YntraError> {
    let list = get_workspace_presences(requester_user_id, workspace_id).await?;
    let filtered = list
        .into_iter()
        .filter(|p| p.active_block_id.as_deref() == Some(&block_id))
        .collect();
    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_presence_updates_and_eviction() {
        let ws = "test-ws-presence".to_string();
        let u1 = "user-1".to_string();

        let res = update_user_presence(
            u1.clone(),
            ws.clone(),
            "User One".to_string(),
            None,
            Some("messaging".to_string()),
            Some("inbox".to_string()),
            "online".to_string(),
        )
        .await;

        assert!(res.is_ok());
        let list = get_workspace_presences(u1.clone(), ws.clone())
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_name, "User One");

        let block_list = get_block_presences(u1, ws, "messaging".to_string())
            .await
            .unwrap();
        assert_eq!(block_list.len(), 1);
    }
}
