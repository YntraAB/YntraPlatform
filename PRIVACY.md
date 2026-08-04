# Privacy Policy (GDPR / CCPA Compliant)

**Effective Date:** August 4, 2026  
**Last Updated:** August 4, 2026

At **Yntra Platform**, privacy is not an afterthought—it is built into our core architecture. This Privacy Policy explains how we process data across our local-first application nodes, UniFFI mobile bindings, and zero-knowledge synchronization protocols.

---

## 1. Information We Collect & Store
Because Yntra Platform is **local-first**, data is stored locally on your device in an embedded SQLite database (or OPFS on WASM):

- **Account Profile**: Name, email, phone number, workspace associations, and encrypted user preferences.
- **Cryptographic Keys & Proofs**: Local envelope keypairs, hardware passkey attestations, and Zero-Knowledge Proof (ZKP) signatures.
- **Operational Data**: Notes, workspace tickets, messages, and fleet routing telemetry.

---

## 2. GDPR & CCPA Compliance Features
We comply fully with the European Union General Data Protection Regulation (GDPR) and the California Consumer Privacy Act (CCPA).

### A. Automated Personal Data Export (GDPR Art. 20 / CCPA §1798.100)
You have the right to request a machine-readable copy of your personal data.
- **Self-Service Endpoint**: Access **Settings > Privacy & Compliance > Export Personal Data**.
- **Output Format**: Standardized JSON payload containing your complete user profile, preferences, signature records, and metadata.

### B. Self-Service Account Deletion (GDPR Art. 17 / CCPA §1798.105)
You have the absolute right to request the erasure of your personal data ("Right to be Forgotten").
- **Self-Service Endpoint**: Access **Settings > Privacy & Compliance > Delete Account**.
- **Execution**: Instantly purges all local SQLite user records, keychains, and broadcasts erasure tombstone markers to authorized workspace sync peers.

### C. Telemetry Opt-Out (GDPR Art. 6 / CCPA Opt-Out)
- Diagnostics and vehicle GPS telemetry gathering are disabled or anonymized according to your privacy choices.
- **Toggle**: Access **Settings > Privacy & Compliance > Telemetry Opt-Out** to disable telemetry data transmission.

---

## 3. Data Security & Storage Safety
- **Encryption at Rest**: AES-256-GCM envelope encryption protecting local database stores and keychain credentials.
- **Zero-Knowledge Authentication**: Authentication utilizing zero-knowledge hardware signatures without transmitting raw credentials to server infrastructure.

---

## 4. Contact Data Protection Officer (DPO)
For privacy requests or questions regarding our processing activities:
**Data Protection Officer**: `dpo@yntra.se`
