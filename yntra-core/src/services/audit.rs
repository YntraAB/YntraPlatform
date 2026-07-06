use crate::{AuditLogEntry, YntraError};
use uuid::Uuid;

use crate::database;
use crate::infra::observer::notify_observers;

fn compute_hash(id: &str, actor_id: &str, target_client_id: Option<&str>, action_type: &str, timestamp: i64, prev_hash: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    
    // Hash each string field with its length prefix to prevent delimiter collisions / input canonicalization
    for field in &[id, actor_id, target_client_id.unwrap_or(""), action_type, prev_hash] {
        hasher.update(&(field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    
    // Hash timestamp
    hasher.update(&timestamp.to_be_bytes());
    
    hasher.finalize().to_hex().to_string()
}

#[uniffi::export]
pub async fn log_action(actor_id: String, target_client_id: Option<String>, action_type: String) -> Result<AuditLogEntry, YntraError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = crate::infra::time::get_current_time_ms();
    
    let conn = database::acquire_connection().await?;
    
    conn.execute("BEGIN IMMEDIATE TRANSACTION", ()).await?;
    
    let result = async {
        // Find actor's workspace_id
        let ws_id: String = conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&actor_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "workspace-1".to_string());

        // Find previous hash for this workspace
        let mut prev_hash = "genesis".to_string();
        let mut stmt = conn.prepare("SELECT curr_hash FROM audit_logs WHERE workspace_id = ?1 ORDER BY rowid DESC LIMIT 1").await?;
        let mut rows = stmt.query(crate::params![&ws_id]).await?;
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
            "INSERT INTO audit_logs (id, workspace_id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            crate::params![
                entry.id,
                ws_id,
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
pub async fn get_audit_logs(requester_user_id: String) -> Result<Vec<AuditLogEntry>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    let (query, params) = if auth.role == "platform_admin" {
        (
            "SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash FROM audit_logs ORDER BY timestamp DESC".to_string(),
            vec![],
        )
    } else if auth.role == "admin" {
        (
            "SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash
             FROM audit_logs
             WHERE workspace_id = ?1
             ORDER BY timestamp DESC".to_string(),
            vec![ws_id],
        )
    } else {
        return Err(YntraError::AuthError("Access denied: only administrators can view audit logs".to_string()));
    };

    let mut stmt = conn.prepare(&query).await?;
    let mut rows = stmt.query(crate::rusqlite::params_from_iter(params)).await?;
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

#[uniffi::export]
pub async fn verify_audit_log_chain() -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    
    // Get list of distinct workspaces
    let mut ws_stmt = conn.prepare("SELECT DISTINCT workspace_id FROM audit_logs").await?;
    let mut ws_rows = ws_stmt.query(()).await?;
    let mut workspaces = Vec::new();
    while let Some(row) = ws_rows.next().await? {
        workspaces.push(row.get::<String>(0)?);
    }
    
    for ws_id in workspaces {
        let mut stmt = conn.prepare("SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash FROM audit_logs WHERE workspace_id = ?1 ORDER BY rowid ASC").await?;
        let mut rows = stmt.query(crate::params![&ws_id]).await?;
        
        let mut expected_prev_hash = "genesis".to_string();
        
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let actor_id: String = row.get(1)?;
            let target_client_id: Option<String> = row.get(2)?;
            let action_type: String = row.get(3)?;
            let timestamp: i64 = row.get(4)?;
            let prev_hash: String = row.get(5)?;
            let curr_hash: String = row.get(6)?;
            
            if prev_hash != expected_prev_hash {
                tracing::error!("Audit log chain broken at log ID {} for workspace {}: expected prev_hash {}, got {}", id, ws_id, expected_prev_hash, prev_hash);
                return Ok(false);
            }
            
            let computed = compute_hash(&id, &actor_id, target_client_id.as_deref(), &action_type, timestamp, &prev_hash);
            if computed != curr_hash {
                tracing::error!("Audit log hash mismatch at log ID {} for workspace {}: computed {}, got {}", id, ws_id, computed, curr_hash);
                return Ok(false);
            }
            
            expected_prev_hash = curr_hash;
        }
    }
    
    Ok(true)
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_audit_log_verification() {
        let res = verify_audit_log_chain().await;
        // In clean test DB setup, this should return Ok(true)
        assert!(res.is_ok());
        assert!(res.unwrap());
    }
}
