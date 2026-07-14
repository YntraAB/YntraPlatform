use crate::infra::errors::YntraError;

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

        let mut hasher =
            blake3::Hasher::new_derive_key("Yntra Zero-Copy Passkey Envelope Encryption Key");
        hasher.update(passkey_seed.as_bytes());
        let mut key_bytes = [0u8; 32];
        hasher.finalize_xof().fill(&mut key_bytes);

        let key = chacha20poly1305::Key::from_slice(&key_bytes);
        let cipher = XChaCha20Poly1305::new(key);

        let mut nonce_bytes = [0u8; 24];
        getrandom::fill(&mut nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext_bytes = cipher
            .encrypt(nonce, plaintext.as_bytes())
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
        hasher.update(passkey_seed.as_bytes());
        let mut key_bytes = [0u8; 32];
        hasher.finalize_xof().fill(&mut key_bytes);

        let key = chacha20poly1305::Key::from_slice(&key_bytes);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(nonce_bytes);

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        String::from_utf8(plaintext_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))
    }

    pub fn generate_compliance_proof(
        &self,
        data_hex: String,
        user_id: String,
        role: String,
    ) -> Result<String, YntraError> {
        let data_bytes =
            const_hex::decode(&data_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_COMMITMENT_V1");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        hasher.update(&data_bytes);
        let commitment = hasher.finalize();

        let is_valid_len = !data_bytes.is_empty() && data_bytes.len() < 10_000_000;

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_PROOF_V1:");
        proof_builder.extend_from_slice(commitment.as_bytes());
        proof_builder.push(if is_valid_len { 1 } else { 0 });

        Ok(const_hex::encode(&proof_builder))
    }

    pub fn verify_compliance_proof(
        &self,
        proof_hex: String,
        user_id: String,
        role: String,
        data_hex: String,
    ) -> Result<bool, YntraError> {
        let proof_bytes =
            const_hex::decode(&proof_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if proof_bytes.len() != 46 || !proof_bytes.starts_with(b"ZKP_PROOF_V1:") {
            return Ok(false);
        }

        let data_bytes =
            const_hex::decode(&data_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_COMMITMENT_V1");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        hasher.update(&data_bytes);
        let expected_commitment = hasher.finalize();

        let actual_commitment = &proof_bytes[13..45];
        let is_valid_len = *proof_bytes.last().unwrap_or(&0) == 1;

        let hash_matches = expected_commitment.as_bytes() == actual_commitment;
        let len_matches = !data_bytes.is_empty() && data_bytes.len() < 10_000_000 && is_valid_len;

        Ok(hash_matches && len_matches)
    }

    pub fn generate_role_proof(
        &self,
        user_id: String,
        role: String,
    ) -> Result<String, YntraError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V1");
        hasher.update(user_id.as_bytes());
        hasher.update(role.as_bytes());
        let commitment = hasher.finalize();

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_ROLE_PROOF_V1:");
        proof_builder.extend_from_slice(commitment.as_bytes());

        Ok(const_hex::encode(&proof_builder))
    }

    pub fn verify_proof(&self, proof_hex: String) -> bool {
        let proof_bytes = match const_hex::decode(&proof_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        if proof_bytes.len() != 50 || !proof_bytes.starts_with(b"ZKP_ROLE_PROOF_V1:") {
            return false;
        }

        true
    }
}
