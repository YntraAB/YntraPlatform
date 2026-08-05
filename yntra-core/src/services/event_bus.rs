use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::time::get_current_time_ms;
use crate::YntraError;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct CoreEventRule {
    pub id: String,
    pub workspace_id: String,
    pub source_block_id: String,
    pub target_block_id: String,
    pub trigger_event: String,
    pub action_type: String,
    pub config_json: String,
    pub is_active: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct CoreEventDispatchResult {
    pub event_id: String,
    pub workspace_id: String,
    pub block_id: String,
    pub event_type: String,
    pub executed_rules_count: i32,
    pub status: String,
}

#[uniffi::export]
pub async fn register_event_rule(
    requester_user_id: String,
    workspace_id: String,
    source_block_id: String,
    target_block_id: String,
    trigger_event: String,
    action_type: String,
    config_json: Option<String>,
) -> Result<CoreEventRule, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let rule_id = format!("rule_{}", Uuid::new_v4().simple());
    let cfg = config_json.unwrap_or_else(|| "{}".to_string());
    let now = get_current_time_ms();

    conn.execute(
        "INSERT INTO event_rules (id, workspace_id, source_block_id, target_block_id, trigger_event, action_type, config_json, is_active, created_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, 'synced')",
        crate::params![
            &rule_id,
            &workspace_id,
            &source_block_id,
            &target_block_id,
            &trigger_event,
            &action_type,
            &cfg,
            now
        ],
    )
    .await?;

    notify_observers();

    Ok(CoreEventRule {
        id: rule_id,
        workspace_id,
        source_block_id,
        target_block_id,
        trigger_event,
        action_type,
        config_json: cfg,
        is_active: true,
    })
}

#[uniffi::export]
pub async fn get_event_rules(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<CoreEventRule>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, source_block_id, target_block_id, trigger_event, action_type, config_json, is_active FROM event_rules WHERE workspace_id = ?1")
        .await?;

    let rules: Vec<CoreEventRule> = stmt
        .query_map(crate::params![&workspace_id], |r| {
            Ok(CoreEventRule {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                source_block_id: r.get(2)?,
                target_block_id: r.get(3)?,
                trigger_event: r.get(4)?,
                action_type: r.get(5)?,
                config_json: r.get(6)?,
                is_active: r.get::<i64>(7)? != 0,
            })
        })
        .await?
        .into_iter()
        .collect();

    Ok(rules)
}

#[uniffi::export]
pub async fn dispatch_core_event(
    requester_user_id: String,
    workspace_id: String,
    block_id: String,
    event_type: String,
    payload_json: String,
) -> Result<CoreEventDispatchResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let now = get_current_time_ms();

    // 1. Log event
    conn.execute(
        "INSERT INTO event_logs (id, workspace_id, block_id, event_type, payload_json, status, created_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, 'processed', ?6, 'synced')",
        crate::params![&event_id, &workspace_id, &block_id, &event_type, &payload_json, now],
    )
    .await?;

    // 2. Fetch matching active event rules
    let mut stmt = conn
        .prepare("SELECT target_block_id, action_type, config_json FROM event_rules WHERE workspace_id = ?1 AND trigger_event = ?2 AND is_active = 1")
        .await?;

    let matching_rules: Vec<(String, String, String)> = stmt
        .query_map(crate::params![&workspace_id, &event_type], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .await?
        .into_iter()
        .collect();

    let mut executed_count = 0;

    for (target_block, action_type, _config) in matching_rules {
        executed_count += 1;
        match action_type.as_str() {
            "UpdateJobCosting" => {
                // Pre-wired pipeline: Insert or log cost adjustment into entities table
                let entity_id = format!("cost_{}", Uuid::new_v4().simple());
                let data_str = format!(
                    "{{\"source_event\":\"{}\",\"action\":\"job_cost_recorded\",\"details\":{}}}",
                    event_id, payload_json
                );
                let _ = conn
                    .execute(
                        "INSERT INTO entities (id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, 'job_cost', ?4, ?5, ?5, 'synced')",
                        crate::params![&entity_id, &workspace_id, &target_block, &data_str, now],
                    )
                    .await;
            }
            "PostEmergencyAlert" => {
                // Pre-wired pipeline: Post alert into messages table
                let msg_id = format!("alert_{}", Uuid::new_v4().simple());
                let alert_content = format!(
                    "[AUTOMATED SYSTEM ALERT] Emergency or high-priority trigger from {}: {}",
                    block_id, payload_json
                );
                let _ = conn
                    .execute(
                        "INSERT INTO messages (id, workspace_id, sender_id, recipient_id, content, created_at) VALUES (?1, ?2, ?3, 'general', ?4, ?5)",
                        crate::params![&msg_id, &workspace_id, &requester_user_id, &alert_content, now],
                    )
                    .await;
            }
            _ => {
                // Generic execution log fallback
                let entity_id = format!("rule_exec_{}", Uuid::new_v4().simple());
                let data_str = format!(
                    "{{\"rule_action\":\"{}\",\"trigger_event\":\"{}\",\"payload\":{}}}",
                    action_type, event_type, payload_json
                );
                let _ = conn
                    .execute(
                        "INSERT INTO entities (id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, 'rule_execution', ?4, ?5, ?5, 'synced')",
                        crate::params![&entity_id, &workspace_id, &target_block, &data_str, now],
                    )
                    .await;
            }
        }
    }

    notify_observers();

    Ok(CoreEventDispatchResult {
        event_id,
        workspace_id,
        block_id,
        event_type,
        executed_rules_count: executed_count,
        status: "processed".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_core_event_bus_flow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-bus-test', 'Bus WS', '[]', '{}')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-bus-admin', 'ws-bus-test', 'bus@test.io', 'hash', 'platform_admin')",
            (),
        )
        .await
        .unwrap();

        // Register rule: TimeLogSubmitted -> UpdateJobCosting
        let rule = register_event_rule(
            "u-bus-admin".to_string(),
            "ws-bus-test".to_string(),
            "time".to_string(),
            "jobs".to_string(),
            "TimeLogSubmitted".to_string(),
            "UpdateJobCosting".to_string(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(rule.trigger_event, "TimeLogSubmitted");

        // Dispatch event
        let res = dispatch_core_event(
            "u-bus-admin".to_string(),
            "ws-bus-test".to_string(),
            "time".to_string(),
            "TimeLogSubmitted".to_string(),
            r#"{"hours": 8, "job_id": "job-101"}"#.to_string(),
        )
        .await
        .unwrap();

        assert_eq!(res.executed_rules_count, 1);
        assert_eq!(res.status, "processed");

        // Verify entities inserted for jobs block
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE workspace_id = 'ws-bus-test' AND block_id = 'jobs' AND entity_type = 'job_cost'",
                (),
                |r| r.get(0),
            )
            .await
            .unwrap();

        assert_eq!(count, 1);
    }
}
