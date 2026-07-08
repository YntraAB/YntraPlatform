use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use std::sync::{Mutex, OnceLock, RwLock};
use std::collections::HashMap;
use zeroize::Zeroize;
use ed25519_dalek::{Signer, Verifier};

#[derive(Clone, Zeroize)]
#[zeroize(drop)]
struct SessionKeys {
    new_key: [u8; 32],
}

static SESSION_KEY: Mutex<Option<SessionKeys>> = Mutex::new(None);
static SYSTEM_SALT: OnceLock<zeroize::Zeroizing<Vec<u8>>> = OnceLock::new();

static AUTH_KEY_CACHE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
static AUTH_EPOCH_CACHE: OnceLock<RwLock<HashMap<String, u64>>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static KEYRING_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();


pub fn get_auth_key_cache() -> &'static RwLock<HashMap<String, String>> {
    AUTH_KEY_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get_auth_epoch_cache() -> &'static RwLock<HashMap<String, u64>> {
    AUTH_EPOCH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
fn get_keyring_lock() -> &'static Mutex<()> {
    KEYRING_MUTEX.get_or_init(|| Mutex::new(()))
}


fn bytes_to_string(bytes: Vec<u8>) -> Result<String, crate::infra::errors::YntraError> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => {
            let mut raw = e.into_bytes();
            raw.zeroize();
            Err(crate::infra::errors::YntraError::CryptoError("invalid_utf8".to_string()))
        }
    }
}pub fn stretch_key_new(key: &[u8]) -> Result<[u8; 32], crate::infra::errors::YntraError> {
    let system_salt = get_system_salt_ref()?;
    
    // Secure 32-byte key derivation using BLAKE3 KDF
    let mut hasher = blake3::Hasher::new_derive_key("Yntra key stretching v1");
    hasher.update(&(system_salt.len() as u64).to_be_bytes());
    hasher.update(system_salt);
    hasher.update(&(key.len() as u64).to_be_bytes());
    hasher.update(key);
    
    let mut derived = [0u8; 32];
    hasher.finalize_xof().fill(&mut derived);
    hasher.zeroize();
    Ok(derived)
}

#[uniffi::export]
pub fn initialize_system_salt(mut salt: String) -> bool {
    let bytes = match const_hex::decode(&salt) {
        Ok(b) => {
            salt.zeroize();
            b
        }
        Err(_) => {
            let b = salt.clone().into_bytes();
            salt.zeroize();
            b
        }
    };
    
    match SYSTEM_SALT.set(zeroize::Zeroizing::new(bytes)) {
        Ok(_) => true,
        Err(mut rejected_salt) => {
            rejected_salt.zeroize();
            false
        }
    }
}

fn get_system_salt_ref() -> Result<&'static [u8], crate::infra::errors::YntraError> {
    if let Some(salt) = SYSTEM_SALT.get() {
        return Ok(&salt[..]);
    }
    ensure_system_salt_initialized()?;
    SYSTEM_SALT.get()
        .map(|s| &s[..])
        .ok_or_else(|| crate::infra::errors::YntraError::CryptoError("system_salt_uninitialized".to_string()))
}

fn ensure_system_salt_initialized() -> Result<(), crate::infra::errors::YntraError> {
    if SYSTEM_SALT.get().is_some() {
        return Ok(());
    }
    
    let mut salt_buf = String::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(mut salt) = std::env::var("YNTRA_ENCRYPTION_SALT") {
            salt_buf.push_str(&salt);
            salt.zeroize();
        }
    }
    
    if cfg!(test) {
        let mut rand_bytes = [0u8; 32];
        let res = getrandom::fill(&mut rand_bytes);
        if res.is_ok() {
            salt_buf.push_str(&const_hex::encode(&rand_bytes));
        }
        rand_bytes.zeroize();
        if res.is_err() {
            salt_buf.zeroize();
            return Err(crate::infra::errors::YntraError::CryptoError("Failed to generate secure random salt for tests".to_string()));
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if salt_buf.is_empty() {
                // Eagerly trigger database initialization to load salt from database
                let _ = crate::database::native::get_database();
                if SYSTEM_SALT.get().is_some() {
                    salt_buf.zeroize();
                    return Ok(());
                }
            }
        }
        
        if salt_buf.is_empty() {
            salt_buf.zeroize();
            return Err(crate::infra::errors::YntraError::CryptoError("Cryptographic system salt was not initialized. Database setup must run first.".to_string()));
        }
    }
    
    let bytes = match const_hex::decode(&salt_buf) {
        Ok(b) => {
            salt_buf.zeroize();
            b
        }
        Err(_) => {
            let b = salt_buf.clone().into_bytes();
            salt_buf.zeroize();
            b
        }
    };
    
    match SYSTEM_SALT.set(zeroize::Zeroizing::new(bytes)) {
        Ok(_) => {}
        Err(mut rejected) => {
            rejected.zeroize();
        }
    }
    Ok(())
}

