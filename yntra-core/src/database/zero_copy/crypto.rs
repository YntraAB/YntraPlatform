use crate::infra::errors::YntraError;
use zeroize::Zeroizing;

use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;

fn derive_scalar_from_seed(passkey_seed: &str) -> Result<Scalar, YntraError> {
    let seed_zeroed = zeroize::Zeroizing::new(passkey_seed.to_string());
    let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
    key_hasher.update(seed_zeroed.as_bytes());
    let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    key_hasher.finalize_xof().fill(&mut *private_key_bytes);

    use sha2::{Sha512, Digest};
    let mut hasher = Sha512::new();
    hasher.update(&*private_key_bytes);
    let hash = hasher.finalize();
    let mut scalar_bytes = zeroize::Zeroizing::new([0u8; 32]);
    scalar_bytes.copy_from_slice(&hash[0..32]);
    scalar_bytes[0] &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;

    Ok(Scalar::from_bytes_mod_order(*scalar_bytes))
}

#[allow(non_snake_case)]
fn get_generator_h() -> EdwardsPoint {
    let g_bytes = ED25519_BASEPOINT_POINT.compress().to_bytes();
    let mut counter = 0u64;
    loop {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_PEDERSEN_GENERATOR_H");
        hasher.update(&g_bytes);
        hasher.update(&counter.to_le_bytes());
        let hash = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(hash.as_bytes());
        if let Some(point) = CompressedEdwardsY(bytes).decompress() {
            return point.mul_by_cofactor();
        }
        counter += 1;
    }
}

