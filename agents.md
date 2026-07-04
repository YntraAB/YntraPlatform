# AI Agent Rules: Yntra Platform

## Workspace Structure
```
YntraPlatform/
├── .agents/                  # Workspace rules and skills for AI assistants
├── docs/                     # Architectural specs and documentation
├── yntra-core/               # Shared engine core (DB, models, FFI exports)
├── yntra-ui/                 # Dioxus 0.7 UI (Web/Desktop) and component library
└── yntra-uniffi-bindgen/     # UniFFI mobile bindings generator utility
```

---

## Coding Constraints
*   **Logic Core**: All business logic, DB queries, validation, and sync **must** live in `yntra-core`. UI layers are pure views.
*   **Local-First**: All UI actions must read/write to the local libSQL database first. Never make direct network calls from UI layers.
*   **Zero-Copy**: Use `rkyv` for serialization across FFI boundaries.
*   **FFI**: Export interfaces via UniFFI attribute macros. Keep complex types internal to `yntra-core`.
*   **Dioxus 0.7**: Use Dioxus 0.7 fine-grained Signals and ensure WASM target compatibility.
