# Yntra Platform Benchmark Suite & Performance Guide

The Yntra Platform features a **State-Of-The-Art (SOTA) Criterion benchmark suite** within `yntra-core` to ensure sub-millisecond, zero-delay performance across all local-first enterprise subsystems.

---

## Benchmark Suite Architecture

The benchmark suite consists of **21 specialized benchmark targets** covering all 5 architectural layers of the platform:

| Layer | Benchmark Target | Subsystem & Metrics Measured | Throughput Metrics |
| :--- | :--- | :--- | :--- |
| **ZK & Cryptography** | [`zk_crypto_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/zk_crypto_benchmarks.rs) | Groth16 (BN254) Zero-Knowledge schema proofs, AOS ring signatures, FDA Part 11 signature verification | `ops/sec` |
| | [`crypto_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/crypto_benchmarks.rs) | Argon2id key derivation, ChaCha20Poly1305 AEAD encryption, BLAKE3 hashing | `MB/sec`, `ops/sec` |
| | [`auth_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/auth_benchmarks.rs) | TOTP secret validation, AuthContext cache lookup & invalidation, ephemeral session token validation | `ops/sec` |
| | [`ssss_and_session_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/ssss_and_session_benchmarks.rs) | Shamir's Secret Sharing Scheme threshold splitting/reconstruction, ephemeral session key derivation | `ops/sec` |
| **Database & Sync** | [`database_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/database_benchmarks.rs) | libSQL in-memory CRUD, transactional batch commits (100, 500 records), SQL AST sanitization, Rx database observer fan-out | `records/sec` |
| | [`sync_crdt_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/sync_crdt_benchmarks.rs) | Loro CRDT document text edits, state vector exports, snapshot imports, multi-peer change merging | `MB/sec` |
| | [`serialization_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/serialization_benchmarks.rs) | `rkyv` zero-copy serialization vs `serde_json` parsing across 10 to 5,000 record arrays | `records/sec` |
| | [`sql_parser_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/sql_parser_benchmarks.rs) | SQLite AST dialect parsing, comment stripper, query rewrite sanitizer | `MB/sec` |
| **Security & Governance** | [`guardrails_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/guardrails_benchmarks.rs) | Semantic guardrails engine, intent lease preflight checks, input threat sanitization | `MB/sec` |
| | [`dlp_and_p2p_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/dlp_and_p2p_benchmarks.rs) | DLP PHI/SSN payload content inspection (64B to 1MB), WebRTC P2P mesh sync router | `MB/sec` |
| | [`siem_and_integrity_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/siem_and_integrity_benchmarks.rs) | SIEM audit log formatters (CEF, Syslog RFC5424), Merkle log hash chain verification | `MB/sec` |
| | [`pre_sync_projection_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/pre_sync_projection_benchmarks.rs) | Role-based field masking projections, client write-back payload sanitization | `payloads/sec` |
| | [`time_integrity_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/time_integrity_benchmarks.rs) | Vector clock monotonicity checks, NTP drift window auditing, batch clock sequence processing | `events/sec` |
| **Enterprise Pipelines** | [`csv_import_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/csv_import_benchmarks.rs) | Delimiter auto-detection, CSV preview parser (10, 100, 1000 rows), bulk DB ingestion | `rows/sec` |
| | [`event_bus_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/event_bus_benchmarks.rs) | Event rule registration, trigger event pattern matching, multi-block event dispatch | `events/sec` |
| | [`healthcare_standards_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/healthcare_standards_benchmarks.rs) | FHIR R4 JSON validation, HL7 v2 ER7 parser, NCPDP SCRIPT XML parser | `MB/sec` |
| | [`ai_automation_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/ai_automation_benchmarks.rs) | AI guardrails evaluation, voice transcript parser, prompt length scaling (100 to 10,000 bytes) | `tokens/sec` |
| **Business & Operations** | [`billing_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/billing_benchmarks.rs) | Seat pricing tier resolution, workspace subscription lookups, feature gate access checks | `checks/sec` |
| | [`recovery_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/recovery_benchmarks.rs) | Passkey threshold keypair generation, SSO VDI fallback key derivation, master vault key reconstruction | `ops/sec` |
| | [`services_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/services_benchmarks.rs) | Dynamic entity role filtering, workspace configuration loading, SIEM formatter benchmarks | `ops/sec` |
| | [`dynamic_entities_and_roles_benchmarks`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/benches/dynamic_entities_and_roles_benchmarks.rs) | Role inheritance hierarchy resolution, dynamic attribute field permissions | `ops/sec` |

---

## Running Benchmarks

### 1. Run All Benchmarks
To run full statistical benchmarking across all 21 targets:
```bash
cargo bench --manifest-path yntra-core/Cargo.toml
```

### 2. Run a Single Benchmark Suite
When modifying a single section of the app, run only that subsystem's benchmark target:
```bash
# Run only Auth benchmarks
cargo bench --manifest-path yntra-core/Cargo.toml --bench auth_benchmarks

# Run only Database benchmarks
cargo bench --manifest-path yntra-core/Cargo.toml --bench database_benchmarks

# Run only CRDT sync benchmarks
cargo bench --manifest-path yntra-core/Cargo.toml --bench sync_crdt_benchmarks
```

### 3. Run a Targeted Benchmark Filter (Specific Function)
To benchmark a single function or sub-section matching a name pattern:
```bash
# Benchmark TOTP operations only
cargo bench --manifest-path yntra-core/Cargo.toml --bench auth_benchmarks -- "totp"

# Benchmark feature tier gate checks only
cargo bench --manifest-path yntra-core/Cargo.toml --bench billing_benchmarks -- "check_feature_tier_gate"
```

### 4. Fast Sub-Second Smoke Verification (`cargo test`)
To run a rapid 1-iteration sanity test without full statistical iterations:
```bash
cargo test --manifest-path yntra-core/Cargo.toml --bench event_bus_benchmarks
cargo test --manifest-path yntra-core/Cargo.toml --bench auth_benchmarks -- "totp"
```
