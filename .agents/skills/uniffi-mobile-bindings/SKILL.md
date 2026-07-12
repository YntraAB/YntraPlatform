---
name: uniffi-mobile-bindings
description: Guidelines for exporting UniFFI interfaces, configuring rkyv zero-copy serialization, implementing and subscribing to the DatabaseObserver, and compiling Kotlin/Swift bindings.
---

# UniFFI Mobile Bindings Guide

This skill defines instructions for modifying the FFI boundary between `yntra-core` (Rust engine) and native mobile clients (SwiftUI iOS & Jetpack Compose Android).

---

## 1. Modifying FFI Signatures (UniFFI Rules)

> [!IMPORTANT]
> Never make unverified FFI changes. Always verify that Rust core signatures compile successfully before running UniFFI bindings generation.

* **Exposing Structures**: Annotate exported structs with `#[derive(uniffi::Record)]` (for plain data objects copied across boundary) or `#[uniffi::Object]` (for shared stateful objects referenced by pointer/ARC).
* **Exposing Functions**: Annotate exported Rust functions at the crate root with `#[uniffi::export]`.
* **WASM Target Safety**: Keep FFI macros target-agnostic or ensure they are excluded when compiling to `wasm32-unknown-unknown` if they rely on features not supported by WASM (such as blocking OS threads).

---

## 2. Zero-Copy FFI Optimization (`rkyv`)

For hot paths (e.g. real-time P2P updates, rendering massive lists of data), use zero-copy serialization to avoid deserialization overhead:

1. Annotate transfer models with `#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]`.
2. Convert structures to binary archives via `rkyv::to_bytes`.
3. Pass raw byte vectors (`Vec<u8>`) over the UniFFI boundary.
4. On the receiving client layer, view the binary bytes as an archive reference instantly without decoding the entire payload.

---

## 3. DatabaseObserver & Reactive Updates

Do not poll the FFI boundary for database updates. Always use the `DatabaseObserver` callback interface to notify UI layers of changes.

* Refer to [examples/database_observer.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/uniffi-mobile-bindings/examples/database_observer.rs) for a complete template showing how to define, register, and notify observers across the FFI boundary.

---

## 4. Verification Workflow

After making FFI or Rust core signature changes, execute the following commands to verify correctness:

1. **Verify Rust Core Compiles**:
   ```bash
   cargo check -p yntra-core
   ```
2. **Verify WASM Compatibility**:
   ```bash
   cargo check --target wasm32-unknown-unknown -p yntra-core
   ```
3. **Regenerate Bindings**:
   ```bash
   cargo run -p yntra-uniffi-bindgen
   ```
   This compiles the latest core, runs WASM target checks, and generates type-safe Kotlin/Swift bindings in the `generated_bindings` directory.
