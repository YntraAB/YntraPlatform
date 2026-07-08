use crate::infra::errors::YntraError;
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use zeroize::Zeroize;
use super::*;

pub(crate) fn get_local_client_pepper() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(entry) = keyring::Entry::new("yntra-platform", "client_pepper") {
            if let Ok(pepper) = entry.get_password() {
                if !pepper.is_empty() {
                    return pepper;
                }
            }
            let mut rand_bytes = [0u8; 32];
            if getrandom::fill(&mut rand_bytes).is_ok() {
                let new_pepper = const_hex::encode(&rand_bytes);
                let _ = entry.set_password(&new_pepper);
                return new_pepper;
            }
        }
        use std::fs;
        use std::path::PathBuf;
        let path = PathBuf::from("yntra_client_pepper.bin");
        if let Ok(pepper) = fs::read_to_string(&path) {
            pepper
        } else {
            let mut rand_bytes = [0u8; 32];
            let _ = getrandom::fill(&mut rand_bytes);
            let new_pepper = const_hex::encode(&rand_bytes);
            let _ = fs::write(&path, &new_pepper);
            new_pepper
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(pepper)) = storage.get_item("yntra_client_pepper") {
                    return pepper;
                } else {
                    let mut rand_bytes = [0u8; 32];
                    let _ = getrandom::fill(&mut rand_bytes);
                    let new_pepper = const_hex::encode(&rand_bytes);
                    let _ = storage.set_item("yntra_client_pepper", &new_pepper);
                    return new_pepper;
                }
            }
        }
        static SESSION_PEPPER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        SESSION_PEPPER.get_or_init(|| {
            let mut rand_bytes = [0u8; 32];
            let _ = getrandom::fill(&mut rand_bytes);
            const_hex::encode(&rand_bytes)
        }).clone()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_local_secret(key: &str) -> Result<Option<String>, YntraError> {
    let key = key.to_string();
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        match entry.get_password() {
            Ok(secret) => {
                if secret.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(secret))
                }
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(YntraError::CryptoError(format!("Keyring access failed: {:?}", e))),
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), YntraError> {
    let key = key.to_string();
    let value = value.to_string();
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        if value.is_empty() {
            let _ = entry.delete_credential();
            Ok(())
        } else {
            if let Err(e) = entry.set_password(&value) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!("Keyring write failed in test/CI environment (swallowing): {:?}", e);
                    Ok(())
                } else {
                    Err(YntraError::CryptoError(format!("Failed to store secret in keyring: {:?}", e)))
                }
            } else {
                Ok(())
            }
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_epoch_if_greater(ws_id: &str, current_epoch: u64) -> Result<u64, YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        
        let cached_epoch = if let Ok(val) = entry.get_password() {
            val.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        if current_epoch < cached_epoch {
            return Err(YntraError::AuthError(
                "Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()
            ));
        }
        
        if current_epoch > cached_epoch {
            if let Err(e) = entry.set_password(&current_epoch.to_string()) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!("Keyring write failed in test/CI environment (swallowing): {:?}", e);
                    Ok(current_epoch)
                } else {
                    Err(YntraError::CryptoError(format!("Failed to store epoch in keyring: {:?}", e)))
                }
            } else {
                Ok(current_epoch)
            }
        } else {
            Ok(cached_epoch)
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(target_arch = "wasm32")]
fn compute_integrity_hmac(key: &str, value: &str) -> Result<String, YntraError> {
    let salt = get_system_salt_ref()?;
    
    let mut hasher = blake3::Hasher::new_derive_key("Yntra Local Storage Integrity v1");
    hasher.update(salt);
    let mut hmac_key = [0u8; 32];
    hasher.finalize_xof().fill(&mut hmac_key);
    hasher.zeroize();

    let mut keyed_hasher = blake3::Hasher::new_keyed(&hmac_key);
    keyed_hasher.update(key.as_bytes());
    keyed_hasher.update(value.as_bytes());
    let hash = keyed_hasher.finalize();
    
    Ok(const_hex::encode(hash.as_bytes()))
}

