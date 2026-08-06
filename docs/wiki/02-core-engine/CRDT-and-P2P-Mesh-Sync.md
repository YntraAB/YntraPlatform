# CRDT & Peer-to-Peer (P2P) Mesh Sync & HIPAA/FERPA Governance

For high-concurrency collaborative editing (notes, field checklists, messaging) without a central cloud server, Yntra integrates **Loro / Automerge binary CRDTs** over WebRTC P2P mesh data channels with an enterprise compliance policy engine.

---

## 1. Zero-Copy Store & CRDT Logs

Collab columns in libSQL store raw binary CRDT operation logs:

```rust
use yntra_core::{ZeroCopyNoteStore, P2PMeshSyncRouter, ComplianceMode, DlpPolicy};

// Write collaborative note update
let store = ZeroCopyNoteStore::new(workspace_id);
store.apply_binary_log(crdt_payload)?;
```

---

## 2. Enterprise Governance & Compliance Modes (`ComplianceMode`)

For healthcare (HIPAA §164.312(b)), educational institutions (FERPA 34 CFR Part 99), and enterprise environments, `P2PMeshSyncRouter` enforces strict compliance operational modes:

| Operational Mode | Description | Direct P2P Channel | Audit Logging | DLP Inspection |
|---|---|---|---|---|
| `StrictServerOnly` | Mandatory for HIPAA/FERPA regulated tiers. Direct P2P disabled. | Disabled | Immutable Server Audit | Proxy / Edge TLS |
| `AuditedProxyRelay` | Reroutes P2P traffic through compliance proxy. | Proxy Relayed | Merkle Ledger Logged | Real-Time Scanner |
| `AuditedLocalP2P` | Local P2P permitted if client-side DLP passes. | Permitted | Local Signed Log | Client-Side Scanner |
| `UnrestrictedLocalP2P` | Legacy un-audited P2P sync for dev environments. | Permitted | None | Disabled |

---

## 3. Real-Time Data Loss Prevention (DLP) & Audit Sequence

```mermaid
sequenceDiagram
    participant Nurse as Nurse Tablet (Client A)
    participant Router as P2PMeshSyncRouter
    participant DLP as DLP Scanner
    participant Audit as ZeroCopyAuditStore
    participant Relay as Audited Compliance Relay Proxy
    participant Doctor as Doctor Workstation (Client B)

    Nurse->>Router: broadcast_write_network(payload)
    Router->>DLP: inspect_payload_dlp_bytes(payload)
    alt PHI or FERPA Violation Detected
        DLP-->>Router: Violation (PHI_DETECTED / FERPA_DETECTED)
        Router->>Audit: Record DlpViolation Event (BLAKE3 Hash)
        Router-->>Nurse: Abort Broadcast (Blocked by DLP Policy)
    else Clean Payload & Compliance Approved
        DLP-->>Router: Clean / Approved
        Router->>Audit: Record Sync Event to Signed Ledger
        Router->>Relay: POST /relay/broadcast (TLS + Signed Envelope)
        Relay->>Doctor: Forward Verified & Audited CRDT Delta
    end
```

---

## 4. CRDT Log Compaction & Garbage Collection (GC) Policy

> [!IMPORTANT]
> Uncompacted CRDT operation logs can cause rapid SQLite storage bloat. `yntra-core` enforces automatic log compaction and GC snapshots.

### Compaction Thresholds & Rules
* **Operation Count Limit**: When a document's operation log exceeds **1,000 operations**, a full snapshot export is triggered.
* **Storage Footprint Limit**: If CRDT BLOB storage for a single document exceeds **5 MB**, `yntra-core` flushes state into a single compressed Loro snapshot BLOB and truncates historical delta logs.
* **Tombstone Pruning**: Deleted CRDT nodes and tombstones older than **30 days** are purged once all known peer devices have acknowledged the latest vector clock epoch.

---

## 5. WebRTC TURN Relay Fallback SLA & NAT Traversal

In enterprise environments with symmetric NATs, strict corporate firewalls, or mobile carrier CGNAT, direct P2P ICE candidate traversal can fail in **15–20% of sessions**.

```mermaid
sequenceDiagram
    participant PeerA as Local Client A
    participant STUN as STUN Server
    participant TURN as TURN Relay Server
    participant PeerB as Remote Client B

    PeerA->>STUN: Request ICE Candidates
    STUN-->>PeerA: Public Reflexive IP/Port
    PeerA->>PeerB: Attempt Direct P2P Connection
    Note over PeerA,PeerB: Direct P2P Fails (Symmetric NAT / Firewall)
    PeerA->>TURN: Request Relayed Connection
    TURN-->>PeerA: Relayed Candidate Allocated
    PeerA->>TURN: Send Encrypted CRDT Delta
    TURN->>PeerB: Forward Encrypted CRDT Delta
```

### TURN Fallback SLA Guarantees
1. **ICE Candidate Timeout**: If direct P2P connectivity is not established within **3,000ms**, `P2PMeshSyncRouter` automatically switches to secure TLS-encrypted TURN relay servers.
2. **End-to-End Encryption (E2EE)**: All CRDT delta payloads routed via TURN servers are encrypted client-side via Noise Protocol / ChaCha20-Poly1305, ensuring TURN server operators cannot view payload contents.
