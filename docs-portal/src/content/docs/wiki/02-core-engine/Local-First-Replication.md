---
title: "Local-First Replication Engine"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra Platform uses **libSQL embedded replicas** as the single source of truth across all platforms, guaranteeing sub-millisecond local reads and writes regardless of network state.

---

## 1. Replication Architecture

```mermaid
sequenceDiagram
    participant UI as Host UI
    participant Core as yntra-core (libSQL Local)
    participant Queue as Offline Queue Journal
    participant Server as sqld / Turso Primary

    UI->>Core: Execute Mutation (add_todo / save_note)
    Core->>Core: Write to local SQLite (< 1ms)
    Core-->>UI: Instantly return & notify observers
    Core->>Queue: Append mutation to replication journal

    alt Online
        Queue->>Server: Flush replication transaction stream
        Server-->>Queue: Acknowledge & commit
    else Offline
        Queue->>Queue: Retain in local journal until reconnection
    end
```

---

## 2. Sync Queue Status Flags

Every table contains a `sync_status` column:
- `pending`: Local modification queued for replication.
- `synced`: Modification successfully acknowledged by primary server.
- `conflict`: Concurrent modification conflict requiring resolution.