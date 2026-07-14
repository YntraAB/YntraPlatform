use crate::infra::errors::YntraError;
use zeroize::Zeroizing;

// --- Pillar 3: Zero-Knowledge Cryptographic Trust (Passkey + ZKP) ---

#[derive(Clone, uniffi::Object)]
pub struct ZkCryptoTrust {}

#[uniffi::export]
impl ZkCryptoTrust {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt_workspace_field(
        &self,
        passkey_seed: String,
        plaintext: String,
    ) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};

        let seed_zeroed = Zeroizing::new(passkey_seed);
        let plaintext_zeroed = Zeroizing::new(plaintext);

        let mut hasher =
            blake3::Hasher::new_derive_key("Yntra Zero-Copy Passkey Envelope Encryption Key");
        hasher.update(seed_zeroed.as_bytes());
        let mut key_bytes = Zeroizing::new([0u8; 32]);
        hasher.finalize_xof().fill(&mut *key_bytes);

        let key = chacha20poly1305::Key::from_slice(&*key_bytes);
        let cipher = XChaCha20Poly1305::new(key);

        let mut nonce_bytes = [0u8; 24];
        getrandom::fill(&mut nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext_bytes = cipher
            .encrypt(nonce, plaintext_zeroed.as_bytes())
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut payload = Vec::new();
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext_bytes);

        Ok(const_hex::encode(&payload))
    }

    pub fn decrypt_workspace_field(
        &self,
        passkey_seed: String,
        ciphertext_hex: String,
    ) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};

        let seed_zeroed = Zeroizing::new(passkey_seed);
        let payload = const_hex::decode(&ciphertext_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if payload.len() < 24 {
            return Err(YntraError::CryptoError(
                "Invalid ciphertext payload length".to_string(),
            ));
        }

        let nonce_bytes = &payload[0..24];
        let ciphertext_bytes = &payload[24..];

        let mut hasher =
            blake3::Hasher::new_derive_key("Yntra Zero-Copy Passkey Envelope Encryption Key");
        hasher.update(seed_zeroed.as_bytes());
        let mut key_bytes = Zeroizing::new([0u8; 32]);
        hasher.finalize_xof().fill(&mut *key_bytes);

        let key = chacha20poly1305::Key::from_slice(&*key_bytes);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(nonce_bytes);

        let decrypted_bytes = cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let decrypted_string = String::from_utf8(decrypted_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        Ok(decrypted_string)
    }

    pub fn generate_compliance_proof(
        &self,
        passkey_seed: String,
        data_hex: String,
        user_id: String,
        role: String,
    ) -> Result<String, YntraError> {
        let data_bytes =
            const_hex::decode(&data_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let data_hash = blake3::hash(&data_bytes);

        // Generate a random 32-byte salt (blinding factor)
        let mut salt_bytes = [0u8; 32];
        getrandom::fill(&mut salt_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        // Derive user Ed25519 signing key from passkey_seed using blake3 derive_key
        let seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
        key_hasher.update(seed_zeroed.as_bytes());
        let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
        key_hasher.finalize_xof().fill(&mut *private_key_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
        let public_key = signing_key.verifying_key();

        // Commitment over (user_id, role, data_hash, salt)
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_COMMITMENT_V2");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        hasher.update(data_hash.as_bytes());
        hasher.update(&salt_bytes);
        let commitment = hasher.finalize();

        // Sign the commitment
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(commitment.as_bytes());

        let is_valid_len = !data_bytes.is_empty() && data_bytes.len() < 10_000_000;

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_PROOF_V2:");
        proof_builder.extend_from_slice(commitment.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&salt_bytes); // 32 bytes
        proof_builder.extend_from_slice(&signature.to_bytes()); // 64 bytes
        proof_builder.extend_from_slice(public_key.as_bytes()); // 32 bytes
        proof_builder.push(if is_valid_len { 1 } else { 0 }); // 1 byte

        Ok(const_hex::encode(&proof_builder))
    }

    pub fn verify_compliance_proof(
        &self,
        proof_hex: String,
        user_id: String,
        role: String,
        data_hash_hex: String,
        public_key_hex: String,
    ) -> Result<bool, YntraError> {
        let proof_bytes =
            const_hex::decode(&proof_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let data_hash_bytes =
            const_hex::decode(&data_hash_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if proof_bytes.starts_with(b"ZKP_PROOF_V2:") && proof_bytes.len() == 174 {
            let actual_commitment = &proof_bytes[13..45];
            let salt_bytes = &proof_bytes[45..77];
            let signature_bytes = &proof_bytes[77..141];
            let proof_public_key = &proof_bytes[141..173];
            let is_valid_len = *proof_bytes.last().unwrap_or(&0) == 1;

            // Verify that the public_key matches the registered public_key_hex (if supplied)
            if !public_key_hex.is_empty() {
                let registered_public_key = const_hex::decode(&public_key_hex)
                    .map_err(|e| YntraError::CryptoError(e.to_string()))?;
                if proof_public_key != registered_public_key {
                    return Ok(false);
                }
            }

            // Verify the signature on commitment using the public key
            use ed25519_dalek::Verifier;
            let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
                proof_public_key.try_into().unwrap(),
            )
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            let signature = ed25519_dalek::Signature::from_bytes(
                signature_bytes.try_into().unwrap(),
            );

            if verifying_key.verify(actual_commitment, &signature).is_err() {
                return Ok(false);
            }

            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_ZKP_COMMITMENT_V2");
            hasher.update(user_id.as_bytes());
            hasher.update(role.as_bytes());
            hasher.update(&data_hash_bytes);
            hasher.update(salt_bytes);
            let expected_commitment = hasher.finalize();

            let hash_matches = constant_time_eq(expected_commitment.as_bytes(), actual_commitment);
            let len_matches = is_valid_len;

            Ok(hash_matches && len_matches)
        } else if proof_bytes.starts_with(b"ZKP_PROOF_V1:") && proof_bytes.len() == 46 {
            let actual_commitment = &proof_bytes[13..45];
            let is_valid_len = *proof_bytes.last().unwrap_or(&0) == 1;

            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_ZKP_COMMITMENT_V1");
            hasher.update(user_id.as_bytes());
            hasher.update(role.as_bytes());
            hasher.update(&data_hash_bytes);
            let expected_commitment = hasher.finalize();

            let hash_matches = constant_time_eq(expected_commitment.as_bytes(), actual_commitment);
            let len_matches = is_valid_len;

            Ok(hash_matches && len_matches)
        } else {
            Ok(false)
        }
    }

    pub fn generate_role_proof(
        &self,
        passkey_seed: String,
        user_id: String,
        role: String,
    ) -> Result<String, YntraError> {
        let mut salt_bytes = [0u8; 32];
        getrandom::fill(&mut salt_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        // Derive user Ed25519 signing key from passkey_seed using blake3 derive_key
        let seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
        key_hasher.update(seed_zeroed.as_bytes());
        let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
        key_hasher.finalize_xof().fill(&mut *private_key_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
        let public_key = signing_key.verifying_key();

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V2");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        hasher.update(&salt_bytes);
        let commitment = hasher.finalize();

        // Sign the commitment
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(commitment.as_bytes());

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_ROLE_PROOF_V2:");
        proof_builder.extend_from_slice(commitment.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&salt_bytes); // 32 bytes
        proof_builder.extend_from_slice(&signature.to_bytes()); // 64 bytes
        proof_builder.extend_from_slice(public_key.as_bytes()); // 32 bytes

        Ok(const_hex::encode(&proof_builder))
    }

    pub fn verify_proof(
        &self,
        proof_hex: String,
        user_id: String,
        role: String,
        public_key_hex: String,
    ) -> bool {
        let proof_bytes = match const_hex::decode(&proof_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        if proof_bytes.starts_with(b"ZKP_ROLE_PROOF_V2:") && proof_bytes.len() == 178 {
            let actual_commitment = &proof_bytes[18..50];
            let salt_bytes = &proof_bytes[50..82];
            let signature_bytes = &proof_bytes[82..146];
            let proof_public_key = &proof_bytes[146..178];

            // Verify that the public_key matches the registered public_key_hex (if supplied)
            if !public_key_hex.is_empty() {
                let registered_public_key = match const_hex::decode(&public_key_hex) {
                    Ok(b) => b,
                    Err(_) => return false,
                };
                if proof_public_key != registered_public_key {
                    return false;
                }
            }

            // Verify the signature on commitment using the public key
            use ed25519_dalek::Verifier;
            let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
                proof_public_key.try_into().unwrap(),
            ) {
                Ok(k) => k,
                Err(_) => return false,
            };

            let signature = ed25519_dalek::Signature::from_bytes(
                signature_bytes.try_into().unwrap(),
            );

            if verifying_key.verify(actual_commitment, &signature).is_err() {
                return false;
            }

            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V2");
            hasher.update(user_id.as_bytes());
            hasher.update(role.as_bytes());
            hasher.update(salt_bytes);
            let expected_commitment = hasher.finalize();

            constant_time_eq(expected_commitment.as_bytes(), actual_commitment)
        } else if proof_bytes.starts_with(b"ZKP_ROLE_PROOF_V1:") && proof_bytes.len() == 82 {
            let actual_commitment = &proof_bytes[18..50];
            let salt_bytes = &proof_bytes[50..82];

            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V1");
            hasher.update(user_id.as_bytes());
            hasher.update(role.as_bytes());
            hasher.update(salt_bytes);
            let expected_commitment = hasher.finalize();

            constant_time_eq(expected_commitment.as_bytes(), actual_commitment)
        } else {
            false
        }
    }
}

#[inline(never)]
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}
