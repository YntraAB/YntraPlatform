use crate::{AuditLogEntry, YntraError};
use uuid::Uuid;

use crate::database;
use crate::infra::observer::notify_observers;

fn compute_hash(id: &str, actor_id: &str, target_client_id: Option<&str>, action_type: &str, timestamp: i64, prev_hash: &str, seq: i64) -> String {
    let mut hasher = blake3::Hasher::new();
    
    // Hash each string field with its length prefix to prevent delimiter collisions / input canonicalization
    for field in &[id, actor_id, target_client_id.unwrap_or(""), action_type, prev_hash] {
        hasher.update(&(field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    
    // Hash timestamp and seq
    hasher.update(&timestamp.to_be_bytes());
    hasher.update(&seq.to_be_bytes());
    
    hasher.finalize().to_hex().to_string()
}

pub async fn log_action_with_conn(
    conn: &database::DbConnection,
    actor_id: String,
    target_client_id: Option<String>,
    action_type: String,
) -> Result<AuditLogEntry, YntraError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = crate::infra::time::get_current_time_ms();

    // Find workspace_id of client or fallback to actor
    let mut ws_id: Option<String> = None;
    if let Some(ref client_id) = target_client_id {
        ws_id = conn.query_row(
            "SELECT workspace_id FROM clients WHERE id = ?1",
            crate::params![client_id],
            |r| r.get(0)
        ).await.ok();
    }

    let ws_id = match ws_id {
        Some(w) => w,
        None => conn.query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&actor_id],
            |r| r.get(0)
        ).await.unwrap_or_else(|_| "workspace-1".to_string()),
    };

    // Find previous hash and seq for this workspace
    let mut prev_hash = "genesis".to_string();
    let mut seq = 0;
    let mut stmt = conn.prepare("SELECT curr_hash, seq FROM audit_logs WHERE workspace_id = ?1 ORDER BY seq DESC LIMIT 1").await?;
    let mut rows = stmt.query(crate::params![&ws_id]).await?;
    if let Some(row) = rows.next().await? {
        prev_hash = row.get(0)?;
        seq = row.get::<i64>(1)? + 1;
    }
    
    let curr_hash = compute_hash(&id, &actor_id, target_client_id.as_deref(), &action_type, timestamp, &prev_hash, seq);
    
    let entry = AuditLogEntry {
        id: id.clone(),
        actor_id: actor_id.clone(),
        target_client_id: target_client_id.clone(),
        action_type: action_type.clone(),
        timestamp,
        prev_hash,
        curr_hash,
        seq,
    };
    
    conn.execute(
        "INSERT INTO audit_logs (id, workspace_id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            entry.id,
            ws_id,
            entry.actor_id,
            entry.target_client_id,
            entry.action_type,
            entry.timestamp,
            entry.prev_hash,
            entry.curr_hash,
            entry.seq
        ],
    ).await?;
    
    Ok(entry)
}

#[uniffi::export]
pub async fn log_action(actor_id: String, target_client_id: Option<String>, action_type: String) -> Result<AuditLogEntry, YntraError> {
    let conn = database::acquire_connection().await?;
    conn.begin_transaction().await?;
    
    let result = log_action_with_conn(&conn, actor_id, target_client_id, action_type).await;

    match result {
        Ok(entry) => {
            conn.commit().await?;
            notify_observers();
            Ok(entry)
        }
        Err(err) => {
            let _ = conn.rollback().await;
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
            "SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq FROM audit_logs ORDER BY timestamp DESC".to_string(),
            vec![],
        )
    } else if auth.role == "admin" {
        (
            "SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq
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
            seq: row.get(7)?,
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
    
    let mut stmt = conn.prepare("SELECT id, actor_id, target_client_id, action_type, timestamp, prev_hash, curr_hash, seq FROM audit_logs WHERE workspace_id = ?1 ORDER BY seq ASC").await?;
    for ws_id in workspaces {
        let mut rows = stmt.query(crate::params![&ws_id]).await?;
        
        let mut last_hash = "genesis".to_string();
        let mut expected_seq = 0;
        
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let actor_id: String = row.get(1)?;
            let target_client_id: Option<String> = row.get(2)?;
            let action_type: String = row.get(3)?;
            let timestamp: i64 = row.get(4)?;
            let prev_hash: String = row.get(5)?;
            let curr_hash: String = row.get(6)?;
            let seq: i64 = row.get(7)?;
            
            if seq != expected_seq {
                tracing::error!("Audit log chain broken at log ID {} for workspace {}: seq {} does not match expected_seq {}", id, ws_id, seq, expected_seq);
                return Ok(false);
            }
            
            if prev_hash != last_hash {
                tracing::error!("Audit log chain broken at log ID {} for workspace {}: prev_hash {} does not match expected last_hash {}", id, ws_id, prev_hash, last_hash);
                return Ok(false);
            }
            
            let computed = compute_hash(&id, &actor_id, target_client_id.as_deref(), &action_type, timestamp, &prev_hash, seq);
            if computed != curr_hash {
                tracing::error!("Audit log hash mismatch at log ID {} for workspace {}: computed {}, got {}", id, ws_id, computed, curr_hash);
                return Ok(false);
            }
            
            last_hash = curr_hash;
            expected_seq += 1;
        }
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_hash_chain() {
        let hash1 = compute_hash("id1", "actor1", Some("client1"), "action1", 1000, "genesis", 0);
        let hash2 = compute_hash("id2", "actor2", Some("client2"), "action2", 2000, &hash1, 1);
        
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_audit_log_target_client_workspace_scoping() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup
        let _ = conn.execute("DELETE FROM audit_logs WHERE actor_id = 'test-actor-1'", ()).await;
        let _ = conn.execute("DELETE FROM clients WHERE id = 'test-client-1'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE id = 'test-actor-1'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('workspace-test-1', 'workspace-test-2')", ()).await;

        // Setup workspace-test-1 and workspace-test-2
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-test-1', 'WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-test-2', 'WS 2', '[]', '{}')", ()).await.unwrap();

        // Actor in workspace-test-1
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-actor-1', 'workspace-test-1', 'actor@ws1.io', 'platform_admin')", ()).await.unwrap();

        // Client in workspace-test-2
        conn.execute("INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, care_level, created_at, updated_at) VALUES ('test-client-1', 'workspace-test-2', 'Jane', 'Doe', 'Normal', '2026-07-05', 0)", ()).await.unwrap();

        // Log action targetting the client
        let entry = log_action("test-actor-1".to_string(), Some("test-client-1".to_string()), "read_medications".to_string()).await.unwrap();

        // The audit log should be associated with workspace-test-2 (client's workspace)
        let logged_ws: String = conn.query_row(
            "SELECT workspace_id FROM audit_logs WHERE id = ?1",
            crate::params![entry.id],
            |r| r.get(0)
        ).await.unwrap();

        assert_eq!(logged_ws, "workspace-test-2");

        // Clean up
        conn.execute("DELETE FROM audit_logs WHERE actor_id = 'test-actor-1'", ()).await.unwrap();
        conn.execute("DELETE FROM clients WHERE id = 'test-client-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'test-actor-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('workspace-test-1', 'workspace-test-2')", ()).await.unwrap();
    }
}
