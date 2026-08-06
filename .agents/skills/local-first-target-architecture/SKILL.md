---
name: local-first-target-architecture
description: Reference guide defining the target absolute SOTA local-first architecture (PGlite / Zero / Loro CRDT) for future codebase enhancements.
---

# Absolute SOTA Local-First Architecture Goal

This document defines the target State-of-the-Art (SOTA), top-tier, and optimal local-first sync architecture for the Yntra Platform.

---

## 1. Zero-Copy Memory-Mapped Persistence (No-Parser DB)

### What it is
The absolute pinnacle of local-first speed: zero parsing, zero deserialization, and zero SQL translation. The database storage on disk matches the exact binary layout of client RAM (using memory-mapped files with zero-copy serialization like `rkyv`).

### How to Implement
1. **Memory-Mapped State**: Persist data structures directly to disk using `mmap`.
2. **0-ns Reads**: Cast disk bytes to Rust structures instantly using `rkyv` archive references, bypassing any parsing or decoding overhead.
3. **Copy-on-Write / CRDT Mutations**: Mutate data structures using local CRDT updates (like `Loro`), appending changes to the memory map.

---

## 2. Geo-Distributed Edge Replicas + P2P Mesh Sync

### What it is
A hybrid sync engine that replicates data to the geographically closest Edge Server (e.g. Turso Edge Replicas / Cloudflare Workers) to minimize latency, combined with a peer-to-peer WebRTC mesh fallback for instant in-room collaboration without hitting the cloud.

### How to Implement
1. **Hybrid Sync Router**: Clients broadcast database writes to nearby peers in real-time over WebRTC Data Channels.
2. **Edge Sync Loop**: Fallback or consolidate transactional updates to the nearest geo-distributed edge database node.
3. **Optimistic Consolidation**: Resolve state diverges using CRDT logical clocks.

---

## 3. Zero-Knowledge Cryptographic Trust (Passkey + ZKP)

### What it is
An end-to-end encrypted (E2EE) database model where the cloud server never sees or stores unencrypted user data, but can cryptographically verify that client database modifications comply with database schemas and business rules using Zero-Knowledge Proofs (ZKPs).

### How to Implement
1. **Local Envelope Encryption**: Encrypt all workspace fields locally using keys derived from user Passkeys/Hardware security keys.
2. **Local ZK Proof Generation**: When writing data, generate a ZK-proof on the device showing that the encrypted write complies with schemas and user authorization policies.
3. **Zero-Knowledge Cloud Verification**: The remote sync engine verifies the ZK-proof and commits the encrypted state without decryptions.

---

## 4. Enterprise Shared Kiosk & Workstation Session Resilience

### What it is
A zero-data-loss persistence layer designed for shared workstation terminals (hospitals, labs) governed by Active Directory or Jamf GPOs that wipe browser site data (OPFS, IndexedDB, site caches) on user logout or session timeout.

### How to Implement
1. **Multi-Tier Persistence Engine**: Web client combines WASM OPFS with in-room P2P mesh mirroring, loopback native daemon IPC (`ws://127.0.0.1:9443`), and synchronous emergency beacon flushes (`navigator.sendBeacon`).
2. **Pre-Logout Guard**: Intercepts session termination events, preventing profile wiping until offline writes are confirmed synced or mirrored to local network peers.
