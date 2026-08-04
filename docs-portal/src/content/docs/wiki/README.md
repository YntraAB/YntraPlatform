---
title: "Yntra Platform Technical Wiki"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

![Version](https://img.shields.io/badge/version-v0.1.0--stable-blue)
![Architecture](https://img.shields.io/badge/architecture-Local--First%20Tripartite-emerald)
![Coverage](https://img.shields.io/badge/coverage-100%25%20Verified-purple)

Welcome to the **Yntra Platform Technical Wiki**—the authoritative, single source of truth for Yntra's **Local-First Tripartite Architecture**, Rust Core Engine, FFI Service Catalog, Native Mobile Integration, Enterprise Security, and Cloud Operations.

---

## 📚 Wiki Sitemap

### [01. Getting Started](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/01-getting-started/Architecture-Overview.md)
- [Architecture Overview](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/01-getting-started/Architecture-Overview.md): Tripartite system design & zero-delay FFI principles.
- [Quickstart: Web & Desktop](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/01-getting-started/Quickstart-Web-Desktop.md): Dioxus 0.7+ workspace setup, `dx serve`, and `dx run`.
- [Quickstart: Native Mobile](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/01-getting-started/Quickstart-Mobile.md): iOS SwiftUI & Android Jetpack Compose FFI linking.

### [02. Core Engine](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/02-core-engine/Database-Schema-Reference.md)
- [Database Schema Reference](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/02-core-engine/Database-Schema-Reference.md): Complete libSQL table dictionary, columns, constraints & migrations.
- [Local-First Replication](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/02-core-engine/Local-First-Replication.md): libSQL replication engine, Web Worker OPFS storage, & offline transaction queues.
- [CRDT & P2P Mesh Sync](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/02-core-engine/CRDT-and-P2P-Mesh-Sync.md): Loro/Automerge binary BLOB merging & WebRTC peer mesh.
- [Zero-Copy Serialization](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/02-core-engine/Zero-Copy-Serialization.md): `rkyv` memory layout & nanosecond FFI boundary crossing.

### [03. API & FFI Reference](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/03-api-and-ffi-reference/FFI-Service-Catalog.md)
- [FFI Service Catalog & Multi-Framework Examples](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/03-api-and-ffi-reference/FFI-Service-Catalog.md): Exhaustive index of exported Rust core service functions with Swift, Kotlin, and Dioxus tabs.
- [Swift Bindings Guide](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/03-api-and-ffi-reference/Swift-Bindings-Guide.md): `SwiftDbObserver` & SwiftUI `ObservableObject` viewmodels.
- [Kotlin Bindings Guide](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/03-api-and-ffi-reference/Kotlin-Bindings-Guide.md): `KotlinDbObserver` & Jetpack Compose `StateFlow` viewmodels.

### [05. Enterprise & Security](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/05-enterprise-and-security/Identity-BankID-and-Passkeys.md)
- [Identity: BankID & Passkeys](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/05-enterprise-and-security/Identity-BankID-and-Passkeys.md): Live BankID v6 mTLS certificates & WebAuthn hardware keys.
- [Enterprise SSO (OIDC / SAML 2.0)](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/05-enterprise-and-security/Enterprise-SSO-OIDC-SAML.md): Okta, Microsoft Entra ID (Azure AD), & domain mapping.

### [06. Operations & Deployment](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/06-operations-and-deployment/Self-Hosting-sqld-libSQL.md)
- [Self-Hosting `sqld` & libSQL](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/06-operations-and-deployment/Self-Hosting-sqld-libSQL.md): Docker Compose, Turso replication, & cloud cluster deployment.
- [Web WASM OPFS Infrastructure & Browser Deployment](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/06-operations-and-deployment/Web-WASM-OPFS-Infrastructure.md): COOP/COEP HTTP response headers, Web Worker threading, & Safari OPFS quota management.
- [Code Signing & Packaging](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/06-operations-and-deployment/Code-Signing-and-Packaging.md): Windows EV `signtool` & macOS Apple Developer ID notarization.
- [Auto-Updater Pipeline](https://github.com/YntraAB/YntraPlatform/blob/main/docs/wiki/06-operations-and-deployment/Auto-Updater-Pipeline.md): Ed25519 update manifests, version comparison & delta staging.