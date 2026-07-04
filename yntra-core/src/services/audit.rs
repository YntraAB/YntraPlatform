use crate::{AuditLogEntry, YntraError};
use uuid::Uuid;
use sha2::{Sha256, Digest};

#[cfg(not(target_arch = "wasm32"))]
use crate::database;
#[cfg(not(target_arch = "wasm32"))]
use crate::infra::observer::notify_observers;

#[cfg(target_arch = "wasm32")]
use crate::infra::wasm_store;
#[cfg(target_arch = "wasm32")]
use crate::infra::observer::notify_observers;

fn compute_hash(id: &str, actor_id: &str, target_client_id: Option<&str>, action_type: &str, timestamp: i64, prev_hash: &str) -> String {
    let mut hasher = Sha256::new();
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
    let result = hasher.finalize();
    
    let mut s = String::with_capacity(result.len() * 2);
    for &b in result.as_slice() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[uniffi::export]
pub async fn log_action(actor_id: String, target_client_id: Option<String>, action_type: String) -> Result<AuditLogEntry, YntraError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = crate::infra::time::get_current_time_ms();
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        
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
        
        notify_observers();
        Ok(entry)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        
        let mut prev_hash = "genesis".to_string();
        if let Some(last) = store.audit_logs.iter().max_by_key(|l| (l.timestamp, l.id.clone())) {
            prev_hash = last.curr_hash.clone();
        }
        
        let curr_hash = compute_hash(&id, &actor_id, target_client_id.as_deref(), &action_type, timestamp, &prev_hash);
        
        let entry = AuditLogEntry {
            id,
            actor_id,
            target_client_id,
            action_type,
            timestamp,
            prev_hash,
            curr_hash,
        };
        
        store.audit_logs.push(entry.clone());
        notify_observers();
        Ok(entry)
    }
}

#[uniffi::export]
pub async fn get_audit_logs() -> Result<Vec<AuditLogEntry>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let mut logs = store.audit_logs.clone();
        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then_with(|| b.id.cmp(&a.id)));
        Ok(logs)
    }
}
