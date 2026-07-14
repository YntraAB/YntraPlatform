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
