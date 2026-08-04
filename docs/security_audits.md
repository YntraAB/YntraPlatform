# Third-Party Security Audits & SAST Guidelines

This document details the audit procedures, SAST tool configurations, and penetration testing guidelines for verifying `yntra-core`, UniFFI bindings, zero-knowledge crypto implementations, and local storage safety.

---

## 1. Audit Scope & Component Boundaries

```
                 +---------------------------------------+
                 |       Yntra Platform Workspace        |
                 +---------------------------------------+
                                     |
         +---------------------------+---------------------------+
         |                                                       |
+-------------------+                                 +-------------------+
|    yntra-core     |                                 |     yntra-ui      |
|  (Rust Engine)    |                                 |  (Dioxus / WASM)  |
+-------------------+                                 +-------------------+
         |                                                       |
         +----------+------------------------+-------------------+
                    |                        |
         +--------------------+    +--------------------+
         |   UniFFI Bindings  |    |  ZK Crypto & OPFS  |
         |  (Android / iOS)   |    | (Local Storage)    |
         +--------------------+    +--------------------+
```

---

## 2. Automated Cargo Audit & SAST Setup

### Running Local Security Audits
To execute security audits locally prior to commit:

```bash
# 1. Audit Rust dependencies against RustSec database
cargo audit

# 2. Static Analysis & Security Linting
cargo clippy --workspace --all-targets -- -D warnings
```

### GitHub Actions CI Verification
Continuous integration automatically executes `cargo audit` and Clippy SAST checks on every pull request and commit to `main`.

---

## 3. Third-Party Audit Checklist

| Target Area | Audit Focus | Verification Methodology | Status |
|---|---|---|---|
| **yntra-core** | Thread safety, WAL transaction isolation, SQL injection prevention | SAST + Rust race detector + DB isolation unit tests | ✅ Compliant |
| **UniFFI Bindings** | Memory safety at FFI boundary, null pointer checks, C-ABI layout | Swift / Kotlin binding generation tests & memory leak analysis | ✅ Compliant |
| **ZK Crypto** | Constant-time execution, zero-knowledge proof generation, hardware key store safety | Cryptographic verification & entropy analysis | ✅ Compliant |
| **Local Storage Safety** | SQLite WAL file permissions, WASM OPFS origin sandbox isolation, AES-256-GCM envelope encryption | Penetration testing & filesystem permission audits | ✅ Compliant |

---

## 4. Penetration Testing Guidelines
1. **Local Storage Inspection**: Verify that unencrypted plain-text secrets are never written to disk or browser local storage.
2. **P2P Channel Interception**: Verify WebRTC DataChannel payloads are resistant to replay and man-in-the-middle attacks using noise protocol / TLS 1.3 frame validation.
3. **Authentication Bypass**: Verify that FFI endpoints perform auth validation using `AuthContext::authorize` prior to executing queries.
