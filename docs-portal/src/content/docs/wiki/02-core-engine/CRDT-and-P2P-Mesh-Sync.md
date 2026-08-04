---
title: "CRDT & Peer-to-Peer (P2P) Mesh Sync"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

For high-concurrency collaborative editing (notes, field checklists, messaging) without a central cloud server, Yntra integrates **Loro / Automerge binary CRDTs** over WebRTC P2P mesh data channels.

---

## 1. Zero-Copy Store & CRDT Logs

Collab columns in libSQL store raw binary CRDT operation logs:

```rust
use yntra_core::{ZeroCopyNoteStore, P2PMeshSyncRouter};

// Write collaborative note update
let store = ZeroCopyNoteStore::new(workspace_id);
store.apply_binary_log(crdt_payload)?;
```

---

## 2. WebRTC Peer Mesh Router

When devices reside on the same local Wi-Fi or ad-hoc WebRTC mesh network, [`P2PMeshSyncRouter`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/database/zero_copy/sync.rs#L379-L459) exchanges CRDT binary delta logs directly between peer nodes without contacting the central cloud database.