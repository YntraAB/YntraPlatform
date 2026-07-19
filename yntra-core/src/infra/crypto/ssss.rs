use std::sync::OnceLock;
use crate::errors::YntraError;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::edwards::CompressedEdwardsY;
use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

static GF256_EXP: OnceLock<[u8; 256]> = OnceLock::new();
static GF256_LOG: OnceLock<[u8; 256]> = OnceLock::new();

fn init_tables() -> (&'static [u8; 256], &'static [u8; 256]) {
    let exp = GF256_EXP.get_or_init(|| {
        let mut e = [0u8; 256];
        let mut x = 1u8;
        for i in 0..255 {
            e[i] = x;
            let mut next = x << 1;
            if x & 0x80 != 0 {
                next ^= 0x1b;
            }
            x ^= next;
        }
        e[255] = e[0];
        e
    });

    let log = GF256_LOG.get_or_init(|| {
        let mut l = [0u8; 256];
        let e = GF256_EXP.get().unwrap();
        for i in 0..255 {
            l[e[i] as usize] = i as u8;
        }
        l[0] = 0;
        l
    });

    (exp, log)
}

fn gf256_add(a: u8, b: u8) -> u8 {
    a ^ b
}

fn gf256_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let (exp, log) = init_tables();
    let sum = log[a as usize] as usize + log[b as usize] as usize;
    exp[sum % 255]
}

fn gf256_div(a: u8, b: u8) -> u8 {
    if b == 0 {
        panic!("Division by zero in GF(256)");
    }
    if a == 0 {
        return 0;
    }
    let (exp, log) = init_tables();
    let diff = (log[a as usize] as i16 - log[b as usize] as i16 + 255) % 255;
    exp[diff as usize]
}

pub fn split_secret(
    secret: &[u8],
    threshold: usize,
    total_shards: usize,
) -> Result<Vec<(u8, Vec<u8>)>, YntraError> {
    if threshold < 1 || total_shards < threshold || total_shards > 255 {
        return Err(YntraError::CryptoError("Invalid SSSS parameters".to_string()));
    }
    let mut shards = vec![vec![0u8; secret.len()]; total_shards];

    for (byte_idx, &s_byte) in secret.iter().enumerate() {
        let mut poly = vec![0u8; threshold];
        poly[0] = s_byte;
        if threshold > 1 {
            let mut rand_coeffs = vec![0u8; threshold - 1];
            getrandom::fill(&mut rand_coeffs)
                .map_err(|e| YntraError::CryptoError(format!("Entropy failure: {:?}", e)))?;
            for i in 1..threshold {
                poly[i] = rand_coeffs[i - 1];
            }
        }

        for x in 1..=total_shards {
            let x_u8 = x as u8;
            let mut val = poly[0];
            let mut x_pow = 1u8;
            for i in 1..threshold {
                x_pow = gf256_mul(x_pow, x_u8);
                let term = gf256_mul(poly[i], x_pow);
                val = gf256_add(val, term);
            }
            shards[x - 1][byte_idx] = val;
        }
    }

    let result = shards
        .into_iter()
        .enumerate()
        .map(|(idx, data)| ((idx + 1) as u8, data))
        .collect();

    Ok(result)
}

pub fn reconstruct_secret(
    shards: &[(u8, Vec<u8>)],
    threshold: usize,
) -> Result<Vec<u8>, YntraError> {
    if shards.len() < threshold {
        return Err(YntraError::CryptoError(
            "Fewer shards than threshold".to_string(),
        ));
    }
    if shards.is_empty() {
        return Err(YntraError::CryptoError("No shards provided".to_string()));
    }
    let len = shards[0].1.len();
    for s in shards {
        if s.1.len() != len {
            return Err(YntraError::CryptoError("Shard size mismatch".to_string()));
        }
        if s.0 == 0 {
            return Err(YntraError::CryptoError("Invalid shard ID 0".to_string()));
        }
    }

    let mut secret = vec![0u8; len];
    for byte_idx in 0..len {
        let mut val = 0u8;
        for i in 0..threshold {
            let xi = shards[i].0;
            let yi = shards[i].1[byte_idx];
            let mut li = 1u8;
            for j in 0..threshold {
                if i != j {
                    let xj = shards[j].0;
                    let num = xj;
                    let denom = gf256_add(xi, xj);
                    let term = gf256_div(num, denom);
                    li = gf256_mul(li, term);
                }
            }
            val = gf256_add(val, gf256_mul(yi, li));
        }
        secret[byte_idx] = val;
    }

    Ok(secret)
}

