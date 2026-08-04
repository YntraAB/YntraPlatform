# Auto-Updater Pipeline & Ed25519 Manifests

Yntra Desktop applications integrate Ed25519-signed in-app auto-updating managed by [`updater.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/services/updater.rs).

---

## 1. Update Manifest Format

The release server serves an Ed25519-signed [`update_manifest.json`](file:///c:/Users/hellich/Desktop/YntraPlatform/packaging/updater/update_manifest.json):

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

1. `is_version_newer()` checks version strings.
2. `verify_manifest_signature()` verifies the Ed25519 payload signature.
3. `stage_binary_update()` verifies SHA-256 binary checksum before staging `.exe.new` for atomic application.
