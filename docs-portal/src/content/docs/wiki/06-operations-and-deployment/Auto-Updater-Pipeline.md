---
title: "Auto-Updater Pipeline & Ed25519 Manifests"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra Desktop applications integrate Ed25519-signed in-app auto-updating managed by [`updater.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs).

---

## 1. Update Manifest Format

The release server serves an Ed25519-signed [`update_manifest.json`](https://github.com/YntraAB/YntraPlatform/blob/main/packaging/updater/update_manifest.json):

```json
{
  "version": "0.2.0",
  "release_notes": "Added cross-platform native installers and automatic delta updates.",
  "pub_date": "2026-08-04T12:00:00Z",
  "download_url": "https://releases.yntra.se/v0.2.0/yntra-ui.exe",
  "signature": "30450221008f172782b537c35272a8c5417df...",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "min_supported_version": "0.1.0"
}
```

---

## 2. Verification & Staging

1. [`is_version_newer`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs#L40-L45) checks version strings.
2. [`verify_manifest_signature`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs#L61-L69) verifies the Ed25519 payload signature.
3. [`stage_binary_update`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/updater.rs#L153-L174) verifies SHA-256 binary checksum before staging `.exe.new` for atomic application.