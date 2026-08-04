---
title: "Client-Side Envelope Encryption & Zero-Knowledge Trust"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Sensitive data fields (personal numbers, client medical logs, financial numbers) are encrypted client-side before being written to SQLite or replicated to the cloud server.

---

## 1. Envelope Encryption Architecture

1. **Master Session Key**: Derived locally using Argon2id or loaded from platform hardware keystore.
2. **Data Encryption Key (DEK)**: Unique AES-256-GCM key generated per sensitive field payload.
3. **Storage**: Encrypted payload and nonce are stored in SQLite BLOBs.

```rust
use yntra_core::infra::crypto::{encrypt_field, decrypt_field};

// Encrypt payload before SQLite write
let encrypted_bytes = encrypt_field(&raw_payload, &session_key)?;
```

---

## 2. Zero-Knowledge Proofs (ZK)

[`ZkCryptoTrust`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/database/zero_copy/crypto.rs#L123-L203) generates ZK proofs for transactional verification without exposing plain-text user secrets across peer sync meshes.