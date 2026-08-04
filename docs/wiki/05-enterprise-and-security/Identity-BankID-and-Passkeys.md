# Identity: Live BankID v6 & WebAuthn Hardware Passkeys

Yntra Platform combines **BankID v6 authentication** for Nordic identity compliance and **WebAuthn Hardware Passkeys** for zero-trust, passwordless login.

---

## 1. BankID v6 Integration

BankID authentication runs against live mTLS production endpoints (`https://appapi2.bankid.com/rp/v6.0/`):
- `initiate_bankid_auth()`: Returns `BankIdAuthSession` with QR AutoStartToken formatting.
- `submit_bankid_pin()`: Verifies pin verification flow.

---

## 2. WebAuthn Hardware Passkeys

Hardware passkeys utilize platform security hardware (Apple Touch ID/Face ID, Android BiometricPrompt, YubiKey FIDO2):
- `register_passkey_credential()`: Registers public key credential with user profile.
- `authenticate_with_passkey()`: Validates cryptographic challenge signature.
