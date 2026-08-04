---
title: "FFI & Toolchain Build Error Troubleshooting"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Common build issues and resolution procedures when compiling `yntra-core` across targets.

---

## 1. MSVC C++ Compiler Missing (`os error 5` / `libsql-ffi`)

- **Symptom**: `cargo build` or release compilation fails with `failed to run custom build command for libsql-ffi`.
- **Fix**: Install Visual Studio Build Tools with **Desktop development with C++** component, or compile using the MSVC developer command prompt.

---

## 2. WASM Target Compatibility Checks

- **Symptom**: `target wasm32-unknown-unknown` check fails during pre-commit hook.
- **Fix**: Run `cargo run -p yntra-uniffi-bindgen` to execute target WASM verification and binding regeneration.