use yntra_core::{ZkCryptoTrust, set_session_key, encrypt_field, decrypt_field};

// Example showing local envelope encryption and ZK proof validation
pub fn run_cryptographic_verification_example() -> Result<(), String> {
    // 1. Establish session key locally (derived from user hardware Passkey)
    let passkey = "user-hardware-security-passkey";
    set_session_key(passkey.to_string()).map_err(|e| e.to_string())?;

    // 2. Encrypt sensitive fields locally before database commit
    let sensitive_payload = "Employee Secret Note Data";
    let encrypted_data = encrypt_field(sensitive_payload.to_string()).map_err(|e| e.to_string())?;
    println!("Encrypted value: {:?}", encrypted_data);

    // 3. Initialize local Zero-Knowledge Trust system
    let trust = ZkCryptoTrust::new();

    // 4. Generate local ZK-proof validating authority (e.g., role is Moderator)
    let user_id = "user-abc";
    let role = "Moderator";
    let proof = trust.generate_role_proof(user_id.to_string(), role.to_string())
        .map_err(|e| e.to_string())?;

    // 5. Verify proof on remote sync engine without exposing user secrets
    let is_valid = trust.verify_proof(&proof);
    println!("Zero-Knowledge proof validation status: {}", is_valid);

    // 6. Decrypt data locally when needed
    let decrypted_payload = decrypt_field(encrypted_data).map_err(|e| e.to_string())?;
    assert_eq!(sensitive_payload, decrypted_payload);

    Ok(())
}
