use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroize;
use ed25519_dalek::{Signer, Verifier};

#[derive(Clone, Zeroize)]
#[zeroize(drop)]
struct SessionKeys {
    new_key: [u8; 32],
}

static SESSION_KEY: Mutex<Option<SessionKeys>> = Mutex::new(None);
static SYSTEM_SALT: OnceLock<zeroize::Zeroizing<Vec<u8>>> = OnceLock::new();

fn bytes_to_string(bytes: Vec<u8>) -> Result<String, crate::infra::errors::YntraError> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => {
            let mut raw = e.into_bytes();
            raw.zeroize();
            Err(crate::infra::errors::YntraError::CryptoError("invalid_utf8".to_string()))
        }
    }
}

fn stretch_key_new(key: &[u8]) -> Result<[u8; 32], crate::infra::errors::YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let system_salt = get_system_salt_ref()?;
    
    // Derive a secure 32-byte salt from the system salt using BLAKE3
    let mut salt_hasher = blake3::Hasher::new_derive_key("Yntra Argon2 salt derivation v1");
    salt_hasher.update(system_salt);
    let mut derived_salt = [0u8; 32];
    salt_hasher.finalize_xof().fill(&mut derived_salt);
    salt_hasher.zeroize();
    
    let mut stretched_key = [0u8; 32];
    
    // Configure Argon2id: 19MB memory (19456 KB), 2 iterations, 1 thread (suitable for mobile/WASM)
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    let res = argon2.hash_password_into(key, &derived_salt, &mut stretched_key)
        .map_err(|_| {
            stretched_key.zeroize();
            crate::infra::errors::YntraError::CryptoError("Argon2 key stretching failed".to_string())
        });
        
    derived_salt.zeroize();
    
    res.map(|_| stretched_key)
}

