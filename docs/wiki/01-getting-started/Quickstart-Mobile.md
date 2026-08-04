# Quickstart: Native Mobile (iOS & Android)

This guide describes linking `yntra-core` FFI bindings to native **iOS (SwiftUI)** and **Android (Jetpack Compose)** applications.

---

## 1. Generating FFI Bindings

Run `yntra-uniffi-bindgen` to generate Swift packages and Kotlin source bindings:

```bash
# Generate Swift & Kotlin bindings
cargo run -p yntra-uniffi-bindgen

# Watch core changes and auto-regenerate
cargo run -p yntra-uniffi-bindgen -- watch
```

---

## 2. iOS Setup (SwiftUI)

Open `yntra-ios/YntraIOS.xcodeproj` in Xcode. The project links `generated_bindings/swift/yntra_core.swift` and subscribes to `SwiftDbObserver` for reactive UI state updates.

---

## 3. Android Setup (Jetpack Compose)

Open `yntra-android` in Android Studio. Kotlin bindings are located at `app/src/main/java/uniffi/yntra_core/yntra_core.kt` and managed by `KotlinDbObserver`.