#[uniffi::export]
pub fn set_session_key(mut key_bytes: Vec<u8>) -> bool {
    let mut lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let mut inner = poisoned.into_inner();
            *inner = None;
            inner
        }
    };
    if let Some(mut old_sk) = lock.take() {
        old_sk.zeroize();
    }
    
    let mut key_arr = [0u8; 32];
    if key_bytes.len() == 32 {
        key_arr.copy_from_slice(&key_bytes);
    } else {
        let hash = blake3::hash(&key_bytes);
        key_arr.copy_from_slice(hash.as_bytes());
    }
    key_bytes.zeroize();
    
    *lock = Some(SessionKeys {
        new_key: key_arr,
    });
    true
}

pub fn get_session_key() -> Option<Vec<u8>> {
    let lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    lock.as_ref().map(|sk| sk.new_key.to_vec())
}

#[uniffi::export]
pub fn is_session_key_set() -> bool {
    let lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    lock.is_some()
}

#[uniffi::export]
pub async fn load_local_workspace_key(workspace_id: String) -> bool {
    let key_name = format!("workspace_key_{}", workspace_id);
    if let Ok(Some(key_hex)) = get_local_secret(&key_name).await {
        if let Ok(key_bytes) = const_hex::decode(&key_hex) {
            return set_session_key(key_bytes);
        }
    }
    false
}

#[uniffi::export]
pub fn clear_session_key() {
    let mut lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let mut inner = poisoned.into_inner();
            *inner = None;
            inner
        }
    };
    if let Some(mut old_sk) = lock.take() {
        old_sk.zeroize();
    }
}

fn get_encryption_keys_internal(
    workspace_id: &str,
) -> Result<zeroize::Zeroizing<[u8; 32]>, crate::infra::errors::YntraError> {
    // 1. Initialize Hasher with domain separation context for key derivation
    let mut hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key derivation v2");

    // 2. Feed workspace_id with length prefix to prevent input canonicalization / collision attacks
    hasher.update(&(workspace_id.len() as u64).to_be_bytes());
    hasher.update(workspace_id.as_bytes());

    // 3. Retrieve and feed salt with length prefix
    let salt = match get_system_salt_ref() {
        Ok(s) => s,
        Err(e) => {
            hasher.zeroize();
            return Err(e);
        }
    };
    hasher.update(&(salt.len() as u64).to_be_bytes());
    hasher.update(salt);

    // 4. Retrieve and verify session key is present (minimize mutex critical section)
    let mut session_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    {
        let mut is_poisoned = false;
        let lock = match SESSION_KEY.lock() {
            Ok(l) => l,
            Err(poisoned) => {
                is_poisoned = true;
                poisoned.into_inner()
            }
        };
        if lock.is_none() && is_poisoned {
            hasher.zeroize();
            return Err(crate::infra::errors::YntraError::CryptoError("session_key_lock_poisoned".to_string()));
        }
        
        if let Some(ref sk) = *lock {
            session_key_bytes.copy_from_slice(&sk.new_key);
        } else {
            hasher.zeroize();
            return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string()));
        }
    }
    hasher.update(&*session_key_bytes);

    // 5. Derive key using single-pass Blake3 XOF directly into Zeroizing
    let mut reader = hasher.finalize_xof();
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    reader.fill(&mut *key);

    hasher.zeroize();
    Ok(key)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const_hex::encode(bytes)
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    const_hex::decode(s).ok()
}

