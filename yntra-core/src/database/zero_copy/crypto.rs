use crate::infra::errors::YntraError;
use zeroize::Zeroizing;

use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;

fn derive_scalar_from_seed(
    passkey_seed: &zeroize::Zeroizing<String>,
) -> Result<Scalar, YntraError> {
    let context_str = crate::infra::crypto::CryptoDomain::UserKeyDerivation.get_context(1)?;
    let mut key_hasher = blake3::Hasher::new_derive_key(context_str);
    key_hasher.update(passkey_seed.as_bytes());
    let mut private_key_bytes = zeroize::Zeroizing::new([0u8; 32]);
    key_hasher.finalize_xof().fill(&mut *private_key_bytes);

    use sha2::{Digest, Sha512};
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
    static GENERATOR_H: std::sync::OnceLock<EdwardsPoint> = std::sync::OnceLock::new();
    *GENERATOR_H.get_or_init(|| {
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
    })
}

#[allow(non_snake_case)]
fn generate_schema_zkp(
    is_valid: bool,
    commitment: &[u8; 32],
) -> Result<(CompressedEdwardsY, Scalar, Scalar), YntraError> {
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
    hasher.update(b"YNTRA_SCHEMA_ZKP_V2");
    hasher.update(&C.compress().to_bytes());
    hasher.update(&T.compress().to_bytes());
    hasher.update(commitment);
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
    commitment: &[u8; 32],
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
    hasher.update(b"YNTRA_SCHEMA_ZKP_V2");
    hasher.update(&C_compressed.to_bytes());
    hasher.update(&T_prime.compress().to_bytes());
    hasher.update(commitment);
    let e_hash = hasher.finalize();
    let e_prime = Scalar::from_bytes_mod_order(*e_hash.as_bytes());

    e_prime == e
}

pub fn derive_escrow_keypair_from_seed(
    escrow_seed: &str,
) -> Result<(ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey), YntraError> {
    let private_key_bytes = match const_hex::decode(escrow_seed) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            Zeroizing::new(arr)
        }
        _ => {
            let escrow_seed_zeroed = Zeroizing::new(escrow_seed.to_string());
            let context_str =
                crate::infra::crypto::CryptoDomain::InstitutionalEscrowKeyDerivation.get_context(1)?;
            let mut key_hasher = blake3::Hasher::new_derive_key(context_str);
            key_hasher.update(escrow_seed_zeroed.as_bytes());
            let mut pk_bytes = Zeroizing::new([0u8; 32]);
            key_hasher.finalize_xof().fill(&mut *pk_bytes);
            pk_bytes
        }
    };
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
    let verifying_key = signing_key.verifying_key();
    Ok((signing_key, verifying_key))
}

// --- Pillar 3: Zero-Knowledge Cryptographic Trust (Passkey + ZKP + Break-Glass Recovery) ---

#[derive(Clone, Debug, uniffi::Record)]
pub struct BreakGlassResult {
    pub plaintext: String,
    pub audit_proof_hex: String,
    pub operator_id: String,
    pub emergency_reason: String,
    pub timestamp: u64,
}

#[derive(Clone, uniffi::Object)]
pub struct ZkCryptoTrust {}

