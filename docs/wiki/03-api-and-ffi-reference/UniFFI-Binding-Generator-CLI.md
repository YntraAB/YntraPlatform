# UniFFI Binding Generator CLI Tool (`yntra-uniffi-bindgen`)

The **`yntra-uniffi-bindgen` CLI tool** ([`yntra-uniffi-bindgen/src/main.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-uniffi-bindgen/src/main.rs)) automates FFI binding generation, scaffolding checks, git pre-commit hook installation, and locale verification across the workspace.

---

## 1. CLI Commands & Execution Modes

Run the bindgen tool via `cargo run -p yntra-uniffi-bindgen -- <command>`:

| Command | Purpose |
| :--- | :--- |
| `generate` | Standard UniFFI binding generator targeting Swift (`generated_bindings/swift`) & Kotlin (`generated_bindings/kotlin`). |
| `watch` / `--watch` | File-watcher mode rebuilding FFI bindings automatically upon saving `yntra-core/src/lib.rs`. |
| `release` / `--release` | Compiles optimized release static/dynamic libraries (`.a`, `.dylib`, `.so`) and headers. |
| `install-hooks` | Configures Git `.git/hooks/pre-commit` to execute binding verification before commits. |
| `check-locales` | Validates Fluent `.ftl` translation bundle parity across `sv`, `no`, `da`, `fi`, `en`. |
| `fix` / `--fix` | Automatically formats and syncs missing key stubs into translation files. |

---

## 2. Binding Output Architecture

```bash
# Generate Swift & Kotlin UniFFI bindings
cargo run -p yntra-uniffi-bindgen generate
```

```
generated_bindings/
├── kotlin/
│   └── uniffi/
│       └── yntra_core/
│           └── yntra_core.kt
└── swift/
    ├── yntra_core.swift
    └── yntra_coreFFI.h
```

---

## 3. Git Pre-Commit Hook Integration

To ensure Rust core interface signatures match FFI bindings before code is committed to version control:

```bash
# Install automated workspace git pre-commit hook
cargo run -p yntra-uniffi-bindgen install-hooks
```
