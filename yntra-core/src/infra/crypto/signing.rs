use crate::infra::errors::YntraError;
use ed25519_dalek::{Signer, Verifier};

fn construct_role_signature_message(
    user_id: &str,
    role: &str,
    workspace_id: &str,
    expires_at: i64,
    epoch: u64,
) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(b"YNTRA_ROLE_SIGNATURE_V3\0");
    message.extend_from_slice(&(user_id.len() as u64).to_be_bytes());
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(&(role.len() as u64).to_be_bytes());
    message.extend_from_slice(role.as_bytes());
    message.extend_from_slice(&(workspace_id.len() as u64).to_be_bytes());
    message.extend_from_slice(workspace_id.as_bytes());
    message.extend_from_slice(&expires_at.to_be_bytes());
    message.extend_from_slice(&epoch.to_be_bytes());
    message
}

/// Derives the verifying public key from a given hex-encoded Ed25519 private key.
#[uniffi::export]
pub fn derive_public_key_from_private_key(private_key_hex: &str) -> Result<String, YntraError> {
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
    Ok(const_hex::encode(signing_key.verifying_key().to_bytes()))
}

/// Generates a standard role signature using the private key.
#[uniffi::export]
pub fn generate_role_signature(
    private_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
) -> Result<String, YntraError> {
    let current_time = chrono::Utc::now().timestamp();
    let expires_at = current_time + 30 * 24 * 60 * 60;
    generate_role_signature_with_expiration(
        private_key_hex,
        user_id,
        role,
        workspace_id,
        expires_at,
    )
}

/// Generates a role signature with a custom expiration timestamp.
#[uniffi::export]
pub fn generate_role_signature_with_expiration(
    private_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    expires_at: i64,
) -> Result<String, YntraError> {
    generate_role_signature_v2(private_key_hex, user_id, role, workspace_id, expires_at, 0)
}

