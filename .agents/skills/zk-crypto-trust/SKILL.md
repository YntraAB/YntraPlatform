---
name: zk-crypto-trust
description: Guidelines for managing local envelope encryption, integrating hardware passkeys, and generating/verifying Zero-Knowledge Proofs for transactional security.
---

# Zero-Knowledge Cryptographic Trust Guide

This skill defines instructions for managing end-to-end encrypted databases (E2EE) and validating access control using Zero-Knowledge Proofs (ZKPs) in the Yntra Platform.

---

## 1. Local Envelope Encryption

* **Hardware Key Derivation**: When a user registers or logs in with their hardware Passkey, generate a session key and register it in memory using `set_session_key(key)`.
* **Field-Level Encryption**: Encrypt sensitive table columns (like time logs, messages, profile details) using `encrypt_field(plaintext)` prior to database writes.
* **Local Decryption**: Decrypt fields on the client layer using `decrypt_field(ciphertext)`. The cloud server should never see or store the plaintext.

---

## 2. Zero-Knowledge Proofs (ZKP)

To allow the cloud sync server to verify write authority without knowing the user's role or identity secrets:

1. **Proof Generation**: The client generates a cryptographic proof locally using `ZkCryptoTrust::generate_role_proof(passkey_seed, user_id, role)`.
2. **Commit Inclusion**: Bundle the proof together with the encrypted transaction data in the sync payload.
3. **Cloud Validation**: The remote node runs `ZkCryptoTrust::verify_proof(proof, user_id, role, public_key_hex)` to authorize changes without decrypting the data.
4. **Refer to Example**: See [examples/zkp_auth.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/zk-crypto-trust/examples/zkp_auth.rs) for a complete workflow.

---

## 3. Keychain Safety Guidelines

* **Zeroization**: Sensitive memory regions holding temporary keys or passwords must implement `zeroize` traits to clean memory footprints immediately after operations complete.
* **Secure Storage Providers**: Integrate platform-specific Keychain features via `SecureStorageProvider` callbacks to handle persistent storage on iOS/Android native backends.

---

## 4. Multi-Recipient Envelope Encryption & Break-Glass Recovery

To protect patient safety and institutional business continuity (HIPAA § 164.312(a)(2)(ii)):

1. **Dual-Recipient Key Wrapping**: Sensitive fields must be envelope-encrypted under both the user's Passkey key wrap AND the Institutional Emergency Escrow key wrap (`encrypt_workspace_field_with_escrow`).
2. **Break-Glass Emergency Decryption**: In emergency healthcare scenarios (e.g. Code Blue, ICU admissions, attending physician off-duty), authorized emergency operators unwrap the DEK using the institutional escrow key via `decrypt_workspace_field_break_glass`.
3. **Cryptographic ZKP Audit Trails**: Break-glass operations generate an immutable ZKP Break-Glass Audit Proof (`generate_break_glass_audit_proof`) signed by the operator attesting to `(operator_id, patient_id, emergency_reason, timestamp, dek_hash)`.
4. **Audit Verification**: Database observers and sync nodes verify break-glass audit proofs via `verify_break_glass_audit_proof` to enforce non-repudiation and prevent unauthorized emergency access.

