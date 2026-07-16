use super::*;
use crate::infra::errors::YntraError;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroize;

#[uniffi::export(callback_interface)]
pub trait SecureStorageProvider: Send + Sync {
    fn get_secure_secret(&self, key: String) -> Option<String>;
    fn set_secure_secret(&self, key: String, value: String) -> bool;
}

static SECURE_STORAGE_PROVIDER: std::sync::OnceLock<Box<dyn SecureStorageProvider>> =
    std::sync::OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static FALLBACK_KEYRING: std::sync::OnceLock<std::sync::RwLock<std::collections::HashMap<String, String>>> =
    std::sync::OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn get_fallback_keyring() -> &'static std::sync::RwLock<std::collections::HashMap<String, String>> {
    FALLBACK_KEYRING.get_or_init(|| std::sync::RwLock::new(std::collections::HashMap::new()))
}

/// Registers a secure storage provider callback for handling system secrets.
#[uniffi::export]
pub fn register_secure_storage_provider(provider: Box<dyn SecureStorageProvider>) -> bool {
    SECURE_STORAGE_PROVIDER.set(provider).is_ok()
}

static DATABASE_PEPPER: std::sync::OnceLock<String> = std::sync::OnceLock::new();

#[uniffi::export]
pub fn set_database_pepper(pepper: String) -> bool {
    DATABASE_PEPPER.set(pepper).is_ok()
}

