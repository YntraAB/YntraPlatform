pub mod domain;
pub mod keychain;
pub mod signing;

pub use domain::*;
pub use keychain::*;
pub use signing::*;

use crate::infra::errors::YntraError;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};
use zeroize::Zeroize;

struct SessionKeys {
    new_key: [u8; 32],
    workspace_id: String,
}

impl Zeroize for SessionKeys {
    fn zeroize(&mut self) {
        self.new_key.zeroize();
        self.workspace_id.zeroize();
    }
}

static SESSION_KEY: Mutex<Option<SessionKeys>> = Mutex::new(None);
static SESSION_KEY_IS_POISONED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct PoisonGuard;
impl Drop for PoisonGuard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            SESSION_KEY_IS_POISONED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
static SYSTEM_SALT: OnceLock<zeroize::Zeroizing<Vec<u8>>> = OnceLock::new();

static AUTH_KEY_CACHE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
static AUTH_EPOCH_CACHE: OnceLock<RwLock<HashMap<String, u64>>> = OnceLock::new();
static WORKSPACE_KEY_CACHE: OnceLock<RwLock<HashMap<String, zeroize::Zeroizing<[u8; 32]>>>> =
    OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static KEYRING_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

pub fn get_auth_key_cache() -> &'static RwLock<HashMap<String, String>> {
    AUTH_KEY_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get_auth_epoch_cache() -> &'static RwLock<HashMap<String, u64>> {
    AUTH_EPOCH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_workspace_key_cache() -> &'static RwLock<HashMap<String, zeroize::Zeroizing<[u8; 32]>>> {
    WORKSPACE_KEY_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get_keyring_lock() -> &'static Mutex<()> {
    KEYRING_MUTEX.get_or_init(|| Mutex::new(()))
}

fn bytes_to_string(bytes: Vec<u8>) -> Result<String, YntraError> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => {
            let mut raw = e.into_bytes();
            raw.zeroize();
            Err(YntraError::CryptoError("invalid_utf8".to_string()))
        }
    }
}

pub fn stretch_key_new(key: &[u8]) -> Result<[u8; 32], YntraError> {
    let system_salt = get_system_salt_ref()?;

    // Secure 32-byte key derivation using BLAKE3 KDF
    let context_str = CryptoDomain::KeyStretching.get_context(1)?;
    let mut hasher = blake3::Hasher::new_derive_key(context_str);
    hasher.update(&(system_salt.len() as u64).to_be_bytes());
    hasher.update(system_salt);
    hasher.update(&(key.len() as u64).to_be_bytes());
    hasher.update(key);

    let mut derived = [0u8; 32];
    hasher.finalize_xof().fill(&mut derived);
    hasher.zeroize();
    Ok(derived)
}

/// Initializes the global system salt. Returns false if already initialized.
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

pub(crate) fn get_system_salt_ref() -> Result<&'static [u8], YntraError> {
    if let Some(salt) = SYSTEM_SALT.get() {
        return Ok(&salt[..]);
    }
    ensure_system_salt_initialized()?;
    SYSTEM_SALT
        .get()
        .map(|s| &s[..])
        .ok_or_else(|| YntraError::CryptoError("system_salt_uninitialized".to_string()))
}

fn ensure_system_salt_initialized() -> Result<(), YntraError> {
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
            return Err(YntraError::CryptoError(
                "Failed to generate secure random salt for tests".to_string(),
            ));
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if salt_buf.is_empty() {
                let _ = crate::database::native::get_database();
                if SYSTEM_SALT.get().is_some() {
                    salt_buf.zeroize();
                    return Ok(());
                }
            }
        }

        if salt_buf.is_empty() {
            salt_buf.zeroize();
            return Err(YntraError::CryptoError(
                "Cryptographic system salt was not initialized. Database setup must run first."
                    .to_string(),
            ));
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

/// Sets the active workspace session key and workspace ID.
#[uniffi::export]
pub fn set_session_key(mut key_bytes: Vec<u8>, workspace_id: String) -> bool {
    let mut lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => {
            let mut inner = poisoned.into_inner();
            *inner = None;
            inner
        }
    };
    let _guard = PoisonGuard;
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

    *lock = Some(SessionKeys { new_key: key_arr, workspace_id });

    if let Ok(mut cache) = get_workspace_key_cache().write() {
        cache.clear();
    }

    SESSION_KEY_IS_POISONED.store(false, std::sync::atomic::Ordering::SeqCst);
    true
}

