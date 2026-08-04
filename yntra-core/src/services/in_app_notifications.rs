use crate::YntraError;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(uniffi::Record, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InAppNotification {
    pub id: String,
    pub workspace_id: String,
    pub recipient_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub title: String,
    pub body: String,
    pub target_url: Option<String>,
    pub notification_type: String, // "mention", "reply", "alert"
    pub is_read: bool,
    pub created_at_ms: u64,
}

static NOTIFICATION_STORES: LazyLock<Mutex<HashMap<String, Vec<InAppNotification>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[uniffi::export]
pub async fn get_user_notifications(
    requester_user_id: String,
    user_id: String,
) -> Result<Vec<InAppNotification>, YntraError> {
    let _ = requester_user_id;
    let store = NOTIFICATION_STORES.lock().unwrap_or_else(|e| e.into_inner());
    
    let mut all_notifs: Vec<InAppNotification> = Vec::new();
    for list in store.values() {
        for notif in list {
            if notif.recipient_id == user_id {
                all_notifs.push(notif.clone());
            }
        }
    }

    all_notifs.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    Ok(all_notifs)
}

#[uniffi::export]
pub async fn create_in_app_notification(
    requester_user_id: String,
    workspace_id: String,
    recipient_id: String,
    sender_name: String,
    title: String,
    body: String,
    target_url: Option<String>,
    notification_type: String,
) -> Result<InAppNotification, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms().max(0) as u64;
    let notif = InAppNotification {
        id: uuid::Uuid::new_v4().to_string(),
        workspace_id: workspace_id.clone(),
        recipient_id,
        sender_id: requester_user_id,
        sender_name,
        title,
        body,
        target_url,
        notification_type,
        is_read: false,
        created_at_ms: now_ms,
    };

    {
        let mut store = NOTIFICATION_STORES.lock().unwrap_or_else(|e| e.into_inner());
        let list = store.entry(workspace_id).or_insert_with(Vec::new);
        list.push(notif.clone());
    }

    // Trigger reactive observer update for notifications
    crate::infra::observer::set_last_modified_table("in_app_notifications");
    crate::infra::observer::notify_observers();

    Ok(notif)
}

#[uniffi::export]
pub async fn mark_notification_read(
    requester_user_id: String,
    notification_id: String,
) -> Result<(), YntraError> {
    let _ = requester_user_id;
    let mut store = NOTIFICATION_STORES.lock().unwrap_or_else(|e| e.into_inner());
    
    for list in store.values_mut() {
        if let Some(n) = list.iter_mut().find(|item| item.id == notification_id) {
            n.is_read = true;
            break;
        }
    }

    crate::infra::observer::set_last_modified_table("in_app_notifications");
    crate::infra::observer::notify_observers();

    Ok(())
}

#[uniffi::export]
pub async fn mark_all_notifications_read(
    requester_user_id: String,
    user_id: String,
) -> Result<(), YntraError> {
    let _ = requester_user_id;
    let mut store = NOTIFICATION_STORES.lock().unwrap_or_else(|e| e.into_inner());
    
    for list in store.values_mut() {
        for item in list.iter_mut() {
            if item.recipient_id == user_id {
                item.is_read = true;
            }
        }
    }

    crate::infra::observer::set_last_modified_table("in_app_notifications");
    crate::infra::observer::notify_observers();

    Ok(())
}

/// Helper function that parses `@name` mentions from text and creates in-app notifications
#[uniffi::export]
pub async fn dispatch_mention_notifications(
    requester_user_id: String,
    workspace_id: String,
    sender_name: String,
    content: String,
    target_url: Option<String>,
) -> Result<Vec<InAppNotification>, YntraError> {
    let users = crate::services::users::get_users(requester_user_id.clone()).await?;
    let mut created_notifs = Vec::new();

    // Find all `@word` tokens
    let mention_tokens: Vec<&str> = content
        .split_whitespace()
        .filter(|w| w.starts_with('@') && w.len() > 1)
        .map(|w| w.trim_start_matches('@').trim_matches(|c: char| !c.is_alphanumeric()))
        .collect();

    if mention_tokens.is_empty() {
        return Ok(created_notifs);
    }

    for user in users {
        if user.id == requester_user_id {
            continue; // Don't notify self
        }

        let user_name_lower = user.full_name.unwrap_or_default().to_lowercase();
        let user_email_prefix = user.email.split('@').next().unwrap_or("").to_lowercase();

        let is_mentioned = mention_tokens.iter().any(|tok| {
            let tok_lower = tok.to_lowercase();
            tok_lower == user.id.to_lowercase()
                || tok_lower == user_email_prefix
                || user_name_lower.contains(&tok_lower)
        });

        if is_mentioned {
            let notif = create_in_app_notification(
                requester_user_id.clone(),
                workspace_id.clone(),
                user.id.clone(),
                sender_name.clone(),
                format!("You were mentioned by {}", sender_name),
                content.clone(),
                target_url.clone(),
                "mention".to_string(),
            )
            .await?;

            created_notifs.push(notif);
        }
    }

    Ok(created_notifs)
}

static PUSH_TOKENS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[uniffi::export]
pub async fn register_device_push_token(
    requester_user_id: String,
    device_token: String,
    platform: String,
) -> Result<(), YntraError> {
    let _ = platform;
    let mut tokens = PUSH_TOKENS.lock().unwrap_or_else(|e| e.into_inner());
    tokens.insert(requester_user_id, device_token);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_app_notification_creation_and_mentions() {
        let ws = "ws-notif-test".to_string();
        let u1 = "user-sender".to_string();
        let u2 = "user-recipient".to_string();

        let notif = create_in_app_notification(
            u1.clone(),
            ws.clone(),
            u2.clone(),
            "Alice".to_string(),
            "Mention".to_string(),
            "Hey @Bob check this out".to_string(),
            None,
            "mention".to_string(),
        )
        .await
        .unwrap();

        let list = get_user_notifications(u2.clone(), u2.clone()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, notif.id);
        assert!(!list[0].is_read);

        mark_notification_read(u2.clone(), notif.id.clone()).await.unwrap();
        let list_updated = get_user_notifications(u2.clone(), u2.clone()).await.unwrap();
        assert!(list_updated[0].is_read);
    }
}
