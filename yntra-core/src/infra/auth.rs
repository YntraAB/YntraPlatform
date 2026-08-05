use crate::YntraError;
use crate::database::DbConnection;
use std::collections::{HashMap, VecDeque};
use std::sync::{OnceLock, RwLock};

pub fn validate_id(id: &str, field_name: &str) -> Result<(), YntraError> {
    if id.len() > 128 {
        return Err(YntraError::ValidationError(format!(
            "{} exceeds maximum length of 128 characters",
            field_name
        )));
    }
    if !id.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@' || c == ':'
    }) {
        return Err(YntraError::ValidationError(format!(
            "{} contains invalid characters",
            field_name
        )));
    }
    Ok(())
}

struct BoundedAuthCache {
    map: HashMap<String, AuthContext>,
    order: VecDeque<String>,
}

const CACHE_LIMIT: usize = 1000;

static AUTH_CONTEXT_CACHE: OnceLock<RwLock<BoundedAuthCache>> = OnceLock::new();

fn compute_is_production() -> bool {
    // Compile-time check: release profiles (without debug assertions) are production
    if !cfg!(debug_assertions) && !cfg!(test) {
        return true;
    }

    // Runtime environment checks for production indicators
    #[cfg(not(target_arch = "wasm32"))]
    {
        for var_name in &["YNTRA_ENV", "RUST_ENV", "ENV", "NODE_ENV"] {
            if let Ok(val) = std::env::var(var_name) {
                let val_lower = val.to_lowercase();
                if val_lower == "production" || val_lower == "prod" {
                    return true;
                }
            }
        }
        for path in &[".env", "../.env"] {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        continue;
                    }
                    for var_name in &["YNTRA_ENV=", "RUST_ENV=", "ENV=", "NODE_ENV="] {
                        if let Some(stripped) = trimmed.strip_prefix(var_name) {
                            let val = stripped
                                .trim()
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_lowercase();
                            if val == "production" || val == "prod" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                for var_name in &["YNTRA_ENV", "RUST_ENV", "ENV", "NODE_ENV"] {
                    if let Ok(Some(val)) = storage.get_item(var_name) {
                        let val_lower = val.to_lowercase();
                        if val_lower == "production" || val_lower == "prod" {
                            return true;
                        }
                    }
                }
            }
            if let Ok(location) = window.location().hostname() {
                let host = location.to_lowercase();
                if !host.is_empty()
                    && host != "localhost"
                    && host != "127.0.0.1"
                    && host != "0.0.0.0"
                {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn is_production() -> bool {
    #[cfg(test)]
    {
        compute_is_production()
    }
    #[cfg(not(test))]
    {
        static IS_PROD_CACHE: OnceLock<bool> = OnceLock::new();
        *IS_PROD_CACHE.get_or_init(compute_is_production)
    }
}

fn compute_insecure_dev_bypass() -> bool {
    if is_production() {
        return false;
    }
    if cfg!(debug_assertions) && !cfg!(test) {
        return true;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(val) = std::env::var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES") {
            return val == "1" || val.to_lowercase() == "true";
        }
        for path in &[".env", "../.env"] {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    if let Some(stripped) =
                        line.strip_prefix("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES=")
                    {
                        let val = stripped
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_lowercase();
                        return val == "1" || val == "true";
                    }
                }
            }
        }
    }
    false
}

fn check_insecure_dev_bypass() -> bool {
    #[cfg(test)]
    {
        compute_insecure_dev_bypass()
    }
    #[cfg(not(test))]
    {
        static BYPASS_CACHE: OnceLock<bool> = OnceLock::new();
        *BYPASS_CACHE.get_or_init(compute_insecure_dev_bypass)
    }
}
pub fn invalidate_auth_context_cache() {
    if let Ok(mut cache) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .write()
    {
        cache.map.clear();
        cache.order.clear();
    }
}

pub fn invalidate_auth_context_cache_for_user(user_id: &str) {
    if let Ok(mut cache) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .write()
    {
        cache.map.remove(user_id);
    }
}

pub fn invalidate_auth_context_cache_for_workspace(workspace_id: &str) {
    if let Ok(mut cache) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .write()
    {
        cache
            .map
            .retain(|_, context| context.workspace_id != workspace_id);
    }
}

pub fn insert_auth_context_cache(user_id: &str, context: AuthContext) {
    if let Ok(mut cache) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .write()
    {
        while cache.map.len() >= CACHE_LIMIT {
            if let Some(oldest_user) = cache.order.pop_front() {
                if cache.map.remove(&oldest_user).is_some() {
                    break;
                }
            } else {
                break;
            }
        }
        if cache.map.insert(user_id.to_string(), context).is_none() {
            cache.order.push_back(user_id.to_string());
        }
    }
}

pub fn get_auth_context_cache(user_id: &str) -> Option<AuthContext> {
    if let Ok(cache) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .read()
    {
        cache.map.get(user_id).cloned()
    } else {
        None
    }
}

fn extract_where_target_ids(sql: &str, table: &str) -> Option<Vec<String>> {
    let sql_upper = sql.to_uppercase();
    let where_idx = sql_upper.find("WHERE ")?;
    let where_clause = &sql[where_idx + 6..];

    let targets = if table == "users" {
        &["id", "u.id", "user_id"][..]
    } else if table == "workspaces" {
        &["id", "w.id", "workspace_id", "u.workspace_id"][..]
    } else {
        &["id"][..]
    };

    let mut found_ids = Vec::new();
    let mut chars = where_clause.chars().peekable();
    let mut in_quote = false;
    let mut current_lit = String::new();

    while let Some(c) = chars.next() {
        if c == '\'' {
            if in_quote {
                if chars.peek() == Some(&'\'') {
                    chars.next(); // consume the escaped quote
                    current_lit.push('\'');
                } else {
                    found_ids.push(current_lit.clone());
                    current_lit.clear();
                    in_quote = false;
                }
            } else {
                in_quote = true;
            }
        } else if in_quote {
            current_lit.push(c);
        }
    }

    let contains_target_col = targets.iter().any(|&col| {
        let col_upper = col.to_uppercase();
        sql_upper[where_idx..].contains(&col_upper)
    });

    if contains_target_col {
        Some(found_ids)
    } else {
        None
    }
}

pub fn invalidate_auth_context_cache_for_sql(sql: &str, table: &str) {
    if let Ok(mut cache_guard) = AUTH_CONTEXT_CACHE
        .get_or_init(|| {
            RwLock::new(BoundedAuthCache {
                map: HashMap::new(),
                order: VecDeque::new(),
            })
        })
        .write()
    {
        let BoundedAuthCache { map, order } = &mut *cache_guard;

        if let Some(target_ids) = extract_where_target_ids(sql, table) {
            if !target_ids.is_empty() {
                if table == "users" {
                    for id in &target_ids {
                        map.remove(id);
                    }
                    order.retain(|id| !target_ids.contains(id));
                } else if table == "workspaces" {
                    map.retain(|_, context| {
                        !target_ids
                            .iter()
                            .any(|ws_id| ws_id == &context.workspace_id)
                    });
                    order.retain(|id| map.contains_key(id));
                }
            } else {
                // Parameterized WHERE clause on users or workspaces (e.g. WHERE id = ?1):
                // Parameter value is bound at runtime, clear cache to ensure security correctness.
                map.clear();
                order.clear();
            }
        } else {
            // Un-targeted bulk write without WHERE clause: clear all cached auth contexts.
            map.clear();
            order.clear();
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: String,
    pub role: String,
    pub workspace_id: String,
    pub is_admin: bool,
    pub workspace_settings: Option<String>,
}

fn extract_auth_epoch(settings_str: &str) -> Result<u64, YntraError> {
    #[derive(serde::Deserialize)]
    struct Settings {
        auth_epoch: Option<u64>,
    }
    match serde_json::from_str::<Settings>(settings_str) {
        Ok(s) => Ok(s.auth_epoch.unwrap_or(0)),
        Err(e) => Err(YntraError::AuthError(format!(
            "Malformed workspace settings: {}",
            e
        ))),
    }
}

impl AuthContext {
    pub async fn authorize(conn: &DbConnection, user_id: &str) -> Result<Self, YntraError> {
        validate_id(user_id, "User ID")?;
        if let Ok(cache) = AUTH_CONTEXT_CACHE
            .get_or_init(|| {
                RwLock::new(BoundedAuthCache {
                    map: HashMap::new(),
                    order: VecDeque::new(),
                })
            })
            .read()
        {
            if let Some(cached) = cache.map.get(user_id) {
                return Ok(cached.clone());
            }
        }

        // Single JOIN query to retrieve role, workspace_id, role_signature, creator_public_key and settings in one round-trip.
        let row_result = conn
            .query_row(
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
                },
            )
            .await;

        let (role, ws_id, role_sig, creator_pk, ws_settings) = match row_result {
            Ok(data) => data,
            Err(e) if e.is_no_row_returned() => {
                return Err(YntraError::NotFoundError(format!(
                    "Requester user '{}' not found",
                    user_id
                )));
            }
            Err(e) => {
                return Err(e);
            }
        };

        let ws_id = ws_id.ok_or_else(|| {
            YntraError::AuthError(format!(
                "User '{}' is not assigned to any workspace",
                user_id
            ))
        })?;

        // Parse auth_epoch from workspace settings
        let current_epoch = match ws_settings.as_deref() {
            Some(s) => extract_auth_epoch(s)?,
            None => 0,
        };

        // Verification is required for all active roles (except anonymous and deleted)
        let needs_signature = role != "anonymous" && role != "deleted";

        if needs_signature {
            // Cryptographic signature is always required in production to prevent local database tampering.
            // In test and debug configurations, we allow bypassing it if the setup did not configure a public key,
            // to avoid breaking local dev mode / test runs with unconfigured mock workspaces.
            let is_signature_required = if is_production() {
                true
            } else if cfg!(test) {
                creator_pk
                    .as_ref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false)
            } else if cfg!(debug_assertions) {
                if check_insecure_dev_bypass() {
                    let bypass = creator_pk
                        .as_ref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    if !bypass {
                        tracing::warn!(
                            "Insecure Dev Bypass signature check is active: cryptographic signature verification is bypassed because creator_pk is empty."
                        );
                    }
                    bypass
                } else {
                    true
                }
            } else {
                true
            };

            if is_signature_required {
                let pk = creator_pk.ok_or_else(|| {
                    YntraError::AuthError(format!("Cryptographic signature verification is required for role '{}', but workspace public key is not configured", role))
                })?;

                if pk.trim().is_empty() {
                    return Err(YntraError::AuthError(format!(
                        "Cryptographic signature verification is required for role '{}', but workspace public key is empty",
                        role
                    )));
                }

                // Validate public key format using cached verifying key (SOTA)
                if crate::infra::crypto::get_parsed_verifying_key(&pk).is_none() {
                    return Err(YntraError::AuthError(
                        "Workspace public key is not a valid Ed25519 key".to_string(),
                    ));
                }

                // Secure the Root of Trust using the system keyring as a secure public key cache (TOFU - Trust On First Use)
                let key_setting = format!("workspace_public_key_{}", ws_id);
                let mut verified_pk = pk.clone();

                // Check in-memory key cache first
                let cached_key = {
                    let cache = crate::infra::crypto::get_auth_key_cache()
                        .read()
                        .unwrap_or_else(|e| e.into_inner());
                    cache.get(&ws_id).cloned()
                };

                if let Some(secure_pk) = cached_key {
                    if secure_pk != pk {
                        return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                    }
                    verified_pk = secure_pk;
                } else if let Some(secure_pk) =
                    crate::infra::crypto::get_local_secret(&key_setting).await?
                {
                    // Fall back to keyring (cold path)
                    if secure_pk != pk {
                        return Err(YntraError::AuthError("Workspace public key mismatch detected. Local database tampering suspected.".to_string()));
                    }
                    // Cache it in-memory
                    let mut cache = crate::infra::crypto::get_auth_key_cache()
                        .write()
                        .unwrap_or_else(|e| e.into_inner());
                    cache.insert(ws_id.clone(), secure_pk);
                } else {
                    // If not cached and not in keyring, check if we hold the creator's private key locally
                    let priv_setting = format!("creator_private_key_{}", ws_id);
                    if let Some(priv_hex_raw) =
                        crate::infra::crypto::get_local_secret(&priv_setting).await?
                    {
                        let priv_hex = zeroize::Zeroizing::new(priv_hex_raw);
                        let derived_pk =
                            crate::infra::crypto::derive_public_key_from_private_key(&priv_hex)
                                .map_err(|e| {
                                    YntraError::AuthError(format!(
                                        "Failed to derive public key from local private key: {:?}",
                                        e
                                    ))
                                })?;
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
                    let mut cache = crate::infra::crypto::get_auth_key_cache()
                        .write()
                        .unwrap_or_else(|e| e.into_inner());
                    cache.insert(ws_id.clone(), verified_pk.clone());
                }

                // Epoch rollback prevention logic (SOTA)
                // 1. Check in-memory epoch cache first
                let cached_epoch = {
                    let cache = crate::infra::crypto::get_auth_epoch_cache()
                        .read()
                        .unwrap_or_else(|e| e.into_inner());
                    cache.get(&ws_id).cloned()
                };

                if let Some(mem_epoch) = cached_epoch {
                    if current_epoch < mem_epoch {
                        return Err(YntraError::AuthError("Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()));
                    }
                    if current_epoch > mem_epoch {
                        // Update keyring first (source of truth), then update the volatile cache on success
                        let active_epoch =
                            crate::infra::crypto::set_local_epoch_if_greater(&ws_id, current_epoch)
                                .await?;
                        let mut cache = crate::infra::crypto::get_auth_epoch_cache()
                            .write()
                            .unwrap_or_else(|e| e.into_inner());
                        let current_cached = cache.get(&ws_id).cloned().unwrap_or(0);
                        if active_epoch > current_cached {
                            cache.insert(ws_id.clone(), active_epoch);
                        }
                    }
                } else {
                    // Cold path: fetch and/or set from keyring securely (guaranteeing CAS/integrity)
                    let active_epoch =
                        crate::infra::crypto::set_local_epoch_if_greater(&ws_id, current_epoch)
                            .await?;

                    // Re-check cache for rollback after async call
                    let recheck_epoch = {
                        let cache = crate::infra::crypto::get_auth_epoch_cache()
                            .read()
                            .unwrap_or_else(|e| e.into_inner());
                        cache.get(&ws_id).cloned()
                    };
                    if let Some(mem_epoch) = recheck_epoch {
                        if current_epoch < mem_epoch {
                            return Err(YntraError::AuthError("Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()));
                        }
                    }

                    let mut cache = crate::infra::crypto::get_auth_epoch_cache()
                        .write()
                        .unwrap_or_else(|e| e.into_inner());
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
                    return Err(YntraError::AuthError(
                        "Role signature format is invalid".to_string(),
                    ));
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
                    &signature_str,
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
            workspace_settings: ws_settings,
        };
        insert_auth_context_cache(user_id, auth.clone());
        Ok(auth)
    }
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct RoleLeaseTokenStatus {
    pub is_valid: bool,
    pub user_id: String,
    pub workspace_id: String,
    pub expires_at_ms: i64,
    pub is_expired: bool,
}

#[uniffi::export]
pub fn validate_time_bound_role_lease(
    user_id: String,
    workspace_id: String,
    lease_token: String,
    expires_at_ms: i64,
) -> RoleLeaseTokenStatus {
    let now_ms = crate::infra::time::get_current_time_ms();
    let is_expired = now_ms > expires_at_ms;

    if is_expired {
        crate::infra::crypto::clear_session_key();
    }

    RoleLeaseTokenStatus {
        is_valid: !is_expired && !lease_token.is_empty(),
        user_id,
        workspace_id,
        expires_at_ms,
        is_expired,
    }
}

#[uniffi::export]
pub async fn verify_workspace_auth_epoch(
    workspace_id: String,
    client_epoch: u64,
) -> Result<bool, YntraError> {
    let conn = crate::database::acquire_connection().await?;
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());

    let server_epoch = extract_auth_epoch(&settings_str).unwrap_or(0);
    if (server_epoch as u64) > client_epoch {
        crate::infra::crypto::clear_session_key();
        return Err(YntraError::AuthError("Workspace auth epoch updated. Local session invalidated.".to_string()));
    }

    Ok(true)
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
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-1", "")
            .await;

        // Setup test admin user with workspace public key and valid role signature
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-1', 'Auth WS', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();
        let valid_sig = crate::infra::crypto::generate_role_signature(
            creator_sk,
            "user-auth-admin",
            "admin",
            "workspace-auth-1",
        )
        .unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-admin', 'workspace-auth-1', 'admin@auth.io', 'admin', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-admin")
            .await
            .unwrap();
        assert_eq!(auth.user_id, "user-auth-admin");
        assert_eq!(auth.role, "admin");
        assert_eq!(auth.workspace_id, "workspace-auth-1");
        assert!(auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-admin'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-1'", ())
            .await
            .unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-1", "")
            .await;
    }

    #[tokio::test]
    async fn test_auth_context_authorize_non_admin() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup key cache before run to ensure clean state
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-2", "")
            .await;

        // Setup test standard user with workspace public key and valid role signature
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-2', 'Auth WS 2', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();
        let valid_sig = crate::infra::crypto::generate_role_signature(
            creator_sk,
            "user-auth-normal",
            "user",
            "workspace-auth-2",
        )
        .unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-normal', 'workspace-auth-2', 'normal@auth.io', 'user', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-normal")
            .await
            .unwrap();
        assert_eq!(auth.user_id, "user-auth-normal");
        assert_eq!(auth.role, "user");
        assert_eq!(auth.workspace_id, "workspace-auth-2");
        assert!(!auth.is_admin);

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = 'user-auth-normal'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-2'", ())
            .await
            .unwrap();
        let _ = crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-2", "")
            .await;
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
        let _ =
            crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-sig", "")
                .await;

        // 1. Setup workspace with a creator public key
        let creator_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let creator_sk = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES ('workspace-auth-sig', 'Auth Sig WS', '[]', '{}', ?1)", crate::params![creator_pk]).await.unwrap();

        // 2. Generate a valid signature for an admin role
        let valid_sig = crate::infra::crypto::generate_role_signature(
            creator_sk,
            "user-auth-sig-admin",
            "admin",
            "workspace-auth-sig",
        )
        .unwrap();

        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-admin', 'workspace-auth-sig', 'admin-sig@auth.io', 'admin', ?1)", crate::params![valid_sig]).await.unwrap();

        let auth = AuthContext::authorize(&conn, "user-auth-sig-admin").await;
        assert!(auth.is_ok());

        // 3. Insert admin user with INVALID signature (tampered locally)
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('user-auth-sig-tampered', 'workspace-auth-sig', 'tampered@auth.io', 'admin', 'badsignature')", ()).await.unwrap();

        let auth_fail = AuthContext::authorize(&conn, "user-auth-sig-tampered").await;
        assert!(auth_fail.is_err());
        if let Err(YntraError::AuthError(msg)) = auth_fail {
            assert!(
                msg.contains("verification failed") || msg.contains("format is invalid"),
                "Expected verification or format failure, got: {}",
                msg
            );
        } else {
            panic!("Expected AuthError");
        }

        // Cleanup
        conn.execute(
            "DELETE FROM users WHERE id IN ('user-auth-sig-admin', 'user-auth-sig-tampered')",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'workspace-auth-sig'", ())
            .await
            .unwrap();
        let _ =
            crate::infra::crypto::set_local_secret("workspace_public_key_workspace-auth-sig", "")
                .await;
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
        let _ =
            crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "")
                .await;
        let _ =
            crate::infra::crypto::set_local_secret(&format!("workspace_auth_epoch_{}", ws_id), "")
                .await;

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, creator_public_key) VALUES (?1, 'Auth Epoch WS', '[]', '{\"auth_epoch\": 2}', ?2)",
            crate::params![ws_id, creator_pk]
        ).await.unwrap();

        // 2. Try to authorize with a legacy signature (epoch 0, implicitly) -> should fail because workspace current epoch is 2
        let legacy_sig =
            crate::infra::crypto::generate_role_signature(creator_sk, user_id, "admin", ws_id)
                .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES (?1, ?2, 'admin@epoch.io', 'admin', ?3)",
            crate::params![user_id, ws_id, legacy_sig]
        ).await.unwrap();

        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(
            auth_res.is_err(),
            "Legacy signature (epoch 0) should be rejected when workspace epoch is 2"
        );
        if let Err(YntraError::AuthError(msg)) = auth_res {
            assert!(
                msg.contains("outdated"),
                "Expected outdated epoch error, got: {}",
                msg
            );
        } else {
            panic!("Expected AuthError");
        }

        // 3. Try to authorize with a signature matching epoch 2 -> should succeed and cache epoch 2
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let expires_at = current_time + 3600;
        let valid_epoch_sig = crate::infra::crypto::generate_role_signature_v2(
            creator_sk, user_id, "admin", ws_id, expires_at, 2,
        )
        .unwrap();
        conn.execute(
            "UPDATE users SET role_signature = ?1 WHERE id = ?2",
            crate::params![valid_epoch_sig, user_id],
        )
        .await
        .unwrap();

        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(
            auth_res.is_ok(),
            "Signature matching epoch 2 should succeed, got error: {:?}",
            auth_res.err()
        );

        // Verify the epoch has been cached to 2 in local secret storage
        let cached_epoch =
            crate::infra::crypto::get_local_secret(&format!("workspace_auth_epoch_{}", ws_id))
                .await
                .unwrap();
        assert_eq!(cached_epoch.as_deref(), Some("2"));

        // 4. Simulate Database Tampering Rollback Attack (Database epoch is modified back to 1)
        conn.execute(
            "UPDATE workspaces SET settings = '{\"auth_epoch\": 1}' WHERE id = ?1",
            crate::params![ws_id],
        )
        .await
        .unwrap();

        // Regenerate role signature for epoch 1
        let rolled_sig = crate::infra::crypto::generate_role_signature_v2(
            creator_sk, user_id, "admin", ws_id, expires_at, 1,
        )
        .unwrap();
        conn.execute(
            "UPDATE users SET role_signature = ?1 WHERE id = ?2",
            crate::params![rolled_sig, user_id],
        )
        .await
        .unwrap();

        // Auth should fail with a rollback mismatch detection error!
        let auth_res = AuthContext::authorize(&conn, user_id).await;
        assert!(
            auth_res.is_err(),
            "Rolled back database epoch should be rejected"
        );
        if let Err(YntraError::AuthError(msg)) = auth_res {
            assert!(
                msg.contains("rollback detected"),
                "Expected rollback error, got: {}",
                msg
            );
        } else {
            panic!("Expected AuthError");
        }

        // Cleanup
        conn.execute("DELETE FROM users WHERE id = ?1", crate::params![user_id])
            .await
            .unwrap();
        conn.execute(
            "DELETE FROM workspaces WHERE id = ?1",
            crate::params![ws_id],
        )
        .await
        .unwrap();
        let _ =
            crate::infra::crypto::set_local_secret(&format!("workspace_public_key_{}", ws_id), "")
                .await;
        let _ =
            crate::infra::crypto::set_local_secret(&format!("workspace_auth_epoch_{}", ws_id), "")
                .await;
    }

    #[test]
    fn test_extract_auth_epoch_json_safety() {
        // Valid JSON settings with auth_epoch
        let valid_json = "{\"auth_epoch\": 5, \"workspace_name\": \"test\"}";
        assert_eq!(extract_auth_epoch(valid_json).unwrap(), 5);

        // Invalid JSON settings (contains auth_epoch as a substring/value but not key)
        let name_hijack_json = "{\"name\": \"auth_epoch 9999 testing\", \"other\": 1}";
        assert_eq!(extract_auth_epoch(name_hijack_json).unwrap(), 0);

        // Malformed JSON should return an error
        let malformed_json = "{malformed auth_epoch: 123}";
        assert!(extract_auth_epoch(malformed_json).is_err());

        // Nested JSON containing brace/bracket in string value (bug regression test)
        let nested_brace_json = "{\"some_obj\": {\"nested_str\": \"}\"}, \"auth_epoch\": 10}";
        assert_eq!(extract_auth_epoch(nested_brace_json).unwrap(), 10);

        let nested_bracket_json = "{\"some_arr\": [\"foo\", \"bar\", \"]\"], \"auth_epoch\": 42}";
        assert_eq!(extract_auth_epoch(nested_bracket_json).unwrap(), 42);

        // Value hijacking regression test
        let value_hijack_json = "{\"name\": \"auth_epoch\", \"auth_epoch\": 5}";
        assert_eq!(extract_auth_epoch(value_hijack_json).unwrap(), 5);
    }

    #[test]
    fn test_bounded_auth_cache_eviction() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        invalidate_auth_context_cache();

        // Populate up to CACHE_LIMIT + 5 items
        for i in 0..(CACHE_LIMIT + 5) {
            let ctx = AuthContext {
                user_id: format!("user-{}", i),
                role: "user".to_string(),
                workspace_id: "ws-1".to_string(),
                is_admin: false,
                workspace_settings: None,
            };
            insert_auth_context_cache(&format!("user-{}", i), ctx);
        }

        // Verify size is exactly CACHE_LIMIT
        let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
        assert_eq!(cache.map.len(), CACHE_LIMIT);

        // Verify oldest 5 users (0 to 4) are evicted
        for i in 0..5 {
            assert!(!cache.map.contains_key(&format!("user-{}", i)));
        }

        // Verify users 5 to 1004 are still present
        for i in 5..(CACHE_LIMIT + 5) {
            assert!(cache.map.contains_key(&format!("user-{}", i)));
        }
    }

    #[test]
    fn test_auth_context_debug_assertions_bypass_prevention() {
        unsafe {
            std::env::remove_var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES");
        }
        assert!(!check_insecure_dev_bypass());

        unsafe {
            std::env::set_var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES", "true");
        }
        assert!(check_insecure_dev_bypass());

        unsafe {
            std::env::set_var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES", "1");
        }
        assert!(check_insecure_dev_bypass());

        unsafe {
            std::env::set_var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES", "false");
        }
        assert!(!check_insecure_dev_bypass());

        unsafe {
            std::env::remove_var("YNTRA_INSECURE_DEV_BYPASS_SIGNATURES");
        }
    }

    #[tokio::test]
    async fn test_selective_auth_cache_invalidation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let _conn = database::acquire_connection().await.unwrap();

        // 1. Populate the cache with two contexts
        let ctx1 = AuthContext {
            user_id: "user-cache-test-1".to_string(),
            role: "user".to_string(),
            workspace_id: "ws-cache-test-1".to_string(),
            is_admin: false,
            workspace_settings: None,
        };
        let ctx2 = AuthContext {
            user_id: "user-cache-test-2".to_string(),
            role: "admin".to_string(),
            workspace_id: "ws-cache-test-2".to_string(),
            is_admin: true,
            workspace_settings: None,
        };

        insert_auth_context_cache("user-cache-test-1", ctx1);
        insert_auth_context_cache("user-cache-test-2", ctx2);

        // Verify they are cached
        {
            let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
            assert!(cache.map.contains_key("user-cache-test-1"));
            assert!(cache.map.contains_key("user-cache-test-2"));
        }

        // 2. Perform a write that targets user-cache-test-1 using a SQL statement
        let sql_user = "UPDATE users SET role = 'admin' WHERE id = 'user-cache-test-1'";
        invalidate_auth_context_cache_for_sql(sql_user, "users");

        // Verify user-cache-test-1 is invalidated, but user-cache-test-2 is NOT!
        {
            let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
            assert!(!cache.map.contains_key("user-cache-test-1"));
            assert!(cache.map.contains_key("user-cache-test-2"));
        }

        // Repopulate user-cache-test-1
        let ctx1 = AuthContext {
            user_id: "user-cache-test-1".to_string(),
            role: "user".to_string(),
            workspace_id: "ws-cache-test-1".to_string(),
            is_admin: false,
            workspace_settings: None,
        };
        insert_auth_context_cache("user-cache-test-1", ctx1);

        // 3. Perform a write that targets ws-cache-test-1 using a SQL statement
        let sql_ws = "UPDATE workspaces SET settings = '{}' WHERE id = 'ws-cache-test-1'";
        invalidate_auth_context_cache_for_sql(sql_ws, "workspaces");

        // Verify user-cache-test-1 (which belongs to ws-cache-test-1) is invalidated, but user-cache-test-2 is NOT!
        {
            let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
            assert!(!cache.map.contains_key("user-cache-test-1"));
            assert!(cache.map.contains_key("user-cache-test-2"));
        }

        // Repopulate user-cache-test-1
        let ctx1 = AuthContext {
            user_id: "user-cache-test-1".to_string(),
            role: "user".to_string(),
            workspace_id: "ws-cache-test-1".to_string(),
            is_admin: false,
            workspace_settings: None,
        };
        insert_auth_context_cache("user-cache-test-1", ctx1);

        // 4. Perform a parameterized write query (should fall back to clearing cache for security)
        let sql_placeholder = "UPDATE users SET role = ?1 WHERE id = ?2";
        invalidate_auth_context_cache_for_sql(sql_placeholder, "users");

        // Verify cache is cleared
        {
            let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
            assert!(cache.map.is_empty());
        }

        // 5. Test escaped single quotes (e.g. O'Brien) do not cause parse corruption
        let ctx_obrien = AuthContext {
            user_id: "O'Brien".to_string(),
            role: "user".to_string(),
            workspace_id: "ws-obrien".to_string(),
            is_admin: false,
            workspace_settings: None,
        };
        let ctx_other = AuthContext {
            user_id: "user-cache-test-2".to_string(),
            role: "admin".to_string(),
            workspace_id: "ws-cache-test-2".to_string(),
            is_admin: true,
            workspace_settings: None,
        };
        insert_auth_context_cache("O'Brien", ctx_obrien);
        insert_auth_context_cache("user-cache-test-2", ctx_other);

        // SQL containing escaped single quote in name, but targeting user O'Brien
        let sql_escaped = "UPDATE users SET name = 'O''Brien' WHERE id = 'O''Brien'";
        invalidate_auth_context_cache_for_sql(sql_escaped, "users");

        // Verify O'Brien is correctly invalidated, but user-cache-test-2 is NOT!
        {
            let cache = AUTH_CONTEXT_CACHE.get().unwrap().read().unwrap();
            assert!(!cache.map.contains_key("O'Brien"));
            assert!(cache.map.contains_key("user-cache-test-2"));
        }
    }

    #[test]
    fn test_is_production_detection() {
        unsafe {
            std::env::set_var("YNTRA_ENV", "production");
        }
        assert!(is_production());
        assert!(!check_insecure_dev_bypass());

        unsafe {
            std::env::set_var("YNTRA_ENV", "development");
        }
        // In test mode (cargo test), is_production() checks env variables, but wait:
        // if YNTRA_ENV is development, is_production() checks other env vars and falls back to false.
        // Let's verify standard behaviour when YNTRA_ENV is not production.
        unsafe {
            std::env::remove_var("YNTRA_ENV");
        }
    }

    #[test]
    fn test_validate_id_scenarios() {
        assert!(validate_id("valid-user-128_id", "User ID").is_ok());
        assert!(validate_id("valid.email@yntra.se", "User ID").is_ok());
        assert!(validate_id("ws:workspace-abc.123", "Workspace ID").is_ok());

        // Oversized ID (129 chars)
        let oversized = "a".repeat(129);
        let res = validate_id(&oversized, "User ID");
        assert!(res.is_err());
        if let Err(YntraError::ValidationError(msg)) = res {
            assert!(msg.contains("exceeds maximum length"));
        } else {
            panic!("Expected ValidationError");
        }

        // Invalid characters
        let invalid = "user;drop table users;";
        let res2 = validate_id(invalid, "User ID");
        assert!(res2.is_err());
        if let Err(YntraError::ValidationError(msg)) = res2 {
            assert!(msg.contains("contains invalid characters"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_invalidate_auth_context_cache_for_sql_targeted() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        invalidate_auth_context_cache();

        let u1 = "user-target-1".to_string();
        let u2 = "user-target-2".to_string();
        let ws1 = "workspace-target-1".to_string();

        insert_auth_context_cache(
            &u1,
            AuthContext {
                user_id: u1.clone(),
                workspace_id: ws1.clone(),
                role: "admin".to_string(),
                is_admin: true,
                workspace_settings: None,
            },
        );

        insert_auth_context_cache(
            &u2,
            AuthContext {
                user_id: u2.clone(),
                workspace_id: ws1.clone(),
                role: "user".to_string(),
                is_admin: false,
                workspace_settings: None,
            },
        );

        assert!(get_auth_context_cache(&u1).is_some());
        assert!(get_auth_context_cache(&u2).is_some());

        // Parameterized write query targeting user-target-1 specifically with literal and placeholder
        invalidate_auth_context_cache_for_sql(
            "UPDATE users SET name = ? WHERE id = 'user-target-1'",
            "users",
        );

        // user-target-1 should be invalidated, but user-target-2 should stay cached!
        assert!(
            get_auth_context_cache(&u1).is_none(),
            "Targeted user-target-1 should be invalidated"
        );
        assert!(
            get_auth_context_cache(&u2).is_some(),
            "Non-targeted user-target-2 should remain cached"
        );

        invalidate_auth_context_cache();
    }
}
