use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroize;

#[derive(Clone, Zeroize)]
struct SessionKeys {
    new_key: [u8; 32],
    legacy_key: [u8; 32],
}

static SESSION_KEY: Mutex<Option<SessionKeys>> = Mutex::new(None);
static SYSTEM_SALT: OnceLock<String> = OnceLock::new();

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

fn stretch_key_legacy(key: &str) -> Result<[u8; 32], &'static str> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let salt = b"yntra-session-key-stretching-salt-2026";
    let mut stretched_key = [0u8; 32];
    
    // Configure Argon2id: 19MB memory (19456 KB), 2 iterations, 1 thread (suitable for mobile/WASM)
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| "Argon2 params invalid")?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    argon2.hash_password_into(key.as_bytes(), salt, &mut stretched_key)
        .map_err(|_| "Argon2 key stretching failed")?;
        
    Ok(stretched_key)
}

fn stretch_key_new(key: &str) -> Result<[u8; 32], &'static str> {
    use argon2::{Argon2, Algorithm, Version, Params};
    
    let system_salt = get_system_salt_ref();
    
    // Derive a secure 32-byte salt from the system salt using BLAKE3
    let mut salt_hasher = blake3::Hasher::new_derive_key("Yntra Argon2 salt derivation v1");
    salt_hasher.update(system_salt.as_bytes());
    let mut derived_salt = [0u8; 32];
    salt_hasher.finalize_xof().fill(&mut derived_salt);
    
    let mut stretched_key = [0u8; 32];
    
    // Configure Argon2id: 19MB memory (19456 KB), 2 iterations, 1 thread (suitable for mobile/WASM)
    let params = Params::new(19456, 2, 1, Some(32)).map_err(|_| "Argon2 params invalid")?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    argon2.hash_password_into(key.as_bytes(), &derived_salt, &mut stretched_key)
        .map_err(|_| "Argon2 key stretching failed")?;
        
    Ok(stretched_key)
}

#[uniffi::export]
pub fn initialize_system_salt(salt: String) -> bool {
    match SYSTEM_SALT.set(salt) {
        Ok(_) => true,
        Err(mut rejected_salt) => {
            rejected_salt.zeroize();
            false
        }
    }
}

fn get_system_salt_ref() -> &'static str {
    if let Some(salt) = SYSTEM_SALT.get() {
        return salt.as_str();
    }
    ensure_system_salt_initialized();
    SYSTEM_SALT.get().unwrap().as_str()
}

fn ensure_system_salt_initialized() {
    if SYSTEM_SALT.get().is_some() {
        return;
    }
    
    let mut salt_buf = String::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(mut salt) = std::env::var("YNTRA_ENCRYPTION_SALT") {
            salt_buf.push_str(&salt);
            salt.zeroize();
        } else {
            salt_buf.push_str("yntra-secure-whistleblower-salt-2026");
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut salt_found = false;
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(mut salt)) = storage.get_item("YNTRA_ENCRYPTION_SALT") {
                    salt_buf.push_str(&salt);
                    salt.zeroize();
                    salt_found = true;
                }
            }
        }
        if !salt_found {
            salt_buf.push_str("yntra-secure-whistleblower-salt-2026");
        }
    }
    
    if SYSTEM_SALT.set(salt_buf.clone()).is_err() {
        salt_buf.zeroize();
    }
}

