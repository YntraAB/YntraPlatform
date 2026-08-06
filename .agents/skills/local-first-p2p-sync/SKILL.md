---
name: local-first-p2p-sync
description: Guidelines for managing the local-first peer-to-peer sync engine, HIPAA/FERPA compliance governance, DLP content inspection, WebRTC Data Channel broadcasts, and state updates with CRDT Loro docs.
---

# Local-First P2P Sync & Enterprise Compliance Governance Guide

This skill guides the design, troubleshooting, enterprise compliance (HIPAA / FERPA), Data Loss Prevention (DLP), and extension of the peer-to-peer synchronization system in the Yntra Platform.

---

## 1. Enterprise Compliance Operating Modes (`ComplianceMode`)

For healthcare (HIPAA §164.312(b)), educational institutions (FERPA 34 CFR Part 99), and enterprise environments, `P2PMeshSyncRouter` enforces four strict governance operational modes:

* **`ComplianceMode::StrictServerOnly`** *(High-Security / Regulated Workspace Default)*:
  * Direct client-to-client WebRTC P2P Data Channels are **strictly disabled**.
  * All database diffs and CRDT logs route via TLS exclusively through the Centralized Compliance Server / Edge Proxy Relay where full proxy inspection and immutable audit logging occur.
* **`ComplianceMode::AuditedProxyRelay`**:
  * P2P mesh sync requests are redirected through an Audited Compliance Proxy.
  * Payloads undergo real-time DLP inspection, generate cryptographic audit logs in `ZeroCopyAuditStore`, and are re-encrypted before target peer delivery.
* **`ComplianceMode::AuditedLocalP2P`**:
  * Permits direct local WebRTC Data Channels provided client-side DLP policies pass.
  * Every sync event creates a signed compliance record in `ZeroCopyAuditStore` for offline-to-online audit sync.
* **`ComplianceMode::UnrestrictedLocalP2P`**:
  * Un-audited local P2P mesh sync (developer / non-regulated test workspaces).

---

## 2. Real-Time Data Loss Prevention (DLP) & Content Inspection (`DlpPolicy`)

Before any CRDT update log is broadcast or applied:

1. **PHI Inspection (HIPAA)**: Scans for Social Security Numbers (SSN), Medical Record Numbers (`MRN-`), ICD-10/11 diagnosis codes, and patient record identifiers.
2. **Educational Record Inspection (FERPA)**: Scans for Student IDs (`SID-`), cumulative GPA transcripts, and student record identifiers.
3. **Custom Keywords**: Scans for workspace-configured sensitive tags.
4. **Action on Violation**:
   * If a match occurs and `policy.block_on_match` or a compliance mode is active, the broadcast is **instantly blocked**.
   * A high-severity `DlpViolation` event is recorded into `ZeroCopyAuditStore` with actor ID, payload hash, classification, and timestamp.

```rust
use yntra_core::{P2PMeshSyncRouter, ComplianceMode, DlpPolicy};

// Initialize router with HIPAA/FERPA compliance policy
let router = P2PMeshSyncRouter::with_compliance(
    Some("https://compliance-relay.yntra.internal/relay".to_string()),
    ComplianceMode::StrictServerOnly,
    DlpPolicy::default(),
);

// P2P writes automatically undergo DLP scan & compliance enforcement
router.broadcast_write_network(peer_id, crdt_payload);
```

---

## 3. WebRTC & Signaling Mesh Router

* **Peer Registration**: Every client node registers using `P2PMeshSyncRouter::register_peer_network(peer_id)`.
* **State Broadcast**: Local writes to zero-copy storage are serialized and broadcast via `broadcast_write_network(peer_id, data)`.
* **Refer to Example**: See [examples/p2p_mesh_router.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/local-first-p2p-sync/examples/p2p_mesh_router.rs) for setup.

---

## 4. CRDT Integration & Immutable Audit Logging

* **Loro CRDT**: Database blocks are serialized into Loro Snapshot documents stored in database BLOBs.
* **Merkle Audit Ledger**: Sync broadcasts generate BLAKE3/Ed25519-signed Merkle-tree entries in `ZeroCopyAuditStore` ensuring 100% auditability for compliance administrators.