pub fn encrypt_with_workspace_pubkey(
    ws_pub_hex: &str,
    plaintext: &[u8],
) -> Result<String, YntraError> {
    let ed_pub_bytes = const_hex::decode(ws_pub_hex)
        .map_err(|e| YntraError::CryptoError(format!("Invalid public key hex: {:?}", e)))?;
    if ed_pub_bytes.len() != 32 {
        return Err(YntraError::CryptoError("Invalid public key length".to_string()));
    }
    let mut ed_pub = [0u8; 32];
    ed_pub.copy_from_slice(&ed_pub_bytes);

    let ws_point = CompressedEdwardsY(ed_pub)
        .decompress()
        .ok_or_else(|| YntraError::CryptoError("Invalid public key point".to_string()))?;

    let mut eph_priv_bytes = [0u8; 32];
    getrandom::fill(&mut eph_priv_bytes)
        .map_err(|e| YntraError::CryptoError(e.to_string()))?;
    let eph_scalar = Scalar::from_bytes_mod_order(eph_priv_bytes);

    let eph_pub_point = &ED25519_BASEPOINT_POINT * &eph_scalar;
    let eph_pub_bytes = eph_pub_point.compress().to_bytes();

    let shared_point = ws_point * eph_scalar;
    let shared_bytes = shared_point.compress().to_bytes();

    let mut hasher = blake3::Hasher::new_derive_key("yntra-recovery-kdf");
    hasher.update(&shared_bytes);
    let sym_key_bytes = hasher.finalize();

    let mut nonce_bytes = [0u8; 24];
    getrandom::fill(&mut nonce_bytes)
        .map_err(|e| YntraError::CryptoError(e.to_string()))?;

    let key = Key::from_slice(sym_key_bytes.as_bytes());
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| YntraError::CryptoError("Encryption failed".to_string()))?;

    Ok(format!(
        "{}:{}:{}",
        const_hex::encode(eph_pub_bytes),
        const_hex::encode(nonce_bytes),
        const_hex::encode(ciphertext)
    ))
}

pub fn decrypt_with_workspace_privkey(
    ws_priv_hex: &str,
    payload: &str,
) -> Result<Vec<u8>, YntraError> {
    let parts: Vec<&str> = payload.split(':').collect();
    if parts.len() != 3 {
        return Err(YntraError::CryptoError("Invalid payload format".to_string()));
    }
    let eph_pub_bytes = const_hex::decode(parts[0])
        .map_err(|e| YntraError::CryptoError(format!("Invalid ephemeral public key: {:?}", e)))?;
    let nonce_bytes = const_hex::decode(parts[1])
        .map_err(|e| YntraError::CryptoError(format!("Invalid nonce: {:?}", e)))?;
    let ciphertext = const_hex::decode(parts[2])
        .map_err(|e| YntraError::CryptoError(format!("Invalid ciphertext: {:?}", e)))?;

    if eph_pub_bytes.len() != 32 || nonce_bytes.len() != 24 {
        return Err(YntraError::CryptoError("Invalid parameter sizes".to_string()));
    }

    let mut eph_pub = [0u8; 32];
    eph_pub.copy_from_slice(&eph_pub_bytes);

    let eph_point = CompressedEdwardsY(eph_pub)
        .decompress()
        .ok_or_else(|| YntraError::CryptoError("Invalid ephemeral public point".to_string()))?;

    let ws_priv_bytes = const_hex::decode(ws_priv_hex)
        .map_err(|e| YntraError::CryptoError(format!("Invalid private key hex: {:?}", e)))?;
    if ws_priv_bytes.len() != 32 {
        return Err(YntraError::CryptoError("Invalid private key length".to_string()));
    }
    let mut ws_priv = [0u8; 32];
    ws_priv.copy_from_slice(&ws_priv_bytes);
    let ws_scalar = ed25519_seed_to_scalar(&ws_priv);

    let shared_point = eph_point * ws_scalar;
    let shared_bytes = shared_point.compress().to_bytes();

    let mut hasher = blake3::Hasher::new_derive_key("yntra-recovery-kdf");
    hasher.update(&shared_bytes);
    let sym_key_bytes = hasher.finalize();

    let key = Key::from_slice(sym_key_bytes.as_bytes());
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let decrypted = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| YntraError::CryptoError("Decryption failed".to_string()))?;

    Ok(decrypted)
}

fn ed25519_seed_to_scalar(seed: &[u8; 32]) -> Scalar {
    use sha2::{Sha512, Digest};
    let mut hasher = Sha512::new();
    hasher.update(seed);
    let hash = hasher.finalize();
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&hash[..32]);
    scalar_bytes[0] &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;
    Scalar::from_bytes_mod_order(scalar_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_reconstruct() {
        let secret = b"super-secret-workspace-encryption-key-12345";
        let threshold = 3;
        let total = 5;

        let shards = split_secret(secret, threshold, total).unwrap();
        assert_eq!(shards.len(), total);

        let recon1 = reconstruct_secret(&shards[0..3], threshold).unwrap();
        assert_eq!(recon1, secret);

        let selected_shards = vec![shards[0].clone(), shards[2].clone(), shards[4].clone()];
        let recon2 = reconstruct_secret(&selected_shards, threshold).unwrap();
        assert_eq!(recon2, secret);

        let too_few = &shards[0..2];
        let recon_fail = reconstruct_secret(too_few, threshold);
        assert!(recon_fail.is_err());
    }

    #[test]
    fn test_e2e_wrapping() {
        let ws_priv_hex = "41528659d48b1bfcb4659b85c13b28b78997a0a0a0a0a0a0a0a0a0a0a0a0a0a0";
        let ws_priv_bytes = const_hex::decode(ws_priv_hex).unwrap();
        let mut ws_priv = [0u8; 32];
        ws_priv.copy_from_slice(&ws_priv_bytes);
        let ws_scalar = ed25519_seed_to_scalar(&ws_priv);
        let ws_pub_point = &ED25519_BASEPOINT_POINT * &ws_scalar;
        let ws_pub_hex = const_hex::encode(ws_pub_point.compress().to_bytes());

        let secret = b"my-private-signing-key";
        let encrypted = encrypt_with_workspace_pubkey(&ws_pub_hex, secret).unwrap();
        let decrypted = decrypt_with_workspace_privkey(ws_priv_hex, &encrypted).unwrap();
        assert_eq!(decrypted, secret);
    }
}