/// Sets the session key used for database field encryption and decryption.
/// 
/// # Arguments
/// * `key` - The raw user PIN or password used to derive the encryption keys.
/// 
/// # Security Warning (FFI Memory Leakage)
/// While this function zeroizes the temporary `key` string buffer on the Rust side
/// immediately after stretching, the FFI boundary (UniFFI/JNI/Swift glue) and host
/// languages (Swift, Java, Kotlin) may create temporary string allocations in garbage-collected
/// or heap-allocated memory that Rust cannot zeroize.
/// 
/// Downstream developers must:
/// 1. Wipe or clear password UI components and text buffers from memory on the host side immediately after use.
/// 2. Avoid storing raw passwords as persistent `String` variables in JVM or Swift heaps.
/// 3. Prefer retrieving PINs/passwords dynamically and invoking this bridge directly, clearing the local host-side variables immediately.
/// 
/// # Performance Warning (WASM / UI Thread Blocking)
/// Since this function runs CPU-intensive Argon2 key stretching (Argon2id with 19MB memory, 2 iterations),
/// invoking it synchronously on the main UI thread in web (WASM) or mobile environments will freeze the user interface
/// for up to several hundred milliseconds.
/// 
/// Developers MUST:
/// 1. Run this function inside a background thread/task (e.g. `tokio::task::spawn_blocking` or a Web Worker/Web Workers pool).
/// 2. Avoid calling it synchronously from Dioxus event handlers on the main thread.
#[uniffi::export]
pub fn set_session_key(mut key: String) -> bool {
    let new_res = stretch_key_new(&key);
    let legacy_res = stretch_key_legacy(&key);
    
    match (new_res, legacy_res) {
        (Ok(new_stretched), Ok(legacy_stretched)) => {
            let mut lock = SESSION_KEY.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(mut old_sk) = lock.take() {
                old_sk.zeroize();
            }
            *lock = Some(SessionKeys {
                new_key: new_stretched,
                legacy_key: legacy_stretched,
            });
            key.zeroize();
            true
        }
        (new_err, legacy_err) => {
            if let Err(e) = new_err {
                tracing::error!("New key stretching failed: {:?}", e);
            }
            if let Err(e) = legacy_err {
                tracing::error!("Legacy key stretching failed: {:?}", e);
            }
            key.zeroize();
            false
        }
    }
}

#[uniffi::export]
pub fn clear_session_key() {
    let mut lock = SESSION_KEY.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(mut old_sk) = lock.take() {
        old_sk.zeroize();
    }
}

fn get_encryption_keys_internal(
    workspace_id: &str,
    use_session_key: bool,
) -> Result<([u8; 32], [u8; 12]), crate::infra::errors::YntraError> {
    // 1. Initialize Hasher with domain separation context for key/nonce derivation
    let mut hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key and nonce derivation v1");

    // 2. Feed workspace_id with length prefix to prevent input canonicalization / collision attacks
    hasher.update(&(workspace_id.len() as u64).to_be_bytes());
    hasher.update(workspace_id.as_bytes());

    // 3. Retrieve and feed salt with length prefix
    let salt = get_system_salt_ref();
    hasher.update(&(salt.len() as u64).to_be_bytes());
    hasher.update(salt.as_bytes());

    // 4. Retrieve and feed session key if requested
    if use_session_key {
        let lock = SESSION_KEY.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(ref sk) = *lock {
            hasher.update(&(sk.new_key.len() as u64).to_be_bytes());
            hasher.update(&sk.new_key);
        } else {
            return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string()));
        }
    }

    // 5. Derive key and nonce using single-pass Blake3 XOF
    let mut reader = hasher.finalize_xof();
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    reader.fill(&mut key);
    reader.fill(&mut nonce);

    Ok((key, nonce))
}

fn get_legacy_encryption_keys_internal(
    workspace_id: &str,
    use_session_key: bool,
) -> Result<([u8; 32], [u8; 12]), crate::infra::errors::YntraError> {
    // 1. Initialize Hasher with domain separation context for key
    let mut key_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key derivation v1");
    key_hasher.update(workspace_id.as_bytes());

    // 2. Fetch and feed salt
    let salt = get_system_salt_ref();
    key_hasher.update(salt.as_bytes());

    // 3. Derive the nonce using the same inputs with distinct domain context
    let mut nonce_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower nonce derivation v1");
    nonce_hasher.update(workspace_id.as_bytes());
    nonce_hasher.update(salt.as_bytes());

    // 4. Retrieve and feed session key if requested (single lock acquisition)
    if use_session_key {
        let lock = SESSION_KEY.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(ref sk) = *lock {
            key_hasher.update(&sk.legacy_key);
            nonce_hasher.update(&sk.legacy_key);
        } else {
            return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string()));
        }
    }

    // 5. Derive the ChaCha key
    let key: [u8; 32] = key_hasher.finalize().into();

    let nonce_bytes: [u8; 32] = nonce_hasher.finalize().into();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes[0..12]);

    Ok((key, nonce))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const_hex::encode(bytes)
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    const_hex::decode(s).ok()
}

pub fn encrypt_field(data: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    let (mut key_bytes, _) = get_encryption_keys_internal(workspace_id, true)?;

    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|e| {
        tracing::error!("Failed to generate random nonce: {:?}", e);
        key_bytes.zeroize();
        crate::infra::errors::YntraError::CryptoError("Failed to generate random nonce".to_string())
    })?;

    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let result = if let Ok(ct) = cipher.encrypt(nonce, data.as_bytes()) {
        Ok(format!("enc:{}:{}", hex_encode(&nonce_bytes), hex_encode(&ct)))
    } else {
        tracing::error!("ChaCha20Poly1305 encryption failed");
        Err(crate::infra::errors::YntraError::CryptoError("encryption_failed".to_string()))
    };
    
    key_bytes.zeroize();
    nonce_bytes.zeroize();
    
    result
}

