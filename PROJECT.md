# Project: Yntra Platform Security Audit and Hardening

## Architecture
- **yntra-core**: Rust engine handling local-first SQLite/libSQL database connections, cryptography operations, and core business services.
- **FFI Bindings**: Exposes `yntra-core` functions to mobile clients (Android/iOS) via UniFFI.
- **yntra-ui**: Frontend client built using Dioxus.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Exploration & Audit | Perform detailed codebase audit for security and bug issues | None | DONE (83e008df-2bf3-484a-8497-12560a6e0117) |
| 2 | Initial Vulnerability Report | Document findings in `vulnerability_report.md` | M1 | DONE (83beb631-d31a-47da-adc7-965aa63d21aa) |
| 3 | Patch Implementation | Fix critical/high severity vulnerabilities in Rust engine/FFI | M2 | DONE (6aaaa79c-d446-4f8c-9b0e-dee5122f0392) |
| 4 | Verification & Audit Gating | Review, challenge, and execute forensic audit verification | M3 | DONE (d64fb1da, 362e055d, 61005e9f, 29ae438b, 075213c2) |
| 5 | Final Documentation | Finalize report with verification results and resolution statuses | M4 | DONE (self) |

## Interface Contracts
- Rust Core APIs exposed to UniFFI must compile successfully without warnings.
- Database access must use thread-safe libsql.
- UI state must not be duplicated, subscribing to FFI database changes reactively.

## Code Layout
- `yntra-core/src/database/`: SQLite connection logic, sync protocols, schema migrations.
- `yntra-core/src/services/`: Core logic (audit, auth, workspaces, notes, time_reports, users, etc.).
- `yntra-ui/`: Dioxus client views and reactive signals.
- `yntra-uniffi-bindgen/`: UniFFI export definition and generator.