#[uniffi::export]
pub fn encrypt_field(data: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.encrypt(data)
}

#[uniffi::export]
pub fn decrypt_field(encrypted_data: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.decrypt(encrypted_data)
}

pub fn encrypt_opt_field(data: Option<String>, workspace_id: &str) -> Result<Option<String>, crate::infra::errors::YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.encrypt_opt(data)
}pub fn decrypt_opt_field(encrypted_data: Option<String>, workspace_id: &str) -> Option<String> {
    encrypted_data.and_then(|d| {
        match WorkspaceCipher::new(workspace_id) {
            Ok(cipher) => match cipher.decrypt(&d) {
                Ok(pt) => Some(pt),
                Err(e) => {
                    tracing::error!("Failed to decrypt optional field in workspace '{}': {:?}", workspace_id, e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("Failed to initialize WorkspaceCipher for optional field decryption in workspace '{}': {:?}", workspace_id, e);
                None
            }
        }
    })
}

fn get_local_client_pepper() -> String {
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

pub fn hash_anonymous_reporter(user_id: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    let salt = get_system_salt_ref()?;
    let client_pepper = get_local_client_pepper();
    
    let mut hasher = blake3::Hasher::new_derive_key("Yntra whistleblower reporter anonymity hash v2");
    hasher.update(&(salt.len() as u64).to_be_bytes());
    hasher.update(salt);
    
    hasher.update(&(workspace_id.len() as u64).to_be_bytes());
    hasher.update(workspace_id.as_bytes());
    
    hasher.update(&(user_id.len() as u64).to_be_bytes());
    hasher.update(user_id.as_bytes());
    
    hasher.update(&(client_pepper.len() as u64).to_be_bytes());
    hasher.update(client_pepper.as_bytes());
    
    let mut output = [0u8; 32];
    hasher.finalize_xof().fill(&mut output);
    hasher.zeroize();
    
    Ok(format!("anon_hash:{}", hex_encode(&output)))
}


pub struct WorkspaceCipher {
    session_key: Option<zeroize::Zeroizing<[u8; 32]>>,
}

impl WorkspaceCipher {
    pub fn new(workspace_id: &str) -> Result<Self, crate::infra::errors::YntraError> {
        let key_res = get_encryption_keys_internal(workspace_id);
        let session_key = match key_res {
            Ok(k) => Some(k),
            Err(crate::infra::errors::YntraError::CryptoError(ref msg)) if msg == "session_key_missing" => {
                None
            }
            Err(e) => return Err(e),
        };
        
        Ok(Self { session_key })
    }

    pub fn encrypt(&self, data: &str) -> Result<String, crate::infra::errors::YntraError> {
        let key_bytes = match &self.session_key {
            Some(k) => k,
            None => return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string())),
        };
        
        let mut nonce_bytes = [0u8; 24];
        getrandom::fill(&mut nonce_bytes).map_err(|e| {
            tracing::error!("Failed to generate random nonce: {:?}", e);
            crate::infra::errors::YntraError::CryptoError("Failed to generate random nonce".to_string())
        })?;
        let zeroizing_nonce = zeroize::Zeroizing::new(nonce_bytes);
        
        let key = Key::from_slice(&key_bytes[..]);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(&zeroizing_nonce[..]);
        
        let result = if let Ok(ct) = cipher.encrypt(nonce, data.as_bytes()) {
            Ok(format!("enc:{}:{}", hex_encode(&zeroizing_nonce[..]), hex_encode(&ct)))
        } else {
            tracing::error!("XChaCha20Poly1305 encryption failed");
            Err(crate::infra::errors::YntraError::CryptoError("encryption_failed".to_string()))
        };
        
        result
    }

    pub fn decrypt(&self, encrypted_data: &str) -> Result<String, crate::infra::errors::YntraError> {
        if !encrypted_data.starts_with("enc:") {
            return Err(crate::infra::errors::YntraError::CryptoError("not_encrypted".to_string()));
        }
        
        let body = &encrypted_data[4..];
        
        let mut parts = body.splitn(3, ':');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(p1), Some(p2), None) => {
                let mut nonce_bytes = [0u8; 24];
                if const_hex::decode_to_slice(p1, &mut nonce_bytes).is_err() {
                    return Err(crate::infra::errors::YntraError::CryptoError("invalid_nonce".to_string()));
                }
                let zeroizing_nonce = zeroize::Zeroizing::new(nonce_bytes);
                
                let ct = match hex_decode(p2) {
                    Some(b) => b,
                    None => {
                        return Err(crate::infra::errors::YntraError::CryptoError("invalid_ciphertext".to_string()));
                    }
                };
                
                let key_bytes = match &self.session_key {
                    Some(k) => k,
                    None => {
                        return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string()));
                    }
                };
                
                let key = Key::from_slice(&key_bytes[..]);
                let cipher = XChaCha20Poly1305::new(key);
                let nonce = XNonce::from_slice(&zeroizing_nonce[..]);
                
                let pt = cipher.decrypt(nonce, ct.as_slice())
                    .map_err(|_| crate::infra::errors::YntraError::CryptoError("decryption_failed".to_string()))?;
                
                bytes_to_string(pt)
            }
            _ => Err(crate::infra::errors::YntraError::CryptoError("invalid_format".to_string())),
        }
    }

    pub fn encrypt_opt(&self, data: Option<String>) -> Result<Option<String>, crate::infra::errors::YntraError> {
        match data {
            Some(mut d) => {
                let res = self.encrypt(&d).map(Some);
                d.zeroize();
                res
            }
            None => Ok(None),
        }
    }

    pub fn decrypt_opt(&self, encrypted_data: Option<String>) -> Option<String> {
        encrypted_data.and_then(|d| {
            self.decrypt(&d).ok()
        })
    }
}

