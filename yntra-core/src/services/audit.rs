use crate::{AuditLogEntry, YntraError};
use uuid::Uuid;

use crate::database;
use crate::infra::observer::notify_observers;

fn compute_hash(id: &str, actor_id: &str, target_client_id: Option<&str>, action_type: &str, timestamp: i64, prev_hash: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    let data = format!(
        "{}:{}:{}:{}:{}:{}",
        id,
        actor_id,
        target_client_id.unwrap_or(""),
        action_type,
        timestamp,
        prev_hash
    );
    hasher.update(data.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[uniffi::export]
pub async fn log_action(actor_id: String, target_client_id: Option<String>, action_type: String) -> Result<AuditLogEntry, YntraError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = crate::infra::time::get_current_time_ms();
    
    let conn = database::acquire_connection().await?;
    
    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;
    
    let result = async {
        // Find previous hash
        let mut prev_hash = "genesis".to_string();
        let mut stmt = conn.prepare("SELECT curr_hash FROM audit_logs ORDER BY timestamp DESC, id DESC LIMIT 1").await?;
        let mut rows = stmt.query(()).await?;
        if let Some(row) = rows.next().await? {
            prev_hash = row.get(0)?;
        }
        
        let curr_hash = compute_hash(&id, &actor_id, target_client_id.as_deref(), &action_type, timestamp, &prev_hash);
        
        let entry = AuditLogEntry {
            id: id.clone(),
            actor_id: actor_id.clone(),
            target_client_id: target_client_id.clone(),
            action_type: action_type.clone(),
            timestamp,
            prev_hash,
            curr_hash,
        };
        
        conn.execute(
            "INSERT INTO audit_logs (id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            crate::params![
                entry.id,
                entry.actor_id,
                entry.target_client_id,
                entry.action_type,
                entry.timestamp,
                entry.prev_hash,
                entry.curr_hash
            ],
        ).await?;
        
        Ok::<AuditLogEntry, YntraError>(entry)
    }.await;

    match result {
        Ok(entry) => {
            conn.execute("COMMIT", ()).await?;
            notify_observers();
            Ok(entry)
        }
        Err(err) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(err)
        }
    }
}

#[uniffi::export]
pub async fn get_audit_logs() -> Result<Vec<AuditLogEntry>, YntraError> {
    let conn = database::acquire_connection().await?;
    let mut stmt = conn.prepare("SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash FROM audit_logs ORDER BY timestamp DESC, id DESC").await?;
    let mut rows = stmt.query(()).await?;
    let mut logs = Vec::new();
    while let Some(row) = rows.next().await? {
        logs.push(AuditLogEntry {
            id: row.get(0)?,
            actor_id: row.get(1)?,
            target_client_id: row.get(2)?,
            action_type: row.get(3)?,
            timestamp: row.get(4)?,
            prev_hash: row.get(5)?,
            curr_hash: row.get(6)?,
        });
    }
    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_hash_chain() {
        let hash1 = compute_hash("id1", "actor1", Some("client1"), "action1", 1000, "genesis");
        let hash2 = compute_hash("id2", "actor2", Some("client2"), "action2", 2000, &hash1);
        
        assert_eq!(hash1.len(), 64);
        assert_eq!(hash2.len(), 64);
        assert_ne!(hash1, hash2);
    }
}
