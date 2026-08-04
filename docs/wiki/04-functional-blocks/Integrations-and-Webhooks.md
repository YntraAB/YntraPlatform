# External Integrations, CSV Import & Webhook Pipelines

This guide details the **Third-Party Integration Engine** ([`yntra-core/src/services/integrations.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/services/integrations.rs)), RFC 4180 CSV parser with fuzzy header mapping, webhook delivery queues, and external ERP/CRM connectors.

---

## 1. Supported Integration Connectors

Yntra integrates natively with Nordic ERP, accounting, payment, and communication systems:

| Connector | Category | Sync Mechanism | Auth Protocol |
| :--- | :--- | :--- | :--- |
| **Fortnox** | Accounting & Invoicing | Bidirectional REST | OAuth 2.0 PKCE |
| **Visma eEkonomi** | Invoicing & Payroll | Batch Sync | OAuth 2.0 |
| **Stripe** | Credit Card Payments | Realtime Webhooks | API Secret Keys |
| **Slack / Teams** | Notifications | Event Webhooks | Webhook URIs |

---

## 2. RFC 4180 CSV Parser & Fuzzy Header Auto-Mapping

To support importing legacy client data, financial sheets, and user rosters, `yntra-core` includes a high-performance, zero-dependency RFC 4180 compliant CSV parser (`parse_rfc4180_csv`):

```rust
// Parses quoted strings, escaped double quotes (""), newlines, & custom delimiters
pub fn parse_rfc4180_csv(raw: &str, delimiter: char) -> Vec<Vec<String>>
```

### Fuzzy Header Matching Protocol
When users upload CSV files, `fuzzy_map_headers` matches column headers against standard schemas using string edit-distance and normalized keyword matching:

* `"PersNr"`, `"Social Security"`, `"Personnummer"` $\rightarrow$ `personnummer`
* `"E-post"`, `"Email Address"`, `"Mail"` $\rightarrow$ `email`
* `"Timmar"`, `"Worked Hours"`, `"Hours"` $\rightarrow$ `hours`

---

## 3. Webhook Delivery & Retry Queues

Outgoing webhook notifications (e.g. `invoice.paid`, `time_report.approved`) are processed via an offline-resilient event queue:

1. **Local Enqueue**: Events are written to the local SQLite `webhook_delivery_queue` table.
2. **Exponential Backoff Retry**: If recipient servers return non-2xx status codes, retries occur at $2^n \times 5$ seconds up to 5 attempts.
3. **Payload Signing**: Every webhook HTTP request includes an `X-Yntra-Signature: sha256=...` header computed using the workspace HMAC secret key.