impl Zeroize for WorkspaceCipher {
    fn zeroize(&mut self) {
        self.session_key.zeroize();
    }
}

impl Drop for WorkspaceCipher {
    fn drop(&mut self) {
        self.zeroize();
    }
}

fn construct_role_signature_message(user_id: &str, role: &str, workspace_id: &str, expires_at: i64, epoch: u64) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(b"YNTRA_ROLE_SIGNATURE_V3\0");
    message.extend_from_slice(&(user_id.len() as u64).to_be_bytes());
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(&(role.len() as u64).to_be_bytes());
    message.extend_from_slice(role.as_bytes());
    message.extend_from_slice(&(workspace_id.len() as u64).to_be_bytes());
    message.extend_from_slice(workspace_id.as_bytes());
    message.extend_from_slice(&expires_at.to_be_bytes());
    message.extend_from_slice(&epoch.to_be_bytes()); // Always include epoch (rigid schema)
    message
}

#[uniffi::export]
pub fn derive_public_key_from_private_key(private_key_hex: &str) -> Result<String, crate::infra::errors::YntraError> {
    let private_key_bytes = zeroize::Zeroizing::new(
        const_hex::decode(private_key_hex)
            .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
    );
    if private_key_bytes.len() != 32 {
        return Err(crate::infra::errors::YntraError::CryptoError("Invalid private key length".to_string()));
    }
    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
    Ok(const_hex::encode(signing_key.verifying_key().to_bytes()))
}

#[uniffi::export]
pub fn generate_role_signature(private_key_hex: &str, user_id: &str, role: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    // Default expiration: 30 days
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
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
) -> Result<String, crate::infra::errors::YntraError> {
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
) -> Result<String, crate::infra::errors::YntraError> {
    let private_key_bytes = zeroize::Zeroizing::new(
        const_hex::decode(private_key_hex)
            .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
    );
    
    let mut private_key_array = zeroize::Zeroizing::new([0u8; 32]);
    if private_key_bytes.len() != 32 {
        return Err(crate::infra::errors::YntraError::CryptoError("Invalid private key length".to_string()));
    }
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
        
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_array);
    let message = construct_role_signature_message(user_id, role, workspace_id, expires_at, epoch);
    let signature = signing_key.sign(&message);
    let signature_hex = const_hex::encode(&signature.to_bytes());
    Ok(format!("{}:{}:{}", epoch, expires_at, signature_hex))
}

