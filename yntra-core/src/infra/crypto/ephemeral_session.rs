use crate::infra::errors::YntraError;
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroize;

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct EphemeralSessionToken {
    pub token_id: String,
    pub user_id: String,
    pub workspace_id: String,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub last_synced_at_ms: i64,
    pub offline_policy_window_seconds: i64,
    pub is_revoked: bool,
}

struct EphemeralSessionStore {
    token: EphemeralSessionToken,
    raw_key: [u8; 32],
}

impl Zeroize for EphemeralSessionStore {
    fn zeroize(&mut self) {
        self.raw_key.zeroize();
        self.token.token_id.zeroize();
        self.token.user_id.zeroize();
        self.token.workspace_id.zeroize();
        self.token.is_revoked = true;
    }
}

static ACTIVE_SESSION: OnceLock<Mutex<Option<EphemeralSessionStore>>> = OnceLock::new();

fn get_session_lock() -> &'static Mutex<Option<EphemeralSessionStore>> {
    ACTIVE_SESSION.get_or_init(|| Mutex::new(None))
}

/// Register a new ephemeral session token and initialize local database encryption keys.
#[uniffi::export]
pub fn register_ephemeral_session_token(
    token_id: String,
    user_id: String,
    workspace_id: String,
    offline_policy_seconds: i64,
    key_hex: String,
) -> Result<EphemeralSessionToken, YntraError> {
    let key_bytes = const_hex::decode(&key_hex)
        .map_err(|e| YntraError::CryptoError(format!("Invalid encryption key hex: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(YntraError::CryptoError(
            "Encryption key must be exactly 32 bytes (64 hex characters)".to_string(),
        ));
    }

    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(&key_bytes);

    let now_ms = crate::infra::time::get_current_time_ms();
    let policy_secs = if offline_policy_seconds <= 0 {
        604_800 // Default 7 days policy window
    } else {
        offline_policy_seconds
    };

    let expires_at_ms = now_ms + (policy_secs * 1000);

    let token = EphemeralSessionToken {
        token_id,
        user_id,
        workspace_id,
        issued_at_ms: now_ms,
        expires_at_ms,
        last_synced_at_ms: now_ms,
        offline_policy_window_seconds: policy_secs,
        is_revoked: false,
    };

    let store = EphemeralSessionStore {
        token: token.clone(),
        raw_key,
    };

    let mut lock = get_session_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Session lock poisoned".to_string()))?;

    if let Some(mut old_store) = lock.take() {
        old_store.zeroize();
    }

    *lock = Some(store);
    Ok(token)
}

/// Reset the offline countdown timer upon a successful server sync.
#[uniffi::export]
pub fn touch_session_sync_timestamp(workspace_id: String) -> Result<(), YntraError> {
    let mut lock = get_session_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Session lock poisoned".to_string()))?;

    if let Some(ref mut store) = *lock {
        if store.token.workspace_id == workspace_id && !store.token.is_revoked {
            let now_ms = crate::infra::time::get_current_time_ms();
            store.token.last_synced_at_ms = now_ms;
            store.token.expires_at_ms = now_ms + (store.token.offline_policy_window_seconds * 1000);
            return Ok(());
        }
    }

    Err(YntraError::AuthError(
        "No active session token found for workspace to update sync timestamp".to_string(),
    ))
}

/// Remotely revoke the active session token and purge encryption keys from memory.
#[uniffi::export]
pub fn revoke_ephemeral_session_token(token_id: String) -> Result<(), YntraError> {
    let mut lock = get_session_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Session lock poisoned".to_string()))?;

    if let Some(mut store) = lock.take() {
        if store.token.token_id == token_id || token_id.is_empty() {
            store.zeroize();
            return Ok(());
        } else {
            *lock = Some(store);
        }
    }

    Ok(())
}

