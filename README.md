# Yntra Platform

Yntra is a **Dynamic Modular Workspace Engine** designed for organizations that need a tailored operational OS without the rigidity of traditional ERPs. It bridges the gap between workforce coordination, internal communication, and administrative management.

This repository uses a **Local-First Tripartite Architecture** designed for sub-millisecond responsiveness across Web, Desktop, and Mobile.

---

## The Vision: Modular Operations

Unlike fixed software, Yntra allows administrators to define the exact structure of their workspace.

- **Industry Templates:** Blueprints for Construction, Healthcare, Education, and more.
- **Block-Based Customization:** Compose tools by selecting functional blocks (Messaging, Scheduling, Daily Notes, Time Reporting) based on real employee needs.
- **Agentic Foundation:** Direct integration with local and cloud-based AI agents for automated operations auditing and workflow triggers.

---

## Tech Stack

To achieve maximum performance and sub-millisecond TTI, the app is split into a shared engine and native/cross-platform UI layers:

*   **Logic Core (`yntra-core`):** A shared Rust library compiling to WASM (Web) and static libraries (iOS, Android, Desktop) containing validation, database queries, and background sync logic.
*   **Web & Desktop Frontend:** **Dioxus 0.7+** (compiled to WASM for Web, native GPUI/Wry for Desktop), styled with Vanilla CSS/Tailwind.
*   **Mobile Frontend:** **SwiftUI (iOS)** and **Jetpack Compose (Android)** for native-fidelity animation, scrolling, and system API access.
*   **FFI Layer:** **UniFFI** for auto-generating type-safe Swift and Kotlin bindings from the Rust core.
*   **Data & State:** **libSQL / Turso** with local-first embedded replicas:
    *   *Web:* libSQL WASM running inside a Web Worker utilizing the **Origin Private File System (OPFS)**.
    *   *Native (Desktop & Mobile):* Embedded libSQL linked in-process using the Rust `libsql` client crate.
*   **Data Sync:** libSQL background replication combined with Automerge/Yjs CRDT binary logs stored in database BLOBs, ensuring deterministic conflict resolution for offline edits.
*   **Serialization:** **`rkyv`** for zero-copy FFI boundary crossing (dropping boundary latency to nanoseconds).

---

## Local Development

### 1. Build the Rust Core
Compile the core logic package:
```bash
cargo build --release -p yntra-core
```

### 2. Generate Mobile Bindings
Generate Swift and Kotlin bindings automatically (cross-platform):
```bash
# Compile core, run WASM target check, and generate Swift/Kotlin bindings:
cargo run -p yntra-uniffi-bindgen

# Generate bindings using the release profile:
cargo run -p yntra-uniffi-bindgen -- release

# Watch for source changes to yntra-core and automatically rebuild/regenerate:
cargo run -p yntra-uniffi-bindgen -- watch

# Configure local git pre-commit hook to automatically verify WASM target compatibility:
cargo run -p yntra-uniffi-bindgen -- install-hooks
```

### 3. Run Web / Desktop (Dioxus)
Run the Dioxus development server:
```bash
# Web
dx serve

# Desktop
dx run --platform desktop
```

### 4. Build for Production
To build the optimized client applications:
```bash
# Web WASM build
dx build --release
```

---

## Data Model & Sync

The app uses local libSQL as the single source of truth. Every transaction is written locally in `< 1ms` using local files (`file:local.db`) and then replicated.

- **Workspace Partitioning:** Row-level isolation using `workspace_id`.
- **Background Replication**: Bidirectional database sync is handled automatically by the libSQL client to a primary server, ensuring data consistency without writing custom sync pipelines.
- **Offline-First:** The app runs with full capability in complete offline environments; modifications are queued in a local transaction log and synced when connection resumes.
