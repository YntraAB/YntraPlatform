# Architecture Overview: Tripartite Local-First System

Yntra Platform implements a **Local-First Tripartite Architecture** engineered for sub-millisecond Time-To-Interactive (TTI) responsiveness across Web, Desktop, and Native Mobile.

---

## 1. System Topology

```mermaid
graph TD
    subgraph UI ["Host UI Layer"]
        Dioxus["Dioxus 0.7+ (Web & Desktop)"]
        SwiftUI["SwiftUI (iOS)"]
        Compose["Jetpack Compose (Android)"]
    end

    subgraph Core ["Shared Rust Engine (yntra-core)"]
        ZeroCopy["rkyv Zero-Copy Store"]
        Validation["Business Logic & Schema"]
        libSQL["Embedded libSQL Client"]
    end

    subgraph Sync ["Sync & Network Layer"]
        Replication["libSQL Background Sync Client"]
        Mesh["P2P WebRTC Mesh Router"]
        Server["libSQL Primary (sqld / Turso)"]
    end

    Dioxus -->|WASM / WASI| Core
    SwiftUI -->|Swift FFI / UniFFI| Core
    Compose -->|JNI / Kotlin UniFFI| Core

    libSQL --> Replication
    Replication --> Server
    ZeroCopy --> Mesh
```

---

## 2. Core Architectural Principles

1. **Local-First Precedence**: Every user mutation is validated and written to local database replicas (`< 1ms`) before any network interaction takes place.
2. **Tripartite Separation**: UI clients (Dioxus, SwiftUI, Compose) act strictly as read-only projections of the local core state. UI components never query APIs directly.
3. **Reactive FFI Observers**: UI components subscribe to `DatabaseObserver` state notifications instead of polling.
4. **Zero-Copy & Versioned Memory Boundary**: Core Rust engine memory operations use `rkyv` zero-copy serialization with version-tagged binary envelopes. Crossing FFI into host UI layers (SwiftUI, Jetpack Compose, JS) materializes native language objects managed by host garbage collection / ARC.
5. **Resilient Network Fallbacks**: P2P WebRTC mesh transfers attempt direct ICE connection with a 3,000ms SLA before seamlessly degrading to TLS-encrypted TURN relay servers.

