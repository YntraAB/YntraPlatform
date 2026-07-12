---
name: wasm-safe-rust-and-sync
description: Guidelines for compiling yntra-core to WASM, target-wasm32 vs target-native compatibility, the SQLite/libSQL dual-database model, and running schemas/migrations.
---

# WASM-Safe Rust Core & SQLite Sync Guide

This skill defines the constraints and patterns for maintaining WebAssembly (WASM) compilation compatibility in the shared logic core (`yntra-core`), alongside instructions for working with the SQLite/libSQL dual-database configuration.

---

## 1. WebAssembly (WASM) Compatibility Rules

The logic core must compile to `wasm32-unknown-unknown` to run inside web browsers and Web Workers.

* **No Blocking Calls**: Avoid thread blocking functions (e.g., `std::thread::sleep`, `std::fs::read` synchronous I/O, or blocking mutex locks across asynchronous boundaries).
* **Use Async Alternatives**: Use async locks (e.g., `futures_util::lock::Mutex`) and standard futures.
* **Isolate Native Operations**: Use conditional compilation block attributes to prevent importing non-WASM compatible libraries.
* **Examples**: Refer to [examples/wasm_conditional.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/wasm-safe-rust-and-sync/examples/wasm_conditional.rs) for conditional target compilation and the single-thread browser wrapper `SendFuture`.

---

## 2. SQLite / libSQL Dual-Database Engine

The database driver changes depending on the execution target:

```
                  +---------------------------+
                  |         yntra-core        |
                  +-------------+-------------+
                                |
             +------------------+------------------+
             |                                     |
   (target_arch != "wasm32")             (target_arch == "wasm32")
             |                                     |
    [Embedded libSQL]                        [JS Worker Bridge]
  - Native file access                     - db-bridge.js
  - Direct connection pool                 - db-worker.js (SQLite WASM)
  - WAL transaction mode                   - OPFS (Origin Private File System)
```

### Native Environment:
* Uses the Rust `libsql` client crate in-process.
* Maintain WAL mode active for performance during test connections.

### Web Environment:
* Uses SQLite WASM running inside a Web Worker utilizing the **Origin Private File System (OPFS)**.
* Communication happens via `wasm_bindgen` calling `yntra_execute_sql` in `db-bridge.js` which posts messages to `db-worker.js`.

---

## 3. Schemas and Database Migrations

* **Standard SQLite Dialect**: Write migration queries compatible with standard SQLite. Do not use server-only features.
* **Partitioning**: Ensure all data tables contain a `workspace_id` column to enforce strict client partitioning.
* **Sync Status**: Include a tracking column:
  ```sql
  sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
  ```
* **Initialization**: Run `setup_schema()` during app startup. The setup routine checks table existence reactively, runs incremental migrations, and sets up tables automatically.

---

## 4. Verification

Always verify WASM target compatibility before committing modifications:
```bash
cargo check --target wasm32-unknown-unknown -p yntra-core
```
If this check fails, verify that no native filesystem or thread operations are imported inside the WASM target path.