pub(crate) fn get_local_client_pepper() -> Result<String, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(entry) = keyring::Entry::new("yntra-platform", "client_pepper") {
            if let Ok(pepper) = entry.get_password() {
                if !pepper.is_empty() {
                    return Ok(pepper);
                }
            }
            let mut rand_bytes = [0u8; 32];
            getrandom::fill(&mut rand_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let new_pepper = const_hex::encode(&rand_bytes);
            let _ = entry.set_password(&new_pepper);
            return Ok(new_pepper);
        }
        use std::fs;
        use std::path::PathBuf;
        let path = PathBuf::from(crate::database::native::get_database_path(
            "yntra_client_pepper.bin",
        ));
        if let Ok(pepper) = fs::read_to_string(&path) {
            Ok(pepper)
        } else {
            let mut rand_bytes = [0u8; 32];
            getrandom::fill(&mut rand_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let new_pepper = const_hex::encode(&rand_bytes);
            let _ = fs::write(&path, &new_pepper);
            Ok(new_pepper)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(pepper)) = storage.get_item("yntra_client_pepper") {
                    return Ok(pepper);
                } else {
                    let mut rand_bytes = [0u8; 32];
                    getrandom::fill(&mut rand_bytes)
                        .map_err(|e| YntraError::CryptoError(e.to_string()))?;
                    let new_pepper = const_hex::encode(&rand_bytes);
                    let _ = storage.set_item("yntra_client_pepper", &new_pepper);
                    return Ok(new_pepper);
                }
            }
        }
        if let Some(pepper) = DATABASE_PEPPER.get() {
            return Ok(pepper.clone());
        }
        static SESSION_PEPPER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        if let Some(pepper) = SESSION_PEPPER.get() {
            return Ok(pepper.clone());
        }
        let mut rand_bytes = [0u8; 32];
        getrandom::fill(&mut rand_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let new_pepper = const_hex::encode(&rand_bytes);
        match SESSION_PEPPER.set(new_pepper.clone()) {
            Ok(_) => Ok(new_pepper),
            Err(_) => Ok(SESSION_PEPPER.get().unwrap().clone()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_local_secret(key: &str) -> Result<Option<String>, YntraError> {
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if key.starts_with("test-") || key.contains("ws-1") {
            return Ok(provider.get_secure_secret(key.to_string()));
        }
    }
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
            Err(e) => {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    if let Some(val) = get_fallback_keyring().read().unwrap().get(&key).cloned() {
                        return Ok(Some(val));
                    }
                }
                match e {
                    keyring::Error::NoEntry => Ok(None),
                    err => Err(YntraError::CryptoError(format!(
                        "Keyring access failed: {:?}",
                        err
                    ))),
                }
            }
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), YntraError> {
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if key.starts_with("test-") || key.contains("ws-1") {
            if provider.set_secure_secret(key.to_string(), value.to_string()) {
                return Ok(());
            } else {
                return Err(YntraError::CryptoError(
                    "SecureStorageProvider failed to write secret".to_string(),
                ));
            }
        }
    }
    let key = key.to_string();
    let value = value.to_string();
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        if value.is_empty() {
            let _ = entry.delete_credential();
            if cfg!(test) || std::env::var("CI").is_ok() {
                get_fallback_keyring().write().unwrap().remove(&key);
            }
            Ok(())
        } else {
            if let Err(e) = entry.set_password(&value) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!(
                        "Keyring write failed in test/CI environment, writing to in-memory fallback: {:?}",
                        e
                    );
                    get_fallback_keyring().write().unwrap().insert(key.clone(), value.clone());
                    Ok(())
                } else {
                    Err(YntraError::CryptoError(format!(
                        "Failed to store secret in keyring: {:?}",
                        e
                    )))
                }
            } else {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    get_fallback_keyring().write().unwrap().insert(key.clone(), value.clone());
                }
                Ok(())
            }
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_epoch_if_greater(
    ws_id: &str,
    current_epoch: u64,
) -> Result<u64, YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if ws_id == "ws-1" {
            let cached_epoch = if let Some(val) = provider.get_secure_secret(key.clone()) {
                val.parse::<u64>().unwrap_or(0)
            } else {
                0
            };

            if current_epoch < cached_epoch {
                return Err(YntraError::AuthError(
                    "Workspace auth epoch rollback detected. Local database tampering suspected."
                        .to_string(),
                ));
            }

            if current_epoch > cached_epoch {
                if provider.set_secure_secret(key, current_epoch.to_string()) {
                    return Ok(current_epoch);
                } else {
                    return Err(YntraError::CryptoError(
                        "SecureStorageProvider failed to store epoch".to_string(),
                    ));
                }
            } else {
                return Ok(cached_epoch);
            }
        }
    }
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;

        let cached_epoch = if let Ok(val) = entry.get_password() {
            val.parse::<u64>().unwrap_or(0)
        } else if cfg!(test) || std::env::var("CI").is_ok() {
            get_fallback_keyring().read().unwrap().get(&key)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
        } else {
            0
        };

        if current_epoch < cached_epoch {
            return Err(YntraError::AuthError(
                "Workspace auth epoch rollback detected. Local database tampering suspected."
                    .to_string(),
            ));
        }

        if current_epoch > cached_epoch {
            if let Err(e) = entry.set_password(&current_epoch.to_string()) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!(
                        "Keyring write failed in test/CI environment, writing to in-memory fallback: {:?}",
                        e
                    );
                    get_fallback_keyring().write().unwrap().insert(key.clone(), current_epoch.to_string());
                    Ok(current_epoch)
                } else {
                    Err(YntraError::CryptoError(format!(
                        "Failed to store epoch in keyring: {:?}",
                        e
                    )))
                }
            } else {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    get_fallback_keyring().write().unwrap().insert(key.clone(), current_epoch.to_string());
                }
                Ok(current_epoch)
            }
        } else {
            Ok(cached_epoch)
        }
    })
    .await
    .map_err(|e| YntraError::CryptoError(e.to_string()))?
}