#[uniffi::export]
pub fn generate_workspace_keypair() -> Result<Vec<String>, crate::infra::errors::YntraError> {
    let mut private_key_bytes = [0u8; 32];
    getrandom::fill(&mut private_key_bytes).map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let public_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());
    let private_key_hex = const_hex::encode(private_key_bytes);
    Ok(vec![public_key_hex, private_key_hex])
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_local_secret(key: &str) -> Result<Option<String>, crate::infra::errors::YntraError> {
    let key = key.to_string();
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        match entry.get_password() {
            Ok(secret) => {
                if secret.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(secret))
                }
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(crate::infra::errors::YntraError::CryptoError(format!("Keyring access failed: {:?}", e))),
        }
    })
    .await
    .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), crate::infra::errors::YntraError> {
    let key = key.to_string();
    let value = value.to_string();
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        if value.is_empty() {
            let _ = entry.delete_credential();
            Ok(())
        } else {
            if let Err(e) = entry.set_password(&value) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!("Keyring write failed in test/CI environment (swallowing): {:?}", e);
                    Ok(())
                } else {
                    Err(crate::infra::errors::YntraError::CryptoError(format!("Failed to store secret in keyring: {:?}", e)))
                }
            } else {
                Ok(())
            }
        }
    })
    .await
    .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_epoch_if_greater(ws_id: &str, current_epoch: u64) -> Result<u64, crate::infra::errors::YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    tokio::task::spawn_blocking(move || {
        let _lock = get_keyring_lock().lock().unwrap_or_else(|e| e.into_inner());
        let entry = keyring::Entry::new("yntra-platform", &key)
            .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("Failed to access keyring: {:?}", e)))?;
        
        let cached_epoch = if let Ok(val) = entry.get_password() {
            val.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        if current_epoch < cached_epoch {
            return Err(crate::infra::errors::YntraError::AuthError(
                "Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()
            ));
        }
        
        if current_epoch > cached_epoch {
            if let Err(e) = entry.set_password(&current_epoch.to_string()) {
                if cfg!(test) || std::env::var("CI").is_ok() {
                    tracing::warn!("Keyring write failed in test/CI environment (swallowing): {:?}", e);
                    Ok(current_epoch)
                } else {
                    Err(crate::infra::errors::YntraError::CryptoError(format!("Failed to store epoch in keyring: {:?}", e)))
                }
            } else {
                Ok(current_epoch)
            }
        } else {
            Ok(cached_epoch)
        }
    })
    .await
    .map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?
}

#[cfg(target_arch = "wasm32")]
fn compute_integrity_hmac(key: &str, value: &str) -> Result<String, crate::infra::errors::YntraError> {
    let salt = get_system_salt_ref()?;
    
    // Derive a dedicated integrity HMAC key from the system salt
    let mut hasher = blake3::Hasher::new_derive_key("Yntra Local Storage Integrity v1");
    hasher.update(salt);
    let mut hmac_key = [0u8; 32];
    hasher.finalize_xof().fill(&mut hmac_key);
    hasher.zeroize();

    // Compute BLAKE3 keyed hash (acts as a secure MAC / HMAC)
    let mut keyed_hasher = blake3::Hasher::new_keyed(&hmac_key);
    keyed_hasher.update(key.as_bytes());
    keyed_hasher.update(value.as_bytes());
    let hash = keyed_hasher.finalize();
    
    Ok(const_hex::encode(hash.as_bytes()))
}

