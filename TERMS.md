# Terms of Service (ToS)

**Effective Date:** August 4, 2026  
**Last Updated:** August 4, 2026

Welcome to **Yntra Platform**. By accessing or using our local-first software application, edge nodes, mobile clients, and peer-to-peer sync services (collectively, the "Platform"), you agree to be bound by these Terms of Service ("Terms").

---

## 1. Local-First & Zero-Knowledge Architecture
1. **Local Data Sovereignty**: Yntra Platform operates primarily as a local-first application using an embedded SQLite/libSQL database. Your workspace data, notes, tasks, messages, and cryptographic signatures reside locally on your device.
2. **Encryption at Rest & Transit**: High-sensitivity data payloads are protected using local envelope encryption, zero-knowledge proofs (ZKPs), and end-to-end encrypted WebRTC / libp2p data channels.

---

## 2. User Rights & Data Privacy (GDPR / CCPA)
1. **Right to Access & Portability**: You retain full ownership of all data created within the Platform. You may export your entire personal dataset at any time in structured JSON format via the self-service **Personal Data Export** endpoint in Settings.
2. **Right to Erasure ("Right to be Forgotten")**: You can execute self-service account deletion at any time via Settings. Deletion purges all local SQLite storage and propagates tombstone signals to peer nodes.
3. **Telemetry & Analytics**: Telemetry collection (such as fleet GPS tracking and diagnostics) is optional. You can enable or disable telemetry at any time via the **Telemetry Opt-Out** toggle in Settings.

---

## 3. Acceptable Use Policy
You agree not to:
- Reverse engineer or exploit the UniFFI rust core bindings or ZK cryptosystems for malicious intent.
- Attempt unauthorized access to other workspaces or disrupt peer-to-peer synchronization meshes.

---

## 4. Limitation of Liability
YNTRA PLATFORM IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND. IN NO EVENT SHALL YNTRA PLATFORM OR ITS CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL DAMAGES ARISING OUT OF THE USE OR INABILITY TO USE THE PLATFORM.

---

## 5. Contact Information
For legal and compliance inquiries, contact: `legal@yntra.se`.