pub fn decrypt_field(encrypted_data: &str, workspace_id: &str) -> Result<String, crate::infra::errors::YntraError> {
    if !encrypted_data.starts_with("enc:") {
        return Err(crate::infra::errors::YntraError::CryptoError("not_encrypted".to_string()));
    }
    
    let body = &encrypted_data[4..];
    
    // Parse parts without heap-allocated Vec
    let mut parts = body.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(p1), Some(p2), None) => {
            // Standard: p1 = nonce hex, p2 = ciphertext hex
            let mut nonce_bytes = [0u8; 12];
            if const_hex::decode_to_slice(p1, &mut nonce_bytes).is_err() {
                return Err(crate::infra::errors::YntraError::CryptoError("invalid_nonce".to_string()));
            }
            
            let ct = match hex_decode(p2) {
                Some(b) => b,
                None => {
                    nonce_bytes.zeroize();
                    return Err(crate::infra::errors::YntraError::CryptoError("invalid_ciphertext".to_string()));
                }
            };
            
            // 1. Try to decrypt using the strong session-derived key first
            match get_encryption_keys_internal(workspace_id, true) {
                Ok((mut key_bytes, _)) => {
                    let key = Key::from_slice(&key_bytes);
                    let cipher = ChaCha20Poly1305::new(key);
                    let nonce = Nonce::from_slice(&nonce_bytes);
                    
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        key_bytes.zeroize();
                        nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                    key_bytes.zeroize();
                }
                Err(crate::infra::errors::YntraError::CryptoError(ref e)) if e == "session_key_missing" => {}
                Err(e) => {
                    nonce_bytes.zeroize();
                    return Err(e);
                }
            }
            
            // Fallback 1b: Try to decrypt using the legacy strong session-derived key
            match get_legacy_encryption_keys_internal(workspace_id, true) {
                Ok((mut key_bytes, _)) => {
                    let key = Key::from_slice(&key_bytes);
                    let cipher = ChaCha20Poly1305::new(key);
                    let nonce = Nonce::from_slice(&nonce_bytes);
                    
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        key_bytes.zeroize();
                        nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                    key_bytes.zeroize();
                }
                Err(crate::infra::errors::YntraError::CryptoError(ref e)) if e == "session_key_missing" => {}
                Err(e) => {
                    nonce_bytes.zeroize();
                    return Err(e);
                }
            }

            // 2. Fall back to decrypting with the new workspace-only key
            if let Ok((mut key_bytes, _)) = get_encryption_keys_internal(workspace_id, false) {
                let key = Key::from_slice(&key_bytes);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                    key_bytes.zeroize();
                    nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }
                key_bytes.zeroize();
            }

            // Fallback 2b: Decrypt with legacy workspace-only key
            if let Ok((mut key_bytes, _)) = get_legacy_encryption_keys_internal(workspace_id, false) {
                let key = Key::from_slice(&key_bytes);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                    key_bytes.zeroize();
                    nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }
                key_bytes.zeroize();
            }

            nonce_bytes.zeroize();
            Err(crate::infra::errors::YntraError::CryptoError("decryption_failed".to_string()))
        }
        (Some(p1), None, None) => {
            // Legacy deterministic nonce fallback (parts.len() != 2)
            let ct = match hex_decode(p1) {
                Some(b) => b,
                None => return Err(crate::infra::errors::YntraError::CryptoError("invalid_ciphertext".to_string())),
            };

            // 1. Try to decrypt using the strong session-derived key first
            match get_encryption_keys_internal(workspace_id, true) {
                Ok((mut key_bytes, mut legacy_nonce_bytes)) => {
                    let key = Key::from_slice(&key_bytes);
                    let cipher = ChaCha20Poly1305::new(key);
                    let nonce = Nonce::from_slice(&legacy_nonce_bytes);
                    
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        key_bytes.zeroize();
                        legacy_nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                    key_bytes.zeroize();
                    legacy_nonce_bytes.zeroize();
                }
                Err(crate::infra::errors::YntraError::CryptoError(ref e)) if e == "session_key_missing" => {}
                Err(e) => return Err(e),
            }
            
            // Try legacy derivation key + legacy derived nonce
            match get_legacy_encryption_keys_internal(workspace_id, true) {
                Ok((mut key_bytes, mut legacy_nonce_bytes)) => {
                    let key = Key::from_slice(&key_bytes);
                    let cipher = ChaCha20Poly1305::new(key);
                    let nonce = Nonce::from_slice(&legacy_nonce_bytes);
                    
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        key_bytes.zeroize();
                        legacy_nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                    key_bytes.zeroize();
                    legacy_nonce_bytes.zeroize();
                }
                Err(crate::infra::errors::YntraError::CryptoError(ref e)) if e == "session_key_missing" => {}
                Err(e) => return Err(e),
            }

            // 2. Fall back to decrypting with the new workspace-only key + new derived nonce
            if let Ok((mut key_bytes, mut legacy_nonce_bytes)) = get_encryption_keys_internal(workspace_id, false) {
                let key = Key::from_slice(&key_bytes);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = Nonce::from_slice(&legacy_nonce_bytes);
                
                if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                    key_bytes.zeroize();
                    legacy_nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }
                key_bytes.zeroize();
                legacy_nonce_bytes.zeroize();
            }

            // Try legacy workspace-only key + legacy derived nonce
            if let Ok((mut key_bytes, mut legacy_nonce_bytes)) = get_legacy_encryption_keys_internal(workspace_id, false) {
                let key = Key::from_slice(&key_bytes);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = Nonce::from_slice(&legacy_nonce_bytes);
                
                if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                    key_bytes.zeroize();
                    legacy_nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }
                key_bytes.zeroize();
                legacy_nonce_bytes.zeroize();
            }

            Err(crate::infra::errors::YntraError::CryptoError("decryption_failed".to_string()))
        }
        _ => Err(crate::infra::errors::YntraError::CryptoError("invalid_format".to_string())),
    }
}

