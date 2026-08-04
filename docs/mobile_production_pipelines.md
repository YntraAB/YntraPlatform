# Mobile Production Pipelines Specification

This document details the mobile CI/CD pipelines, code signing configurations, Fastlane release lanes, store metadata, and privacy label requirements for **Yntra Platform** on Android (`.aab` / `.apk`) and iOS (`.ipa` / TestFlight).

---

## 1. Android Release & Keystore Signing

### Android App Bundle (.aab) Build
* **Gradle Configuration**: [`yntra-android/app/build.gradle.kts`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-android/app/build.gradle.kts)
* **JNI Cross-Compilation**: Compiles `libyntra_core.so` for `aarch64-linux-android`, `armv7-linux-androideabi`, and `x86_64-linux-android`.

### Keystore Environment Variables
Set the following secrets in CI / GitHub Secrets:
* `ANDROID_KEYSTORE_BASE64`: Base64-encoded `.keystore` / `.jks` file content.
* `ANDROID_KEYSTORE_PASSWORD`: Keystore password.
* `ANDROID_KEY_ALIAS`: Key alias name.
* `ANDROID_KEY_PASSWORD`: Key password.
* `PLAY_STORE_JSON_KEY`: Google Play Developer API service account JSON key.

---

## 2. iOS Release, Signing & TestFlight

### Xcode Archive & Export
* **Xcode Project Generator**: `xcodegen` using [`yntra-ios/project.yml`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-ios/project.yml).
* **Export Options**: [`packaging/mobile/ExportOptions.plist`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/mobile/ExportOptions.plist) (`app-store` method).
* **Rust Core Target**: `aarch64-apple-ios`.

### App Store Connect API Keys
* `APP_STORE_CONNECT_API_KEY_ID`: Key ID from App Store Connect.
* `APP_STORE_CONNECT_ISSUER_ID`: Issuer ID UUID.
* `APP_STORE_CONNECT_API_KEY_BASE64`: Base64-encoded `.p8` API key file.
* `IOS_CERTIFICATE_BASE64`: Base64-encoded Apple Distribution Certificate (`.p12`).

---

## 3. Fastlane Deployment Lanes

Fastlane configuration is located in [`packaging/mobile/fastlane/`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/mobile/fastlane/):
* **`fastlane android_beta`**: Compiles signed `.aab` and uploads to Google Play Internal Test track.
* **`fastlane ios_beta`**: Builds `.ipa` and submits to Apple TestFlight automatically.

---

## 4. App Store & Play Store Metadata & Privacy Labels

Store metadata files are defined under [`packaging/mobile/metadata/`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/mobile/metadata/):
* **Android**: `title.txt`, `short_description.txt`, `full_description.txt`, `privacy_policy.txt`.
* **iOS**: `name.txt`, `subtitle.txt`, `description.txt`, `keywords.txt`.
* **Apple Privacy Labels**: Specified in [`privacy_labels.json`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/mobile/metadata/ios/privacy_labels.json) covering User Identifiers, Diagnostics, and User Content collected for app functionality and local-first sync.
