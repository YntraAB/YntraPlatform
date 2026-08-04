---
title: "Zero-Copy Serialization (rkyv)"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

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

This reduces cross-boundary latency from milliseconds to nanoseconds.