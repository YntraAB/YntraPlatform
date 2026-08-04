# FFI Service Catalog & Multi-Framework Examples

Exhaustive reference index of exported Rust core service functions (`yntra-core/src/lib.rs`) across all 24 operational service modules.

---

## 1. Core Service Modules Index

| Service Module | Source File / Directory | Key UniFFI Export Functions |
| :--- | :--- | :--- |
| **Authentication** | [`services/auth/sso.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/sso.rs) | [`initiate_enterprise_sso`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/sso.rs#L14-L45), [`complete_enterprise_sso_login`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/sso.rs#L50-L90) |
| **Hardware Passkeys** | [`services/auth/hardware/mod.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/hardware/mod.rs) | [`register_passkey_credential`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/hardware/mod.rs#L120-L160), [`authenticate_with_passkey`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/hardware/mod.rs#L180-L220) |
| **Workspaces** | [`services/workspaces.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/workspaces.rs) | [`get_workspaces`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/workspaces.rs#L82-L120), [`create_workspace`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/workspaces.rs#L150-L190), [`update_workspace_modules`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/workspaces.rs#L210-L240) |
| **Users & GDPR** | [`services/users/mod.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/users/mod.rs) | [`get_users`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/users/mod.rs#L40-L80), [`get_user_by_email`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/users/mod.rs#L100-L130), [`export_user_personal_data`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/users/mod.rs#L378-L430) |
| **Teams & Members** | [`services/teams.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/teams.rs) | [`get_teams`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/teams.rs#L25-L60), [`create_team`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/teams.rs#L70-L110) |
| **Time Reporting** | [`services/time_reports/mod.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/time_reports/mod.rs) | [`get_time_reports`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/time_reports/mod.rs#L45-L90), [`submit_time_report`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/time_reports/mod.rs#L100-L140) |
| **Care Assistance** | [`services/clients/profile.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/clients/profile.rs) | [`get_care_clients`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/clients/profile.rs#L30-L75), [`log_medication_event`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/clients/profile.rs#L120-L160) |
| **Jobs & Media** | [`services/jobs/media.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/jobs/media.rs) | [`upload_media_chunk`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/jobs/media.rs#L152-L210), [`enqueue_offline_media_blob`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/jobs/media.rs#L50-L95) |
| **Vehicle Fleet** | [`services/vehicles/mod.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/vehicles/mod.rs) | [`get_fleet_vehicles`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/vehicles/mod.rs#L20-L60), [`record_vehicle_inspection`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/vehicles/mod.rs#L80-L120) |
| **Notifications** | [`services/in_app_notifications.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/in_app_notifications.rs) | [`register_device_push_token`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/in_app_notifications.rs#L188-L230), [`get_user_notifications`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/in_app_notifications.rs#L40-L85) |
| **Auto Updater** | [`services/updater.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs) | [`verify_manifest_signature`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs#L190-L220), [`stage_binary_update`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs#L240-L280) |

---

## 2. Multi-Framework Implementation Examples

:::tabs

== Swift (iOS)
```swift
import SwiftUI
import yntra_core

func handleEnterpriseLogin(email: String) async {
    do {
        let session = try await initiateEnterpriseSso(
            userEmailOrDomain: email,
            redirectUri: "yntra://sso/callback"
        )
        print("SSO Domain: \(session.domain), Auth URL: \(session.authorizationUrl)")
    } catch {
        print("SSO Initiation error: \(error)")
    }
}
```

== Kotlin (Android)
```kotlin
import kotlinx.coroutines.launch
import uniffi.yntra_core.initiateEnterpriseSso

fun handleEnterpriseLogin(email: String) {
    viewModelScope.launch {
        try {
            val session = initiateEnterpriseSso(
                userEmailOrDomain = email,
                redirectUri = "yntra://sso/callback"
            )
            println("SSO Domain: ${session.domain}, Auth URL: ${session.authorizationUrl}")
        } catch (e: Exception) {
            println("SSO Initiation error: ${e.message}")
        }
    }
}
```

== Dioxus (Rust Web/Desktop)
```rust
use dioxus::prelude::*;
use yntra_core::initiate_enterprise_sso;

async fn trigger_sso(email: String) {
    if let Ok(session) = initiate_enterprise_sso(email, "https://app.yntra.se/sso/callback".to_string()).await {
        println!("SSO Session: {}, URL: {}", session.session_id, session.authorization_url);
    }
}
```

:::
