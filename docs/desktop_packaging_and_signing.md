# Desktop Packaging & Code Signing Specification

This document details the cross-platform packaging, installer generation, and code signing procedures for **Yntra Platform** on Windows, macOS, and Linux.

---

## 1. Windows Packaging & EV Code Signing

### Installer Generation (NSIS / WiX)
* **Configuration File**: [`packaging/windows/installer.nsi`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/windows/installer.nsi)
* **Executable**: Built via `cargo build --release -p yntra-ui`.
* **Installer Output**: `target/release/YntraPlatform-Setup.exe`.

### EV Code Signing Process
Windows SmartScreen requires executables and installers to be signed with an Extended Validation (EV) Code Signing Certificate.

#### Local / Automated Execution
Run the PowerShell script:
```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\sign-installer.ps1
```

#### Secret Environment Variables
* `EV_CERT_PATH`: Path to `.pfx` EV certificate file.
* `EV_CERT_PASSWORD`: Password for the PFX certificate.
* `EV_CERT_SUBJECT`: Subject name if using Windows Certificate Store.
* `TSA_SERVER_URL`: RFC 3161 Timestamp Server (default: `http://timestamp.digicert.com`).

---

## 2. macOS Bundling, Developer ID Signing & Notarization

### App Bundle & DMG Creation
* **Info.plist**: [`packaging/macos/Info.plist`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/macos/Info.plist)
* **Entitlements**: [`packaging/macos/entitlements.plist`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/macos/entitlements.plist) (Hardened Runtime).
* **DMG Script**: [`packaging/macos/create-dmg.sh`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/macos/create-dmg.sh).

### Signing & Notarization Workflow
macOS Gatekeeper requires Developer ID Application signing and Apple Notarization via `xcrun notarytool`:

```bash
bash packaging/macos/notarize-app.sh
```

#### Secret Environment Variables
* `APPLE_DEVELOPER_IDENTITY`: E.g., `"Developer ID Application: Yntra Technologies Inc (TEAMID123)"`.
* `APPLE_ID`: Apple Developer account email.
* `APPLE_TEAM_ID`: Apple Developer 10-character Team ID.
* `APPLE_APP_SPECIFIC_PASSWORD`: App-specific password generated via appleid.apple.com.

---

## 3. Linux Packaging (AppImage, Debian `.deb`, Flatpak)

### AppImage
* **Script**: [`packaging/linux/build-appimage.sh`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/linux/build-appimage.sh)
* **Output**: `target/release/YntraPlatform-x86_64.AppImage`

### Debian (.deb)
* **Script**: [`packaging/linux/build-deb.sh`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/linux/build-deb.sh)
* **Output**: `target/release/yntra-platform_0.1.0_amd64.deb`

### Flatpak Manifest
* **Manifest**: [`packaging/linux/com.yntra.YntraPlatform.yml`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/linux/com.yntra.YntraPlatform.yml)

---

## 4. Local Build Script

To build release desktop binaries and compile installers locally on Windows:
```cmd
build-desktop-installers.bat
```
