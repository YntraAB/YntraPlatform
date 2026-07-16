use crate::database;
use crate::{AuditLogEntry, YntraError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, LazyLock};
use uuid::Uuid;

static AUDIT_STORES: LazyLock<Mutex<HashMap<String, Arc<crate::ZeroCopyAuditStore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(target_arch = "wasm32")]
fn get_audit_store_path(workspace_id: &str) -> String {
    format!("yntra_zero_copy_audit_{}.db", workspace_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_audit_store_path(workspace_id: &str) -> String {
    #[cfg(test)]
    {
        let path = std::env::temp_dir()
            .join(format!("yntra_zero_copy_audit_{}_test.db", workspace_id))
            .to_string_lossy()
            .to_string();
        let _ = std::fs::remove_file(&path);
        path
    }
    #[cfg(not(test))]
    {
        crate::database::native::get_database_path(&format!("yntra_zero_copy_audit_{}.db", workspace_id))
    }
}

pub(crate) fn get_audit_store(workspace_id: &str) -> Arc<crate::ZeroCopyAuditStore> {
    let mut stores = AUDIT_STORES.lock().unwrap_or_else(|e| e.into_inner());
    stores
        .entry(workspace_id.to_string())
        .or_insert_with(|| {
            let path = get_audit_store_path(workspace_id);
            Arc::new(crate::ZeroCopyAuditStore::new(path).expect("Failed to initialize ZeroCopyAuditStore for Audit Logs"))
        })
        .clone()
}

pub async fn load_audits_from_opfs_internal(workspace_id: &str) -> Result<(), YntraError> {
    let store = get_audit_store(workspace_id);
    store.load_from_opfs().await?;
    Ok(())
}

#[uniffi::export]
pub async fn load_audits_from_opfs(workspace_id: String) -> Result<(), YntraError> {
    load_audits_from_opfs_internal(&workspace_id).await
}

fn compute_hash(
    id: &str,
    actor_id: &str,
    target_client_id: Option<&str>,
    action_type: &str,
    timestamp: i64,
    prev_hash: &str,
    seq: i64,
) -> String {
    let mut hasher = blake3::Hasher::new();

    // Hash each string field with its length prefix to prevent delimiter collisions / input canonicalization
    for field in &[
        id,
        actor_id,
        target_client_id.unwrap_or(""),
        action_type,
        prev_hash,
    ] {
        hasher.update(&(field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }

    // Hash timestamp and seq
    hasher.update(&timestamp.to_be_bytes());
    hasher.update(&seq.to_be_bytes());

    hasher.finalize().to_hex().to_string()
}

async fn get_workspace_signing_key(workspace_id: &str) -> Option<String> {
    let private_key_setting = format!("creator_private_key_{}", workspace_id);
    let creator_sk: Option<String> = crate::infra::crypto::get_local_secret(&private_key_setting)
        .await
        .unwrap_or(None);
    if let Some(ref sk) = creator_sk {
        if !sk.trim().is_empty() {
            return Some(sk.clone());
        }
    }
    None
}

fn sign_hash(private_key_hex: &str, hash_hex: &str) -> Result<String, YntraError> {
    let private_key_bytes = zeroize::Zeroizing::new(
        const_hex::decode(private_key_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?,
    );
    if private_key_bytes.len() != 32 {
        return Err(YntraError::CryptoError(
            "Invalid private key length".to_string(),
        ));
    }
    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);

    use ed25519_dalek::Signer;
    let signature = signing_key.sign(hash_hex.as_bytes());
    Ok(const_hex::encode(&signature.to_bytes()))
}

fn verify_signature(public_key_hex: &str, hash_hex: &str, signature_hex: &str) -> bool {
    let public_key_bytes = match const_hex::decode(public_key_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature_bytes = match const_hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let public_key_array: [u8; 32] = match public_key_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&public_key_array) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let signature_array: [u8; 64] = match signature_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let signature = ed25519_dalek::Signature::from_bytes(&signature_array);

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(hash_hex.as_bytes(), &signature)
        .is_ok()
}

async fn get_workspace_mappings(
    conn: &database::DbConnection,
    actor_ids: std::collections::HashSet<String>,
    client_ids: std::collections::HashSet<String>,
) -> Result<
    (
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, String>,
    ),
    YntraError,
> {
    let mut user_ws_map = std::collections::HashMap::new();
    let mut client_ws_map = std::collections::HashMap::new();

    if !actor_ids.is_empty() {
        let actor_vec: Vec<String> = actor_ids.into_iter().collect();
        for chunk in actor_vec.chunks(999) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{}", i)).collect();
            let sql = format!(
                "SELECT id, workspace_id FROM users WHERE id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql).await?;
            let mut rows = stmt
                .query(crate::rusqlite::params_from_iter(chunk.to_vec()))
                .await?;
            while let Some(row) = rows.next().await? {
                user_ws_map.insert(row.get::<String>(0)?, row.get::<String>(1)?);
            }
        }
    }

    if !client_ids.is_empty() {
        let client_vec: Vec<String> = client_ids.into_iter().collect();
        for chunk in client_vec.chunks(999) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{}", i)).collect();
            let sql = format!(
                "SELECT id, workspace_id FROM clients WHERE id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql).await?;
            let mut rows = stmt
                .query(crate::rusqlite::params_from_iter(chunk.to_vec()))
                .await?;
            while let Some(row) = rows.next().await? {
                client_ws_map.insert(row.get::<String>(0)?, row.get::<String>(1)?);
            }
        }
    }

    Ok((user_ws_map, client_ws_map))
}

pub async fn log_action_with_conn(
    conn: &database::DbConnection,
    actor_id: String,
    target_client_id: Option<String>,
    action_type: String,
) -> Result<AuditLogEntry, YntraError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = crate::infra::time::get_current_time_ms();

    // Load users & clients to resolve workspace IDs only for the current action
    let mut actor_ids = std::collections::HashSet::new();
    let mut client_ids = std::collections::HashSet::new();
    actor_ids.insert(actor_id.clone());
    if let Some(ref cid) = target_client_id {
        client_ids.insert(cid.clone());
    }

    let (user_ws_map, client_ws_map) = get_workspace_mappings(conn, actor_ids, client_ids).await?;

    let ws_id = if let Some(ref client_id) = target_client_id {
        client_ws_map.get(client_id).cloned().unwrap_or_else(|| {
            user_ws_map
                .get(&actor_id)
                .cloned()
                .unwrap_or_else(|| "workspace-1".to_string())
        })
    } else {
        user_ws_map
            .get(&actor_id)
            .cloned()
            .unwrap_or_else(|| "workspace-1".to_string())
    };

    let store = get_audit_store(&ws_id);
    let mut all_entries = store.read_all_audit_logs()?;

    // Find previous hash and seq for this workspace
    let mut prev_hash = "genesis".to_string();
    let mut seq = 0;

    let mut last_entry: Option<&AuditLogEntry> = None;
    for entry in all_entries.iter() {
        if entry.workspace_id == ws_id {
            if last_entry.is_none() || entry.seq > last_entry.unwrap().seq {
                last_entry = Some(entry);
            }
        }
    }

    if let Some(last) = last_entry {
        prev_hash = last.curr_hash.clone();
        seq = last.seq + 1;
    }

    let curr_hash = compute_hash(
        &id,
        &actor_id,
        target_client_id.as_deref(),
        &action_type,
        timestamp,
        &prev_hash,
        seq,
    );

    // Retrieve private key and sign hash
    let signature = if let Some(sk) = get_workspace_signing_key(&ws_id).await {
        sign_hash(&sk, &curr_hash).ok()
    } else {
        None
    };

    let entry = AuditLogEntry {
        id: id.clone(),
        workspace_id: ws_id.clone(),
        actor_id: actor_id.clone(),
        target_client_id: target_client_id.clone(),
        action_type: action_type.clone(),
        timestamp,
        prev_hash,
        curr_hash,
        seq,
        signature: signature.clone(),
    };

    all_entries.push(entry.clone());
    store.write_audit_logs(all_entries)?;

    Ok(entry)
}

#[uniffi::export]
pub async fn log_action(
    requester_user_id: String,
    actor_id: String,
    target_client_id: Option<String>,
    action_type: String,
) -> Result<AuditLogEntry, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Permission Check: Requester must be the actor OR have admin privileges
    if auth.role != "platform_admin" && auth.role != "admin" && requester_user_id != actor_id {
        return Err(YntraError::AuthError(
            "Access denied: you cannot log actions on behalf of another user".to_string(),
        ));
    }

    // Workspace scoping: If requester is not a platform admin, ensure actor belongs to the same workspace
    if auth.role != "platform_admin" {
        let actor_ws: Option<String> = conn
            .query_row(
                "SELECT workspace_id FROM users WHERE id = ?1",
                crate::params![&actor_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        if actor_ws.is_none() || actor_ws != Some(auth.workspace_id) {
            return Err(YntraError::AuthError(
                "Access denied: actor is in a different workspace".to_string(),
            ));
        }
    }

    let result = log_action_with_conn(&conn, actor_id, target_client_id, action_type).await;

    if result.is_ok() {
        crate::infra::observer::notify_observers();
    }
    result
}

#[uniffi::export]
pub async fn get_audit_logs(requester_user_id: String) -> Result<Vec<AuditLogEntry>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    if auth.role != "platform_admin" && auth.role != "admin" {
        return Err(YntraError::AuthError(
            "Access denied: only administrators can view audit logs".to_string(),
        ));
    }

    let store = get_audit_store(&ws_id);
    let all = store.read_all_audit_logs()?;

    if auth.role == "platform_admin" {
        let mut list = all;
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        return Ok(list);
    }

    let filtered: Vec<AuditLogEntry> = all
        .into_iter()
        .filter(|entry| entry.workspace_id == ws_id)
        .collect();

    // Sort by timestamp descending
    let mut list = filtered;
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(list)
}

#[uniffi::export]
pub async fn verify_audit_log_chain(requester_user_id: String) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.role != "admin" {
        return Err(YntraError::AuthError(
            "Access denied: administrator privileges required".to_string(),
        ));
    }

    let ws_id = auth.workspace_id.clone();
    let store = get_audit_store(&ws_id);
    let all = store.read_all_audit_logs()?;

    // Group by workspace
    let mut groups: std::collections::HashMap<String, Vec<AuditLogEntry>> =
        std::collections::HashMap::new();
    for entry in all {
        groups
            .entry(entry.workspace_id.clone())
            .or_default()
            .push(entry);
    }

    for (ws_id, mut ws_entries) in groups {
        if auth.role != "platform_admin" && ws_id != auth.workspace_id {
            continue;
        }
        let creator_pub: Option<String> = conn
            .query_row(
                "SELECT creator_public_key FROM workspaces WHERE id = ?1",
                crate::params![&ws_id],
                |r| r.get(0),
            )
            .await
            .ok();

        // Sort by seq ascending
        ws_entries.sort_by_key(|e| e.seq);

        let mut last_hash = "genesis".to_string();
        let mut expected_seq = 0;

        for entry in ws_entries {
            if entry.seq != expected_seq {
                tracing::error!(
                    "Audit log chain broken at log ID {} for workspace {}: seq {} does not match expected_seq {}",
                    entry.id,
                    ws_id,
                    entry.seq,
                    expected_seq
                );
                return Ok(false);
            }

            if entry.prev_hash != last_hash {
                tracing::error!(
                    "Audit log chain broken at log ID {} for workspace {}: prev_hash {} does not match expected last_hash {}",
                    entry.id,
                    ws_id,
                    entry.prev_hash,
                    last_hash
                );
                return Ok(false);
            }

            let computed = compute_hash(
                &entry.id,
                &entry.actor_id,
                entry.target_client_id.as_deref(),
                &entry.action_type,
                entry.timestamp,
                &entry.prev_hash,
                entry.seq,
            );
            if computed != entry.curr_hash {
                tracing::error!(
                    "Audit log hash mismatch at log ID {} for workspace {}: computed {}, got {}",
                    entry.id,
                    ws_id,
                    computed,
                    entry.curr_hash
                );
                return Ok(false);
            }

            // Verify signature
            if let Some(ref pub_key) = creator_pub {
                if !pub_key.trim().is_empty() {
                    if let Some(ref sig) = entry.signature {
                        if !verify_signature(pub_key, &entry.curr_hash, sig) {
                            tracing::error!(
                                "Audit log signature mismatch at log ID {} for workspace {}",
                                entry.id,
                                ws_id
                            );
                            return Ok(false);
                        }
                    } else {
                        tracing::error!(
                            "Missing audit log signature at log ID {} for workspace {}",
                            entry.id,
                            ws_id
                        );
                        return Ok(false);
                    }
                }
            }

            last_hash = entry.curr_hash.clone();
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
        let hash1 = compute_hash(
            "id1",
            "actor1",
            Some("client1"),
            "action1",
            1000,
            "genesis",
            0,
        );
        let hash2 = compute_hash("id2", "actor2", Some("client2"), "action2", 2000, &hash1, 1);

        assert_eq!(hash1.len(), 64);
        assert_eq!(hash2.len(), 64);
        assert_ne!(hash1, hash2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_audit_log_verification() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();
        // Clear audit store
        let _ = get_audit_store("workspace-test-verify").write_audit_logs(Vec::new());

        // Setup platform_admin user
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-test-verify', 'Verify WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-verify-admin', 'workspace-test-verify', 'verify@admin.io', 'platform_admin')", ()).await.unwrap();

        let res = verify_audit_log_chain("u-verify-admin".to_string()).await;
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Clean up
        conn.execute("DELETE FROM users WHERE id = 'u-verify-admin'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-test-verify'", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_audit_log_target_client_workspace_scoping()
    -> Result<(), Box<dyn std::error::Error>> {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Clear audit store
        let _ = get_audit_store("workspace-test-2").write_audit_logs(Vec::new());
        let _ = get_audit_store("workspace-test-1").write_audit_logs(Vec::new());

        // Setup workspace-test-1 and workspace-test-2
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-test-1', 'WS 1', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('workspace-test-2', 'WS 2', '[]', '{}')", ()).await.unwrap();

        // Generate and set up keypair for workspace-test-2 to test signing
        let keys = crate::infra::crypto::generate_workspace_keypair().unwrap();
        let pub_hex = keys.public_key();
        let priv_hex = keys.private_key();
        conn.execute(
            "UPDATE workspaces SET creator_public_key = ?1 WHERE id = 'workspace-test-2'",
            crate::params![pub_hex],
        )
        .await
        .unwrap();
        crate::infra::crypto::set_local_secret("creator_private_key_workspace-test-2", &priv_hex)
            .await
            .unwrap();

        // Actor in workspace-test-1
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('test-actor-1', 'workspace-test-1', 'actor@ws1.io', 'platform_admin')", ()).await.unwrap();

        // Client in workspace-test-2
        conn.execute("INSERT OR REPLACE INTO clients (id, workspace_id, first_name, last_name, care_level, created_at, updated_at) VALUES ('test-client-1', 'workspace-test-2', 'Jane', 'Doe', 'Normal', '2026-07-05', 0)", ()).await.unwrap();

        // Log action targetting the client
        let entry = log_action(
            "test-actor-1".to_string(),
            "test-actor-1".to_string(),
            Some("test-client-1".to_string()),
            "read_medications".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(entry.workspace_id, "workspace-test-2");

        // The entry signature should be present
        assert!(entry.signature.is_some());

        // Verify the entire chain is valid
        let chain_ok = verify_audit_log_chain("test-actor-1".to_string()).await.unwrap();
        assert!(chain_ok);

        // Clean up
        let _ = get_audit_store("workspace-test-2").write_audit_logs(Vec::new());
        let _ = get_audit_store("workspace-test-1").write_audit_logs(Vec::new());
        conn.execute("DELETE FROM clients WHERE id = 'test-client-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE id = 'test-actor-1'", ())
            .await
            .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id IN ('workspace-test-1', 'workspace-test-2')",
            (),
        )
        .await
        .unwrap();
        let _ = crate::infra::crypto::set_local_secret("creator_private_key_workspace-test-2", "")
            .await;
        Ok(())
    }
}