pub fn get_session_key() -> Option<zeroize::Zeroizing<[u8; 32]>> {
    let lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    let _guard = PoisonGuard;
    if SESSION_KEY_IS_POISONED.load(std::sync::atomic::Ordering::SeqCst) {
        return None;
    }
    lock.as_ref().map(|sk| zeroize::Zeroizing::new(sk.new_key))
}

/// Checks if a session key is currently loaded.
#[uniffi::export]
pub fn is_session_key_set() -> bool {
    let lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    let _guard = PoisonGuard;
    if SESSION_KEY_IS_POISONED.load(std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    lock.is_some()
}

/// Authenticates a user and loads the workspace key from secure storage if successful.
#[uniffi::export]
pub async fn load_local_workspace_key(workspace_id: String, user_id: String) -> bool {
    if let Ok(conn) = crate::database::acquire_connection().await {
        if let Ok(auth) = crate::infra::auth::AuthContext::authorize(&conn, &user_id).await {
            if auth.workspace_id != workspace_id {
                tracing::error!("AuthContext workspace mismatch for user '{}'", user_id);
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    let key_name = format!("workspace_key_{}", workspace_id);
    if let Ok(Some(mut key_hex)) = get_local_secret(&key_name).await {
        if let Ok(key_bytes) = const_hex::decode(&key_hex) {
            let res = set_session_key(key_bytes, workspace_id);
            key_hex.zeroize();
            return res;
        }
        key_hex.zeroize();
    }
    false
}

/// Clears the active session key and zeroizes cached workspace keys.
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
    let _guard = PoisonGuard;
    if let Some(mut old_sk) = lock.take() {
        old_sk.zeroize();
    }

    if let Ok(mut cache) = get_workspace_key_cache().write() {
        cache.clear();
    }
    SESSION_KEY_IS_POISONED.store(false, std::sync::atomic::Ordering::SeqCst);
}

fn get_encryption_keys_internal(
    workspace_id: &str,
) -> Result<zeroize::Zeroizing<[u8; 32]>, YntraError> {
    if let Ok(cache) = get_workspace_key_cache().read() {
        if let Some(cached_key) = cache.get(workspace_id) {
            return Ok(cached_key.clone());
        }
    }

    let context_str = CryptoDomain::WhistleblowerKeyDerivation.get_context(2)?;
    let mut hasher = blake3::Hasher::new_derive_key(context_str);

    hasher.update(&(workspace_id.len() as u64).to_be_bytes());
    hasher.update(workspace_id.as_bytes());

    let salt = match get_system_salt_ref() {
        Ok(s) => s,
        Err(e) => {
            hasher.zeroize();
            return Err(e);
        }
    };
    hasher.update(&(salt.len() as u64).to_be_bytes());
    hasher.update(salt);

    let mut session_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    {
        let lock = match SESSION_KEY.lock() {
            Ok(l) => l,
            Err(poisoned) => poisoned.into_inner(),
        };
        let _guard = PoisonGuard;
        if SESSION_KEY_IS_POISONED.load(std::sync::atomic::Ordering::SeqCst) {
            hasher.zeroize();
            return Err(YntraError::CryptoError(
                "session_key_lock_poisoned".to_string(),
            ));
        }

        if let Some(ref sk) = *lock {
            if sk.workspace_id != workspace_id {
                hasher.zeroize();
                return Err(YntraError::CryptoError("session_key_workspace_mismatch".to_string()));
            }
            session_key_bytes.copy_from_slice(&sk.new_key);
        } else {
            hasher.zeroize();
            return Err(YntraError::CryptoError("session_key_missing".to_string()));
        }
    }
    hasher.update(&*session_key_bytes);

    let mut reader = hasher.finalize_xof();
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    reader.fill(&mut *key);

    hasher.zeroize();

    if let Ok(mut cache) = get_workspace_key_cache().write() {
        cache.insert(workspace_id.to_string(), key.clone());
    }

    Ok(key)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const_hex::encode(bytes)
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    const_hex::decode(s).ok()
}

/// Encrypts a single string field for the given workspace.
#[uniffi::export]
pub fn encrypt_field(data: &str, workspace_id: &str) -> Result<String, YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.encrypt(data).map_err(|e| {
        if let YntraError::CryptoError(ref msg) = e {
            if msg == "session_key_missing" {
                return YntraError::CryptoError("Session key is missing. Please unlock the workspace.".to_string());
            }
        }
        e
    })
}

/// Decrypts a single string field for the given workspace.
#[uniffi::export]
pub fn decrypt_field(encrypted_data: &str, workspace_id: &str) -> Result<String, YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.decrypt(encrypted_data).map_err(|e| {
        if let YntraError::CryptoError(ref msg) = e {
            if msg == "session_key_missing" {
                return YntraError::CryptoError("Session key is missing. Please unlock the workspace.".to_string());
            }
        }
        e
    })
}

/// Encrypts a list of string fields for the given workspace.
#[uniffi::export]
pub fn encrypt_fields(data: Vec<String>, workspace_id: &str) -> Result<Vec<String>, YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    let mut encrypted = Vec::with_capacity(data.len());
    for item in data {
        encrypted.push(cipher.encrypt(&item)?);
    }
    Ok(encrypted)
}