pub fn encrypt_opt_field(data: Option<String>, workspace_id: &str) -> Result<Option<String>, crate::infra::errors::YntraError> {
    match data {
        Some(mut d) => {
            let res = encrypt_field(&d, workspace_id).map(Some);
            d.zeroize();
            res
        }
        None => Ok(None),
    }
}

pub fn decrypt_opt_field(encrypted_data: Option<String>, workspace_id: &str) -> Option<String> {
    encrypted_data.and_then(|d| {
        decrypt_field(&d, workspace_id).ok()
    })
}

pub struct WorkspaceCipher {
    session_cipher: Option<ChaCha20Poly1305>,
    session_nonce: Option<[u8; 12]>,
    
    legacy_session_cipher: Option<ChaCha20Poly1305>,
    legacy_session_nonce: Option<[u8; 12]>,
    
    workspace_cipher: ChaCha20Poly1305,
    workspace_nonce: [u8; 12],
    
    legacy_workspace_cipher: ChaCha20Poly1305,
    legacy_workspace_nonce: [u8; 12],
}

impl WorkspaceCipher {
    pub fn new(workspace_id: &str) -> Self {
        // 1. Try to get session keys
        let (session_cipher, session_nonce, legacy_session_cipher, legacy_session_nonce) = {
            let lock = SESSION_KEY.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(ref sk) = *lock {
                // Derive new session key
                let mut hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key and nonce derivation v1");
                hasher.update(&(workspace_id.len() as u64).to_be_bytes());
                hasher.update(workspace_id.as_bytes());
                let salt = get_system_salt_ref();
                hasher.update(&(salt.len() as u64).to_be_bytes());
                hasher.update(salt.as_bytes());
                hasher.update(&(sk.new_key.len() as u64).to_be_bytes());
                hasher.update(&sk.new_key);
                
                let mut reader = hasher.finalize_xof();
                let mut k1 = [0u8; 32];
                let mut n1 = [0u8; 12];
                reader.fill(&mut k1);
                reader.fill(&mut n1);
                
                // Derive legacy session key
                let mut key_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key derivation v1");
                key_hasher.update(workspace_id.as_bytes());
                key_hasher.update(salt.as_bytes());
                key_hasher.update(&sk.legacy_key);
                
                let mut nonce_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower nonce derivation v1");
                nonce_hasher.update(workspace_id.as_bytes());
                nonce_hasher.update(salt.as_bytes());
                nonce_hasher.update(&sk.legacy_key);
                
                let k2: [u8; 32] = key_hasher.finalize().into();
                let n2_bytes: [u8; 32] = nonce_hasher.finalize().into();
                let mut n2 = [0u8; 12];
                n2.copy_from_slice(&n2_bytes[0..12]);
                
                let c1 = ChaCha20Poly1305::new(Key::from_slice(&k1));
                let c2 = ChaCha20Poly1305::new(Key::from_slice(&k2));
                
                (Some(c1), Some(n1), Some(c2), Some(n2))
            } else {
                (None, None, None, None)
            }
        };
        
        // 2. Derive workspace-only keys
        let salt = get_system_salt_ref();
        
        // New workspace-only key
        let mut hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key and nonce derivation v1");
        hasher.update(&(workspace_id.len() as u64).to_be_bytes());
        hasher.update(workspace_id.as_bytes());
        hasher.update(&(salt.len() as u64).to_be_bytes());
        hasher.update(salt.as_bytes());
        
        let mut reader = hasher.finalize_xof();
        let mut k3 = [0u8; 32];
        let mut n3 = [0u8; 12];
        reader.fill(&mut k3);
        reader.fill(&mut n3);
        
        // Legacy workspace-only key
        let mut key_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower key derivation v1");
        key_hasher.update(workspace_id.as_bytes());
        key_hasher.update(salt.as_bytes());
        
        let mut nonce_hasher = blake3::Hasher::new_derive_key("Yntra whistleblower nonce derivation v1");
        nonce_hasher.update(workspace_id.as_bytes());
        nonce_hasher.update(salt.as_bytes());
        
        let k4: [u8; 32] = key_hasher.finalize().into();
        let n4_bytes: [u8; 32] = nonce_hasher.finalize().into();
        let mut n4 = [0u8; 12];
        n4.copy_from_slice(&n4_bytes[0..12]);
        
        let c3 = ChaCha20Poly1305::new(Key::from_slice(&k3));
        let c4 = ChaCha20Poly1305::new(Key::from_slice(&k4));
        
        Self {
            session_cipher,
            session_nonce,
            legacy_session_cipher,
            legacy_session_nonce,
            workspace_cipher: c3,
            workspace_nonce: n3,
            legacy_workspace_cipher: c4,
            legacy_workspace_nonce: n4,
        }
    }

