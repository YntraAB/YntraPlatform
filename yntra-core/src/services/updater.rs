use crate::infra::errors::YntraError;
use ed25519_dalek::Verifier;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Update release manifest structure served by update servers.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct UpdateManifest {
    pub version: String,
    pub release_notes: String,
    pub pub_date: String,
    pub download_url: String,
    pub signature: String,
    pub sha256: String,
    pub min_supported_version: Option<String>,
}

/// Result of checking for updates against current app version.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub manifest: Option<UpdateManifest>,
}

/// Helper function to parse semantic versioning (MAJOR.MINOR.PATCH).
pub fn parse_semver(version: &str) -> (u64, u64, u64) {
    let clean = version.trim().trim_start_matches('v');
    let parts: Vec<&str> = clean.split('.').collect();
    let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.split('-').next()?.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// Compares two version strings. Returns true if `latest` is newer than `current`.
#[uniffi::export]
pub fn is_version_newer(current_version: &str, latest_version: &str) -> bool {
    let current = parse_semver(current_version);
    let latest = parse_semver(latest_version);
    latest > current
}

/// Constructs the canonical message for Ed25519 signature verification.
pub fn construct_update_signature_message(version: &str, download_url: &str, sha256: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.extend_from_slice(b"YNTRA_BINARY_UPDATE_V1\0");
    msg.extend_from_slice(&(version.len() as u64).to_be_bytes());
    msg.extend_from_slice(version.as_bytes());
    msg.extend_from_slice(&(download_url.len() as u64).to_be_bytes());
    msg.extend_from_slice(download_url.as_bytes());
    msg.extend_from_slice(&(sha256.len() as u64).to_be_bytes());
    msg.extend_from_slice(sha256.as_bytes());
    msg
}

/// Verifies Ed25519 signature of an UpdateManifest against a public key hex.
#[uniffi::export]
pub fn verify_manifest_signature(public_key_hex: &str, manifest: &UpdateManifest) -> bool {
    verify_update_payload_signature(
        public_key_hex,
        &manifest.version,
        &manifest.download_url,
        &manifest.sha256,
        &manifest.signature,
    )
}

/// Verifies Ed25519 signature for update payload details.
#[uniffi::export]
pub fn verify_update_payload_signature(
    public_key_hex: &str,
    version: &str,
    download_url: &str,
    sha256_hex: &str,
    signature_hex: &str,
) -> bool {
    if public_key_hex.len() != 64 || signature_hex.len() != 128 {
        return false;
    }

    let pk_bytes = match const_hex::decode(public_key_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let pk_arr: [u8; 32] = match pk_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&pk_arr) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_bytes = match const_hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);
    let message = construct_update_signature_message(version, download_url, sha256_hex);
    verifying_key.verify(&message, &signature).is_ok()
}

/// Validates SHA-256 digest of binary payload.
#[uniffi::export]
pub fn verify_binary_checksum(payload: &[u8], expected_sha256: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let result = hasher.finalize();
    let calculated_hex = const_hex::encode(result);
    calculated_hex.eq_ignore_ascii_case(expected_sha256.trim())
}

/// Parses update manifest JSON text and verifies signature if public key provided.
#[uniffi::export]
pub fn process_update_manifest(
    manifest_json: &str,
    current_version: &str,
    public_key_hex: Option<String>,
) -> Result<UpdateCheckResult, YntraError> {
    let manifest: UpdateManifest = serde_json::from_str(manifest_json)
        .map_err(|e| YntraError::SerializationError(format!("Invalid update manifest JSON: {}", e)))?;

    if let Some(pk) = public_key_hex {
        if !pk.trim().is_empty() {
            if !verify_manifest_signature(&pk, &manifest) {
                return Err(YntraError::CryptoError(
                    "Update manifest Ed25519 signature verification failed".to_string(),
                ));
            }
        }
    }

    let newer = is_version_newer(current_version, &manifest.version);
    Ok(UpdateCheckResult {
        update_available: newer,
        current_version: current_version.to_string(),
        latest_version: manifest.version.clone(),
        manifest: if newer { Some(manifest) } else { None },
    })
}

/// Stages verified binary update on disk for atomic application.
#[uniffi::export]
pub fn stage_binary_update(
    binary_data: &[u8],
    expected_sha256: &str,
    target_path_override: Option<String>,
) -> Result<String, YntraError> {
    if !verify_binary_checksum(binary_data, expected_sha256) {
        return Err(YntraError::CryptoError(
            "Downloaded binary SHA-256 checksum mismatch".to_string(),
        ));
    }

    let base_path = match target_path_override {
        Some(p) => PathBuf::from(p),
        None => std::env::current_exe().map_err(|e| YntraError::ValidationError(e.to_string()))?,
    };

    let update_staged_path = base_path.with_extension("exe.new");
    std::fs::write(&update_staged_path, binary_data)
        .map_err(|e| YntraError::ValidationError(format!("Failed to stage update binary: {}", e)))?;

    Ok(update_staged_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn test_semver_parsing_and_comparison() {
        assert_eq!(parse_semver("0.1.0"), (0, 1, 0));
        assert_eq!(parse_semver("v1.2.3-alpha"), (1, 2, 3));

        assert!(is_version_newer("0.1.0", "0.2.0"));
        assert!(is_version_newer("0.1.0", "1.0.0"));
        assert!(is_version_newer("0.1.9", "0.2.0"));
        assert!(!is_version_newer("0.2.0", "0.1.0"));
        assert!(!is_version_newer("0.1.0", "0.1.0"));
    }

    #[test]
    fn test_manifest_signature_verification() {
        let mut priv_bytes = [0u8; 32];
        getrandom::fill(&mut priv_bytes).unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&priv_bytes);
        let pub_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

        let version = "0.2.0";
        let download_url = "https://releases.yntra.io/v0.2.0/yntra-ui.exe";
        let sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let msg = construct_update_signature_message(version, download_url, sha256);
        let sig = signing_key.sign(&msg);
        let sig_hex = const_hex::encode(sig.to_bytes());

        let manifest = UpdateManifest {
            version: version.to_string(),
            release_notes: "Feature updates and performance enhancements.".to_string(),
            pub_date: "2026-08-04T12:00:00Z".to_string(),
            download_url: download_url.to_string(),
            signature: sig_hex.clone(),
            sha256: sha256.to_string(),
            min_supported_version: Some("0.1.0".to_string()),
        };

        assert!(verify_manifest_signature(&pub_hex, &manifest));

        // Test invalid signature
        let mut invalid_manifest = manifest.clone();
        invalid_manifest.sha256 = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        assert!(!verify_manifest_signature(&pub_hex, &invalid_manifest));
    }

    #[test]
    fn test_binary_checksum_and_staging() {
        let payload = b"YNTRA_BINARY_MOCK_PAYLOAD_DATA";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let sha256_hex = const_hex::encode(hasher.finalize());

        assert!(verify_binary_checksum(payload, &sha256_hex));
        assert!(!verify_binary_checksum(payload, "invalid_sha256"));

        let temp_dir = std::env::temp_dir();
        let mock_target = temp_dir.join("yntra_mock_target.exe");
        let staged_path = stage_binary_update(payload, &sha256_hex, Some(mock_target.to_string_lossy().to_string())).unwrap();

        assert!(std::path::Path::new(&staged_path).exists());
        let read_back = std::fs::read(&staged_path).unwrap();
        assert_eq!(read_back, payload);

        let _ = std::fs::remove_file(&staged_path);
    }
}
