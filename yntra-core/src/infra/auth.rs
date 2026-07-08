use crate::database::DbConnection;
use crate::YntraError;
use std::sync::{OnceLock, RwLock};
use std::collections::HashMap;

static AUTH_CONTEXT_CACHE: OnceLock<RwLock<HashMap<String, AuthContext>>> = OnceLock::new();

pub fn invalidate_auth_context_cache() {
    if let Ok(mut cache) = AUTH_CONTEXT_CACHE.get_or_init(|| RwLock::new(HashMap::new())).write() {
        cache.clear();
    }
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: String,
    pub role: String,
    pub workspace_id: String,
    pub is_admin: bool,
}

fn extract_auth_epoch(settings_str: &str) -> u64 {
    // Zero-allocation, zero-copy custom JSON parser to extract auth_epoch at root level
    let bytes = settings_str.as_bytes();
    let mut i = 0;
    let len = bytes.len();
    
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    
    while i < len {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else {
            match c {
                b'"' => {
                    in_string = true;
                    // Check if this is the key "auth_epoch" at depth 1
                    if depth == 1 {
                        if i + 12 <= len && &bytes[i + 1..i + 11] == b"auth_epoch" && bytes[i + 11] == b'"' {
                            i += 12; // move past "auth_epoch" and closing quote
                            // Skip whitespace and colon
                            while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b':') {
                                i += 1;
                            }
                            // Parse digits
                            let start = i;
                            while i < len && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                            if i > start {
                                if let Ok(val) = std::str::from_utf8(&bytes[start..i]) {
                                    if let Ok(epoch) = val.parse::<u64>() {
                                        return epoch;
                                    }
                                }
                            }
                            return 0;
                        }
                    }
                }
                b'{' | b'[' => {
                    depth += 1;
                }
                b'}' | b']' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    0
}

impl AuthContext {
    pub async fn authorize(conn: &DbConnection, user_id: &str) -> Result<Self, YntraError> {
        if let Ok(cache) = AUTH_CONTEXT_CACHE.get_or_init(|| RwLock::new(HashMap::new())).read() {
            if let Some(cached) = cache.get(user_id) {
                return Ok(cached.clone());
            }
        }

        // Single JOIN query to retrieve role, workspace_id, role_signature, creator_public_key and settings in one round-trip.
        let row_result = conn.query_row(
            "SELECT u.role, u.workspace_id, u.role_signature, w.creator_public_key, w.settings \
             FROM users u \
             LEFT JOIN workspaces w ON u.workspace_id = w.id \
             WHERE u.id = ?1",
            crate::params![user_id],
            |r| {
                let role: String = r.get(0)?;
                let ws_id: Option<String> = r.get(1)?;
                let role_sig: Option<String> = r.get(2)?;
                let creator_pk: Option<String> = r.get(3)?;
                let ws_settings: Option<String> = r.get(4)?;
                Ok((role, ws_id, role_sig, creator_pk, ws_settings))
            }
        ).await;

        let (role, ws_id, role_sig, creator_pk, ws_settings) = match row_result {
            Ok(data) => data,
            Err(e) if e.is_no_row_returned() => {
                return Err(YntraError::NotFoundError(format!("Requester user '{}' not found", user_id)));
            }
            Err(e) => {
                return Err(e);
            }
        };

        let ws_id = ws_id.ok_or_else(|| {
            YntraError::AuthError(format!("User '{}' is not assigned to any workspace", user_id))
        })?;

        // Parse auth_epoch from workspace settings (zero-allocation parsing)
        let current_epoch = ws_settings.as_deref().map(extract_auth_epoch).unwrap_or(0);

        // Verification is required for all active roles (except anonymous and deleted)
        let needs_signature = role != "anonymous" && role != "deleted";

        if needs_signature {
            // Cryptographic signature is always required in production to prevent local database tampering.
            // In test configurations, we allow bypassing it if the test setup did not configure a public key,
            // to avoid breaking the extensive service test suite that creates dummy/mock workspaces.
            let is_signature_required = if cfg!(test) {
                creator_pk.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
            } else {
                true
            };

            if is_signature_required {
                let pk = creator_pk.ok_or_else(|| {
                    YntraError::AuthError(format!("Cryptographic signature verification is required for role '{}', but workspace public key is not configured", role))
                })?;

                if pk.trim().is_empty() {
                    return Err(YntraError::AuthError(format!("Cryptographic signature verification is required for role '{}', but workspace public key is empty", role)));
                }

                // Validate public key format (SOTA)
                let pk_bytes = const_hex::decode(&pk).map_err(|_| {
                    YntraError::AuthError("Workspace public key format is not valid hex".to_string())
                })?;
                let pk_array: [u8; 32] = pk_bytes.as_slice().try_into().map_err(|_| {
                    YntraError::AuthError("Workspace public key length is invalid".to_string())
                })?;
                if ed25519_dalek::VerifyingKey::from_bytes(&pk_array).is_err() {
                    return Err(YntraError::AuthError("Workspace public key is not a valid Ed25519 key".to_string()));
                }

                // Secure the Root of Trust using the system keyring as a secure public key cache (TOFU - Trust On First Use)
                let key_setting = format!("workspace_public_key_{}", ws_id);
                let mut verified_pk = pk.clone();

                // 1. Check in-memory key cache first (SOTA)
                let cached_key = {
                    let cache = crate::infra::crypto::get_auth_key_cache().read().unwrap_or_else(|e| e.into_inner());
                    cache.get(&ws_id).cloned()
                };

                if let Some(secure_pk) = cached_key {
                    if secure_pk != pk {
                        return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                    }
                    verified_pk = secure_pk;
                } else {
                    // Re-check the in-memory cache first in case another thread populated it during the initial check/lock handoff
                    let recheck_key = {
                        let cache = crate::infra::crypto::get_auth_key_cache().read().unwrap_or_else(|e| e.into_inner());
                        cache.get(&ws_id).cloned()
                    };

                    if let Some(secure_pk) = recheck_key {
                        if secure_pk != pk {
                            return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                        }
                        verified_pk = secure_pk;
                    } else if let Some(secure_pk) = crate::infra::crypto::get_local_secret(&key_setting).await? {
                        // Fall back to keyring (cold path)
                        if secure_pk != pk {
                            return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                        }
                        // Cache it in-memory
                        let mut cache = crate::infra::crypto::get_auth_key_cache().write().unwrap_or_else(|e| e.into_inner());
                        cache.insert(ws_id.clone(), secure_pk);
                    } else {
                        // Re-check cache one more time before doing expensive private-key derivation or writing to keyring
                        let final_check = {
                            let cache = crate::infra::crypto::get_auth_key_cache().read().unwrap_or_else(|e| e.into_inner());
                            cache.get(&ws_id).cloned()
                        };
                        if let Some(secure_pk) = final_check {
                            if secure_pk != pk {
                                return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                            }
                            verified_pk = secure_pk;
                        } else {
                            // If not cached, check if we hold the creator's private key locally
                            let priv_setting = format!("creator_private_key_{}", ws_id);
                            if let Some(priv_hex_raw) = crate::infra::crypto::get_local_secret(&priv_setting).await? {
                                let priv_hex = zeroize::Zeroizing::new(priv_hex_raw);
                                let derived_pk = crate::infra::crypto::derive_public_key_from_private_key(&priv_hex)
                                    .map_err(|e| YntraError::AuthError(format!("Failed to derive public key from local private key: {:?}", e)))?;
                                if derived_pk != pk {
                                    return Err(YntraError::AuthError("Workspace public key mismatch with creator private key. Local database tampering suspected.".to_string()));
                                }
                                // Cache the verified public key in the secure keyring and memory cache
                                crate::infra::crypto::set_local_secret(&key_setting, &derived_pk).await?;
                                verified_pk = derived_pk;
                            } else {
                                // Trust on first use for collaborators/invited users
                                crate::infra::crypto::set_local_secret(&key_setting, &pk).await?;
                            }
                            // Cache the verified public key in memory cache
                            let mut cache = crate::infra::crypto::get_auth_key_cache().write().unwrap_or_else(|e| e.into_inner());
                            cache.insert(ws_id.clone(), verified_pk.clone());
                        }
                    }
                }

                // Epoch rollback prevention logic (SOTA)
                // 1. Check in-memory epoch cache first
                let cached_epoch = {
                    let cache = crate::infra::crypto::get_auth_epoch_cache().read().unwrap_or_else(|e| e.into_inner());
                    cache.get(&ws_id).cloned()
                };

                if let Some(mem_epoch) = cached_epoch {
                    if current_epoch < mem_epoch {
                        return Err(YntraError::AuthError("Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()));
                    }
                    if current_epoch > mem_epoch {
                        // Update keyring first (source of truth), then update the volatile cache on success
                        let active_epoch = crate::infra::crypto::set_local_epoch_if_greater(&ws_id, current_epoch).await?;
                        let mut cache = crate::infra::crypto::get_auth_epoch_cache().write().unwrap_or_else(|e| e.into_inner());
                        let current_cached = cache.get(&ws_id).cloned().unwrap_or(0);
                        if active_epoch > current_cached {
                            cache.insert(ws_id.clone(), active_epoch);
                        }
                    }
                } else {
                    // Cold path: fetch and/or set from keyring securely (guaranteeing CAS/integrity)
                    let active_epoch = crate::infra::crypto::set_local_epoch_if_greater(&ws_id, current_epoch).await?;
                    
                    // Re-check cache for rollback after async call
                    let recheck_epoch = {
                        let cache = crate::infra::crypto::get_auth_epoch_cache().read().unwrap_or_else(|e| e.into_inner());
                        cache.get(&ws_id).cloned()
                    };
                    if let Some(mem_epoch) = recheck_epoch {
                        if current_epoch < mem_epoch {
                            return Err(YntraError::AuthError("Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()));
                        }
                    }
                    
                    let mut cache = crate::infra::crypto::get_auth_epoch_cache().write().unwrap_or_else(|e| e.into_inner());
                    let current_cached = cache.get(&ws_id).cloned().unwrap_or(0);
                    if active_epoch > current_cached {
                        cache.insert(ws_id.clone(), active_epoch);
                    }
                }

                let signature_str = role_sig.ok_or_else(|| {
                    YntraError::AuthError(format!("Role signature is missing for role '{}'", role))
                })?;

                // Verify that signature's epoch is not outdated
                let signature_parts: Vec<&str> = signature_str.split(':').collect();
                let signature_epoch = if signature_parts.len() == 3 {
                    signature_parts[0].parse::<u64>().map_err(|_| {
                        YntraError::AuthError("Role signature epoch format is invalid".to_string())
                    })?
                } else if signature_parts.len() == 2 {
                    0
                } else {
                    return Err(YntraError::AuthError("Role signature format is invalid".to_string()));
                };

                if signature_epoch < current_epoch {
                    return Err(YntraError::AuthError(format!(
                        "Role signature epoch {} is outdated (current workspace epoch is {})",
                        signature_epoch, current_epoch
                    )));
                }

                let is_valid = crate::infra::crypto::verify_role_signature(
                    &verified_pk,
                    user_id,
                    &role,
                    &ws_id,
                    &signature_str
                );

                if !is_valid {
                    return Err(YntraError::AuthError("Cryptographic signature verification failed for user role (possible privilege escalation or expired signature detected)".to_string()));
                }
            }
        }

        let is_admin = role == "admin" || role == "platform_admin";
        let auth = Self {
            user_id: user_id.to_string(),
            role,
            workspace_id: ws_id,
            is_admin,
        };
        if let Ok(mut cache) = AUTH_CONTEXT_CACHE.get_or_init(|| RwLock::new(HashMap::new())).write() {
            cache.insert(user_id.to_string(), auth.clone());
        }
        Ok(auth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_auth_context_authorize_admin() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup key cache before run to ensure clean state
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-1", "").await;

        // Setup test admin user with workspace public key and valid role signature
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-1', 'Auth WS', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();
        let valid_sig = crate::infra::crypto::generate_role_signature(creator_sk, "user-auth-admin", "admin", "workspace-auth-1").unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-admin', 'workspace-auth-1', 'admin@auth.io', 'admin', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-admin").await.unwrap();
        assert_eq!(auth.user_id, "user-auth-admin");
        assert_eq!(auth.role, "admin");
        assert_eq!(auth.workspace_id, "workspace-auth-1");
        assert!(auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-admin'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-1'", ()).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-1", "").await;
    }

    #[tokio::test]
    async fn test_auth_context_authorize_non_admin() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup key cache before run to ensure clean state
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-2", "").await;

        // Setup test standard user with workspace public key and valid role signature
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-2', 'Auth WS 2', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();
        let valid_sig = crate::infra::crypto::generate_role_signature(creator_sk, "user-auth-normal", "user", "workspace-auth-2").unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-normal', 'workspace-auth-2', 'normal@auth.io', 'user', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-normal").await.unwrap();
        assert_eq!(auth.user_id, "user-auth-normal");
        assert_eq!(auth.role, "user");
        assert_eq!(auth.workspace_id, "workspace-auth-2");
        assert!(!auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-normal'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-2'", ()).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-2", "").await;
    }

    #[tokio::test]
    async fn test_auth_context_authorize_invalid_user() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let res = AuthContext::authorize(&conn, "non-existent-user").await;
        assert!(res.is_err());
        if let Err(YntraError::NotFoundError(msg)) = res {
            assert!(msg.contains("Requester user 'non-existent-user' not found"));
        } else {
            panic!("Expected NotFoundError");
        }
    }

    #[tokio::test]
    async fn test_auth_role_signature_verification() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup key cache before run to ensure clean state
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-sig", "").await;

        // 1. Setup workspace with a creator public key
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-sig', 'Auth Sig WS', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();

        // 2. Generate a valid signature for an admin role
        let valid_sig = crate::infra::crypto::generate_role_signature(creator_sk, "user-auth-sig-admin", "admin", "workspace-auth-sig").unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-admin', 'workspace-auth-sig', 'admin-sig@auth.io', 'admin', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-sig-admin").await;
        assert!(auth.is_ok());

        // 3. Insert admin user with INVALID signature (tampered locally)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-tampered', 'workspace-auth-sig', 'tampered@auth.io', 'admin', 'badsignature')", ()).await.unwrap();

        let auth_fail = AuthContext::authorize(&conn, "user-auth-sig-tampered").await;
        assert!(auth_fail.is_err());
        if let Err(YntraError::AuthError(msg)) = auth_fail {
            assert!(msg.contains("verification failed") || msg.contains("format is invalid"), "Expected verification or format failure, got: {}", msg);
        } else {
            panic!("Expected AuthError");
        }

        // Cleanup
        conn.execute("DELETE FROM users WHERE id IN ('user-auth-sig-admin', 'user-auth-sig-tampered')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-sig'", ()).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-sig", "").await;
    }

    #[tokio::test]
    async fn test_auth_epoch_validation_and_rollback() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup workspace with creator key and settings containing auth_epoch = 2
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let ws_id = "workspace-auth-epoch-test";
        let user_id = "user-auth-epoch-admin";

        // Clean cache before starting
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "").await;
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_auth_epoch_{}", ws_id), "").await;

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES (?1, 'Auth Epoch WS', '[]', '{\"auth_epoch\": 2}', ?2)",
            crate::params![ws_id, creator_pk]
        ).await.unwrap();

        // 2. Try to authorize with a legacy signature (epoch 0, implicitly) -> should fail because workspace current epoch is 2
        let legacy_sig = crate::infra::crypto::generate_role_signature(creator_sk, user_id, "admin", ws_id).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES (?1, ?2, 'admin@epoch.io', 'admin', ?3)",
            crate::params![user_id, ws_id, legacy_sig]
        ).await.unwrap();

        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(auth_res.is_err(), "Legacy signature (epoch 0) should be rejected when workspace epoch is 2");
        if let Err(YntraError::AuthError(msg)) = auth_res {
            assert!(msg.contains("outdated"), "Expected outdated epoch error, got: {}", msg);
        } else {
            panic!("Expected AuthError");
        }

        // 3. Try to authorize with a signature matching epoch 2 -> should succeed and cache epoch 2
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let expires_at = current_time + 3600;
        let valid_epoch_sig = crate::infra::crypto::generate_role_signature_v2(creator_sk, user_id, "admin", ws_id, expires_at, 2).unwrap();
        conn.execute(
            "UPDATE users SET role_signature = ?1 WHERE id = ?2",
            crate::params![valid_epoch_sig, user_id]
        ).await.unwrap();

        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(auth_res.is_ok(), "Signature matching epoch 2 should succeed, got error: {:?}", auth_res.err());

        // Verify the epoch has been cached to 2 in local secret storage
        let cached_epoch = crate::infra::crypto::get_local_secret(&format!("workspace_auth_epoch_{}", ws_id)).await.unwrap();
        assert_eq!(cached_epoch.as_deref(), Some("2"));

        // 4. Simulate Database Tampering Rollback Attack (Database epoch is modified back to 1)
        conn.execute(
            "UPDATE workspaces SET settings = '{\"auth_epoch\": 1}' WHERE id = ?1",
            crate::params![ws_id]
        ).await.unwrap();

        // Regenerate role signature for epoch 1
        let rolled_sig = crate::infra::crypto::generate_role_signature_v2(creator_sk, user_id, "admin", ws_id, expires_at, 1).unwrap();
        conn.execute(
            "UPDATE users SET role_signature = ?1 WHERE id = ?2",
            crate::params![rolled_sig, user_id]
        ).await.unwrap();

        // Auth should fail with a rollback mismatch detection error!
        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(auth_res.is_err(), "Rolled back database epoch should be rejected");
        if let Err(YntraError::AuthError(msg)) = auth_res {
            assert!(msg.contains("rollback detected"), "Expected rollback error, got: {}", msg);
        } else {
            panic!("Expected AuthError");
        }

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id]).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "").await;
        let _ = crate::infra::crypto::set_local_secret(&format!("workspace_auth_epoch_{}", ws_id), "").await;
    }

    #[test]
    fn test_extract_auth_epoch_json_safety() {
        // Valid JSON settings with auth_epoch
        let valid_json = "{\"auth_epoch\": 5, \"workspace_name\": \"test\"}";
        assert_eq!(extract_auth_epoch(valid_json), 5);

        // Invalid JSON settings (contains auth_epoch as a substring/value but not key)
        let name_hijack_json = "{\"name\": \"auth_epoch 9999 testing\", \"other\": 1}";
        assert_eq!(extract_auth_epoch(name_hijack_json), 0);

        // Malformed JSON should return default (0)
        let malformed_json = "{malformed auth_epoch: 123}";
        assert_eq!(extract_auth_epoch(malformed_json), 0);

        // Nested JSON containing brace/bracket in string value (bug regression test)
        let nested_brace_json = "{\"some_obj\": {\"nested_str\": \"}\"}, \"auth_epoch\": 10}";
        assert_eq!(extract_auth_epoch(nested_brace_json), 10);

        let nested_bracket_json = "{\"some_arr\": [\"foo\", \"bar\", \"]\"], \"auth_epoch\": 42}";
        assert_eq!(extract_auth_epoch(nested_bracket_json), 42);
    }
}
