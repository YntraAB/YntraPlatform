# Storage & Quota Management Troubleshooting

Managing browser OPFS quotas, native SQLite file handles, and local blob sandboxes.

---

## 1. Web OPFS Storage Quotas

- **Symptom**: Browser throws `QuotaExceededError` when uploading large media payloads.
- **Fix**: Yntra automatically streams binary media in 512KB chunks to `offline_media_blobs` and purges temporary canvas data upon sync completion.

---

## 2. Android & iOS App Sandbox Boundaries

- **iOS Sandbox**: Blobs saved to `ApplicationSupport/yntra/blobs/<hash>.dat`.
- **Android Sandbox**: Blobs saved to `files/blobs/<hash>.dat`.

---

## 3. Shared Kiosk Browser Profile Wipes & Unsynced Journal Recovery

- **Symptom**: IT Group Policy (GPO / Jamf) wipes browser OPFS and IndexedDB on user logout or session timeout on shared workstation terminals.
- **Diagnostic Procedure**:
  1. Inspect `window.yntra_check_kiosk_unsynced_data()` state in browser DevTools.
  2. Verify active loopback daemon connection status on `ws://127.0.0.1:9443`.
  3. Check if P2P peer mesh sync (`ComplianceMode::AuditedLocalP2P`) is enabled to mirror un-synced WAL frames across adjacent room nodes.
  4. Ensure `visibilitychange` emergency beacon dispatch endpoint (`/v2/pipeline/beacon`) is reachable through network firewall rules.
