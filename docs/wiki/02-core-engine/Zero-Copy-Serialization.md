# Zero-Copy Serialization (`rkyv`)

To eliminate millisecond-scale JSON parsing overhead across FFI memory boundaries, Yntra utilizes **`rkyv` zero-copy serialization**.

---

## 1. Zero-Copy Principle

Instead of deserializing a byte buffer into heap objects, host runtimes cast the memory buffer directly into an archived type reference (`rkyv::Archived<T>`).

```rust
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
pub struct ZeroCopyNotePayload {
    pub id: String,
    pub content: String,
    pub updated_at_ms: i64,
}
```

This reduces cross-boundary latency from milliseconds to nanoseconds within Rust memory buffers.

---

## 2. Host Boundary Allocations & FFI Realities

> [!WARNING]
> While `rkyv` provides nanosecond zero-copy dereferencing inside Rust memory space, crossing FFI boundaries into host UI runtimes (SwiftUI, Jetpack Compose, V8 JavaScript) **requires host heap allocations** (Swift structs, JVM objects, JS heap objects). Zero-copy benefits apply strictly to the core engine process space.

---

## 3. Schema Versioning & Migration Protocol

Because `rkyv` archived types are tightly bound to exact Rust struct memory layouts, binary struct changes can cause runtime deserialization panic errors on cached BLOBs.

To guarantee backward compatibility across app updates, all persisted `rkyv` payloads are wrapped in a **version-tagged binary envelope header**:

```rust
#[repr(C)]
pub struct ZeroCopyEnvelope {
    pub magic: [u8; 4],     // b"YNTR"
    pub schema_version: u16, // Version number (e.g. 1, 2)
    pub payload_len: u32,   // Length of binary payload
}
```

### Fallback Migration Strategy
When reading archived BLOBs:
1. **Magic & Version Verification**: If `schema_version` matches current binary layout, access directly via `rkyv::archived_root::<T>()`.
2. **Legacy Version Fallback**: If `schema_version` is older, deserialize using a fallback JSON/MessagePack payload decoder, apply migration transform, and re-archive using current `rkyv` struct version before writing back to disk.