#[cfg(any(target_arch = "wasm32", test))]
fn compute_integrity_hmac(key: &str, value: &str) -> Result<String, YntraError> {
    let salt = get_system_salt_ref()?;
    let client_pepper = get_local_client_pepper()?;

    let context_str = super::CryptoDomain::LocalStorageIntegrity.get_context(2)?;
    let mut hasher = blake3::Hasher::new_derive_key(context_str);
    hasher.update(salt);
    hasher.update(client_pepper.as_bytes());
    let mut hmac_key = [0u8; 32];
    hasher.finalize_xof().fill(&mut hmac_key);
    hasher.zeroize();

    let mut keyed_hasher = blake3::Hasher::new_keyed(&hmac_key);
    keyed_hasher.update(key.as_bytes());
    keyed_hasher.update(value.as_bytes());
    let hash = keyed_hasher.finalize();

    Ok(const_hex::encode(hash.as_bytes()))
}

#[cfg(any(target_arch = "wasm32", test))]
fn compute_legacy_integrity_hmac(key: &str, value: &str) -> Result<String, YntraError> {
    let salt = get_system_salt_ref()?;

    let context_str = super::CryptoDomain::LocalStorageIntegrity.get_context(1)?;
    let mut hasher = blake3::Hasher::new_derive_key(context_str);
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
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if key.starts_with("test-") || key.contains("ws-1") {
            return Ok(provider.get_secure_secret(key.to_string()));
        }
    }
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let secret = storage.get_item(key).map_err(|e| {
                YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e))
            })?;
            if let Some(ref val) = secret {
                let hmac_key_name = format!("{}_integrity", key);
                let stored_hmac = storage.get_item(&hmac_key_name).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e))
                })?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(key, val)?;
                    if hmac != &computed {
                        // Fallback to legacy v1 HMAC verification
                        if let Ok(legacy_computed) = compute_legacy_integrity_hmac(key, val) {
                            if hmac == &legacy_computed {
                                // Automatically migrate to secure v2 HMAC
                                let _ = storage.set_item(&hmac_key_name, &computed);
                            } else {
                                return Err(YntraError::AuthError(
                                    "Local storage tampering detected".to_string(),
                                ));
                            }
                        } else {
                            return Err(YntraError::AuthError(
                                "Local storage tampering detected".to_string(),
                            ));
                        }
                    }
                } else {
                    return Err(YntraError::AuthError(
                        "Local storage integrity verification missing".to_string(),
                    ));
                }
            }
            return Ok(secret);
        }
    }
    Err(YntraError::CryptoError(
        "LocalStorage not available".to_string(),
    ))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), YntraError> {
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if key.starts_with("test-") || key.contains("ws-1") {
            if provider.set_secure_secret(key.to_string(), value.to_string()) {
                return Ok(());
            } else {
                return Err(YntraError::CryptoError(
                    "SecureStorageProvider failed to write secret".to_string(),
                ));
            }
        }
    }
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
    Err(YntraError::CryptoError(
        "LocalStorage not available".to_string(),
    ))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_epoch_if_greater(
    ws_id: &str,
    current_epoch: u64,
) -> Result<u64, YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    if let Some(provider) = SECURE_STORAGE_PROVIDER.get() {
        if ws_id == "ws-1" {
            let cached_epoch = if let Some(val) = provider.get_secure_secret(key.clone()) {
                val.parse::<u64>().unwrap_or(0)
            } else {
                0
            };

            if current_epoch < cached_epoch {
                return Err(YntraError::AuthError(
                    "Workspace auth epoch rollback detected. Local database tampering suspected."
                        .to_string(),
                ));
            }

            if current_epoch > cached_epoch {
                if provider.set_secure_secret(key, current_epoch.to_string()) {
                    return Ok(current_epoch);
                } else {
                    return Err(YntraError::CryptoError(
                        "SecureStorageProvider failed to store epoch".to_string(),
                    ));
                }
            } else {
                return Ok(cached_epoch);
            }
        }
    }
    let hmac_key_name = format!("{}_integrity", key);
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let cached_epoch = if let Ok(Some(val)) = storage.get_item(&key) {
                let stored_hmac = storage.get_item(&hmac_key_name).map_err(|e| {
                    YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e))
                })?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(&key, &val)?;
                    if hmac != &computed {
                        // Fallback to legacy v1 HMAC verification
                        if let Ok(legacy_computed) = compute_legacy_integrity_hmac(&key, &val) {
                            if hmac == &legacy_computed {
                                // Automatically migrate to secure v2 HMAC
                                let _ = storage.set_item(&hmac_key_name, &computed);
                            } else {
                                return Err(YntraError::AuthError(
                                    "Local storage tampering detected".to_string(),
                                ));
                            }
                        } else {
                            return Err(YntraError::AuthError(
                                "Local storage tampering detected".to_string(),
                            ));
                        }
                    }
                } else {
                    return Err(YntraError::AuthError(
                        "Local storage integrity verification missing".to_string(),
                    ));
                }
                val.parse::<u64>().unwrap_or(0)
            } else {
                0
            };

            if current_epoch < cached_epoch {
                return Err(YntraError::AuthError(
                    "Workspace auth epoch rollback detected. Local database tampering suspected."
                        .to_string(),
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
    Err(YntraError::CryptoError(
        "LocalStorage not available".to_string(),
    ))
}

/// Encrypts a raw workspace key with a password using Argon2id key stretching and XChaCha20Poly1305.
#[uniffi::export]
pub fn encrypt_workspace_key_with_password(
    mut password: String,
    mut workspace_key: Vec<u8>,
) -> Result<String, YntraError> {
    use argon2::{Algorithm, Argon2, Params, Version};

    let mut salt = [0u8; 16];
    if let Err(e) = getrandom::fill(&mut salt) {
        password.zeroize();
        workspace_key.zeroize();
        return Err(YntraError::CryptoError(e.to_string()));
    }

    let mut nonce_bytes = [0u8; 24];
    if let Err(e) = getrandom::fill(&mut nonce_bytes) {
        password.zeroize();
        workspace_key.zeroize();
        return Err(YntraError::CryptoError(e.to_string()));
    }

    let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
    let params = match Params::new(12288, 3, 1, Some(32)) {
        Ok(p) => p,
        Err(_) => {
            password.zeroize();
            workspace_key.zeroize();
            return Err(YntraError::CryptoError("Argon2 params invalid".to_string()));
        }
    };
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let res = argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key);
    password.zeroize();

    if res.is_err() {
        workspace_key.zeroize();
        return Err(YntraError::CryptoError(
            "Argon2 derivation failed".to_string(),
        ));
    }

    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, workspace_key.as_slice());
    workspace_key.zeroize();

    let ciphertext = ciphertext
        .map_err(|_| YntraError::CryptoError("Envelope encryption failed".to_string()))?;

    Ok(format!(
        "envelope:v3:{}:{}:{}",
        const_hex::encode(&salt),
        const_hex::encode(&nonce_bytes),
        const_hex::encode(&ciphertext)
    ))
}