/// Validate if an active session token is valid and unexpired under the offline policy window.
#[uniffi::export]
pub fn validate_active_session_token(workspace_id: &str) -> Result<EphemeralSessionToken, YntraError> {
    let mut lock = get_session_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Session lock poisoned".to_string()))?;

    let is_expired_or_invalid = if let Some(ref store) = *lock {
        if !workspace_id.is_empty() && store.token.workspace_id != workspace_id {
            return Err(YntraError::AuthError(format!(
                "Session token workspace mismatch: expected {}, active is {}",
                workspace_id, store.token.workspace_id
            )));
        }

        if store.token.is_revoked {
            true
        } else {
            let now_ms = crate::infra::time::get_current_time_ms();
            let elapsed_seconds = (now_ms - store.token.last_synced_at_ms) / 1000;
            elapsed_seconds > store.token.offline_policy_window_seconds
        }
    } else {
        false
    };

    if is_expired_or_invalid {
        if let Some(mut expired_store) = lock.take() {
            let policy_window = expired_store.token.offline_policy_window_seconds;
            let last_sync = expired_store.token.last_synced_at_ms;
            expired_store.zeroize();
            let now_ms = crate::infra::time::get_current_time_ms();
            let elapsed = (now_ms - last_sync) / 1000;

            return Err(YntraError::AuthError(format!(
                "Local edge database session key expired or revoked due to offline policy limit ({} seconds offline > {} seconds allowed)",
                elapsed, policy_window
            )));
        }
    }

    if let Some(ref store) = *lock {
        return Ok(store.token.clone());
    }

    Err(YntraError::AuthError(
        "No active ephemeral session token registered for local database access".to_string(),
    ))
}

/// Retrieve the active raw 32-byte database encryption key.
pub fn get_active_session_encryption_key(workspace_id: &str) -> Result<[u8; 32], YntraError> {
    let lock = get_session_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Session lock poisoned".to_string()))?;

    if let Some(ref store) = *lock {
        if !workspace_id.is_empty() && store.token.workspace_id != workspace_id {
            return Err(YntraError::AuthError(
                "Workspace ID mismatch for database encryption key".to_string(),
            ));
        }

        if store.token.is_revoked {
            return Err(YntraError::AuthError(
                "Encryption key revoked".to_string(),
            ));
        }

        let now_ms = crate::infra::time::get_current_time_ms();
        let elapsed_seconds = (now_ms - store.token.last_synced_at_ms) / 1000;

        if elapsed_seconds > store.token.offline_policy_window_seconds {
            return Err(YntraError::AuthError(
                "Offline policy window exceeded, key expired".to_string(),
            ));
        }

        return Ok(store.raw_key);
    }

    Err(YntraError::AuthError(
        "No active database session key initialized".to_string(),
    ))
}

/// Explicitly clear and zeroize active session key from memory.
#[uniffi::export]
pub fn clear_active_session_token() {
    if let Ok(mut lock) = get_session_lock().lock() {
        if let Some(mut store) = lock.take() {
            store.zeroize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_session_lifecycle() {
        clear_active_session_token();

        let key_hex = const_hex::encode(&[0x42u8; 32]);
        let token = register_ephemeral_session_token(
            "tok-1".to_string(),
            "user-1".to_string(),
            "ws-1".to_string(),
            10, // 10 seconds policy window
            key_hex,
        )
        .unwrap();

        assert_eq!(token.token_id, "tok-1");
        assert_eq!(token.workspace_id, "ws-1");

        // Validate active token
        let validated = validate_active_session_token("ws-1").unwrap();
        assert_eq!(validated.user_id, "user-1");

        // Extract raw key
        let raw_key = get_active_session_encryption_key("ws-1").unwrap();
        assert_eq!(raw_key, [0x42u8; 32]);

        // Touch sync timestamp
        assert!(touch_session_sync_timestamp("ws-1".to_string()).is_ok());

        // Revoke token
        revoke_ephemeral_session_token("tok-1".to_string()).unwrap();

        // Verification after revocation must fail
        assert!(validate_active_session_token("ws-1").is_err());
        clear_active_session_token();
    }
}
