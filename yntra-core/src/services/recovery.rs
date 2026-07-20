use crate::database;
use crate::errors::YntraError;
use crate::infra::crypto;
use crate::infra::observer::notify_observers;

#[uniffi::export]
pub async fn split_workspace_key(
    requester_user_id: String,
    workspace_id: String,
    threshold: u32,
    total: u32,
) -> Result<Vec<String>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Admin privileges required".to_string()));
    }
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Workspace mismatch".to_string()));
    }

    let priv_key_opt = crypto::get_local_secret(&format!("creator_private_key_{}", workspace_id)).await?;
    let priv_key = priv_key_opt.ok_or_else(|| {
        YntraError::CryptoError("Workspace private key not found on this device".to_string())
    })?;

    let shards = crypto::split_secret(priv_key.as_bytes(), threshold as usize, total as usize)?;
    let shard_strings = shards
        .into_iter()
        .map(|(idx, data)| format!("{}:{}", idx, const_hex::encode(&data)))
        .collect();

    let settings_str: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT settings FROM workspaces WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![workspace_id]).await?;
        if let Some(row) = rows.next().await? {
            Some(row.get(0)?)
        } else {
            None
        }
    };

    if let Some(settings_str) = settings_str {
        let mut settings_json: serde_json::Value =
            serde_json::from_str(&settings_str).unwrap_or_else(|_| serde_json::json!({}));

        settings_json["recovery_policy"] = serde_json::json!({
            "threshold": threshold,
            "total_shards": total,
        });

        let new_settings_str = settings_json.to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute(
            "UPDATE workspaces SET settings = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![new_settings_str, now_ms, workspace_id],
        ).await?;
    }

    notify_observers();
    Ok(shard_strings)
}

