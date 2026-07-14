use ed25519_dalek::{Signer, Verifier};
use crate::infra::errors::YntraError;

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

#[uniffi::export]
pub fn derive_public_key_from_private_key(private_key_hex: &str) -> Result<String, YntraError> {
    let private_key_bytes = zeroize::Zeroizing::new(
        const_hex::decode(private_key_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?
    );
    if private_key_bytes.len() != 32 {
        return Err(YntraError::CryptoError("Invalid private key length".to_string()));
    }
    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
    Ok(const_hex::encode(signing_key.verifying_key().to_bytes()))
}

#[uniffi::export]
pub fn generate_role_signature(
    private_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
) -> Result<String, YntraError> {
    let current_time = chrono::Utc::now().timestamp();
    let expires_at = current_time + 30 * 24 * 60 * 60;
    generate_role_signature_with_expiration(private_key_hex, user_id, role, workspace_id, expires_at)
}

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
        const_hex::decode(private_key_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?
    );
    
    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    if private_key_bytes.len() != 32 {
        return Err(YntraError::CryptoError("Invalid private key length".to_string()));
    }
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
        
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
    let message = construct_role_signature_message(user_id, role, workspace_id, expires_at, epoch);
    let signature = signing_key.sign(&message);
    let signature_hex = const_hex::encode(&signature.to_bytes());
    Ok(format!("{}:{}:{}", epoch, expires_at, signature_hex))
}

#[uniffi::export]
pub fn generate_workspace_keypair() -> Result<Vec<String>, YntraError> {
    let mut private_key_bytes = [0u8; 32];
    getrandom::fill(&mut private_key_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
    let private_key_hex = const_hex::encode(private_key_bytes);
    Ok(vec![public_key_hex, private_key_hex])
}

#[uniffi::export]
pub fn verify_role_signature(
    public_key_hex: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    signature_hex: &str,
) -> bool {
    let parts: Vec<&str> = signature_hex.split(':').collect();
    let (epoch, expires_at, sig_hex) = match parts.len() {
        3 => {
            let epoch = match parts[0].parse::<u64>() {
                Ok(val) => val,
                Err(_) => return false,
            };
            let expires = match parts[1].parse::<i64>() {
                Ok(val) => val,
                Err(_) => return false,
            };
            (epoch, expires, parts[2])
        }
        2 => {
            let expires = match parts[0].parse::<i64>() {
                Ok(val) => val,
                Err(_) => return false,
            };
            (0, expires, parts[1])
        }
        _ => return false,
    };
    
    let current_time = chrono::Utc::now().timestamp();
        
    if current_time > expires_at {
        tracing::warn!("Role signature for user {} has expired", user_id);
        return false;
    }

    let public_key_bytes = match const_hex::decode(public_key_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature_bytes = match const_hex::decode(sig_hex) {
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
    let message = construct_role_signature_message(user_id, role, workspace_id, expires_at, epoch);
    verifying_key.verify(&message, &signature).is_ok()
}
