# Shamir's Secret Sharing (SSSS) & Key Recovery

For decentralized workspace key recovery and master secret backup without cloud master keys, Yntra implements **Shamir's Secret Sharing Scheme (SSSS)** ([`yntra-core/src/infra/crypto/ssss.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/infra/crypto/ssss.rs)).

---

## 1. Galois Field $GF(2^8)$ Mathematics

Secret sharing operates over Galois Field $GF(2^8)$ using the irreducible polynomial $x^8 + x^4 + x^3 + x + 1$ (Rijndael field `0x11b`).

```rust
// Static multiplication & logarithm tables for GF(2^8) byte arithmetic
static GF256_EXP: OnceLock<[u8; 256]> = OnceLock::new();
static GF256_LOG: OnceLock<[u8; 256]> = OnceLock::new();
```

* **Zero-Knowledge Security**: Any subset of shares less than the threshold $k$ reveals **zero information** about the secret master key.

---

## 2. $(k, n)$ Threshold Secret Splitting

Master session keys can be split into $n$ shares, requiring any $k$ threshold shares to reconstruct:

$$\text{Secret Key } S = f(0) = a_0$$

$$f(x) = a_0 + a_1 x + a_2 x^2 + \dots + a_{k-1} x^{k-1} \pmod{GF(2^8)}$$

```rust
// Split master key into n shares with threshold k
pub fn split_secret(secret: &[u8], k: u8, n: u8) -> Result<Vec<SecretShare>, YntraError>
```

### Enterprise Recovery Architecture Demarcation

> [!IMPORTANT]
> **Routine Resets vs. Emergency Master Vault Recovery**
> - **Routine User Account Recovery**: For high-volume staff or student account unlocks and password resets (e.g. 10,000 university students or 2,000 hospital staff), organizations must configure **Enterprise SAML 2.0 / OIDC Identity Provider SSO** (Okta, Microsoft Entra ID) via `initiate_enterprise_sso()` or delegated Helpdesk role authorization.
> - **Emergency Master Vault Recovery (SSSS)**: Shamir's $(k, n)$ threshold secret sharing is strictly reserved for catastrophic master root key recovery (e.g., total loss of organization root keys or offline cold-storage backup).

* **3-of-5 Admin Escrow**: Master root encryption keys can be split into 5 shares distributed among designated workspace administrators. Any 3 administrators combining their shares can reconstruct the master root key during a disaster recovery event.

---

## 3. Lagrange Polynomial Reconstruction

Reconstruction uses Lagrange polynomial interpolation over $GF(2^8)$:

$$\ell_j(0) = \prod_{m = 1, m \neq j}^{k} \frac{x_m}{x_m \oplus x_j}$$

$$\text{Reconstructed Secret } S = \bigoplus_{j=1}^{k} y_j \cdot \ell_j(0)$$

```rust
// Reconstruct master key from k threshold shares
pub fn combine_shares(shares: &[SecretShare]) -> Result<Vec<u8>, YntraError>
```

---

## 4. Out-of-Band Standalone CLI Recovery & WebAuthn PRF SAML Key Wrapping Standard

To eliminate administrative UI lockouts during total master database encryption events and bridge identity verification to zero-knowledge local storage:

* **Zero-Knowledge SAML/OIDC Key Wrapping**: When users authenticate via SAML 2.0 / OIDC (Okta, Entra ID), identity is authenticated by the IdP while local database encryption keys are wrapped/unwrapped using hardware passkeys (**WebAuthn PRF extension** via `derive_webauthn_prf_key`) or threshold node escrows (`reconstruct_passkey_from_threshold_shares`).
* **Out-of-Band Standalone CLI Recovery (`reconstruct_master_vault_key_raw`)**: If an organization suffers total master vault lockout and the local database is encrypted, administrators do not log into the Yntra UI. Instead, 3-of-5 threshold holders combine their hex shares directly in raw memory via `yntra-cli recover-vault` without requiring an active database connection or UI session.

---

## 5. Process Table Security (`stdin` Prompts) & VDI Enclave Fallback Standard

* **Process Table Exposure Mitigation (`reconstruct_master_vault_key_from_stdin_prompt`)**: Secret shares are passed via `stdin` masked interactive prompts or restricted key files (`0600`) rather than command-line `argv` arguments, preventing unprivileged local users from reading shares via `ps aux` or Process Explorer. Memory buffers are zeroized (`Zeroizing<T>`) immediately following key combination.
* **VDI Terminal Key Derivation (`derive_sso_vdi_fallback_key`)**: Hospital clinicians logging in on virtualized VDI terminals (Citrix Receiver / VMware Horizon) where WebAuthn USB passthrough is disabled by enterprise GPO seamlessly derive database encryption keys via SAML OIDC JWT assertions combined with the Organization KMS Key Escrow Enclave.
