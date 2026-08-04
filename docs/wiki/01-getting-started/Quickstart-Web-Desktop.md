# Quickstart: Web & Desktop (Dioxus 0.7+)

This guide walks through setting up, running, and building Yntra's Dioxus frontend ([`yntra-ui`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ui)) for Web (WASM) and Desktop (Native Wry/GPUI).

---

## 1. Prerequisites

- **Rust Toolchain**: 1.75+ with `wasm32-unknown-unknown` target.
- **Dioxus CLI**: Installed via `cargo install dioxus-cli`.
- **Node.js**: 20+ (for E2E Playwright testing).

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Dioxus CLI
cargo install dioxus-cli
```

---

## 2. Running Web (WASM)

```bash
# Serve Dioxus Web with hot-reloading
dx serve

# Build optimized Web WASM release payload
dx build --release
```

---

## 3. Running Desktop (Native Wry / GPUI)

```bash
# Run native desktop application
dx run --platform desktop

# Build desktop installer setup executable
build-desktop-installers.bat
```