#[uniffi::export]
impl ZkCryptoTrust {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt_workspace_field_with_escrow(
        &self,
        passkey_seed: String,
        plaintext: String,
        escrow_public_key_hex: String,
    ) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};

        let passkey_seed_zeroed = Zeroizing::new(passkey_seed);
        let plaintext_zeroed = Zeroizing::new(plaintext);

        // 1. Generate random 32-byte DEK (Data Encryption Key)
        let mut dek_bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut *dek_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        // 2. Encrypt plaintext payload using DEK
        let key = chacha20poly1305::Key::from_slice(&*dek_bytes);
        let cipher_dek = XChaCha20Poly1305::new(key);
        let mut payload_nonce_bytes = [0u8; 24];
        getrandom::fill(&mut payload_nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let payload_nonce = XNonce::from_slice(&payload_nonce_bytes);
        let payload_ciphertext = cipher_dek
            .encrypt(payload_nonce, plaintext_zeroed.as_bytes())
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        // 3. Wrap DEK for User (Passkey Key Wrap)
        let user_context_str =
            crate::infra::crypto::CryptoDomain::PasskeyEnvelopeEncryption.get_context(1)?;
        let mut user_key_hasher = blake3::Hasher::new_derive_key(user_context_str);
        user_key_hasher.update(passkey_seed_zeroed.as_bytes());
        let mut user_key_bytes = Zeroizing::new([0u8; 32]);
        user_key_hasher.finalize_xof().fill(&mut *user_key_bytes);

        let user_wrap_cipher =
            XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&*user_key_bytes));
        let mut user_wrap_nonce_bytes = [0u8; 24];
        getrandom::fill(&mut user_wrap_nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let user_wrap_nonce = XNonce::from_slice(&user_wrap_nonce_bytes);
        let enc_user_dek = user_wrap_cipher
            .encrypt(user_wrap_nonce, dek_bytes.as_slice())
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut user_wrap_payload = Vec::new();
        user_wrap_payload.extend_from_slice(&user_wrap_nonce_bytes);
        user_wrap_payload.extend_from_slice(&enc_user_dek);

        // 4. Wrap DEK for Institutional Escrow (Break-Glass Key Wrap)
        let escrow_pk_bytes = const_hex::decode(&escrow_public_key_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        if escrow_pk_bytes.len() != 32 {
            return Err(YntraError::CryptoError(
                "Invalid escrow public key length".to_string(),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&escrow_pk_bytes);
        let escrow_point = CompressedEdwardsY(arr)
            .decompress()
            .ok_or_else(|| YntraError::CryptoError("Invalid escrow public key point".to_string()))?;

        let mut ephem_r_bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut *ephem_r_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let ephem_scalar = Scalar::from_bytes_mod_order(*ephem_r_bytes);
        let ephem_pub_point = ephem_scalar * ED25519_BASEPOINT_POINT;
        let ephem_pub_bytes = ephem_pub_point.compress().to_bytes();

        let dh_point = ephem_scalar * escrow_point;
        let dh_bytes = dh_point.compress().to_bytes();

        let escrow_context_str =
            crate::infra::crypto::CryptoDomain::BreakGlassEnvelopeEncryption.get_context(1)?;
        let mut escrow_key_hasher = blake3::Hasher::new_derive_key(escrow_context_str);
        escrow_key_hasher.update(&dh_bytes);
        let mut escrow_key_bytes = Zeroizing::new([0u8; 32]);
        escrow_key_hasher.finalize_xof().fill(&mut *escrow_key_bytes);

        let escrow_wrap_cipher =
            XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&*escrow_key_bytes));
        let mut escrow_wrap_nonce_bytes = [0u8; 24];
        getrandom::fill(&mut escrow_wrap_nonce_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let escrow_wrap_nonce = XNonce::from_slice(&escrow_wrap_nonce_bytes);
        let enc_escrow_dek = escrow_wrap_cipher
            .encrypt(escrow_wrap_nonce, dek_bytes.as_slice())
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let mut break_glass_wrap_payload = Vec::new();
        break_glass_wrap_payload.extend_from_slice(&ephem_pub_bytes);
        break_glass_wrap_payload.extend_from_slice(&escrow_wrap_nonce_bytes);
        break_glass_wrap_payload.extend_from_slice(&enc_escrow_dek);

        Ok(format!(
            "zero_copy_escrow_v1:{}:{}:{}:{}",
            const_hex::encode(&user_wrap_payload),
            const_hex::encode(&break_glass_wrap_payload),
            const_hex::encode(&payload_nonce_bytes),
            const_hex::encode(&payload_ciphertext)
        ))
    }

    pub fn encrypt_workspace_field_for_workspace(
        &self,
        passkey_seed: String,
        plaintext: String,
        workspace_id: String,
    ) -> Result<String, YntraError> {
        let context_str =
            crate::infra::crypto::CryptoDomain::InstitutionalEscrowKeyDerivation.get_context(1)?;
        let mut hasher = blake3::Hasher::new_derive_key(context_str);
        hasher.update(workspace_id.as_bytes());
        let mut private_key_bytes = Zeroizing::new([0u8; 32]);
        hasher.finalize_xof().fill(&mut *private_key_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
        let escrow_pk_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

        self.encrypt_workspace_field_with_escrow(passkey_seed, plaintext, escrow_pk_hex)
    }

    pub fn encrypt_workspace_field(
        &self,
        passkey_seed: String,
        plaintext: String,
    ) -> Result<String, YntraError> {
        let default_escrow_seed = "YNTRA_DEFAULT_INSTITUTIONAL_ESCROW_MASTER_SEED";
        let (_, escrow_vk) = derive_escrow_keypair_from_seed(default_escrow_seed)?;
        let escrow_pk_hex = const_hex::encode(escrow_vk.to_bytes());
        self.encrypt_workspace_field_with_escrow(passkey_seed, plaintext, escrow_pk_hex)
    }

    pub fn generate_threshold_escrow_shares(
        &self,
        master_seed: String,
        k: u32,
        n: u32,
    ) -> Result<Vec<String>, YntraError> {
        if k < 1 || n < k || n > 255 {
            return Err(YntraError::CryptoError(
                "Invalid threshold parameters: require 1 <= k <= n <= 255".to_string(),
            ));
        }
        let mut arr = [0u8; 32];
        let bytes = master_seed.as_bytes();
        let copy_len = bytes.len().min(32);
        arr[..copy_len].copy_from_slice(&bytes[..copy_len]);
        let secret_scalar = Scalar::from_bytes_mod_order(arr);

        let mut coefficients = Vec::with_capacity(k as usize);
        coefficients.push(secret_scalar);
        for _ in 1..k {
            let mut r_bytes = [0u8; 32];
            getrandom::fill(&mut r_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
            coefficients.push(Scalar::from_bytes_mod_order(r_bytes));
        }

        let mut shares = Vec::with_capacity(n as usize);
        for x_idx in 1..=n {
            let x_scalar = Scalar::from(x_idx as u64);
            let mut y = Scalar::ZERO;
            let mut x_pow = Scalar::ONE;
            for coeff in &coefficients {
                y += coeff * x_pow;
                x_pow *= x_scalar;
            }
            shares.push(format!(
                "THRESHOLD_SHARE_V1:{}:{}",
                x_idx,
                const_hex::encode(y.to_bytes())
            ));
        }

        Ok(shares)
    }

    pub fn combine_threshold_escrow_shares(
        &self,
        shares: Vec<String>,
    ) -> Result<String, YntraError> {
        if shares.is_empty() {
            return Err(YntraError::CryptoError(
                "No threshold shares provided".to_string(),
            ));
        }

        let mut parsed_shares: Vec<(u64, Scalar)> = Vec::new();
        for share_str in &shares {
            if !share_str.starts_with("THRESHOLD_SHARE_V1:") {
                return Err(YntraError::CryptoError(
                    "Invalid threshold share format".to_string(),
                ));
            }
            let parts: Vec<&str> = share_str.split(':').collect();
            if parts.len() != 3 {
                return Err(YntraError::CryptoError(
                    "Invalid threshold share components".to_string(),
                ));
            }
            let x_idx: u64 = parts[1]
                .parse()
                .map_err(|_| YntraError::CryptoError("Invalid share index".to_string()))?;
            let y_bytes = const_hex::decode(parts[2])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if y_bytes.len() != 32 {
                return Err(YntraError::CryptoError("Invalid y scalar length".to_string()));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&y_bytes);
            let y_scalar = Scalar::from_bytes_mod_order(arr);
            parsed_shares.push((x_idx, y_scalar));
        }

        let k = parsed_shares.len();
        let mut secret = Scalar::ZERO;
        for j in 0..k {
            let (x_j_idx, y_j) = parsed_shares[j];
            let x_j = Scalar::from(x_j_idx);
            let mut num = Scalar::ONE;
            let mut den = Scalar::ONE;

            for m in 0..k {
                if m == j {
                    continue;
                }
                let (x_m_idx, _) = parsed_shares[m];
                let x_m = Scalar::from(x_m_idx);
                num *= -x_m;
                den *= x_j - x_m;
            }

            let den_inv_opt: Option<Scalar> = den.invert().into();
            if den_inv_opt.is_none() {
                return Err(YntraError::CryptoError(
                    "Duplicate share index in threshold reconstruction".to_string(),
                ));
            }
            let l_j = num * den_inv_opt.unwrap();
            secret += y_j * l_j;
        }

        let secret_bytes = secret.to_bytes();
        let trimmed_bytes = secret_bytes.iter().copied().take_while(|&b| b != 0).collect::<Vec<u8>>();
        if let Ok(utf8_str) = String::from_utf8(trimmed_bytes.clone()) {
            if !utf8_str.is_empty() {
                return Ok(utf8_str);
            }
        }
        Ok(const_hex::encode(secret_bytes))
    }

    pub fn decrypt_workspace_field(
        &self,
        passkey_seed: String,
        ciphertext_hex: String,
    ) -> Result<String, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};

        if ciphertext_hex.starts_with("zero_copy_escrow_v1:") {
            let parts: Vec<&str> = ciphertext_hex.split(':').collect();
            if parts.len() != 5 {
                return Err(YntraError::CryptoError(
                    "Invalid zero_copy_escrow_v1 envelope structure".to_string(),
                ));
            }
            let user_wrap_bytes = const_hex::decode(parts[1])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let payload_nonce_bytes = const_hex::decode(parts[3])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let payload_ciphertext_bytes = const_hex::decode(parts[4])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            if user_wrap_bytes.len() < 24 + 32 {
                return Err(YntraError::CryptoError(
                    "Invalid user wrap length".to_string(),
                ));
            }

            let user_wrap_nonce_bytes = &user_wrap_bytes[0..24];
            let enc_user_dek = &user_wrap_bytes[24..];

            let passkey_seed_zeroed = Zeroizing::new(passkey_seed);
            let user_context_str =
                crate::infra::crypto::CryptoDomain::PasskeyEnvelopeEncryption.get_context(1)?;
            let mut user_key_hasher = blake3::Hasher::new_derive_key(user_context_str);
            user_key_hasher.update(passkey_seed_zeroed.as_bytes());
            let mut user_key_bytes = Zeroizing::new([0u8; 32]);
            user_key_hasher.finalize_xof().fill(&mut *user_key_bytes);

            let user_wrap_cipher =
                XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&*user_key_bytes));
            let dek_bytes = user_wrap_cipher
                .decrypt(XNonce::from_slice(user_wrap_nonce_bytes), enc_user_dek)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            if dek_bytes.len() != 32 {
                return Err(YntraError::CryptoError("Invalid DEK length".to_string()));
            }

            let cipher_dek =
                XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&dek_bytes));
            let plaintext_bytes = cipher_dek
                .decrypt(
                    XNonce::from_slice(&payload_nonce_bytes),
                    payload_ciphertext_bytes.as_slice(),
                )
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            return String::from_utf8(plaintext_bytes)
                .map_err(|e| YntraError::CryptoError(e.to_string()));
        }

        // Backward compatibility for legacy single-recipient payloads
        let raw_hex = if ciphertext_hex.starts_with("zero_copy_enc:") {
            let parts: Vec<&str> = ciphertext_hex.split(':').collect();
            if parts.len() == 3 {
                parts[2]
            } else {
                &ciphertext_hex
            }
        } else {
            &ciphertext_hex
        };

        let passkey_seed_zeroed = Zeroizing::new(passkey_seed);
        let payload = match const_hex::decode(raw_hex) {
            Ok(p) => p,
            Err(e) => {
                return Err(YntraError::CryptoError(e.to_string()));
            }
        };

        if payload.len() < 24 {
            return Err(YntraError::CryptoError(
                "Invalid ciphertext payload length".to_string(),
            ));
        }

        let nonce_bytes = &payload[0..24];
        let ciphertext_bytes = &payload[24..];

        let context_str =
            crate::infra::crypto::CryptoDomain::PasskeyEnvelopeEncryption.get_context(1)?;
        let mut hasher = blake3::Hasher::new_derive_key(context_str);
        hasher.update(passkey_seed_zeroed.as_bytes());
        let mut key_bytes = Zeroizing::new([0u8; 32]);
        hasher.finalize_xof().fill(&mut *key_bytes);

        let key = chacha20poly1305::Key::from_slice(&*key_bytes);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(nonce_bytes);

        let decrypted_bytes = cipher.decrypt(nonce, ciphertext_bytes);
        let decrypted_bytes =
            decrypted_bytes.map_err(|e| YntraError::CryptoError(e.to_string()))?;

        String::from_utf8(decrypted_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))
    }

    pub fn decrypt_workspace_field_break_glass(
        &self,
        escrow_private_seed: String,
        ciphertext_hex: String,
        operator_id: String,
        patient_id: String,
        emergency_reason: String,
    ) -> Result<BreakGlassResult, YntraError> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};

        if operator_id.trim().is_empty()
            || patient_id.trim().is_empty()
            || emergency_reason.trim().is_empty()
        {
            return Err(YntraError::CryptoError(
                "Pre-decryption gatekeeping failed: operator_id, patient_id, and emergency_reason must be non-empty".to_string(),
            ));
        }

        if !ciphertext_hex.starts_with("zero_copy_escrow_v1:") {
            return Err(YntraError::CryptoError(
                "Break-glass decryption requires zero_copy_escrow_v1 envelope payload".to_string(),
            ));
        }

        let parts: Vec<&str> = ciphertext_hex.split(':').collect();
        if parts.len() != 5 {
            return Err(YntraError::CryptoError(
                "Invalid zero_copy_escrow_v1 envelope structure".to_string(),
            ));
        }

        let break_glass_wrap_bytes = const_hex::decode(parts[2])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let payload_nonce_bytes = const_hex::decode(parts[3])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let payload_ciphertext_bytes = const_hex::decode(parts[4])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if break_glass_wrap_bytes.len() < 32 + 24 + 32 {
            return Err(YntraError::CryptoError(
                "Invalid break_glass_wrap payload length".to_string(),
            ));
        }

        let ephem_pub_bytes = &break_glass_wrap_bytes[0..32];
        let escrow_wrap_nonce_bytes = &break_glass_wrap_bytes[32..56];
        let enc_escrow_dek = &break_glass_wrap_bytes[56..];

        let (escrow_signing_key, _) = derive_escrow_keypair_from_seed(&escrow_private_seed)?;
        let escrow_private_bytes = escrow_signing_key.to_bytes();

        use sha2::{Digest, Sha512};
        let mut hasher = Sha512::new();
        hasher.update(&escrow_private_bytes);
        let hash = hasher.finalize();
        let mut scalar_bytes = Zeroizing::new([0u8; 32]);
        scalar_bytes.copy_from_slice(&hash[0..32]);
        scalar_bytes[0] &= 248;
        scalar_bytes[31] &= 127;
        scalar_bytes[31] |= 64;
        let escrow_scalar = Scalar::from_bytes_mod_order(*scalar_bytes);

        let mut ephem_arr = [0u8; 32];
        ephem_arr.copy_from_slice(ephem_pub_bytes);
        let ephem_pub_point = CompressedEdwardsY(ephem_arr)
            .decompress()
            .ok_or_else(|| YntraError::CryptoError("Invalid ephemeral public point".to_string()))?;

        let dh_point = escrow_scalar * ephem_pub_point;
        let dh_bytes = dh_point.compress().to_bytes();

        let escrow_context_str =
            crate::infra::crypto::CryptoDomain::BreakGlassEnvelopeEncryption.get_context(1)?;
        let mut escrow_key_hasher = blake3::Hasher::new_derive_key(escrow_context_str);
        escrow_key_hasher.update(&dh_bytes);
        let mut escrow_key_bytes = Zeroizing::new([0u8; 32]);
        escrow_key_hasher.finalize_xof().fill(&mut *escrow_key_bytes);

        let escrow_wrap_cipher =
            XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&*escrow_key_bytes));
        let dek_bytes = escrow_wrap_cipher
            .decrypt(XNonce::from_slice(escrow_wrap_nonce_bytes), enc_escrow_dek)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if dek_bytes.len() != 32 {
            return Err(YntraError::CryptoError("Invalid DEK length in break glass".to_string()));
        }

        let cipher_dek = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&dek_bytes));
        let plaintext_bytes = cipher_dek
            .decrypt(
                XNonce::from_slice(&payload_nonce_bytes),
                payload_ciphertext_bytes.as_slice(),
            )
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let plaintext = String::from_utf8(plaintext_bytes)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let dek_hash = blake3::hash(&dek_bytes);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let audit_proof_hex = self.generate_break_glass_audit_proof_with_key(
            &escrow_signing_key,
            &operator_id,
            &patient_id,
            &emergency_reason,
            timestamp,
            dek_hash.as_bytes(),
        )?;

        Ok(BreakGlassResult {
            plaintext,
            audit_proof_hex,
            operator_id,
            emergency_reason,
            timestamp,
        })
    }

    pub fn decrypt_workspace_field_break_glass_threshold(
        &self,
        threshold_shares: Vec<String>,
        ciphertext_hex: String,
        operator_id: String,
        patient_id: String,
        emergency_reason: String,
    ) -> Result<BreakGlassResult, YntraError> {
        if operator_id.trim().is_empty()
            || patient_id.trim().is_empty()
            || emergency_reason.trim().is_empty()
        {
            return Err(YntraError::CryptoError(
                "Pre-decryption gatekeeping failed: operator_id, patient_id, and emergency_reason must be non-empty".to_string(),
            ));
        }

        let reconstructed_seed =
            Zeroizing::new(self.combine_threshold_escrow_shares(threshold_shares)?);
        self.decrypt_workspace_field_break_glass(
            reconstructed_seed.to_string(),
            ciphertext_hex,
            operator_id,
            patient_id,
            emergency_reason,
        )
    }


    pub fn generate_break_glass_audit_proof(
        &self,
        escrow_private_seed: String,
        operator_id: String,
        patient_id: String,
        emergency_reason: String,
        dek_hash_hex: String,
    ) -> Result<String, YntraError> {
        let (escrow_signing_key, _) = derive_escrow_keypair_from_seed(&escrow_private_seed)?;
        let dek_hash_bytes = const_hex::decode(&dek_hash_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.generate_break_glass_audit_proof_with_key(
            &escrow_signing_key,
            &operator_id,
            &patient_id,
            &emergency_reason,
            timestamp,
            &dek_hash_bytes,
        )
    }

    pub fn verify_break_glass_audit_proof(
        &self,
        audit_proof_hex: String,
        operator_id: String,
        patient_id: String,
        emergency_reason: String,
        dek_hash_hex: String,
        escrow_public_key_hex: String,
    ) -> Result<bool, YntraError> {
        if !audit_proof_hex.starts_with("ZKP_BREAK_GLASS_AUDIT_V1:") {
            return Ok(false);
        }
        let parts: Vec<&str> = audit_proof_hex.split(':').collect();
        if parts.len() != 5 {
            return Ok(false);
        }
        let commitment_bytes = match const_hex::decode(parts[1]) {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };
        let timestamp: u64 = match parts[2].parse() {
            Ok(t) => t,
            Err(_) => return Ok(false),
        };
        let signature_bytes = match const_hex::decode(parts[3]) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        let proof_pubkey_hex = parts[4];

        if !escrow_public_key_hex.is_empty() && proof_pubkey_hex != escrow_public_key_hex {
            return Ok(false);
        }

        let dek_hash_bytes = match const_hex::decode(&dek_hash_hex) {
            Ok(d) => d,
            Err(_) => return Ok(false),
        };

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_BREAK_GLASS_AUDIT_COMMITMENT_V1");
        hasher.update(operator_id.as_bytes());
        hasher.update(patient_id.as_bytes());
        hasher.update(emergency_reason.as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&dek_hash_bytes);
        let expected_commitment = hasher.finalize();

        if !constant_time_eq(expected_commitment.as_bytes(), &commitment_bytes) {
            return Ok(false);
        }

        use ed25519_dalek::Verifier;
        let pubkey_bytes = match const_hex::decode(proof_pubkey_hex) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };
        if pubkey_bytes.len() != 32 {
            return Ok(false);
        }
        let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(pubkey_bytes.as_slice().try_into().unwrap()) {
            Ok(vk) => vk,
            Err(_) => return Ok(false),
        };
        let signature = match ed25519_dalek::Signature::from_slice(signature_bytes.as_slice()) {
            Ok(sig) => sig,
            Err(_) => return Ok(false),
        };

        Ok(verifying_key.verify(&commitment_bytes, &signature).is_ok())
    }
}