#[uniffi::export]
pub async fn reconstruct_workspace_key(
    workspace_id: String,
    shard_strings: Vec<String>,
    threshold: u32,
) -> Result<(), YntraError> {
    let mut shards = Vec::new();
    for s in &shard_strings {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(YntraError::CryptoError("Invalid shard format".to_string()));
        }
        let idx = parts[0]
            .parse::<u8>()
            .map_err(|_| YntraError::CryptoError("Invalid shard index".to_string()))?;
        let data = const_hex::decode(parts[1])
            .map_err(|e| YntraError::CryptoError(format!("Invalid shard hex: {:?}", e)))?;
        shards.push((idx, data));
    }

    let reconstructed_bytes = crypto::reconstruct_secret(&shards, threshold as usize)?;
    let reconstructed_str = String::from_utf8(reconstructed_bytes)
        .map_err(|_| YntraError::CryptoError("Reconstructed key is invalid UTF-8".to_string()))?;

    crypto::set_local_secret(
        &format!("creator_private_key_{}", workspace_id),
        &reconstructed_str,
    )
    .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn escrow_user_key(
    requester_user_id: String,
    target_user_id: String,
    encrypted_key: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if requester_user_id != target_user_id {
        return Err(YntraError::AuthError("Users can only escrow their own key".to_string()));
    }

    let metadata_str: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT metadata FROM users WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![target_user_id]).await?;
        if let Some(row) = rows.next().await? {
            Some(row.get(0)?)
        } else {
            None
        }
    };

    if let Some(metadata_str) = metadata_str {
        let mut metadata_json: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({}));

        metadata_json["escrowed_private_key"] = serde_json::Value::String(encrypted_key);
        let new_metadata_str = metadata_json.to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute(
            "UPDATE users SET metadata = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![new_metadata_str, now_ms, target_user_id],
        ).await?;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn request_key_recovery(requester_user_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let metadata_str: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT metadata FROM users WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![requester_user_id]).await?;
        if let Some(row) = rows.next().await? {
            Some(row.get(0)?)
        } else {
            None
        }
    };

    if let Some(metadata_str) = metadata_str {
        let mut metadata_json: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({}));

        metadata_json["recovery_status"] = serde_json::Value::String("pending".to_string());
        let new_metadata_str = metadata_json.to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute(
            "UPDATE users SET metadata = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![new_metadata_str, now_ms, requester_user_id],
        ).await?;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn approve_key_recovery(
    admin_user_id: String,
    target_user_id: String,
    temp_wrapped_key: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &admin_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Admin privileges required".to_string()));
    }

    let metadata_str: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT metadata FROM users WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![target_user_id]).await?;
        if let Some(row) = rows.next().await? {
            Some(row.get(0)?)
        } else {
            None
        }
    };

    if let Some(metadata_str) = metadata_str {
        let mut metadata_json: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({}));

        metadata_json["recovery_status"] = serde_json::Value::String("approved".to_string());
        metadata_json["temp_wrapped_key"] = serde_json::Value::String(temp_wrapped_key);
        let new_metadata_str = metadata_json.to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute(
            "UPDATE users SET metadata = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![new_metadata_str, now_ms, target_user_id],
        ).await?;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn claim_recovered_key(
    requester_user_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let metadata_str: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT metadata FROM users WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![requester_user_id]).await?;
        if let Some(row) = rows.next().await? {
            Some(row.get(0)?)
        } else {
            None
        }
    };

    if let Some(metadata_str) = metadata_str {
        let mut metadata_json: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({}));

        let status = metadata_json
            .get("recovery_status")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if status != "approved" {
            return Err(YntraError::AuthError("Recovery has not been approved yet".to_string()));
        }

        let temp_wrapped_key = metadata_json
            .get("temp_wrapped_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        metadata_json.as_object_mut().map(|m| {
            m.remove("recovery_status");
            m.remove("temp_wrapped_key");
        });

        let new_metadata_str = metadata_json.to_string();
        let now_ms = crate::infra::time::get_current_time_ms();
        conn.execute(
            "UPDATE users SET metadata = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
            crate::params![new_metadata_str, now_ms, requester_user_id],
        ).await?;

        notify_observers();
        return Ok(temp_wrapped_key);
    }

    Err(YntraError::AuthError("User not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_recovery_lifecycle() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup mock workspace and users
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-recovery-test', 'Test School', '{}', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, password_hash) VALUES ('u-admin', 'ws-recovery-test', 'admin@school.com', 'admin', 'hash')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, password_hash) VALUES ('u-teacher', 'ws-recovery-test', 'teacher@school.com', 'user', 'hash')", ()).await.unwrap();

        let initial_priv_key = "41528659d48b1bfcb4659b85c13b28b78997a0a0a0a0a0a0a0a0a0a0a0a0a0a0";
        crypto::set_local_secret("creator_private_key_ws-recovery-test", initial_priv_key).await.unwrap();

        // 2. Split Workspace Key
        let shards = split_workspace_key("u-admin".to_string(), "ws-recovery-test".to_string(), 2, 3).await.unwrap();
        assert_eq!(shards.len(), 3);

        // 3. Simulate lockout (delete local private key)
        crypto::set_local_secret("creator_private_key_ws-recovery-test", "").await.unwrap();
        let missing = crypto::get_local_secret("creator_private_key_ws-recovery-test").await.unwrap();
        assert!(missing.unwrap_or_default().is_empty());

        // 4. Reconstruct from threshold (2 shards)
        let selected_shards = vec![shards[0].clone(), shards[2].clone()];
        reconstruct_workspace_key("ws-recovery-test".to_string(), selected_shards, 2).await.unwrap();

        // 5. Verify restored
        let restored = crypto::get_local_secret("creator_private_key_ws-recovery-test").await.unwrap().unwrap();
        assert_eq!(restored, initial_priv_key);

        // 6. User Escrow Key
        let encrypted_secret_key = "enc:nonce:ciphertext";
        escrow_user_key("u-teacher".to_string(), "u-teacher".to_string(), encrypted_secret_key.to_string()).await.unwrap();

        // 7. Request Recovery
        request_key_recovery("u-teacher".to_string()).await.unwrap();

        // 8. Approve Recovery
        approve_key_recovery("u-admin".to_string(), "u-teacher".to_string(), "temp-wrapped-key-value".to_string()).await.unwrap();

        // 9. Claim Recovery
        let claimed = claim_recovered_key("u-teacher".to_string()).await.unwrap();
        assert_eq!(claimed, "temp-wrapped-key-value");

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-recovery-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-recovery-test'", ()).await.unwrap();
    }
}

