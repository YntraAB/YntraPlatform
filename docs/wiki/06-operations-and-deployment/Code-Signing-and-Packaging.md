# Code Signing & Desktop Packaging Guide

This guide describes building signed, production-ready desktop installers for Windows and macOS.

---

## 1. Windows EV Code Signing

Execute [`build-desktop-installers.bat`](file:///c:/Users/hellich/Desktop/YntraPlatform/build-desktop-installers.bat), which compiles NSIS installer packages and invokes [`sign-installer.ps1`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/windows/sign-installer.ps1):

```powershell
# Sign Windows installer executable
signtool sign /f certificate.pfx /p $env:WINDOWS_SIGNING_PFX_PASS /tr http://timestamp.digicert.com /td sha256 target/release/YntraPlatform-Setup.exe
```

---

## 2. macOS Notarization & Packaging

Execute [`packaging/macos/notarize-app.sh`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/macos/notarize-app.sh):

```bash
# Submit to Apple Notary Service
xcrun notarytool submit target/release/Yntra.dmg --apple-id "support@yntra.se" --team-id "TEAMID12345" --wait

# Staple notarization ticket
xcrun stapler staple target/release/Yntra.dmg
```
