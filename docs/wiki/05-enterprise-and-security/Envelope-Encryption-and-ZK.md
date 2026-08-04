# Client-Side Envelope Encryption & Zero-Knowledge Trust

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

`ZkCryptoTrust` generates ZK proofs for transactional verification without exposing plain-text user secrets across peer sync meshes.

---

## 3. Asynchronous & Off-Thread ZK Proving Pipeline

> [!CAUTION]
> Generating Groth16 proofs (`ark-bn254` / `ark-groth16`) on client hardware requires significant CPU cycles and memory allocations. Synchronous proof generation on the main thread will lock the UI event loop and destroy TTI responsiveness.

### Proving Offload Protocol
1. **Dedicated Worker Threads**:
   * **Native Desktop/Mobile**: ZK proofs are calculated using a dedicated `rayon` worker thread pool scoped with low OS thread priority (`nice`).
   * **Web WASM**: Proof generation is dispatched to a dedicated **Web Worker** instance running WASM multithreading (`SharedArrayBuffer`), keeping the main Dioxus rendering thread at 60 FPS.
2. **Optimistic Local Mutation**: The user's transaction UI state updates optimistically in `< 1ms` using local SQLite records, while the background ZK proof is computed asynchronously.
3. **Background Proof Attestation**: Once generated, the ZK proof is attached to the outgoing CRDT or sync packet before network broadcast.

