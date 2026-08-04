# Security Policy & Audit Framework

## Security Overview
Yntra Platform is committed to maintaining high standards of cryptographic integrity, data isolation, and software supply-chain security across all operating environments (Desktop, Web WASM, iOS, Android).

---

## 1. Vulnerability Reporting & Disclosure
If you discover a security vulnerability in `yntra-core`, UniFFI bindings, or `yntra-ui`:
- **Email**: `security@yntra.io`
- **Response SLA**: Initial response within 24 hours; fix deployment target within 7 business days for critical issues.
- **Encrypted Communication**: PGP key available upon request.

---

## 2. Automated Security Audits (SAST & Cargo Audit)
- **Dependency Audit**: Continuous security checks utilizing `cargo audit` in GitHub Actions CI to scan `Cargo.lock` against the RustSec Advisory Database.
- **Static Analysis (SAST)**: Mandatory `cargo clippy --workspace -- -D warnings` enforcement guarding against unsafe blocks, memory leaks, and arithmetic overflow vulnerabilities.

---

## 3. Cryptographic & Architecture Audits
- **`yntra-core` Engine**: Thread-safe Rust database abstraction layer backed by SQLite in WAL mode.
- **UniFFI Mobile Bindings**: Type-safe FFI memory boundary verified against memory leaks and invalid pointer dereferences.
- **Zero-Knowledge Crypto**: Envelope encryption (AES-256-GCM / ChaCha20-Poly1305) and ZK proof verifications for hardware passkeys and SITHS card identities.
- **Local Storage Safety**: SQLite WAL isolation on desktop/mobile and OPFS sandbox isolation on WASM targets.

For complete audit guidelines and penetration testing parameters, see [`docs/security_audits.md`](file:///c:/Users/hellich/Desktop/YntraPlatform/docs/security_audits.md).
