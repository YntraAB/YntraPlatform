// Privacy-Preserving Telemetry & Performance Observability Service
// Measures TTI, FFI cross-boundary latency, frame render times, and sync reconciliation metrics.
// Strictly enforces OPT-IN user consent and ZERO-PII data collection.

use crate::YntraError;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static IS_TELEMETRY_OPTED_IN: AtomicBool = AtomicBool::new(false);

// Aggregate performance counters
static TTI_MILLIS: AtomicU64 = AtomicU64::new(0);
static FFI_TOTAL_CALLS: AtomicU64 = AtomicU64::new(0);
static FFI_TOTAL_MICROS: AtomicU64 = AtomicU64::new(0);
static FRAME_RENDER_TOTAL_CALLS: AtomicU64 = AtomicU64::new(0);
static FRAME_RENDER_TOTAL_MICROS: AtomicU64 = AtomicU64::new(0);
static SYNC_RECON_TOTAL_CALLS: AtomicU64 = AtomicU64::new(0);
static SYNC_RECON_TOTAL_MICROS: AtomicU64 = AtomicU64::new(0);
static SYNC_RECON_BYTES_PROCESSED: AtomicU64 = AtomicU64::new(0);

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct TelemetryOptInSettings {
    pub opt_in: bool,
    pub allow_performance_metrics: bool,
    pub allow_crash_diagnostics: bool,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetricSummary {
    pub tti_ms: u64,
    pub ffi_calls_count: u64,
    pub avg_ffi_latency_us: f64,
    pub frame_renders_count: u64,
    pub avg_frame_render_ms: f64,
    pub sync_recon_count: u64,
    pub avg_sync_recon_ms: f64,
    pub total_sync_bytes: u64,
}

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricCategory {
    TimeToInteractive,
    FfiCrossBoundary,
    FrameRender,
    SyncReconciliation,
}

#[uniffi::export]
pub fn set_telemetry_opt_in(settings: TelemetryOptInSettings) -> Result<bool, YntraError> {
    IS_TELEMETRY_OPTED_IN.store(settings.opt_in, Ordering::SeqCst);
    tracing::info!(
        "Telemetry opt-in status set to: {} (perf={}, crash={})",
        settings.opt_in,
        settings.allow_performance_metrics,
        settings.allow_crash_diagnostics
    );
    Ok(settings.opt_in)
}

#[uniffi::export]
pub fn is_telemetry_opted_in() -> bool {
    IS_TELEMETRY_OPTED_IN.load(Ordering::SeqCst)
}

#[uniffi::export]
pub fn record_tti_metric(duration_ms: u64) {
    if !is_telemetry_opted_in() { return; }
    TTI_MILLIS.store(duration_ms, Ordering::SeqCst);
    tracing::debug!("Recorded Time to Interactive (TTI): {} ms", duration_ms);
}

#[uniffi::export]
pub fn record_ffi_latency(duration_micros: u64) {
    if !is_telemetry_opted_in() { return; }
    FFI_TOTAL_CALLS.fetch_add(1, Ordering::SeqCst);
    FFI_TOTAL_MICROS.fetch_add(duration_micros, Ordering::SeqCst);
}

#[uniffi::export]
pub fn record_frame_render_time(duration_micros: u64) {
    if !is_telemetry_opted_in() { return; }
    FRAME_RENDER_TOTAL_CALLS.fetch_add(1, Ordering::SeqCst);
    FRAME_RENDER_TOTAL_MICROS.fetch_add(duration_micros, Ordering::SeqCst);
}

#[uniffi::export]
pub fn record_sync_reconciliation(duration_micros: u64, payload_bytes: u64) {
    if !is_telemetry_opted_in() { return; }
    SYNC_RECON_TOTAL_CALLS.fetch_add(1, Ordering::SeqCst);
    SYNC_RECON_TOTAL_MICROS.fetch_add(duration_micros, Ordering::SeqCst);
    SYNC_RECON_BYTES_PROCESSED.fetch_add(payload_bytes, Ordering::SeqCst);
}

#[uniffi::export]
pub fn get_performance_summary() -> PerformanceMetricSummary {
    let tti = TTI_MILLIS.load(Ordering::SeqCst);
    let ffi_count = FFI_TOTAL_CALLS.load(Ordering::SeqCst);
    let ffi_micros = FFI_TOTAL_MICROS.load(Ordering::SeqCst);
    let frame_count = FRAME_RENDER_TOTAL_CALLS.load(Ordering::SeqCst);
    let frame_micros = FRAME_RENDER_TOTAL_MICROS.load(Ordering::SeqCst);
    let sync_count = SYNC_RECON_TOTAL_CALLS.load(Ordering::SeqCst);
    let sync_micros = SYNC_RECON_TOTAL_MICROS.load(Ordering::SeqCst);
    let sync_bytes = SYNC_RECON_BYTES_PROCESSED.load(Ordering::SeqCst);

    PerformanceMetricSummary {
        tti_ms: tti,
        ffi_calls_count: ffi_count,
        avg_ffi_latency_us: if ffi_count > 0 { ffi_micros as f64 / ffi_count as f64 } else { 0.0 },
        frame_renders_count: frame_count,
        avg_frame_render_ms: if frame_count > 0 { (frame_micros as f64 / frame_count as f64) / 1000.0 } else { 0.0 },
        sync_recon_count: sync_count,
        avg_sync_recon_ms: if sync_count > 0 { (sync_micros as f64 / sync_count as f64) / 1000.0 } else { 0.0 },
        total_sync_bytes: sync_bytes,
    }
}
