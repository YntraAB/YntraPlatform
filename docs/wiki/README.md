# Yntra Platform Technical Wiki

![Version](https://img.shields.io/badge/version-v0.1.0--stable-blue)
![Architecture](https://img.shields.io/badge/architecture-Local--First%20Tripartite-emerald)
![Coverage](https://img.shields.io/badge/coverage-100%25%20Verified-purple)

Welcome to the **Yntra Platform Technical Wiki**—the authoritative, single source of truth for Yntra's **Local-First Tripartite Architecture**, Rust Core Engine, FFI Service Catalog, Native Mobile Integration, Enterprise Security, and Cloud Operations.

---

## 📚 Wiki Sitemap

### [01. Getting Started](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/01-getting-started/Architecture-Overview.md)
- [Architecture Overview](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/01-getting-started/Architecture-Overview.md): Tripartite system design & zero-delay FFI principles.
- [Quickstart: Web & Desktop](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/01-getting-started/Quickstart-Web-Desktop.md): Dioxus 0.7+ workspace setup, `dx serve`, and `dx run`.
- [Quickstart: Native Mobile](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/01-getting-started/Quickstart-Mobile.md): iOS SwiftUI & Android Jetpack Compose FFI linking.

### [02. Core Engine](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/02-core-engine/Database-Schema-Reference.md)
- [Database Schema Reference](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/02-core-engine/Database-Schema-Reference.md): Complete libSQL table dictionary, columns, constraints & migrations.
- [Local-First Replication](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/02-core-engine/Local-First-Replication.md): libSQL replication engine, Web Worker OPFS storage, & offline transaction queues.
- [CRDT & P2P Mesh Sync](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/02-core-engine/CRDT-and-P2P-Mesh-Sync.md): Loro/Automerge binary BLOB merging & WebRTC peer mesh.
- [Zero-Copy Serialization](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/02-core-engine/Zero-Copy-Serialization.md): `rkyv` memory layout & nanosecond FFI boundary crossing.

### [03. API & FFI Reference](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/03-api-and-ffi-reference/FFI-Service-Catalog.md)
- [FFI Service Catalog & Multi-Framework Examples](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/03-api-and-ffi-reference/FFI-Service-Catalog.md): Exhaustive index of exported Rust core service functions with Swift, Kotlin, and Dioxus tabs.
- [Swift Bindings Guide](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/03-api-and-ffi-reference/Swift-Bindings-Guide.md): `SwiftDbObserver` & SwiftUI `ObservableObject` viewmodels.
- [Kotlin Bindings Guide](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/03-api-and-ffi-reference/Kotlin-Bindings-Guide.md): `KotlinDbObserver` & Jetpack Compose `StateFlow` viewmodels.

### [04. Functional Blocks](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Block-Registry-and-Routing.md)
- [Block Registry & Routing](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Block-Registry-and-Routing.md): UI block registration, navigation menus, and Dioxus router integration.
- [AI Automation Engine](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/AI-Automation-Engine.md): Natural language action triggers, voice report proposals, and daily digests.
- [Integrations & Webhooks](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Integrations-and-Webhooks.md): Fortnox, Visma, Stripe, RFC 4180 CSV parsing, & webhooks.
- [i18n & Fluent Localization](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/i18n-Localization-System.md): Mozilla Fluent localization (`sv`, `no`, `da`, `fi`, `en`) and static caching.
- [Reports & Data Import](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Reports-and-Data-Import.md): Analytics reports, CSV streaming import, & transactional batching.
- [Dynamic Entity Schemas](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Dynamic-Entity-Schemas.md): Dynamic custom field definitions & JSON schema validation.
- [Industry Templates](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/04-functional-blocks/Industry-Templates.md): Domain presets for Care, Field Jobs, School, & Enterprise.

### [05. Enterprise & Security](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Identity-BankID-and-Passkeys.md)
- [Identity: BankID & Passkeys](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Identity-BankID-and-Passkeys.md): Live BankID v6 mTLS certificates & WebAuthn hardware keys.
- [Enterprise SSO (OIDC / SAML 2.0)](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Enterprise-SSO-OIDC-SAML.md): Okta, Microsoft Entra ID (Azure AD), & domain mapping.
- [Envelope Encryption & ZK Proofs](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Envelope-Encryption-and-ZK.md): Field encryption & off-thread Groth16 ZK proof pipeline.
- [Shamir's Secret Sharing (SSSS)](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Shamir-Secret-Sharing-and-Key-Recovery.md): $GF(2^8)$ Galois Field $(k, n)$ threshold secret splitting & key recovery.
- [Labor Law & GDPR Compliance](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/05-enterprise-and-security/Labor-Law-and-GDPR-Compliance.md): Nordic labor law working-hour caps (*Arbetstidslagen*) & GDPR Article 17 hard-deletion.

### [06. Operations & Deployment](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/Self-Hosting-sqld-libSQL.md)
- [Self-Hosting `sqld` & libSQL](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/Self-Hosting-sqld-libSQL.md): Docker Compose, Turso replication, & cloud cluster deployment.
- [Web WASM OPFS Infrastructure & Browser Deployment](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/Web-WASM-OPFS-Infrastructure.md): COOP/COEP HTTP response headers, dual-domain embeds, & Safari OPFS quota management.
- [Code Signing & Packaging](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/Code-Signing-and-Packaging.md): Windows EV `signtool` & macOS Apple Developer ID notarization.
- [Auto-Updater Pipeline](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/Auto-Updater-Pipeline.md): Ed25519 update manifests, version comparison & delta staging.
- [End-to-End Testing with Playwright](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/06-operations-and-deployment/End-to-End-Testing-with-Playwright.md): Multi-browser test runner, mobile emulation, & failure video/trace capture.

### [07. Troubleshooting](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/07-troubleshooting/FFI-and-Build-Errors.md)
- [FFI & Toolchain Build Errors](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/07-troubleshooting/FFI-and-Build-Errors.md): MSVC C++ toolchain & WASM target resolution.
- [Storage & Quota Limits](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/wiki/07-troubleshooting/Storage-and-Quota-Limits.md): OPFS quota management & mobile sandbox paths.