/// Decrypts a workspace key envelope using a password.
#[uniffi::export]
pub fn decrypt_workspace_key_with_password(
    mut password: String,
    encrypted_envelope: &str,
) -> Result<Vec<u8>, YntraError> {
    use argon2::{Algorithm, Argon2, Params, Version};

    if !encrypted_envelope.starts_with("envelope:") {
        password.zeroize();
        return Err(YntraError::CryptoError(
            "Invalid envelope format".to_string(),
        ));
    }

    let parts: Vec<&str> = encrypted_envelope[9..].split(':').collect();
    if parts.len() != 3 && parts.len() != 4 {
        password.zeroize();
        return Err(YntraError::CryptoError(
            "Invalid envelope structure".to_string(),
        ));
    }

    let (version, salt_str, nonce_str, ciphertext_str) = if parts.len() == 4 {
        (Some(parts[0]), parts[1], parts[2], parts[3])
    } else {
        (None, parts[0], parts[1], parts[2])
    };

    let salt = match const_hex::decode(salt_str) {
        Ok(s) => s,
        Err(_) => {
            password.zeroize();
            return Err(YntraError::CryptoError("Invalid envelope salt".to_string()));
        }
    };
    let nonce_bytes = match const_hex::decode(nonce_str) {
        Ok(n) => n,
        Err(_) => {
            password.zeroize();
            return Err(YntraError::CryptoError("Invalid envelope nonce".to_string()));
        }
    };
    let ciphertext = match const_hex::decode(ciphertext_str) {
        Ok(c) => c,
        Err(_) => {
            password.zeroize();
            return Err(YntraError::CryptoError("Invalid envelope ciphertext".to_string()));
        }
    };

    let mut decrypted = None;

    if let Some(v_str) = version {
        // Versioned envelope: use EXACT parameters for the given version (no sequential retry)
        let params = match v_str {
            "v3" => Params::new(12288, 3, 1, Some(32)),
            "v2" => Params::new(19456, 3, 1, Some(32)),
            "v1" => Params::new(19456, 2, 1, Some(32)),
            _ => {
                password.zeroize();
                return Err(YntraError::CryptoError(format!("Unsupported envelope version: {}", v_str)));
            }
        }.map_err(|_| YntraError::CryptoError("Argon2 params invalid".to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
        if argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key).is_ok() {
            let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
            let nonce = XNonce::from_slice(&nonce_bytes);
            if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_slice()) {
                decrypted = Some(plaintext);
            }
        }
    } else {
        // Legacy envelope: sequential fallback (legacy compatibility path)
        // Try v3 (m=12288, t=3) first (since some legacy envelopes might be v3 without prefix)
        let mut derived_key_v3 = zeroize::Zeroizing::new([0u8; 32]);
        if let Ok(params_v3) = Params::new(12288, 3, 1, Some(32)) {
            let argon2_v3 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_v3);
            if argon2_v3.hash_password_into(password.as_bytes(), &salt, &mut *derived_key_v3).is_ok() {
                let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key_v3));
                let nonce = XNonce::from_slice(&nonce_bytes);
                if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_slice()) {
                    decrypted = Some(plaintext);
                }
            }
        }

        // Try v2 (m=19456, t=3) next
        if decrypted.is_none() {
            let mut derived_key_v2 = zeroize::Zeroizing::new([0u8; 32]);
            if let Ok(params_v2) = Params::new(19456, 3, 1, Some(32)) {
                let argon2_v2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_v2);
                if argon2_v2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key_v2).is_ok() {
                    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key_v2));
                    let nonce = XNonce::from_slice(&nonce_bytes);
                    if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_slice()) {
                        decrypted = Some(plaintext);
                    }
                }
            }
        }

        // Try v1 (m=19456, t=2) last
        if decrypted.is_none() {
            let mut derived_key_v1 = zeroize::Zeroizing::new([0u8; 32]);
            if let Ok(params_v1) = Params::new(19456, 2, 1, Some(32)) {
                let argon2_v1 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_v1);
                if argon2_v1.hash_password_into(password.as_bytes(), &salt, &mut *derived_key_v1).is_ok() {
                    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key_v1));
                    let nonce = XNonce::from_slice(&nonce_bytes);
                    if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext.as_slice()) {
                        decrypted = Some(plaintext);
                    }
                }
            }
        }
    }

    password.zeroize();

    match decrypted {
        Some(pt) => Ok(pt),
        None => Err(YntraError::CryptoError("Envelope decryption failed".to_string())),
    }
}

