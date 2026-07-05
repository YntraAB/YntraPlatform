use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use std::sync::{Mutex, OnceLock};

static SESSION_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[uniffi::export]
pub fn set_session_key(key: String) {
    if let Ok(mut lock) = SESSION_KEY.get_or_init(|| Mutex::new(None)).lock() {
        *lock = Some(key);
    }
}

#[uniffi::export]
pub fn clear_session_key() {
    if let Ok(mut lock) = SESSION_KEY.get_or_init(|| Mutex::new(None)).lock() {
        *lock = None;
    }
}

pub fn get_encryption_keys(workspace_id: &str) -> ([u8; 32], [u8; 12]) {
    get_encryption_keys_internal(workspace_id, true)
}

fn get_encryption_keys_internal(workspace_id: &str, use_session_key: bool) -> ([u8; 32], [u8; 12]) {
    let mut key_material = workspace_id.to_string();
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(salt) = std::env::var("YNTRA_ENCRYPTION_SALT") {
            key_material.push_str(&salt);
        } else {
            key_material.push_str("yntra-secure-whistleblower-salt-2026");
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        key_material.push_str("yntra-secure-whistleblower-salt-2026");
    }

    if use_session_key {
        if let Ok(lock) = SESSION_KEY.get_or_init(|| Mutex::new(None)).lock() {
            if let Some(ref sk) = *lock {
                key_material.push_str(sk);
            }
        }
    }
    
    // Derive the 32-byte key
    let key = blake3::derive_key("Yntra whistleblower key derivation v1", key_material.as_bytes());
    
    // Derive the 12-byte nonce
    let nonce_bytes = blake3::derive_key("Yntra whistleblower nonce derivation v1", key_material.as_bytes());
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes[0..12]);
    
    (key, nonce)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const_hex::encode(bytes)
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    const_hex::decode(s).ok()
}

pub fn encrypt_field(data: &str, workspace_id: &str) -> String {
    let mut nonce_bytes = [0u8; 12];
    if let Err(e) = getrandom::fill(&mut nonce_bytes) {
        tracing::error!("Failed to generate random nonce: {:?}", e);
        return data.to_string();
    }

    // Encrypt using the strong key derived with session key (if set)
    let (key_bytes, _) = get_encryption_keys_internal(workspace_id, true);
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    if let Ok(ct) = cipher.encrypt(nonce, data.as_bytes()) {
        format!("enc:{}:{}", hex_encode(&nonce_bytes), hex_encode(&ct))
    } else {
        data.to_string()
    }
}

pub fn decrypt_field(encrypted_data: &str, workspace_id: &str) -> String {
    if !encrypted_data.starts_with("enc:") {
        return encrypted_data.to_string();
    }
    
    let body = &encrypted_data[4..];
    let parts: Vec<&str> = body.split(':').collect();

    if parts.len() == 2 {
        // New format: enc:{nonce_hex}:{ciphertext_hex}
        let nonce_bytes = match hex_decode(parts[0]) {
            Some(b) if b.len() == 12 => {
                let mut n = [0u8; 12];
                n.copy_from_slice(&b);
                n
            }
            _ => return encrypted_data.to_string(),
        };
        let ct = match hex_decode(parts[1]) {
            Some(b) => b,
            None => return encrypted_data.to_string(),
        };

        // 1. Try to decrypt using the strong session-derived key first
        let (key_bytes, _) = get_encryption_keys_internal(workspace_id, true);
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
            return String::from_utf8(pt).unwrap_or_else(|_| encrypted_data.to_string());
        }

        // 2. Fall back to decrypting with the legacy workspace-only key
        let (key_bytes, _) = get_encryption_keys_internal(workspace_id, false);
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
            String::from_utf8(pt).unwrap_or_else(|_| encrypted_data.to_string())
        } else {
            encrypted_data.to_string()
        }
    } else {
        // Legacy fallback format: enc:{ciphertext_hex}
        let ct = match hex_decode(body) {
            Some(b) => b,
            None => return encrypted_data.to_string(),
        };

        // 1. Try to decrypt using the strong session-derived key first
        let (key_bytes, legacy_nonce_bytes) = get_encryption_keys_internal(workspace_id, true);
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&legacy_nonce_bytes);
        
        if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
            return String::from_utf8(pt).unwrap_or_else(|_| encrypted_data.to_string());
        }

        // 2. Fall back to decrypting with the legacy workspace-only key
        let (key_bytes, legacy_nonce_bytes) = get_encryption_keys_internal(workspace_id, false);
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&legacy_nonce_bytes);
        
        if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
            String::from_utf8(pt).unwrap_or_else(|_| encrypted_data.to_string())
        } else {
            encrypted_data.to_string()
        }
    }
}

pub fn encrypt_opt_field(data: Option<String>, workspace_id: Option<String>) -> Option<String> {
    let ws_id = workspace_id.unwrap_or_else(|| "workspace-1".to_string());
    data.map(|d| encrypt_field(&d, &ws_id))
}

pub fn decrypt_opt_field(encrypted_data: Option<String>, workspace_id: Option<String>) -> Option<String> {
    let ws_id = workspace_id.unwrap_or_else(|| "workspace-1".to_string());
    encrypted_data.map(|d| decrypt_field(&d, &ws_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chacha_encryption_decryption() {
        let plaintext = "Sensitive whistleblowing report text";
        let workspace_id = "test-workspace-123";
        let encrypted = encrypt_field(plaintext, workspace_id);
        assert!(encrypted.starts_with("enc:"));
        
        // Assert that the encrypted format contains two colon-separated hex strings
        let body = &encrypted[4..];
        let parts: Vec<&str> = body.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 24); // 12-byte hex nonce is 24 characters
        
        let decrypted = decrypt_field(&encrypted, workspace_id);
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_legacy_decryption_fallback() {
        let plaintext = "Legacy encrypted field value";
        let workspace_id = "test-workspace-123";
        
        // Emulate legacy encryption (using the static nonce derived from get_encryption_keys)
        let (key_bytes, legacy_nonce_bytes) = get_encryption_keys_internal(workspace_id, false);
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&legacy_nonce_bytes);
        let ct = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
        let legacy_encrypted = format!("enc:{}", hex_encode(&ct));
        
        // Decrypt using the updated decrypt_field which should trigger the fallback
        let decrypted = decrypt_field(&legacy_encrypted, workspace_id);
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_strong_session_key_derivation() {
        let plaintext = "Highly sensitive user data";
        let workspace_id = "test-workspace-456";

        // Set session key
        set_session_key("my-super-secret-user-password-or-pin".to_string());

        let encrypted = encrypt_field(plaintext, workspace_id);
        
        // Decrypt with correct session key set
        let decrypted = decrypt_field(&encrypted, workspace_id);
        assert_eq!(plaintext, decrypted);

        // Temporarily clear session key and ensure decryption falls back or fails gracefully
        clear_session_key();
        let decrypted_without_key = decrypt_field(&encrypted, workspace_id);
        // It should NOT decrypt, returning the original ciphertext string since the session key is missing
        assert_eq!(encrypted, decrypted_without_key);

        // Reset session key and ensure it works again
        set_session_key("my-super-secret-user-password-or-pin".to_string());
        let decrypted_with_key_again = decrypt_field(&encrypted, workspace_id);
        assert_eq!(plaintext, decrypted_with_key_again);

        clear_session_key();
    }
}