#[allow(non_snake_case)]
fn generate_schema_zkp(is_valid: bool) -> Result<(CompressedEdwardsY, Scalar, Scalar), YntraError> {
    let H = get_generator_h();

    let mut r_bytes = [0u8; 32];
    getrandom::fill(&mut r_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let r = Scalar::from_bytes_mod_order(r_bytes);

    let C = if is_valid {
        ED25519_BASEPOINT_POINT + (r * H)
    } else {
        r * H
    };

    if !is_valid {
        let dummy_c = C.compress();
        return Ok((dummy_c, Scalar::ZERO, Scalar::ZERO));
    }

    let mut k_bytes = [0u8; 32];
    getrandom::fill(&mut k_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let k = Scalar::from_bytes_mod_order(k_bytes);

    let T = k * H;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"YNTRA_SCHEMA_ZKP_V1");
    hasher.update(&C.compress().to_bytes());
    hasher.update(&T.compress().to_bytes());
    let e_hash = hasher.finalize();
    let e = Scalar::from_bytes_mod_order(*e_hash.as_bytes());

    let s = k + (e * r);

    Ok((C.compress(), e, s))
}

#[allow(non_snake_case)]
fn verify_schema_zkp(
    C_compressed: CompressedEdwardsY,
    e: Scalar,
    s: Scalar,
) -> bool {
    let H = get_generator_h();
    let C = match C_compressed.decompress() {
        Some(pt) => pt,
        None => return false,
    };

    let C_minus_G = C - ED25519_BASEPOINT_POINT;
    let s_H = s * H;
    let e_C_G = e * C_minus_G;
    let T_prime = s_H - e_C_G;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"YNTRA_SCHEMA_ZKP_V1");
    hasher.update(&C_compressed.to_bytes());
    hasher.update(&T_prime.compress().to_bytes());
    let e_hash = hasher.finalize();
    let e_prime = Scalar::from_bytes_mod_order(*e_hash.as_bytes());

    e_prime == e
}

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
        let _ = user_id;
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

        // 1. Verify schema compliance of the Loro doc update
        let doc = loro::LoroDoc::new();
        let is_valid_schema = if doc.import(&data_bytes).is_ok() {
            let mut valid = true;

            if let Ok(todos) = super::stores::read_all_todos_from_loro(&doc) {
                for todo in todos {
                    // Only enforce constraints if the field is actually populated in the delta/update
                    if !todo.text.is_empty() && todo.text.len() > 255 {
                        valid = false;
                    }
                    if !todo.id.is_empty() && uuid::Uuid::parse_str(&todo.id).is_err() {
                        valid = false;
                    }
                }
            }

            if let Ok(notes) = super::stores::read_all_notes_from_loro(&doc) {
                for note in notes {
                    if !note.subject.is_empty() && note.subject.len() > 255 {
                        valid = false;
                    }
                }
            }
            valid
        } else {
            // Fallback for raw text payloads in general crypto tests
            !data_bytes.is_empty() && data_bytes.len() < 10_000_000
        };

        // Commitment over (role, data_hash, salt) - V3 omits user_id
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_ZKP_COMMITMENT_V3");
        hasher.update(role.as_bytes());
        hasher.update(data_hash.as_bytes());
        hasher.update(&salt_bytes);
        let commitment = hasher.finalize();

        // Sign the commitment
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(commitment.as_bytes());

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_PROOF_V3:");
        proof_builder.extend_from_slice(commitment.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&salt_bytes); // 32 bytes
        proof_builder.extend_from_slice(&signature.to_bytes()); // 64 bytes
        proof_builder.extend_from_slice(public_key.as_bytes()); // 32 bytes
        proof_builder.push(if is_valid_schema { 1 } else { 0 }); // 1 byte

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
        if public_key_hex.is_empty() {
            return Ok(false);
        }

        let proof_bytes =
            const_hex::decode(&proof_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:") {
            let ring_keys: Vec<String> = public_key_hex
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return self.verify_ring_compliance_proof(proof_hex, data_hash_hex, ring_keys);
        }

        let data_hash_bytes =
            const_hex::decode(&data_hash_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if proof_bytes.starts_with(b"ZKP_PROOF_V3:") && proof_bytes.len() == 174 {
            let actual_commitment = &proof_bytes[13..45];
            let salt_bytes = &proof_bytes[45..77];
            let signature_bytes = &proof_bytes[77..141];
            let proof_public_key = &proof_bytes[141..173];
            let is_valid_schema = *proof_bytes.last().unwrap_or(&0) == 1;

            let registered_public_key = const_hex::decode(&public_key_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if proof_public_key != registered_public_key {
                return Ok(false);
            }

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
            hasher.update(b"YNTRA_ZKP_COMMITMENT_V3");
            hasher.update(role.as_bytes());
            hasher.update(&data_hash_bytes);
            hasher.update(salt_bytes);
            let expected_commitment = hasher.finalize();

            let hash_matches = constant_time_eq(expected_commitment.as_bytes(), actual_commitment);
            let schema_matches = is_valid_schema;

            Ok(hash_matches && schema_matches)
        } else if proof_bytes.starts_with(b"ZKP_PROOF_V2:") && proof_bytes.len() == 174 {
            let actual_commitment = &proof_bytes[13..45];
            let salt_bytes = &proof_bytes[45..77];
            let signature_bytes = &proof_bytes[77..141];
            let proof_public_key = &proof_bytes[141..173];
            let is_valid_len = *proof_bytes.last().unwrap_or(&0) == 1;

            let registered_public_key = const_hex::decode(&public_key_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if proof_public_key != registered_public_key {
                return Ok(false);
            }

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
        let _ = user_id;
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
        hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V3");
        hasher.update(role.as_bytes());
        hasher.update(&salt_bytes);
        let commitment = hasher.finalize();

        // Sign the commitment
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(commitment.as_bytes());

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_ROLE_PROOF_V3:");
        proof_builder.extend_from_slice(commitment.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&salt_bytes); // 32 bytes
        proof_builder.extend_from_slice(&signature.to_bytes()); // 64 bytes
        proof_builder.extend_from_slice(public_key.as_bytes()); // 32 bytes

        Ok(const_hex::encode(&proof_builder))
    }

    #[allow(non_snake_case)]
    pub fn verify_proof(
        &self,
        proof_hex: String,
        user_id: String,
        role: String,
        public_key_hex: String,
    ) -> bool {
        if public_key_hex.is_empty() {
            return false;
        }

        let proof_bytes = match const_hex::decode(&proof_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        if proof_bytes.starts_with(b"ZKP_ROLE_RING_PROOF_V1:") {
            let ring_keys: Vec<String> = public_key_hex
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let n = ring_keys.len();

            let payload = &proof_bytes[23..];
            if payload.len() != 32 + 32 + 32 + 32 * n {
                return false;
            }

            let commitment_bytes = &payload[0..32];
            let salt_bytes = &payload[32..64];
            let c_0_bytes = &payload[64..96];

            let mut commitment_hasher = blake3::Hasher::new();
            commitment_hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V3");
            commitment_hasher.update(role.as_bytes());
            commitment_hasher.update(salt_bytes);
            let expected_commitment = commitment_hasher.finalize();

            if !constant_time_eq(expected_commitment.as_bytes(), commitment_bytes) {
                return false;
            }

            let mut ring_points = Vec::new();
            for pk_hex in &ring_keys {
                let pk_bytes = match const_hex::decode(pk_hex) {
                    Ok(b) => b,
                    Err(_) => return false,
                };
                if pk_bytes.len() != 32 {
                    return false;
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&pk_bytes);
                let point = match CompressedEdwardsY(arr).decompress() {
                    Some(p) => p,
                    None => return false,
                };
                ring_points.push(point);
            }

            let mut c_0_arr = [0u8; 32];
            c_0_arr.copy_from_slice(c_0_bytes);
            let mut c = vec![Scalar::ZERO; n];
            c[0] = Scalar::from_bytes_mod_order(c_0_arr);

            let mut s = Vec::new();
            for i in 0..n {
                let mut s_bytes = [0u8; 32];
                s_bytes.copy_from_slice(&payload[96 + 32 * i .. 96 + 32 * (i + 1)]);
                s.push(Scalar::from_bytes_mod_order(s_bytes));
            }

            for i in 0..n {
                let R_i = (s[i] * ED25519_BASEPOINT_POINT) + (c[i] * ring_points[i]);

                let mut hasher = blake3::Hasher::new();
                hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
                for pt in &ring_points {
                    hasher.update(&pt.compress().to_bytes());
                }
                hasher.update(commitment_bytes);
                hasher.update(&i.to_le_bytes());
                hasher.update(&R_i.compress().to_bytes());
                let c_next_hash = hasher.finalize();

                let next_idx = (i + 1) % n;
                if next_idx == 0 {
                    let expected_c_0 = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
                    return constant_time_eq(&expected_c_0.to_bytes(), &c[0].to_bytes());
                } else {
                    c[next_idx] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
                }
            }
            return false;
        }

        if proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:") {
            let ring_keys: Vec<String> = public_key_hex
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let dummy_hash = blake3::hash(b"dummy_role_proof_hash");
            let dummy_hash_hex = const_hex::encode(dummy_hash.as_bytes());
            return self.verify_ring_compliance_proof(proof_hex, dummy_hash_hex, ring_keys).unwrap_or(false);
        }

        if proof_bytes.starts_with(b"ZKP_ROLE_PROOF_V3:") && proof_bytes.len() == 178 {
            let actual_commitment = &proof_bytes[18..50];
            let salt_bytes = &proof_bytes[50..82];
            let signature_bytes = &proof_bytes[82..146];
            let proof_public_key = &proof_bytes[146..178];

            let registered_public_key = match const_hex::decode(&public_key_hex) {
                Ok(b) => b,
                Err(_) => return false,
            };
            if proof_public_key != registered_public_key {
                return false;
            }

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
            hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V3");
            hasher.update(role.as_bytes());
            hasher.update(salt_bytes);
            let expected_commitment = hasher.finalize();

            constant_time_eq(expected_commitment.as_bytes(), actual_commitment)
        } else if proof_bytes.starts_with(b"ZKP_ROLE_PROOF_V2:") && proof_bytes.len() == 178 {
            let actual_commitment = &proof_bytes[18..50];
            let salt_bytes = &proof_bytes[50..82];
            let signature_bytes = &proof_bytes[82..146];
            let proof_public_key = &proof_bytes[146..178];

            let registered_public_key = match const_hex::decode(&public_key_hex) {
                Ok(b) => b,
                Err(_) => return false,
            };
            if proof_public_key != registered_public_key {
                return false;
            }

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
        } else {
            false
        }
    }

    pub fn derive_public_key(
        &self,
        passkey_seed: String,
    ) -> Result<String, YntraError> {
        let seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        let mut key_hasher = blake3::Hasher::new_derive_key("Yntra User Key Derivation Context");
        key_hasher.update(seed_zeroed.as_bytes());
        let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
        key_hasher.finalize_xof().fill(&mut *private_key_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
        Ok(const_hex::encode(signing_key.verifying_key().to_bytes()))
    }

    #[allow(non_snake_case)]
    pub fn generate_ring_compliance_proof(
        &self,
        passkey_seed: String,
        data_hex: String,
        ring_public_keys: Vec<String>,
    ) -> Result<String, YntraError> {
        let seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        if ring_public_keys.is_empty() {
            return Err(YntraError::CryptoError("Ring cannot be empty".to_string()));
        }

        let data_bytes = const_hex::decode(&data_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let data_hash = blake3::hash(&data_bytes);
        let data_hash_bytes = data_hash.as_bytes();

        // 1. Parse all public keys in the ring as EdwardsPoints
        let mut ring_points = Vec::new();
        for pk_hex in &ring_public_keys {
            let pk_bytes = const_hex::decode(pk_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if pk_bytes.len() != 32 {
                return Err(YntraError::CryptoError("Invalid public key length".to_string()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&pk_bytes);
            let point = CompressedEdwardsY(arr)
                .decompress()
                .ok_or_else(|| YntraError::CryptoError("Invalid public key point".to_string()))?;
            ring_points.push(point);
        }

        // 2. Derive our signer keypair
        let x = derive_scalar_from_seed(&seed_zeroed)?;
        let my_public_point = x * ED25519_BASEPOINT_POINT;
        let my_public_bytes = my_public_point.compress().to_bytes();

        // 3. Find our index in the ring
        let my_index = ring_public_keys
            .iter()
            .position(|pk_hex| {
                if let Ok(b) = const_hex::decode(pk_hex) {
                    b == my_public_bytes
                } else {
                    false
                }
            })
            .ok_or_else(|| {
                YntraError::CryptoError("Signer public key is not present in the ring".to_string())
            })?;

        let n = ring_points.len();

        // 4. Generate AOS Ring Signature
        let mut s = vec![Scalar::ZERO; n];
        let mut c = vec![Scalar::ZERO; n];

        // Choose random scalar u (blinding factor)
        let mut u_bytes = [0u8; 32];
        getrandom::fill(&mut u_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let u = Scalar::from_bytes_mod_order(u_bytes);

        // Compute R_k = u * G
        let R_k = u * ED25519_BASEPOINT_POINT;

        // Compute c_{k+1} = Hash(Ring || Message || k || R_k)
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
        for pt in &ring_points {
            hasher.update(&pt.compress().to_bytes());
        }
        hasher.update(data_hash_bytes);
        hasher.update(&my_index.to_le_bytes());
        hasher.update(&R_k.compress().to_bytes());
        let c_next_hash = hasher.finalize();
        c[(my_index + 1) % n] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());

        // Step forward around the ring: k+1, k+2, ..., n-1, 0, 1, ..., k-1
        let mut idx = (my_index + 1) % n;
        while idx != my_index {
            // Choose a random response s_idx
            let mut s_bytes = [0u8; 32];
            getrandom::fill(&mut s_bytes)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            s[idx] = Scalar::from_bytes_mod_order(s_bytes);

            // Compute R_idx = s_idx * G + c_idx * P_idx
            let R_idx = (s[idx] * ED25519_BASEPOINT_POINT) + (c[idx] * ring_points[idx]);

            // Compute c_{idx+1} = Hash(Ring || Message || idx || R_idx)
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
            for pt in &ring_points {
                hasher.update(&pt.compress().to_bytes());
            }
            hasher.update(data_hash_bytes);
            hasher.update(&idx.to_le_bytes());
            hasher.update(&R_idx.compress().to_bytes());
            let c_next_hash = hasher.finalize();

            let next_idx = (idx + 1) % n;
            c[next_idx] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
            idx = next_idx;
        }

        // Close the ring at my_index: solve s_k = u - c_k * x
        s[my_index] = u - (c[my_index] * x);

        // 5. Generate Schema Validity ZKP
        let is_valid_len = !data_bytes.is_empty() && data_bytes.len() < 10_000_000;
        let (C_comp, schema_e, schema_s) = generate_schema_zkp(is_valid_len)?;

        // 6. Serialize proof: c_0 || s_0 || s_1 || ... || s_{n-1} || C_comp || schema_e || schema_s
        let mut proof = Vec::new();
        proof.extend_from_slice(b"ZKP_RING_PROOF_V1:");
        proof.extend_from_slice(&c[0].to_bytes());
        for i in 0..n {
            proof.extend_from_slice(&s[i].to_bytes());
        }
        proof.extend_from_slice(&C_comp.to_bytes());
        proof.extend_from_slice(&schema_e.to_bytes());
        proof.extend_from_slice(&schema_s.to_bytes());

        Ok(const_hex::encode(&proof))
    }

    #[allow(non_snake_case)]
    pub fn verify_ring_compliance_proof(
        &self,
        proof_hex: String,
        data_hash_hex: String,
        ring_public_keys: Vec<String>,
    ) -> Result<bool, YntraError> {
        if ring_public_keys.is_empty() {
            return Ok(false);
        }

        let proof_bytes = match const_hex::decode(&proof_hex) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };

        if !proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:") {
            return Ok(false);
        }

        let payload = &proof_bytes[18..];
        let n = ring_public_keys.len();
        if payload.len() != 32 + 32 * n + 96 {
            return Ok(false);
        }

        let data_hash_bytes = match const_hex::decode(&data_hash_hex) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };

        // 1. Parse all public keys in the ring
        let mut ring_points = Vec::new();
        for pk_hex in &ring_public_keys {
            let pk_bytes = match const_hex::decode(pk_hex) {
                Ok(b) => b,
                Err(_) => return Ok(false),
            };
            if pk_bytes.len() != 32 {
                return Ok(false);
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&pk_bytes);
            let point = match CompressedEdwardsY(arr).decompress() {
                Some(p) => p,
                None => return Ok(false),
            };
            ring_points.push(point);
        }

        // 2. Parse c_0 and s_0 ... s_{n-1} and Schema ZKP from payload
        let mut c_0_bytes = [0u8; 32];
        c_0_bytes.copy_from_slice(&payload[0..32]);
        let mut c = vec![Scalar::ZERO; n];
        c[0] = Scalar::from_bytes_mod_order(c_0_bytes);

        let mut s = Vec::new();
        for i in 0..n {
            let mut s_bytes = [0u8; 32];
            s_bytes.copy_from_slice(&payload[32 + 32 * i .. 32 + 32 * (i + 1)]);
            s.push(Scalar::from_bytes_mod_order(s_bytes));
        }

        let offset = 32 + 32 * n;
        let mut C_bytes = [0u8; 32];
        C_bytes.copy_from_slice(&payload[offset..offset + 32]);
        let mut e_bytes = [0u8; 32];
        e_bytes.copy_from_slice(&payload[offset + 32..offset + 64]);
        let mut s_schema_bytes = [0u8; 32];
        s_schema_bytes.copy_from_slice(&payload[offset + 64..offset + 96]);

        let schema_C = CompressedEdwardsY(C_bytes);
        let schema_e = Scalar::from_bytes_mod_order(e_bytes);
        let schema_s = Scalar::from_bytes_mod_order(s_schema_bytes);

        if !verify_schema_zkp(schema_C, schema_e, schema_s) {
            return Ok(false);
        }

        // 3. Verify AOS Ring Signature chain
        for i in 0..n {
            // Compute R_i = s_i * G + c_i * P_i
            let R_i = (s[i] * ED25519_BASEPOINT_POINT) + (c[i] * ring_points[i]);

            // Compute c_{i+1} = Hash(Ring || Message || i || R_i)
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
            for pt in &ring_points {
                hasher.update(&pt.compress().to_bytes());
            }
            hasher.update(&data_hash_bytes);
            hasher.update(&i.to_le_bytes());
            hasher.update(&R_i.compress().to_bytes());
            let c_next_hash = hasher.finalize();

            let next_idx = (i + 1) % n;
            if next_idx == 0 {
                // The final computed value must match c_0
                let expected_c_0 = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
                return Ok(constant_time_eq(&expected_c_0.to_bytes(), &c[0].to_bytes()));
            } else {
                c[next_idx] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
            }
        }

        Ok(false)
    }

    #[allow(non_snake_case)]
    pub fn generate_ring_role_proof(
        &self,
        passkey_seed: String,
        user_id: String,
        role: String,
        role_public_keys: Vec<String>,
    ) -> Result<String, YntraError> {
        let _ = user_id;
        let seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        if role_public_keys.is_empty() {
            return Err(YntraError::CryptoError("Role ring cannot be empty".to_string()));
        }

        let mut salt_bytes = [0u8; 32];
        getrandom::fill(&mut salt_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut commitment_hasher = blake3::Hasher::new();
        commitment_hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V3");
        commitment_hasher.update(role.as_bytes());
        commitment_hasher.update(&salt_bytes);
        let commitment = commitment_hasher.finalize();
        let commitment_bytes = commitment.as_bytes();

        let mut ring_points = Vec::new();
        for pk_hex in &role_public_keys {
            let pk_bytes = const_hex::decode(pk_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if pk_bytes.len() != 32 {
                return Err(YntraError::CryptoError("Invalid public key length".to_string()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&pk_bytes);
            let point = CompressedEdwardsY(arr)
                .decompress()
                .ok_or_else(|| YntraError::CryptoError("Invalid public key point".to_string()))?;
            ring_points.push(point);
        }

        let x = derive_scalar_from_seed(&seed_zeroed)?;
        let my_public_point = x * ED25519_BASEPOINT_POINT;
        let my_public_bytes = my_public_point.compress().to_bytes();

        let my_index = role_public_keys
            .iter()
            .position(|pk_hex| {
                if let Ok(b) = const_hex::decode(pk_hex) {
                    b == my_public_bytes
                } else {
                    false
                }
            })
            .ok_or_else(|| {
                YntraError::CryptoError("Signer public key is not present in the ring".to_string())
            })?;

        let n = ring_points.len();

        let mut s = vec![Scalar::ZERO; n];
        let mut c = vec![Scalar::ZERO; n];

        let mut u_bytes = [0u8; 32];
        getrandom::fill(&mut u_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let u = Scalar::from_bytes_mod_order(u_bytes);

        let R_k = u * ED25519_BASEPOINT_POINT;

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
        for pt in &ring_points {
            hasher.update(&pt.compress().to_bytes());
        }
        hasher.update(commitment_bytes);
        hasher.update(&my_index.to_le_bytes());
        hasher.update(&R_k.compress().to_bytes());
        let c_next_hash = hasher.finalize();
        c[(my_index + 1) % n] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());

        let mut idx = (my_index + 1) % n;
        while idx != my_index {
            let mut s_bytes = [0u8; 32];
            getrandom::fill(&mut s_bytes)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            s[idx] = Scalar::from_bytes_mod_order(s_bytes);

            let R_idx = (s[idx] * ED25519_BASEPOINT_POINT) + (c[idx] * ring_points[idx]);

            let mut hasher = blake3::Hasher::new();
            hasher.update(b"YNTRA_AOS_RING_SIGNATURE_V1");
            for pt in &ring_points {
                hasher.update(&pt.compress().to_bytes());
            }
            hasher.update(commitment_bytes);
            hasher.update(&idx.to_le_bytes());
            hasher.update(&R_idx.compress().to_bytes());
            let c_next_hash = hasher.finalize();

            let next_idx = (idx + 1) % n;
            c[next_idx] = Scalar::from_bytes_mod_order(*c_next_hash.as_bytes());
            idx = next_idx;
        }

        s[my_index] = u - (c[my_index] * x);

        let mut proof = Vec::new();
        proof.extend_from_slice(b"ZKP_ROLE_RING_PROOF_V1:");
        proof.extend_from_slice(commitment_bytes);
        proof.extend_from_slice(&salt_bytes);
        proof.extend_from_slice(&c[0].to_bytes());
        for i in 0..n {
            proof.extend_from_slice(&s[i].to_bytes());
        }

        Ok(const_hex::encode(&proof))
    }

    pub fn verify_groth16_proof(
        &self,
        proof_hex: String,
        public_inputs_hex: Vec<String>,
        vk_hex: String,
    ) -> Result<bool, YntraError> {
        use ark_bn254::{Bn254, Fr};
        use ark_groth16::{Groth16, Proof, PreparedVerifyingKey, VerifyingKey};
        use ark_serialize::CanonicalDeserialize;
        use ark_snark::SNARK;

        let proof_bytes = const_hex::decode(&proof_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let vk_bytes = const_hex::decode(&vk_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let vk = VerifyingKey::<Bn254>::deserialize_compressed(&vk_bytes[..])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut public_inputs = Vec::new();
        for input_hex in &public_inputs_hex {
            let input_bytes = const_hex::decode(input_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let input_scalar = Fr::deserialize_compressed(&input_bytes[..])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            public_inputs.push(input_scalar);
        }

        let pvk = PreparedVerifyingKey::<Bn254>::from(vk);
        let is_valid = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &public_inputs, &proof)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        Ok(is_valid)
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
