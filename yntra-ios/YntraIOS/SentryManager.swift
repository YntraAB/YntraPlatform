import Foundation
import Sentry

/// Unified Sentry Crash Reporting Manager for iOS SwiftUI Client
public final class SentryManager {
    public static let shared = SentryManager()

    private init() {}

    public func configure(
        dsn: String = ProcessInfo.processInfo.environment["SENTRY_DSN"] ?? "https://public@sentry.io/1234567",
        environment: String = ProcessInfo.processInfo.environment["SENTRY_ENVIRONMENT"] ?? "production",
        release: String = ProcessInfo.processInfo.environment["SENTRY_RELEASE"] ?? "yntra-ios@1.0.0"
    ) {
        // 1. Initialize iOS Native Sentry Cocoa SDK
        SentrySDK.start { options in
            options.dsn = dsn
            options.environment = environment
            options.releaseName = release
            options.tracesSampleRate = 0.1
            options.enableAutoSessionTracking = true
            options.enableAppHangTracking = true
        }

        // 2. Initialize Rust Shared Core Telemetry
        do {
            let config = SentryConfig(
                dsn: dsn,
                environment: environment,
                release: release,
                sampleRate: 0.1,
                debug: false
            )
            _ = try initTelemetry(config: config)
            print("[SentryManager] iOS & Rust core telemetry initialized successfully")
        } catch {
            print("[SentryManager] Failed to initialize Rust core telemetry: \(error)")
        }
    }
}
