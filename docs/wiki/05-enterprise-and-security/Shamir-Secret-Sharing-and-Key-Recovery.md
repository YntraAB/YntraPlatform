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

### Enterprise Recovery Scenarios
* **3-of-5 Admin Escrow**: A workspace master key is split into 5 shares distributed among designated workspace administrators. Any 3 administrators combining their shares can recover the workspace encryption key if a device is lost.
* **Shamir Social Recovery**: Users split emergency backup keys among 3 trusted contact devices.

---

## 3. Lagrange Polynomial Reconstruction

Reconstruction uses Lagrange polynomial interpolation over $GF(2^8)$:

$$\ell_j(0) = \prod_{m = 1, m \neq j}^{k} \frac{x_m}{x_m \oplus x_j}$$

$$\text{Reconstructed Secret } S = \bigoplus_{j=1}^{k} y_j \cdot \ell_j(0)$$

```rust
// Reconstruct master key from k threshold shares
pub fn combine_shares(shares: &[SecretShare]) -> Result<Vec<u8>, YntraError>
```