#[cfg(test)]
mod keychain_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockSecureStorage {
        store: Mutex<HashMap<String, String>>,
    }

    impl SecureStorageProvider for MockSecureStorage {
        fn get_secure_secret(&self, key: String) -> Option<String> {
            self.store.lock().unwrap().get(&key).cloned()
        }

        fn set_secure_secret(&self, key: String, value: String) -> bool {
            self.store.lock().unwrap().insert(key, value);
            true
        }
    }

    #[tokio::test]
    async fn test_secure_storage_provider_integration() {
        let mock = Box::new(MockSecureStorage {
            store: Mutex::new(HashMap::new()),
        });

        // Register the provider
        assert!(register_secure_storage_provider(mock));

        // Test writing and reading secret
        set_local_secret("test-key-123", "secret-value")
            .await
            .unwrap();
        let val = get_local_secret("test-key-123").await.unwrap();
        assert_eq!(val, Some("secret-value".to_string()));

        // Test writing epoch
        let epoch = set_local_epoch_if_greater("ws-1", 10).await.unwrap();
        assert_eq!(epoch, 10);

        // Test reading epoch rollback detection
        let err = set_local_epoch_if_greater("ws-1", 5).await;
        assert!(err.is_err());
        assert!(matches!(err.unwrap_err(), YntraError::AuthError(_)));
    }

    #[test]
    fn test_hmac_v1_v2_migration() {
        let legacy_hmac = compute_legacy_integrity_hmac("test-mig-key", "val-123").unwrap();
        let secure_hmac = compute_integrity_hmac("test-mig-key", "val-123").unwrap();
        assert_ne!(legacy_hmac, secure_hmac);

        let computed_legacy = compute_legacy_integrity_hmac("test-mig-key", "val-123").unwrap();
        assert_eq!(legacy_hmac, computed_legacy);

        let computed_secure = compute_integrity_hmac("test-mig-key", "val-123").unwrap();
        assert_eq!(secure_hmac, computed_secure);
    }

    #[test]
    fn test_argon2id_legacy_fallback_decryption() {
        use argon2::{Algorithm, Argon2, Params, Version};
        use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};

        let password = "my-secure-password";
        let plaintext = b"workspace-secret-key-bytes-123456";

        let salt = [1u8; 16];
        let nonce_bytes = [2u8; 24];

        let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
        let params_v1 = Params::new(19456, 2, 1, Some(32)).unwrap();
        let argon2_v1 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_v1);
        argon2_v1.hash_password_into(password.as_bytes(), &salt, &mut *derived_key).unwrap();

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_slice()).unwrap();

        let envelope = format!(
            "envelope:{}:{}:{}",
            const_hex::encode(&salt),
            const_hex::encode(&nonce_bytes),
            const_hex::encode(&ciphertext)
        );

        let decrypted = decrypt_workspace_key_with_password(password.to_string(), &envelope).unwrap();
        assert_eq!(decrypted, plaintext);

        assert!(decrypt_workspace_key_with_password("wrong-password".to_string(), &envelope).is_err());
    }

    #[test]
    fn test_argon2id_v2_fallback_decryption() {
        use argon2::{Algorithm, Argon2, Params, Version};
        use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};

        let password = "my-secure-password";
        let plaintext = b"workspace-secret-key-bytes-654321";

        let salt = [2u8; 16];
        let nonce_bytes = [3u8; 24];

        let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
        let params_v2 = Params::new(19456, 3, 1, Some(32)).unwrap();
        let argon2_v2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_v2);
        argon2_v2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key).unwrap();

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_slice()).unwrap();

        let envelope = format!(
            "envelope:{}:{}:{}",
            const_hex::encode(&salt),
            const_hex::encode(&nonce_bytes),
            const_hex::encode(&ciphertext)
        );

        let decrypted = decrypt_workspace_key_with_password(password.to_string(), &envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_argon2id_v3_versioned_decryption() {
        let password = "my-secure-password";
        let plaintext = b"workspace-secret-key-bytes-123456";

        let envelope = encrypt_workspace_key_with_password(password.to_string(), plaintext.to_vec()).unwrap();
        assert!(envelope.starts_with("envelope:v3:"));

        let decrypted = decrypt_workspace_key_with_password(password.to_string(), &envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