impl ZkCryptoTrust {
    fn generate_break_glass_audit_proof_with_key(
        &self,
        escrow_signing_key: &ed25519_dalek::SigningKey,
        operator_id: &str,
        patient_id: &str,
        emergency_reason: &str,
        timestamp: u64,
        dek_hash_bytes: &[u8],
    ) -> Result<String, YntraError> {
        use ed25519_dalek::Signer;

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"YNTRA_BREAK_GLASS_AUDIT_COMMITMENT_V1");
        hasher.update(operator_id.as_bytes());
        hasher.update(patient_id.as_bytes());
        hasher.update(emergency_reason.as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(dek_hash_bytes);
        let commitment_hash = hasher.finalize();
        let signature = escrow_signing_key.sign(commitment_hash.as_bytes());

        let proof_str = format!(
            "ZKP_BREAK_GLASS_AUDIT_V1:{}:{}:{}:{}",
            const_hex::encode(commitment_hash.as_bytes()),
            timestamp,
            const_hex::encode(signature.to_bytes()),
            const_hex::encode(escrow_signing_key.verifying_key().as_bytes())
        );

        Ok(proof_str)
    }

    pub fn generate_compliance_proof(
        &self,
        passkey_seed: String,
        data_hex: String,
        user_id: String,
        role: String,
    ) -> Result<String, YntraError> {
        let _ = user_id;
        let passkey_seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        if data_hex.len() > 10_000_000 {
            return Err(YntraError::CryptoError(
                "Data payload is too large".to_string(),
            ));
        }
        let data_bytes = match const_hex::decode(&data_hex) {
            Ok(d) => d,
            Err(_) => data_hex.as_bytes().to_vec(),
        };

        let data_hash = blake3::hash(&data_bytes);

        // Generate a random 32-byte salt (blinding factor)
        let mut salt_bytes = [0u8; 32];
        if let Err(e) = getrandom::fill(&mut salt_bytes) {
            return Err(YntraError::CryptoError(e.to_string()));
        }

        // Derive user Ed25519 signing key from passkey_seed using blake3 derive_key
        let context_str = crate::infra::crypto::CryptoDomain::UserKeyDerivation.get_context(1)?;
        let mut key_hasher = blake3::Hasher::new_derive_key(context_str);
        key_hasher.update(passkey_seed_zeroed.as_bytes());
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

        // 5. Generate Schema Validity ZKP using the Schnorr-like sigma protocol
        let (c_comp, schema_e, schema_s) =
            generate_schema_zkp(is_valid_schema, commitment.as_bytes())?;

        let mut proof_builder = Vec::new();
        proof_builder.extend_from_slice(b"ZKP_PROOF_V3:");
        proof_builder.extend_from_slice(commitment.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&salt_bytes); // 32 bytes
        proof_builder.extend_from_slice(&signature.to_bytes()); // 64 bytes
        proof_builder.extend_from_slice(public_key.as_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&c_comp.to_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&schema_e.to_bytes()); // 32 bytes
        proof_builder.extend_from_slice(&schema_s.to_bytes()); // 32 bytes

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

        let data_hash_bytes = match const_hex::decode(&data_hash_hex) {
            Ok(d) => d,
            Err(_) => data_hash_hex.as_bytes().to_vec(),
        };

        if proof_bytes.starts_with(b"ZKP_PROOF_V3:") && proof_bytes.len() == 269 {
            let actual_commitment = &proof_bytes[13..45];
            let salt_bytes = &proof_bytes[45..77];
            let signature_bytes = &proof_bytes[77..141];
            let proof_public_key = &proof_bytes[141..173];

            let schema_c_bytes = &proof_bytes[173..205];
            let schema_e_bytes = &proof_bytes[205..237];
            let schema_s_bytes = &proof_bytes[237..269];

            let mut arr = [0u8; 32];
            arr.copy_from_slice(schema_c_bytes);
            let schema_c = CompressedEdwardsY(arr);
            let schema_e = Scalar::from_bytes_mod_order(schema_e_bytes.try_into().unwrap());
            let schema_s = Scalar::from_bytes_mod_order(schema_s_bytes.try_into().unwrap());

            let mut actual_commitment_arr = [0u8; 32];
            actual_commitment_arr.copy_from_slice(actual_commitment);
            // Verify schema ZKP using verify_schema_zkp
            let is_schema_valid =
                verify_schema_zkp(schema_c, schema_e, schema_s, &actual_commitment_arr);

            let registered_public_key = const_hex::decode(&public_key_hex)
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            if proof_public_key != registered_public_key {
                return Ok(false);
            }

            use ed25519_dalek::Verifier;
            let verifying_key =
                ed25519_dalek::VerifyingKey::from_bytes(proof_public_key.try_into().unwrap())
                    .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            let signature =
                ed25519_dalek::Signature::from_bytes(signature_bytes.try_into().unwrap());

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

            Ok(hash_matches && is_schema_valid)
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
            let verifying_key =
                ed25519_dalek::VerifyingKey::from_bytes(proof_public_key.try_into().unwrap())
                    .map_err(|e| YntraError::CryptoError(e.to_string()))?;

            let signature =
                ed25519_dalek::Signature::from_bytes(signature_bytes.try_into().unwrap());

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
        let passkey_seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        let mut salt_bytes = [0u8; 32];
        if let Err(e) = getrandom::fill(&mut salt_bytes) {
            return Err(YntraError::CryptoError(e.to_string()));
        }

        // Derive user Ed25519 signing key from passkey_seed using blake3 derive_key
        let context_str = crate::infra::crypto::CryptoDomain::UserKeyDerivation.get_context(1)?;
        let mut key_hasher = blake3::Hasher::new_derive_key(context_str);
        key_hasher.update(passkey_seed_zeroed.as_bytes());
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
                s_bytes.copy_from_slice(&payload[96 + 32 * i..96 + 32 * (i + 1)]);
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
            let verifying_key =
                match ed25519_dalek::VerifyingKey::from_bytes(proof_public_key.try_into().unwrap())
                {
                    Ok(k) => k,
                    Err(_) => return false,
                };

            let signature =
                ed25519_dalek::Signature::from_bytes(signature_bytes.try_into().unwrap());

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
            let verifying_key =
                match ed25519_dalek::VerifyingKey::from_bytes(proof_public_key.try_into().unwrap())
                {
                    Ok(k) => k,
                    Err(_) => return false,
                };

            let signature =
                ed25519_dalek::Signature::from_bytes(signature_bytes.try_into().unwrap());

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

    pub fn derive_public_key(&self, passkey_seed: String) -> Result<String, YntraError> {
        let passkey_seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        let context_str = crate::infra::crypto::CryptoDomain::UserKeyDerivation.get_context(1)?;
        let mut key_hasher = blake3::Hasher::new_derive_key(context_str);
        key_hasher.update(passkey_seed_zeroed.as_bytes());
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
        let passkey_seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        if ring_public_keys.is_empty() {
            return Err(YntraError::CryptoError("Ring cannot be empty".to_string()));
        }

        if data_hex.len() > 10_000_000 {
            return Err(YntraError::CryptoError(
                "Data payload is too large".to_string(),
            ));
        }

        let data_bytes = match const_hex::decode(&data_hex) {
            Ok(d) => d,
            Err(e) => {
                return Err(YntraError::CryptoError(e.to_string()));
            }
        };
        let data_hash = blake3::hash(&data_bytes);
        let data_hash_bytes = data_hash.as_bytes();

        // 1. Parse all public keys in the ring as EdwardsPoints
        let mut ring_points = Vec::new();
        for pk_hex in &ring_public_keys {
            let pk_bytes = match const_hex::decode(pk_hex) {
                Ok(b) => b,
                Err(e) => {
                    return Err(YntraError::CryptoError(e.to_string()));
                }
            };
            if pk_bytes.len() != 32 {
                return Err(YntraError::CryptoError(
                    "Invalid public key length".to_string(),
                ));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&pk_bytes);
            let point = match CompressedEdwardsY(arr).decompress() {
                Some(p) => p,
                None => {
                    return Err(YntraError::CryptoError(
                        "Invalid public key point".to_string(),
                    ));
                }
            };
            ring_points.push(point);
        }

        // 2. Derive our signer keypair
        let x = match derive_scalar_from_seed(&passkey_seed_zeroed) {
            Ok(val) => val,
            Err(e) => {
                return Err(e);
            }
        };
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
        getrandom::fill(&mut u_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
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
            getrandom::fill(&mut s_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
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
        let (c_comp, schema_e, schema_s) = generate_schema_zkp(is_valid_len, data_hash_bytes)?;

        // 6. Serialize proof: c_0 || s_0 || s_1 || ... || s_{n-1} || c_comp || schema_e || schema_s
        let mut proof = Vec::new();
        proof.extend_from_slice(b"ZKP_RING_PROOF_V1:");
        proof.extend_from_slice(&c[0].to_bytes());
        for i in 0..n {
            proof.extend_from_slice(&s[i].to_bytes());
        }
        proof.extend_from_slice(&c_comp.to_bytes());
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
            s_bytes.copy_from_slice(&payload[32 + 32 * i..32 + 32 * (i + 1)]);
            s.push(Scalar::from_bytes_mod_order(s_bytes));
        }

        let offset = 32 + 32 * n;
        let mut c_bytes = [0u8; 32];
        c_bytes.copy_from_slice(&payload[offset..offset + 32]);
        let mut e_bytes = [0u8; 32];
        e_bytes.copy_from_slice(&payload[offset + 32..offset + 64]);
        let mut s_schema_bytes = [0u8; 32];
        s_schema_bytes.copy_from_slice(&payload[offset + 64..offset + 96]);

        let schema_c = CompressedEdwardsY(c_bytes);
        let schema_e = Scalar::from_bytes_mod_order(e_bytes);
        let schema_s = Scalar::from_bytes_mod_order(s_schema_bytes);

        let data_hash_arr: [u8; 32] = match data_hash_bytes.clone().try_into() {
            Ok(arr) => arr,
            Err(_) => return Ok(false),
        };
        if !verify_schema_zkp(schema_c, schema_e, schema_s, &data_hash_arr) {
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
        let passkey_seed_zeroed = zeroize::Zeroizing::new(passkey_seed);
        if role_public_keys.is_empty() {
            return Err(YntraError::CryptoError(
                "Role ring cannot be empty".to_string(),
            ));
        }

        let mut salt_bytes = [0u8; 32];
        if let Err(e) = getrandom::fill(&mut salt_bytes) {
            return Err(YntraError::CryptoError(e.to_string()));
        }

        let mut commitment_hasher = blake3::Hasher::new();
        commitment_hasher.update(b"YNTRA_ZKP_ROLE_COMMITMENT_V3");
        commitment_hasher.update(role.as_bytes());
        commitment_hasher.update(&salt_bytes);
        let commitment = commitment_hasher.finalize();
        let commitment_bytes = commitment.as_bytes();

        let mut ring_points = Vec::new();
        for pk_hex in &role_public_keys {
            let pk_bytes = match const_hex::decode(pk_hex) {
                Ok(b) => b,
                Err(e) => {
                    return Err(YntraError::CryptoError(e.to_string()));
                }
            };
            if pk_bytes.len() != 32 {
                return Err(YntraError::CryptoError(
                    "Invalid public key length".to_string(),
                ));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&pk_bytes);
            let point = match CompressedEdwardsY(arr).decompress() {
                Some(p) => p,
                None => {
                    return Err(YntraError::CryptoError(
                        "Invalid public key point".to_string(),
                    ));
                }
            };
            ring_points.push(point);
        }

        let x = match derive_scalar_from_seed(&passkey_seed_zeroed) {
            Ok(val) => val,
            Err(e) => {
                return Err(e);
            }
        };
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
        getrandom::fill(&mut u_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
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
            getrandom::fill(&mut s_bytes).map_err(|e| YntraError::CryptoError(e.to_string()))?;
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
        use ark_groth16::{Groth16, Proof};
        use ark_serialize::CanonicalDeserialize;
        use ark_snark::SNARK;

        let proof_bytes =
            const_hex::decode(&proof_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let vk_bytes =
            const_hex::decode(&vk_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;

        let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..])
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;
        let pvk = get_cached_pvk(&vk_bytes)?;

        let mut public_inputs = Vec::with_capacity(public_inputs_hex.len());
        for input_hex in &public_inputs_hex {
            let input_bytes =
                const_hex::decode(input_hex).map_err(|e| YntraError::CryptoError(e.to_string()))?;
            let input_scalar = Fr::deserialize_compressed(&input_bytes[..])
                .map_err(|e| YntraError::CryptoError(e.to_string()))?;
            public_inputs.push(input_scalar);
        }

        let is_valid = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &public_inputs, &proof)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        Ok(is_valid)
    }
}

type CachedPvk = std::sync::Arc<ark_groth16::PreparedVerifyingKey<ark_bn254::Bn254>>;

pub fn get_cached_pvk(vk_bytes: &[u8]) -> Result<CachedPvk, YntraError> {
    static VK_CACHE: std::sync::OnceLock<
        std::sync::RwLock<std::collections::HashMap<Vec<u8>, CachedPvk>>,
    > = std::sync::OnceLock::new();
    let cache = VK_CACHE.get_or_init(|| std::sync::RwLock::new(std::collections::HashMap::new()));

    if let Ok(guard) = cache.read() {
        if let Some(pvk) = guard.get(vk_bytes) {
            return Ok(pvk.clone());
        }
    }

    use ark_bn254::Bn254;
    use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
    use ark_serialize::CanonicalDeserialize;

    let vk = VerifyingKey::<Bn254>::deserialize_compressed(vk_bytes)
        .map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let pvk = std::sync::Arc::new(PreparedVerifyingKey::<Bn254>::from(vk));

    if let Ok(mut guard) = cache.write() {
        guard.insert(vk_bytes.to_vec(), pvk.clone());
    }

    Ok(pvk)
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
