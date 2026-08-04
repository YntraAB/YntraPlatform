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
