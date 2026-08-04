# Nordic Labor Law & GDPR Article 17 Compliance

Yntra Platform enforces automated **Nordic Labor Law Compliance** ([`yntra-core/src/infra/compliance.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/infra/compliance.rs)) and **GDPR Article 17 Data Erasure Protocols** ([`yntra-core/src/services/users/mod.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/services/users/mod.rs)).

---

## 1. Nordic Labor Law Compliance Registry

`ComplianceRegistry` enforces statutory daily and weekly working hour limits, overtime caps, and mandatory rest periods per jurisdiction:

| Country Code | Statute / Law Name | Max Daily Hours | Max Weekly Hours | Mandatory Daily Rest |
| :--- | :--- | :--- | :--- | :--- |
| **Sweden (`SE`)** | *Swedish Arbetstidslagen* | 8.0h (Max 13.0h overtime) | 40.0h (Max 48.0h exemption) | 11.0 consecutive hours |
| **Norway (`NO`)** | *Norwegian Arbeidsmiljøloven § 10-4* | 9.0h (Max 13.0h overtime) | 40.0h (Max 48.0h exemption) | 11.0 consecutive hours |
| **Finland (`FI`)** | *Finnish Työaikalaki* | 8.0h (Max 12.0h overtime) | 40.0h (Max 48.0h exemption) | 11.0 consecutive hours |
| **Denmark (`DK`)** | *Danish Arbejdstidsloven* | 8.0h (Max 13.0h overtime) | 37.0h (Max 48.0h exemption) | 11.0 consecutive hours |

```rust
let rule = ComplianceRegistry::get_rule("SE");
if submitted_hours > rule.max_daily_limit_with_overtime {
    return Err(YntraError::LaborLawViolation("Exceeds Arbetstidslagen daily cap".into()));
}
```

---

## 2. GDPR Article 17 (Right to be Forgotten) Protocol

Under GDPR Article 17, users can request complete data erasure or pseudonymization across local SQLite replicas and cloud servers:

```rust
// Hard deletion & pseudonymization engine
pub fn export_and_purge_user_data(user_id: &str) -> Result<GdprPurgeReport, YntraError>
```

### Erasure Rules
1. **Personal Identity Data**: User name, personal identity number (*personnummer*), email, phone, and push tokens are hard-deleted from `users` and `push_tokens` tables.
2. **Time Logs & Financial Records**: Immutable audit logs and financial tax records are pseudonymized (replacing `user_id` with `anon-hash-xxxx`) to preserve historical accounting compliance without retaining PII.
3. **Local Store Purge**: Issues a purge signal across `DatabaseObserver` to clean local OPFS and SQLite caches on device replicas.