#[uniffi::export]
pub fn initialize_system_salt(mut salt: String) -> bool {
    let bytes = match const_hex::decode(&salt) {
        Ok(b) => {
            salt.zeroize();
            b
        }
        Err(_) => salt.into_bytes(),
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
        Err(_) => salt_buf.into_bytes(),
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
pub fn set_session_key(key_bytes: Vec<u8>) -> bool {
    let zeroizing_key = zeroize::Zeroizing::new(key_bytes);
    let new_res = stretch_key_new(&zeroizing_key);
    
    match new_res {
        Ok(new_stretched) => {
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
            *lock = Some(SessionKeys {
                new_key: new_stretched,
            });
            true
        }
        Err(e) => {
            tracing::error!("Key stretching failed: {:?}", e);
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
            false
        }
    }
}

#[uniffi::export]
pub fn get_session_key() -> Option<Vec<u8>> {
    let lock = match SESSION_KEY.lock() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    lock.as_ref().map(|sk| sk.new_key.to_vec())
}

#[uniffi::export]
pub async fn load_local_workspace_key(workspace_id: String) -> bool {
    let key_name = format!("workspace_key_{}", workspace_id);
    if let Some(key_hex) = get_local_secret(&key_name).await {
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
    if let Ok(cipher) = WorkspaceCipher::new(workspace_id) {
        cipher.decrypt_opt(encrypted_data)
    } else {
        None
    }
}

fn get_local_client_pepper() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::fs;
        use std::path::PathBuf;
        let path = PathBuf::from("yntra_client_pepper.bin");
        if let Ok(pepper) = fs::read_to_string(&path) {
            pepper
        } else {
            let new_pepper = uuid::Uuid::new_v4().to_string();
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
                    let new_pepper = uuid::Uuid::new_v4().to_string();
                    let _ = storage.set_item("yntra_client_pepper", &new_pepper);
                    return new_pepper;
                }
            }
        }
        static SESSION_PEPPER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        SESSION_PEPPER.get_or_init(|| {
            uuid::Uuid::new_v4().to_string()
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

#[uniffi::export]
pub fn generate_role_signature(private_key_hex: &str, user_id: &str, role: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
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
    let message = format!("{}:{}:{}", user_id, role, workspace_id);
    let signature = signing_key.sign(message.as_bytes());
    Ok(const_hex::encode(&signature.to_bytes()))
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
pub async fn get_local_secret(key: &str) -> Option<String> {
    let db = match libsql::Builder::new_local("yntra_local_secrets.db").build().await {
        Ok(d) => d,
        Err(_) => return None,
    };
    let conn = match db.connect() {
        Ok(c) => c,
        Err(_) => return None,
    };
    let _ = conn.execute("CREATE TABLE IF NOT EXISTS local_secrets (key TEXT PRIMARY KEY, value TEXT)", ()).await;
    
    let mut stmt = match conn.prepare("SELECT value FROM local_secrets WHERE key = ?1").await {
        Ok(s) => s,
        Err(_) => return None,
    };
    let mut rows = match stmt.query(libsql::params![key]).await {
        Ok(r) => r,
        Err(_) => return None,
    };
    if let Ok(Some(row)) = rows.next().await {
        let val = row.get::<String>(0).ok()?;
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    } else {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), crate::infra::errors::YntraError> {
    let db = libsql::Builder::new_local("yntra_local_secrets.db").build().await
        .map_err(|e| crate::infra::errors::YntraError::DbError(e.to_string()))?;
    let conn = db.connect()
        .map_err(|e| crate::infra::errors::YntraError::DbError(e.to_string()))?;
    conn.execute("CREATE TABLE IF NOT EXISTS local_secrets (key TEXT PRIMARY KEY, value TEXT)", ()).await
        .map_err(|e| crate::infra::errors::YntraError::DbError(e.to_string()))?;
    conn.execute("INSERT OR REPLACE INTO local_secrets (key, value) VALUES (?1, ?2)", libsql::params![key, value]).await
        .map_err(|e| crate::infra::errors::YntraError::DbError(e.to_string()))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn get_local_secret(key: &str) -> Option<String> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let val = storage.get_item(key).ok().flatten()?;
            if val.is_empty() {
                return None;
            }
            return Some(val);
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
pub async fn set_local_secret(key: &str, value: &str) -> Result<(), crate::infra::errors::YntraError> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage.set_item(key, value)
                .map_err(|_| crate::infra::errors::YntraError::CryptoError("local_storage_write_failed".to_string()))?;
            return Ok(());
        }
    }
    Err(crate::infra::errors::YntraError::CryptoError("local_storage_unavailable".to_string()))
}

#[uniffi::export]
pub fn encrypt_workspace_key_with_password(password: &str, workspace_key: Vec<u8>) -> Result<String, crate::infra::errors::YntraError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?;
    
    let mut nonce_bytes = [0u8; 24];
    getrandom::fill(&mut nonce_bytes).map_err(|e| crate::infra::errors::YntraError::CryptoError(e.to_string()))?;
    
    let mut derived_key = [0u8; 32];
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2.hash_password_into(password.as_bytes(), &salt, &mut derived_key)
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 derivation failed".to_string()))?;
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, workspace_key.as_slice())
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Envelope encryption failed".to_string()))?;
        
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
        
    let mut derived_key = [0u8; 32];
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 params invalid".to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2.hash_password_into(password.as_bytes(), &salt, &mut derived_key)
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Argon2 derivation failed".to_string()))?;
        
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&derived_key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| crate::infra::errors::YntraError::CryptoError("Envelope decryption failed".to_string()))?;
        
    Ok(plaintext)
}




#[uniffi::export]
pub fn verify_role_signature(public_key_hex: &str, user_id: &str, role: &str, workspace_id: &str, signature_hex: &str) -> bool {
    let public_key_bytes = match const_hex::decode(public_key_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature_bytes = match const_hex::decode(signature_hex) {
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
    let message = format!("{}:{}:{}", user_id, role, workspace_id);
    verifying_key.verify(message.as_bytes(), &signature).is_ok()
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