/// Decrypts a list of string fields for the given workspace.
#[uniffi::export]
pub fn decrypt_fields(
    encrypted_data: Vec<String>,
    workspace_id: &str,
) -> Result<Vec<String>, YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    let mut decrypted = Vec::with_capacity(encrypted_data.len());
    for item in encrypted_data {
        decrypted.push(cipher.decrypt(&item)?);
    }
    Ok(decrypted)
}

pub fn encrypt_opt_field(
    data: Option<String>,
    workspace_id: &str,
) -> Result<Option<String>, YntraError> {
    let cipher = WorkspaceCipher::new(workspace_id)?;
    cipher.encrypt_opt(data)
}

pub fn decrypt_opt_field(encrypted_data: Option<String>, workspace_id: &str) -> Option<String> {
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

pub fn hash_anonymous_reporter(user_id: &str, workspace_id: &str, report_id: &str) -> Result<String, YntraError> {
    let salt = get_system_salt_ref()?;
    let client_pepper = get_local_client_pepper()?;

    let context_str = CryptoDomain::WhistleblowerAnonymityHash.get_context(2)?;
    let mut hasher = blake3::Hasher::new_derive_key(context_str);
    hasher.update(&(salt.len() as u64).to_be_bytes());
    hasher.update(salt);

    hasher.update(&(workspace_id.len() as u64).to_be_bytes());
    hasher.update(workspace_id.as_bytes());

    hasher.update(&(user_id.len() as u64).to_be_bytes());
    hasher.update(user_id.as_bytes());

    hasher.update(&(client_pepper.len() as u64).to_be_bytes());
    hasher.update(client_pepper.as_bytes());

    hasher.update(&(report_id.len() as u64).to_be_bytes());
    hasher.update(report_id.as_bytes());

    let mut output = [0u8; 32];
    hasher.finalize_xof().fill(&mut output);
    hasher.zeroize();

    Ok(format!("anon_hash:{}", hex_encode(&output)))
}

pub struct WorkspaceCipher {
    session_key: Option<zeroize::Zeroizing<[u8; 32]>>,
}

impl WorkspaceCipher {
    pub fn new(workspace_id: &str) -> Result<Self, YntraError> {
        let key_res = get_encryption_keys_internal(workspace_id);
        let session_key = match key_res {
            Ok(k) => Some(k),
            Err(YntraError::CryptoError(ref msg)) if msg == "session_key_missing" => None,
            Err(e) => return Err(e),
        };

        Ok(Self { session_key })
    }

    pub fn encrypt(&self, data: &str) -> Result<String, YntraError> {
        let key_bytes = match &self.session_key {
            Some(k) => k,
            None => return Err(YntraError::CryptoError("session_key_missing".to_string())),
        };

        let mut nonce_bytes = [0u8; 24];
        getrandom::fill(&mut nonce_bytes).map_err(|e| {
            tracing::error!("Failed to generate random nonce: {:?}", e);
            YntraError::CryptoError("Failed to generate random nonce".to_string())
        })?;
        let zeroizing_nonce = zeroize::Zeroizing::new(nonce_bytes);

        let key = Key::from_slice(&key_bytes[..]);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(&zeroizing_nonce[..]);

        let result = if let Ok(ct) = cipher.encrypt(nonce, data.as_bytes()) {
            Ok(format!(
                "enc:{}:{}",
                hex_encode(&zeroizing_nonce[..]),
                hex_encode(&ct)
            ))
        } else {
            tracing::error!("XChaCha20Poly1305 encryption failed");
            Err(YntraError::CryptoError("encryption_failed".to_string()))
        };

        result
    }

    pub fn decrypt(&self, encrypted_data: &str) -> Result<String, YntraError> {
        if !encrypted_data.starts_with("enc:") {
            return Err(YntraError::CryptoError("not_encrypted".to_string()));
        }

        let body = &encrypted_data[4..];

        let mut parts = body.splitn(3, ':');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(p1), Some(p2), None) => {
                let mut nonce_bytes = [0u8; 24];
                if const_hex::decode_to_slice(p1, &mut nonce_bytes).is_err() {
                    return Err(YntraError::CryptoError("invalid_nonce".to_string()));
                }
                let zeroizing_nonce = zeroize::Zeroizing::new(nonce_bytes);

                let ct = match hex_decode(p2) {
                    Some(b) => b,
                    None => {
                        return Err(YntraError::CryptoError("invalid_ciphertext".to_string()));
                    }
                };

                let key_bytes = match &self.session_key {
                    Some(k) => k,
                    None => {
                        return Err(YntraError::CryptoError("session_key_missing".to_string()));
                    }
                };

                let key = Key::from_slice(&key_bytes[..]);
                let cipher = XChaCha20Poly1305::new(key);
                let nonce = XNonce::from_slice(&zeroizing_nonce[..]);

                let pt = cipher
                    .decrypt(nonce, ct.as_slice())
                    .map_err(|_| YntraError::CryptoError("decryption_failed".to_string()))?;

                bytes_to_string(pt)
            }
            _ => Err(YntraError::CryptoError("invalid_format".to_string())),
        }
    }

    pub fn encrypt_opt(&self, data: Option<String>) -> Result<Option<String>, YntraError> {
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
        encrypted_data.and_then(|d| self.decrypt(&d).ok())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chacha_encryption_decryption() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        set_session_key("test-session-key".to_string().into_bytes(), "test-workspace-123".to_string());

        let plaintext = "Sensitive whistleblowing report text";
        let workspace_id = "test-workspace-123";
        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();
        assert!(encrypted.starts_with("enc:"));

        let body = &encrypted[4..];
        let parts: Vec<&str> = body.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 48);

        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);

        clear_session_key();
    }

    #[test]
    fn test_strong_session_key_derivation() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let plaintext = "Highly sensitive user data";
        let workspace_id = "test-workspace-456";

        set_session_key(
            "my-super-secret-user-password-or-pin"
                .to_string()
                .into_bytes(),
            "test-workspace-456".to_string(),
        );

        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();

        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);

        clear_session_key();
        let decrypted_without_key_res = decrypt_field(&encrypted, workspace_id);
        assert!(decrypted_without_key_res.is_err());

        set_session_key(
            "my-super-secret-user-password-or-pin"
                .to_string()
                .into_bytes(),
            "test-workspace-456".to_string(),
        );
        let decrypted_with_key_again = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted_with_key_again);

        clear_session_key();
    }

    #[test]
    fn test_session_key_poisoning_recovery() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        
        // 1. Poison when None
        clear_session_key();
        let _ = std::panic::catch_unwind(|| {
            let _lock = match SESSION_KEY.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let _guard = PoisonGuard;
            panic!("poisoning lock when None");
        });

        let res = get_encryption_keys_internal("some-workspace");
        assert!(res.is_err());
        if let Err(YntraError::CryptoError(msg)) = res {
            assert_eq!(msg, "session_key_lock_poisoned");
        } else {
            panic!("Expected CryptoError(session_key_lock_poisoned)");
        }

        // Recovery
        assert!(set_session_key(
            "my-new-session-key".to_string().into_bytes(),
            "some-workspace".to_string()
        ));

        // 2. Poison when Some
        let _ = std::panic::catch_unwind(|| {
            let _lock = match SESSION_KEY.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let _guard = PoisonGuard;
            panic!("poisoning lock when Some");
        });

        let res = get_encryption_keys_internal("some-workspace");
        assert!(res.is_err());
        if let Err(YntraError::CryptoError(msg)) = res {
            assert_eq!(msg, "session_key_lock_poisoned");
        } else {
            panic!("Expected CryptoError(session_key_lock_poisoned)");
        }

        // Recovery again
        assert!(set_session_key(
            "my-new-session-key-2".to_string().into_bytes(),
            "some-workspace".to_string()
        ));

        let lock = match SESSION_KEY.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        assert!(lock.is_some());
    }

    #[test]
    fn test_system_salt_duplicate_initialization() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let initial_salt =
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string();

        let _ = initialize_system_salt(initial_salt);

        let duplicate_salt =
            "303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f".to_string();
        let second_res = initialize_system_salt(duplicate_salt);
        assert!(!second_res);
    }

    #[test]
    fn test_collaborative_workspace_key_match() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let plaintext = "Shared patient health data";
        let workspace_id = "shared-workspace-xyz";

        set_session_key("shared-workspace-session-key".to_string().into_bytes(), "shared-workspace-xyz".to_string());
        let encrypted_by_a = encrypt_field(plaintext, workspace_id).unwrap();
        clear_session_key();

        set_session_key("different-workspace-session-key".to_string().into_bytes(), "shared-workspace-xyz".to_string());
        let decrypted_by_b_wrong = decrypt_field(&encrypted_by_a, workspace_id);
        assert!(decrypted_by_b_wrong.is_err());
        clear_session_key();

        set_session_key("shared-workspace-session-key".to_string().into_bytes(), "shared-workspace-xyz".to_string());
        let decrypted_by_b = decrypt_field(&encrypted_by_a, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted_by_b);

        clear_session_key();
    }

    #[test]
    fn test_bulk_encryption_decryption() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let workspace_id = "bulk-workspace-123";
        set_session_key("bulk-session-key".to_string().into_bytes(), "bulk-workspace-123".to_string());

        let plaintexts = vec![
            "Plaintext message 1".to_string(),
            "Plaintext message 2".to_string(),
            "Plaintext message 3".to_string(),
        ];

        let encrypted = encrypt_fields(plaintexts.clone(), workspace_id).unwrap();
        assert_eq!(encrypted.len(), 3);
        for enc in &encrypted {
            assert!(enc.starts_with("enc:"));
        }

        let decrypted = decrypt_fields(encrypted, workspace_id).unwrap();
        assert_eq!(plaintexts, decrypted);

        clear_session_key();
    }

    #[test]
    fn test_hash_anonymous_reporter_unlinkability() {
        let _test_lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let user_id = "user-123";
        let workspace_id = "workspace-abc";

        let hash1 = hash_anonymous_reporter(user_id, workspace_id, "report-1").unwrap();
        let hash2 = hash_anonymous_reporter(user_id, workspace_id, "report-2").unwrap();

        assert_ne!(hash1, hash2);

        let hash1_again = hash_anonymous_reporter(user_id, workspace_id, "report-1").unwrap();
        assert_eq!(hash1, hash1_again);
    }
}
