package com.yntra.app

import android.app.Application
import android.util.Log
import io.sentry.Sentry
import io.sentry.android.core.SentryAndroid
import uniffi.yntra_core.SentryConfig
import uniffi.yntra_core.initTelemetry

class YntraApplication : Application() {
    override fun onCreate() {
        super.onCreate()

        val sentryDsn = System.getenv("SENTRY_DSN") ?: "https://public@sentry.io/1234567"
        val env = System.getenv("SENTRY_ENVIRONMENT") ?: "production"
        val release = System.getenv("SENTRY_RELEASE") ?: "yntra-android@1.0.0"

        // 1. Initialize Android Native Sentry SDK
        SentryAndroid.init(this) { options ->
            options.dsn = sentryDsn
            options.environment = env
            options.release = release
            options.tracesSampleRate = 0.1
            options.isDebug = false
        }

        // 2. Initialize Rust Shared Core Telemetry
        try {
            val rustConfig = SentryConfig(
                dsn = sentryDsn,
                environment = env,
                release = release,
                sampleRate = 0.1,
                debug = false
            )
            initTelemetry(rustConfig)
            Log.i("YntraApplication", "Rust core telemetry bridge initialized successfully")
        } catch (e: Exception) {
            Log.e("YntraApplication", "Failed to initialize Rust core telemetry bridge: ${e.message}")
        }
    }
}