#[cfg(target_arch = "wasm32")]
pub async fn get_local_secret(key: &str) -> Result<Option<String>, YntraError> {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let secret = storage.get_item(key)
                .map_err(|e| YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
            if let Some(ref val) = secret {
                let hmac_key_name = format!("{}_integrity", key);
                let stored_hmac = storage.get_item(&hmac_key_name)
                    .map_err(|e| YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(key, val)?;
                    if hmac != &computed {
                        return Err(YntraError::AuthError("Local storage tampering detected".to_string()));
                    }
                } else {
                    return Err(YntraError::AuthError("Local storage integrity verification missing".to_string()));
                }
            }
            return Ok(secret);
        }
    }
    Err(YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), YntraError> {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let hmac_key_name = format!("{}_integrity", key);
            if value.is_empty() {
                let _ = storage.remove_item(key);
                let _ = storage.remove_item(&hmac_key_name);
            } else {
                let hmac = compute_integrity_hmac(key, value)?;
                storage.set_item(key, value).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                storage.set_item(&hmac_key_name, &hmac).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
            }
            return Ok(());
        }
    }
    Err(YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_epoch_if_greater(ws_id: &str, current_epoch: u64) -> Result<u64, YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    let hmac_key_name = format!("{}_integrity", key);
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let cached_epoch = if let Ok(Some(val)) = storage.get_item(&key) {
                let stored_hmac = storage.get_item(&hmac_key_name)
                    .map_err(|e| YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(&key, &val)?;
                    if hmac != &computed {
                        return Err(YntraError::AuthError("Local storage tampering detected".to_string()));
                    }
                } else {
                    return Err(YntraError::AuthError("Local storage integrity verification missing".to_string()));
                }
                val.parse::<u64>().unwrap_or(0)
            } else {
                0
            };
            
            if current_epoch < cached_epoch {
                return Err(YntraError::AuthError(
                    "Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()
                ));
            }
            
            if current_epoch > cached_epoch {
                let epoch_str = current_epoch.to_string();
                let hmac = compute_integrity_hmac(&key, &epoch_str)?;
                storage.set_item(&key, &epoch_str).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                storage.set_item(&hmac_key_name, &hmac).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                return Ok(current_epoch);
            } else {
                return Ok(cached_epoch);
            }
        }
    }
    Err(YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[uniffi::export]
pub fn encrypt_workspace_key_with_password(
    password: &str,
    mut workspace_key: Vec<u8>,
) -> Result<String, YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    
    let mut nonce_bytes = [0u8; 24];
    getrandom::fill(&mut nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    
    let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    let res = argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key);
    if res.is_err() {
        workspace_key.zeroize();
        return Err(YntraError::CryptoError("Argon2 derivation failed".to_string()));
    }
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, workspace_key.as_slice());
    workspace_key.zeroize();
    
    let ciphertext = ciphertext.map_err(|_| YntraError::CryptoError("Envelope encryption failed".to_string()))?;
        
    Ok(format!(
        "envelope:{}:{}:{}",
        const_hex::encode(&salt),
        const_hex::encode(&nonce_bytes),
        const_hex::encode(&ciphertext)
    ))
}

#[uniffi::export]
pub fn decrypt_workspace_key_with_password(
    password: &str,
    encrypted_envelope: &str,
) -> Result<Vec<u8>, YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    if !encrypted_envelope.starts_with("envelope:") {
        return Err(YntraError::CryptoError("Invalid envelope format".to_string()));
    }
    
    let parts: Vec<&str> = encrypted_envelope[9..].split(':').collect();
    if parts.len() != 3 {
        return Err(YntraError::CryptoError("Invalid envelope structure".to_string()));
    }
    
    let salt = const_hex::decode(parts[0])
        .map_err(|_| YntraError::CryptoError("Invalid envelope salt".to_string()))?;
    let nonce_bytes = const_hex::decode(parts[1])
        .map_err(|_| YntraError::CryptoError("Invalid envelope nonce".to_string()))?;
    let ciphertext = const_hex::decode(parts[2])
        .map_err(|_| YntraError::CryptoError("Invalid envelope ciphertext".to_string()))?;
        
    let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key)
        .map_err(|_| YntraError::CryptoError("Argon2 derivation failed".to_string()))?;
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| YntraError::CryptoError("Envelope decryption failed".to_string()))?;
        
    Ok(plaintext)
}