#[cfg(target_arch = "wasm32")]
pub async fn get_local_secret(key: &str) -> Result<Option<String>, crate::infra::errors::YntraError> {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let secret = storage.get_item(key)
                .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
            if let Some(ref val) = secret {
                let hmac_key_name = format!("{}_integrity", key);
                let stored_hmac = storage.get_item(&hmac_key_name)
                    .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(key, val)?;
                    if hmac != &computed {
                        return Err(crate::infra::errors::YntraError::AuthError("Local storage tampering detected".to_string()));
                    }
                } else {
                    return Err(crate::infra::errors::YntraError::AuthError("Local storage integrity verification missing".to_string()));
                }
            }
            return Ok(secret);
        }
    }
    Err(crate::infra::errors::YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), crate::infra::errors::YntraError> {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let hmac_key_name = format!("{}_integrity", key);
            if value.is_empty() {
                let _ = storage.remove_item(key);
                let _ = storage.remove_item(&hmac_key_name);
            } else {
                let hmac = compute_integrity_hmac(key, value)?;
                storage.set_item(key, value).map_err(|e| {
                    crate::infra::errors::YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                storage.set_item(&hmac_key_name, &hmac).map_err(|e| {
                    crate::infra::errors::YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
            }
            return Ok(());
        }
    }
    Err(crate::infra::errors::YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_epoch_if_greater(ws_id: &str, current_epoch: u64) -> Result<u64, crate::infra::errors::YntraError> {
    let key = format!("workspace_auth_epoch_{}", ws_id);
    let hmac_key_name = format!("{}_integrity", key);
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let cached_epoch = if let Ok(Some(val)) = storage.get_item(&key) {
                // Verify integrity of the existing value
                let stored_hmac = storage.get_item(&hmac_key_name)
                    .map_err(|e| crate::infra::errors::YntraError::CryptoError(format!("LocalStorage read failed: {:?}", e)))?;
                if let Some(ref hmac) = stored_hmac {
                    let computed = compute_integrity_hmac(&key, &val)?;
                    if hmac != &computed {
                        return Err(crate::infra::errors::YntraError::AuthError("Local storage tampering detected".to_string()));
                    }
                } else {
                    return Err(crate::infra::errors::YntraError::AuthError("Local storage integrity verification missing".to_string()));
                }
                val.parse::<u64>().unwrap_or(0)
            } else {
                0
            };
            
            if current_epoch < cached_epoch {
                return Err(crate::infra::errors::YntraError::AuthError(
                    "Workspace auth epoch rollback detected. Local database tampering suspected.".to_string()
                ));
            }
            
            if current_epoch > cached_epoch {
                let epoch_str = current_epoch.to_string();
                let hmac = compute_integrity_hmac(&key, &epoch_str)?;
                storage.set_item(&key, &epoch_str).map_err(|e| {
                    crate::infra::errors::YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                storage.set_item(&hmac_key_name, &hmac).map_err(|e| {
                    crate::infra::errors::YntraError::CryptoError(format!("LocalStorage set failed: {:?}", e))
                })?;
                return Ok(current_epoch);
            } else {
                return Ok(cached_epoch);
            }
        }
    }
    Err(crate::infra::errors::YntraError::CryptoError("LocalStorage not available".to_string()))
}

#[uniffi::export]
pub fn encrypt_workspace_key_with_password(password: &str, mut workspace_key: Vec<u8>) -> Result<String, crate::infra::errors::YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?;
    
    let mut nonce_bytes = [0u8; 24];
    getrandom::fill(&mut nonce_bytes).map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?;
    
    let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    let res = argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key);
    if res.is_err() {
        workspace_key.zeroize();
        return Err(crate::infra::errors::YntraError::CryptoError("Argon2 derivation failed".to_string()));
    }
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, workspace_key.as_slice());
    workspace_key.zeroize();
    
    let ciphertext = ciphertext.map_err(|_| crate::infra::errors::YntraError::CryptoError("Envelope encryption failed".to_string()))?;
        
    Ok(format!(
        "envelope:{}:{}:{}",
        const_hex::encode(&salt),
        const_hex::encode(&nonce_bytes),
        const_hex::encode(&ciphertext)
    ))
}

#[uniffi::export]
pub fn decrypt_workspace_key_with_password(password: &str, encrypted_envelope: &str) -> Result<Vec<u8>, crate::infra::errors::YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    if !encrypted_envelope.starts_with("envelope:") {
        return Err(crate::infra::errors::YntraError::CryptoError("Invalid envelope format".to_string()));
    }
    
    let parts: Vec<&str> = encrypted_envelope[9..].split(':').collect();
    if parts.len() != 3 {
        return Err(crate::infra::errors::YntraError::CryptoError("Invalid envelope structure".to_string()));
    }
    
    let salt = const_hex::decode(parts[0])
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Invalid envelope salt".to_string()))?;
    let nonce_bytes = const_hex::decode(parts[1])
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Invalid envelope nonce".to_string()))?;
    let ciphertext = const_hex::decode(parts[2])
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Invalid envelope ciphertext".to_string()))?;
        
    let mut derived_key = zeroize::Zeroizing::new([0u8; 32]);
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2.hash_password_into(password.as_bytes(), &salt, &mut *derived_key)
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 derivation failed".to_string()))?;
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Envelope decryption failed".to_string()))?;
        
    Ok(plaintext)
}




