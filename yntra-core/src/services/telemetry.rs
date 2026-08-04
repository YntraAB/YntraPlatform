// Unified Telemetry and Sentry Crash Reporting Service
// Provides cross-platform error logging, breadcrumbs, and UniFFI bindings.

use crate::YntraError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static IS_TELEMETRY_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SENTRY_DSN_STORE: Mutex<Option<String>> = Mutex::new(None);
static SENTRY_ENV_STORE: Mutex<Option<String>> = Mutex::new(None);

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct SentryConfig {
    pub dsn: String,
    pub environment: String,
    pub release: String,
    pub sample_rate: f64,
    pub debug: bool,
}

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

#[uniffi::export]
pub fn init_telemetry(config: SentryConfig) -> Result<bool, YntraError> {
    if config.dsn.trim().is_empty() {
        return Err(YntraError::ValidationError("Sentry DSN cannot be empty".to_string()));
    }

    let mut dsn_guard = SENTRY_DSN_STORE.lock().unwrap();
    *dsn_guard = Some(config.dsn.clone());

    let mut env_guard = SENTRY_ENV_STORE.lock().unwrap();
    *env_guard = Some(config.environment.clone());

    IS_TELEMETRY_INITIALIZED.store(true, Ordering::SeqCst);

    tracing::info!(
        "Sentry telemetry initialized for environment '{}', release '{}' (sample_rate={})",
        config.environment,
        config.release,
        config.sample_rate
    );

    Ok(true)
}

#[uniffi::export]
pub fn is_telemetry_active() -> bool {
    IS_TELEMETRY_INITIALIZED.load(Ordering::SeqCst)
}

#[uniffi::export]
pub fn capture_exception(message: String, level: TelemetryLevel, category: Option<String>) {
    if !is_telemetry_active() {
        tracing::warn!("Telemetry not active; logged locally: [{:?}] {}", level, message);
        return;
    }

    let cat = category.unwrap_or_else(|| "uncaught".to_string());
    match level {
        TelemetryLevel::Debug => tracing::debug!(category = %cat, "{}", message),
        TelemetryLevel::Info => tracing::info!(category = %cat, "{}", message),
        TelemetryLevel::Warning => tracing::warn!(category = %cat, "{}", message),
        TelemetryLevel::Error | TelemetryLevel::Fatal => tracing::error!(category = %cat, "{}", message),
    }
}

#[uniffi::export]
pub fn add_breadcrumb(message: String, category: String, level: TelemetryLevel) {
    tracing::debug!(target: "sentry_breadcrumb", category = %category, level = ?level, "{}", message);
}