/// Generates a version 2 role signature containing epoch and custom expiration.
#[uniffi::export]
pub fn generate_role_signature_v2(
    private_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    expires_at: i64,
    epoch: u64,
) -> Result<String, YntraError> {
    let private_key_bytes = zeroize::Zeroizing::new(
        const_hex::decode(private_key_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?,
    );

    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    if private_key_bytes.len() != 32 {
        return Err(YntraError::CryptoError(
            "Invalid private key length".to_string(),
        ));
    }
    private_key_array.copy_from_slice(&private_key_bytes[..32]);

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
    let message = construct_role_signature_message(user_id, role, workspace_id, expires_at, epoch);
    let signature = signing_key.sign(&message);
    let signature_hex = const_hex::encode(&signature.to_bytes());
    Ok(format!("{}:{}:{}", epoch, expires_at, signature_hex))
}

use std::sync::Mutex;
use std::sync::Arc;
use zeroize::Zeroize;

#[derive(uniffi::Object)]
pub struct WorkspaceKeyPair {
    public_key: String,
    private_key: Mutex<zeroize::Zeroizing<String>>,
}

#[uniffi::export]
impl WorkspaceKeyPair {
    /// Returns the public key hex string.
    pub fn public_key(&self) -> String {
        self.public_key.clone()
    }
}

// Internal Rust-only methods (not exported to FFI)
impl WorkspaceKeyPair {
    /// Returns the private key hex string.
    pub fn private_key(&self) -> zeroize::Zeroizing<String> {
        self.private_key.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for WorkspaceKeyPair {
    fn drop(&mut self) {
        self.private_key.lock().unwrap_or_else(|e| e.into_inner()).zeroize();
    }
}

/// Generates a new cryptographic keypair for a workspace.
#[uniffi::export]
pub fn generate_workspace_keypair() -> Result<Arc<WorkspaceKeyPair>, YntraError> {
    let mut private_key_bytes = [0u8; 32];
    getrandom::fill(&mut private_key_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
    let private_key_hex = const_hex::encode(private_key_bytes);
    private_key_bytes.zeroize();
    Ok(Arc::new(WorkspaceKeyPair {
        public_key: public_key_hex,
        private_key: Mutex::new(zeroize::Zeroizing::new(private_key_hex)),
    }))
}

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static VERIFYING_KEY_CACHE: OnceLock<RwLock<HashMap<String, ed25519_dalek::VerifyingKey>>> = OnceLock::new();

pub fn get_parsed_verifying_key(public_key_hex: &str) -> Option<ed25519_dalek::VerifyingKey> {
    let cache_lock = VERIFYING_KEY_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(cache) = cache_lock.read() {
        if let Some(vk) = cache.get(public_key_hex) {
            return Some(*vk);
        }
    }

    if public_key_hex.len() != 64 {
        return None;
    }
    let bytes = const_hex::decode(public_key_hex).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&arr).ok()?;

    if let Ok(mut cache) = cache_lock.write() {
        cache.insert(public_key_hex.to_string(), vk);
    }
    Some(vk)
}

/// Verifies a user's role signature against a given public key.
#[uniffi::export]
pub fn verify_role_signature(
    public_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    signature_hex: &str,
) -> bool {
    let parts: Vec<&str> = signature_hex.split(':').collect();
    let mut is_valid = true;

    let (epoch_str, expires_str, sig_hex_str) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        2 => ("0", parts[0], parts[1]),
        _ => {
            is_valid = false;
            ("0", "0", "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
        }
    };

    let epoch = match epoch_str.parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            is_valid = false;
            0
        }
    };

    let expires_at = match expires_str.parse::<i64>() {
        Ok(val) => val,
        Err(_) => {
            is_valid = false;
            0
        }
    };

    let verifying_key = match get_parsed_verifying_key(public_key_hex) {
        Some(k) => k,
        None => {
            is_valid = false;
            ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]).verifying_key()
        }
    };

    let sig_hex = if sig_hex_str.len() == 128 {
        sig_hex_str
    } else {
        is_valid = false;
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
    };

    let signature_bytes = match const_hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => {
            is_valid = false;
            vec![0u8; 64]
        }
    };

    let signature_array: [u8; 64] = match signature_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            is_valid = false;
            [0u8; 64]
        }
    };

    let signature = ed25519_dalek::Signature::from_bytes(&signature_array);
    let message = construct_role_signature_message(user_id, role, workspace_id, expires_at, epoch);
    let verified = verifying_key.verify(&message, &signature).is_ok();

    let current_time = chrono::Utc::now().timestamp();
    let time_valid = current_time <= expires_at;

    let final_valid = is_valid & verified & time_valid;

    if !final_valid {
        if current_time > expires_at && is_valid {
            tracing::warn!("Role signature for user {} has expired", user_id);
        }
    }

    final_valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_role_signature_timing_resistant() {
        let keys = generate_workspace_keypair().unwrap();
        let pub_hex = keys.public_key();
        let priv_hex = keys.private_key();

        // 1. Valid signature
        let sig_hex = generate_role_signature(&priv_hex, "user-1", "admin", "ws-1").unwrap();
        assert!(verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", &sig_hex));

        // 2. Invalid role
        assert!(!verify_role_signature(&pub_hex, "user-1", "member", "ws-1", &sig_hex));

        // 3. Invalid user
        assert!(!verify_role_signature(&pub_hex, "user-2", "admin", "ws-1", &sig_hex));

        // 4. Invalid workspace
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-2", &sig_hex));

        // 5. Malformed signature string (invalid colons / components)
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", "invalid_format"));
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", "123:invalid_format"));

        // 6. Invalid hex signature
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", "0:123456789:not_hex"));

        // 7. Invalid length hex signature
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", "0:123456789:aabbcc"));

        // 8. Expired signature
        let current_time = chrono::Utc::now().timestamp();
        let expired_sig = generate_role_signature_v2(&priv_hex, "user-1", "admin", "ws-1", current_time - 10, 0).unwrap();
        assert!(!verify_role_signature(&pub_hex, "user-1", "admin", "ws-1", &expired_sig));
    }
}
