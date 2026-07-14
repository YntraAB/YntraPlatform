use yntra_core::{ZkCryptoTrust, set_session_key, encrypt_field, decrypt_field};

// Example showing local envelope encryption and ZK proof validation
pub fn run_cryptographic_verification_example() -> Result<(), String> {
    // 1. Establish session key locally (derived from user hardware Passkey)
    let passkey_seed = "user-hardware-security-passkey".to_string();
    set_session_key(passkey_seed.clone().into_bytes());

    // 2. Encrypt sensitive fields locally before database commit
    let sensitive_payload = "Employee Secret Note Data";
    let workspace_id = "ws_abc";
    let encrypted_data = encrypt_field(sensitive_payload, workspace_id).map_err(|e| e.to_string())?;
    println!("Encrypted value: {:?}", encrypted_data);

    // 3. Initialize local Zero-Knowledge Trust system
    let trust = ZkCryptoTrust::new();

    // 4. Derive user public key for verification
    let public_key_hex = trust.derive_public_key(passkey_seed.clone()).map_err(|e| e.to_string())?;

    // 5. Generate local ZK-proof validating authority (e.g., role is Moderator)
    let user_id = "user-abc";
    let role = "Moderator";
    let proof = trust.generate_role_proof(passkey_seed, user_id.to_string(), role.to_string())
        .map_err(|e| e.to_string())?;

    // 6. Verify proof on remote sync engine without exposing user secrets
    let is_valid = trust.verify_proof(proof, user_id.to_string(), role.to_string(), public_key_hex);
    println!("Zero-Knowledge proof validation status: {}", is_valid);

    // 7. Decrypt data locally when needed
    let decrypted_payload = decrypt_field(&encrypted_data, workspace_id).map_err(|e| e.to_string())?;
    assert_eq!(sensitive_payload, decrypted_payload);

    Ok(())
}
