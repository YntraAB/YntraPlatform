---
name: local-first-p2p-sync
description: Guidelines for managing the local-first peer-to-peer sync engine, handling WebRTC Data Channel broadcasts, and integrating state updates with CRDT Loro docs.
---

# Local-First P2P Sync & Mesh Routing Guide

This skill guides the design, troubleshooting, and extension of the peer-to-peer synchronization system in the Yntra Platform.

---

## 1. WebRTC & Signaling Mesh Router

Yntra implements real-time collaboration using a WebRTC peer mesh coordination system.

* **Peer Registration**: Every client node must register on the coordination network using `P2PMeshSyncRouter::register_peer_network(peer_id)`.
* **State Broadcast**: Whenever local writes commit successfully to the zero-copy storage, the client should serialize the updated logs and broadcast them via the router to other peers.
* **Refer to Example**: See [examples/p2p_mesh_router.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/local-first-p2p-sync/examples/p2p_mesh_router.rs) for a complete Rust API orchestration setup.

---

## 2. CRDT Integration & Conflict Resolution

* **Loro CRDT**: The database blocks are serialized into Loro Snapshot documents stored in database BLOBs.
* **Deterministic Merges**: When updates are received from the mesh network, they are merged back using Loro document updates. The CRDT resolves divergent state changes automatically based on logical timestamps.
* **Loopback Channel**: A loopback channel mock can be enabled during test environments to simulate synchronization between simulated client threads in the browser playground.

---

## 3. Best Practices

* **Bandwidth Optimization**: Only broadcast delta updates (binary logs) rather than full database snapshot states.
* **Reconnection Handling**: Implement exponential backoff if the signaling server disconnects. Maintain local transaction log buffers so messages can be sent once the connection is recovered.
