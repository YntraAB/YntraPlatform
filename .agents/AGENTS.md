# Project Rules: Yntra Platform

This file defines the project-scoped rules for AI assistants (like Antigravity) working in this workspace.

---

## Workspace Rules

1.  **Architecture Integrity**:
    *   Maintain the strict division between `yntra-core` (Rust engine) and UI clients.
    *   Never write UI component changes that directly contact APIs or invoke raw filesystems, bypassing the libSQL database layer.
2.  **No Unverified FFI Changes**:
    *   Always verify that Rust core signatures compile successfully before running UniFFI bindings generation.
3.  **Rust Coding Standards**:
    *   Use the thread-safe global `libsql` connection for database queries. Ensure WAL mode is active for all test connections.
    *   Use `rkyv` for serialization. Avoid `serde_json` for hot paths.
4.  **UI Updates**:
    *   Follow Dioxus 0.7+ fine-grained signal and resource patterns. Do not use legacy hook systems.
    *   Ensure all desktop UI dependencies remain multi-platform compatible (Wry/GPUI friendly).
5.  **Reactive FFI & Single Source of Truth**:
    *   Do not poll the FFI boundary for data updates. Always use the `DatabaseObserver` callback interface to notify UI layers of changes.
    *   Do not duplicate database state in UI components. The UI must act as a read-only projection of the core database state, triggered by observer notifications.