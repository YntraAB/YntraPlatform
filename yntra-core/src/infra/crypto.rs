use sha2::{Digest, Sha256};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn get_encryption_keys(workspace_id: &str) -> ([u8; 32], [u8; 16]) {
    // Hash workspace_id with a hardcoded static salt/pepper to derive key and IV
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update(b"yntra-secure-whistleblower-salt-2026");
    let hash = hasher.finalize();
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash[0..32]);
    
    let mut iv = [0u8; 16];
    // Hash again for IV
    let mut hasher = Sha256::new();
    hasher.update(hash);
    hasher.update(b"yntra-secure-whistleblower-iv-2026");
    let iv_hash = hasher.finalize();
    iv.copy_from_slice(&iv_hash[0..16]);
    
    (key, iv)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        if i + 2 > s.len() {
            return None;
        }
        let hex_digit = &s[i..i+2];
        let byte = u8::from_str_radix(hex_digit, 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

pub fn encrypt_field(data: &str, workspace_id: &str) -> String {
    let (key, iv) = get_encryption_keys(workspace_id);
    let mut buf = vec![0u8; data.len() + 16];
    buf[..data.len()].copy_from_slice(data.as_bytes());

    if let Ok(ct) = Aes256CbcEnc::new(&key.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, data.len()) {
        format!("enc:{}", hex_encode(ct))
    } else {
        data.to_string()
    }
}

pub fn decrypt_field(encrypted_data: &str, workspace_id: &str) -> String {
    if !encrypted_data.starts_with("enc:") {
        return encrypted_data.to_string();
    }
    
    let hex_ciphertext = &encrypted_data[4..];
    let mut buf = match hex_decode(hex_ciphertext) {
        Some(b) => b,
        None => return encrypted_data.to_string(),
    };

    let (key, iv) = get_encryption_keys(workspace_id);
    if let Ok(pt) = Aes256CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf) {
        String::from_utf8(pt.to_vec()).unwrap_or_else(|_| encrypted_data.to_string())
    } else {
        encrypted_data.to_string()
    }
}

pub fn encrypt_opt_field(data: Option<String>, workspace_id: Option<String>) -> Option<String> {
    let ws_id = workspace_id.unwrap_or_else(|| "workspace-1".to_string());
    data.map(|d| encrypt_field(&d, &ws_id))
}

pub fn decrypt_opt_field(encrypted_data: Option<String>, workspace_id: Option<String>) -> Option<String> {
    let ws_id = workspace_id.unwrap_or_else(|| "workspace-1".to_string());
    encrypted_data.map(|d| decrypt_field(&d, &ws_id))
}
