---
title: "Identity Live BankID v6 & WebAuthn Hardware Passkeys"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra Platform combines **BankID v6 authentication** for Nordic identity compliance and **WebAuthn Hardware Passkeys** for zero-trust, passwordless login.

---

## 1. BankID v6 Integration

BankID authentication runs against live mTLS production endpoints (`https://appapi2.bankid.com/rp/v6.0/`):
- [`initiate_bankid_auth`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/bankid.rs#L316-L429): Returns [`BankIdAuthSession`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/models/auth.rs#L15-L51) with QR AutoStartToken formatting.
- [`submit_bankid_pin`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/bankid.rs#L470-L661): Verifies pin verification flow.

---

## 2. WebAuthn Hardware Passkeys

Hardware passkeys utilize platform security hardware (Apple Touch ID/Face ID, Android BiometricPrompt, YubiKey FIDO2):
- [`register_passkey_credential`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/hardware/mod.rs#L316-L344): Registers public key credential with user profile.
- [`authenticate_with_passkey`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/services/auth/hardware/mod.rs#L347-L408): Validates cryptographic challenge signature.