    pub fn encrypt(&self, data: &str) -> Result<String, crate::infra::errors::YntraError> {
        let cipher = match &self.session_cipher {
            Some(c) => c,
            None => return Err(crate::infra::errors::YntraError::CryptoError("session_key_missing".to_string())),
        };
        
        let mut nonce_bytes = [0u8; 12];
        getrandom::fill(&mut nonce_bytes).map_err(|e| {
            tracing::error!("Failed to generate random nonce: {:?}", e);
            crate::infra::errors::YntraError::CryptoError("Failed to generate random nonce".to_string())
        })?;
        
        let nonce = Nonce::from_slice(&nonce_bytes);
        let result = if let Ok(ct) = cipher.encrypt(nonce, data.as_bytes()) {
            Ok(format!("enc:{}:{}", hex_encode(&nonce_bytes), hex_encode(&ct)))
        } else {
            tracing::error!("ChaCha20Poly1305 encryption failed");
            Err(crate::infra::errors::YntraError::CryptoError("encryption_failed".to_string()))
        };
        
        nonce_bytes.zeroize();
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
                let mut nonce_bytes = [0u8; 12];
                if const_hex::decode_to_slice(p1, &mut nonce_bytes).is_err() {
                    return Err(crate::infra::errors::YntraError::CryptoError("invalid_nonce".to_string()));
                }
                
                let ct = match hex_decode(p2) {
                    Some(b) => b,
                    None => {
                        nonce_bytes.zeroize();
                        return Err(crate::infra::errors::YntraError::CryptoError("invalid_ciphertext".to_string()));
                    }
                };
                
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                if let Some(ref cipher) = self.session_cipher {
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                }
                
                if let Some(ref cipher) = self.legacy_session_cipher {
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        nonce_bytes.zeroize();
                        return bytes_to_string(pt);
                    }
                }

                if let Ok(pt) = self.workspace_cipher.decrypt(nonce, ct.as_slice()) {
                    nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }

                if let Ok(pt) = self.legacy_workspace_cipher.decrypt(nonce, ct.as_slice()) {
                    nonce_bytes.zeroize();
                    return bytes_to_string(pt);
                }

