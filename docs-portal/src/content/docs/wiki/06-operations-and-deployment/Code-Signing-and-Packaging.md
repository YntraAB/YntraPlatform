---
title: "Code Signing & Desktop Packaging Guide"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

This guide describes building signed, production-ready desktop installers for Windows and macOS.

---

## 1. Windows EV Code Signing

Execute [`build-desktop-installers.bat`](https://github.com/YntraAB/YntraPlatform/blob/main/build-desktop-installers.bat), which compiles NSIS installer packages and invokes [`sign-installer.ps1`](https://github.com/YntraAB/YntraPlatform/blob/main/packaging/windows/sign-installer.ps1):

```powershell
# Sign Windows installer executable
signtool sign /f certificate.pfx /p $env:WINDOWS_SIGNING_PFX_PASS /tr http://timestamp.digicert.com /td sha256 target/release/YntraPlatform-Setup.exe
```

---

## 2. macOS Notarization & Packaging

Execute [`packaging/macos/notarize-app.sh`](https://github.com/YntraAB/YntraPlatform/blob/main/packaging/macos/notarize-app.sh):

```bash
# Submit to Apple Notary Service
xcrun notarytool submit target/release/Yntra.dmg --apple-id "support@yntra.se" --team-id "TEAMID12345" --wait

# Staple notarization ticket
xcrun stapler staple target/release/Yntra.dmg
```