#[uniffi::export]
pub fn verify_role_signature(public_key_hex: &str, user_id: &str, role: &str, workspace_id: &str, signature_hex: &str) -> bool {
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
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
        
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chacha_encryption_decryption() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        set_session_key("test-session-key".to_string().into_bytes());
        
        let plaintext = "Sensitive whistleblowing report text";
        let workspace_id = "test-workspace-123";
        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();
        assert!(encrypted.starts_with("enc:"));
        
        // Assert that the encrypted format contains two colon-separated hex strings
        let body = &encrypted[4..];
        let parts: Vec<&str> = body.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 48); // 24-byte hex nonce is 48 characters
        
        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);
        
        clear_session_key();
    }

    #[test]
    fn test_strong_session_key_derivation() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let plaintext = "Highly sensitive user data";
        let workspace_id = "test-workspace-456";

        // Set session key
        set_session_key("my-super-secret-user-password-or-pin".to_string().into_bytes());

        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();
        
        // Decrypt with correct session key set
        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);

        // Temporarily clear session key and ensure decryption falls back or fails gracefully
        clear_session_key();
        let decrypted_without_key_res = decrypt_field(&encrypted, workspace_id);
        assert!(decrypted_without_key_res.is_err());

        // Reset session key and ensure it works again
        set_session_key("my-super-secret-user-password-or-pin".to_string().into_bytes());
        let decrypted_with_key_again = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted_with_key_again);

        clear_session_key();
    }

    #[test]
    fn test_session_key_poisoning_recovery() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        clear_session_key();
        // Poison the mutex by panicking while holding the lock
        let _ = std::panic::catch_unwind(|| {
            let _lock = SESSION_KEY.lock().unwrap();
            panic!("poisoning lock");
        });

        // Mutex is now poisoned. Check that get_encryption_keys_internal returns the expected error
        let res = get_encryption_keys_internal("some-workspace");
        assert!(res.is_err());
        if let Err(crate::infra::errors::YntraError::CryptoError(msg)) = res {
            assert_eq!(msg, "session_key_lock_poisoned");
        } else {
            panic!("Expected CryptoError(session_key_lock_poisoned)");
        }

        // Recover by calling set_session_key (which resets the poisoned state)
        assert!(set_session_key("my-new-session-key".to_string().into_bytes()));

        // Check lock is usable again and holds the stretched key
        let lock = match SESSION_KEY.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        assert!(lock.is_some());
    }

    #[test]
    fn test_system_salt_duplicate_initialization() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let initial_salt = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string();
        
        // This may succeed or fail depending on whether it's already set by another test/startup.
        let _ = initialize_system_salt(initial_salt);
        
        // Subsequent initialization must fail
        let duplicate_salt = "303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f".to_string();
        let second_res = initialize_system_salt(duplicate_salt);
        assert!(!second_res); // Should return false
    }

    #[test]
    fn test_collaborative_workspace_key_match() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let plaintext = "Shared patient health data";
        let workspace_id = "shared-workspace-xyz";

        // User A logs in and encrypts data
        set_session_key("shared-workspace-session-key".to_string().into_bytes());
        let encrypted_by_a = encrypt_field(plaintext, workspace_id).unwrap();
        clear_session_key();

        // User B attempts to log in with a DIFFERENT session key and decrypt (should fail)
        set_session_key("different-workspace-session-key".to_string().into_bytes());
        let decrypted_by_b_wrong = decrypt_field(&encrypted_by_a, workspace_id);
        assert!(decrypted_by_b_wrong.is_err());
        clear_session_key();

        // User B logs in with the CORRECT shared workspace session key (should succeed)
        set_session_key("shared-workspace-session-key".to_string().into_bytes());
        let decrypted_by_b = decrypt_field(&encrypted_by_a, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted_by_b);

        clear_session_key();
    }
}