                nonce_bytes.zeroize();
                Err(crate::infra::errors::YntraError::CryptoError("decryption_failed".to_string()))
            }
            (Some(p1), None, None) => {
                let ct = match hex_decode(p1) {
                    Some(b) => b,
                    None => return Err(crate::infra::errors::YntraError::CryptoError("invalid_ciphertext".to_string())),
                };

                if let (Some(cipher), Some(legacy_nonce)) = (&self.session_cipher, &self.session_nonce) {
                    let nonce = Nonce::from_slice(legacy_nonce);
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        return bytes_to_string(pt);
                    }
                }
                
                if let (Some(cipher), Some(legacy_nonce)) = (&self.legacy_session_cipher, &self.legacy_session_nonce) {
                    let nonce = Nonce::from_slice(legacy_nonce);
                    if let Ok(pt) = cipher.decrypt(nonce, ct.as_slice()) {
                        return bytes_to_string(pt);
                    }
                }

                {
                    let nonce = Nonce::from_slice(&self.workspace_nonce);
                    if let Ok(pt) = self.workspace_cipher.decrypt(nonce, ct.as_slice()) {
                        return bytes_to_string(pt);
                    }
                }

                {
                    let nonce = Nonce::from_slice(&self.legacy_workspace_nonce);
                    if let Ok(pt) = self.legacy_workspace_cipher.decrypt(nonce, ct.as_slice()) {
                        return bytes_to_string(pt);
                    }
                }

                Err(crate::infra::errors::YntraError::CryptoError("decryption_failed".to_string()))
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
        if let Some(ref mut n) = self.session_nonce {
            n.zeroize();
        }
        if let Some(ref mut n) = self.legacy_session_nonce {
            n.zeroize();
        }
        self.workspace_nonce.zeroize();
        self.legacy_workspace_nonce.zeroize();
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
        set_session_key("test-session-key".to_string());
        
        let plaintext = "Sensitive whistleblowing report text";
        let workspace_id = "test-workspace-123";
        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();
        assert!(encrypted.starts_with("enc:"));
        
        // Assert that the encrypted format contains two colon-separated hex strings
        let body = &encrypted[4..];
        let parts: Vec<&str> = body.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 24); // 12-byte hex nonce is 24 characters
        
        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);
        
        clear_session_key();
    }

    #[test]
    fn test_legacy_decryption_fallback() {
        let plaintext = "Legacy encrypted field value";
        let workspace_id = "test-workspace-123";
        
        // Emulate legacy encryption (using the static nonce derived from get_legacy_encryption_keys_internal)
        let (key_bytes, legacy_nonce_bytes) = get_legacy_encryption_keys_internal(workspace_id, false).unwrap();
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&legacy_nonce_bytes);
        let ct = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
        let legacy_encrypted = format!("enc:{}", hex_encode(&ct));
        
        // Decrypt using the updated decrypt_field which should trigger the fallback
        let decrypted = decrypt_field(&legacy_encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_strong_session_key_derivation() {
        let plaintext = "Highly sensitive user data";
        let workspace_id = "test-workspace-456";

        // Set session key
        set_session_key("my-super-secret-user-password-or-pin".to_string());

        let encrypted = encrypt_field(plaintext, workspace_id).unwrap();
        
        // Decrypt with correct session key set
        let decrypted = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);

        // Temporarily clear session key and ensure decryption falls back or fails gracefully
        clear_session_key();
        let decrypted_without_key_res = decrypt_field(&encrypted, workspace_id);
        // It should NOT decrypt, returning an Err since the session key is missing
        assert!(decrypted_without_key_res.is_err());

        // Reset session key and ensure it works again
        set_session_key("my-super-secret-user-password-or-pin".to_string());
        let decrypted_with_key_again = decrypt_field(&encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted_with_key_again);

        clear_session_key();
    }

    #[test]
    fn test_pre_refactor_backward_compatibility() {
        // Enforce backward compatibility specifically for old keys
        let plaintext = "Important archive data";
        let workspace_id = "test-workspace-compat";

        // Set session key
        set_session_key("compatibility-pin-123".to_string());

        // Encrypt with legacy keys manually
        let (key_bytes, _) = get_legacy_encryption_keys_internal(workspace_id, true).unwrap();
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        
        let mut nonce_bytes = [0u8; 12];
        getrandom::fill(&mut nonce_bytes).unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
        let legacy_encrypted = format!("enc:{}:{}", hex_encode(&nonce_bytes), hex_encode(&ct));

        // Decrypt with current decrypt_field which has the legacy fallback
        let decrypted = decrypt_field(&legacy_encrypted, workspace_id).unwrap();
        assert_eq!(plaintext, decrypted);

        clear_session_key();
    }
}
