# Privacy-Preserving Telemetry & Observability Architecture

Comprehensive specification for Yntra Platform's opt-in performance telemetry, zero-PII data collection, and observability infrastructure.

---

## 1. Core Principles & Zero-PII Policy

Yntra Platform implements a strict **Zero-PII (Personally Identifiable Information)** telemetry architecture:

- **Opt-In Default**: Performance telemetry is disabled by default (`opt_in = false`). Collection only starts after explicit user consent.
- **Anonymized Aggregation**: Metric durations (TTI, FFI cross-boundary latency, frame render times, sync reconciliation) are aggregated in-memory as numerical averages and counters.
- **Zero Content Logging**: No user identifiers, email addresses, IP addresses, workspace names, or document contents are ever transmitted or stored.

---

## 2. Tracked Performance Metrics

| Metric Category | Target Symbol | Measurement Unit | Description |
| :--- | :--- | :--- | :--- |
| **Time to Interactive (TTI)** | `TTI_MILLIS` | Milliseconds (`ms`) | Duration from initial web/desktop boot to WASM database ready state. |
| **FFI Cross-Boundary Latency** | `FFI_TOTAL_MICROS` | Microseconds (`µs`) | Microseconds spent marshalling UniFFI arguments and return values across native boundaries. |
| **Frame Render Duration** | `FRAME_RENDER_TOTAL_MICROS` | Microseconds (`µs`) | Time spent executing Dioxus component tree updates and DOM diffing. |
| **Sync Reconciliation** | `SYNC_RECON_TOTAL_MICROS` | Microseconds (`µs`) | Duration and payload byte volume of Loro CRDT document delta reconciliations. |

---

## 3. Rust Core Telemetry API (`yntra-core`)

The metrics collector exposes UniFFI records and export functions:

```rust
use yntra_core::{
    set_telemetry_opt_in, get_performance_summary, record_ffi_latency,
    record_sync_reconciliation, TelemetryOptInSettings
};

// 1. Set user consent
set_telemetry_opt_in(TelemetryOptInSettings {
    opt_in: true,
    allow_performance_metrics: true,
    allow_crash_diagnostics: true,
})?;

// 2. Log UniFFI cross-boundary call duration
let start = std::time::Instant::now();
// ... execute FFI database operation ...
record_ffi_latency(start.elapsed().as_micros() as u64);

// 3. Fetch aggregated summary
let summary = get_performance_summary();
println!("Avg FFI Latency: {} us, Avg Sync Time: {} ms", summary.avg_ffi_latency_us, summary.avg_sync_recon_ms);
```

---

## 4. UI Consent Component (`yntra-ui`)

The `TelemetryPrivacySettingsModal` provides users with transparent controls:

```rust
components::TelemetryPrivacySettingsModal {}
```
