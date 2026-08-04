# Crash & Error Reporting Architecture

Comprehensive documentation for Yntra Platform's unified crash reporting, real-time error logging, and Sentry SDK integration across all client targets (Rust Shared Core, Dioxus Web/Desktop, Android Compose, iOS SwiftUI).

---

## 1. Multi-Target Telemetry Matrix

| Target Layer | Sentry SDK / Module | Initialization Entrypoint | Target Signal Capture |
| :--- | :--- | :--- | :--- |
| **Rust Engine Core** | `yntra_core::services::telemetry` | `init_telemetry(config)` | Panic hook, structured `tracing` logs, UniFFI error bridge |
| **Dioxus Web/Desktop** | `sentry-init.js` + `Sentry.Browser` | `<script src="/public/sentry-init.js">` | Uncaught WASM panics, JS runtime errors, session replay |
| **Android (Kotlin)** | `io.sentry:sentry-android` | `YntraApplication.kt` | Native ANRs, Kotlin uncaught exceptions, JNI panic bridge |
| **iOS (Swift)** | `Sentry` Cocoa SDK | `SentryManager.swift` | Mach-O crashes, Swift runtime errors, Rust UniFFI exceptions |

---

## 2. Shared Environment & DSN Parameters

Sentry telemetry is configured dynamically via environment variables across all native targets:

```env
SENTRY_DSN=https://<public_key>@sentry.io/<project_id>
SENTRY_ENVIRONMENT=production
SENTRY_RELEASE=yntra-platform@1.0.0
```

---

## 3. Rust Core Telemetry API (`yntra-core`)

Rust shared engine exports UniFFI records and helper methods:

```rust
use yntra_core::{init_telemetry, capture_exception, SentryConfig, TelemetryLevel};

let config = SentryConfig {
    dsn: "https://public@sentry.io/1234567".to_string(),
    environment: "production".to_string(),
    release: "yntra-core@1.0.0".to_string(),
    sample_rate: 0.1,
    debug: false,
};

init_telemetry(config)?;
capture_exception("Database connection dropped".to_string(), TelemetryLevel::Error, Some("db".to_string()));
```

---

## 4. Native App Entrypoints

### Android (`YntraApplication.kt`)
```kotlin
SentryAndroid.init(this) { options ->
    options.dsn = sentryDsn
    options.environment = env
    options.release = release
}
initTelemetry(SentryConfig(...))
```

### iOS (`SentryManager.swift`)
```swift
SentryManager.shared.configure(dsn: dsn, environment: env, release: release)
```

### Web (`sentry-init.js`)
```javascript
window.Sentry.init({
  dsn: sentryDsn,
  environment: environment,
  release: release,
  tracesSampleRate: 0.1
});
```
