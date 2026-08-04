# Reports Engine, Analytics & CSV Streaming Import

This guide covers the **Reports Engine** ([`yntra-core/src/services/reports.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/services/reports.rs)) and **CSV Import Pipeline** ([`yntra-core/src/services/csv_import.rs`](file:///c:/Users/hellich/Desktop/YntraPlatform/yntra-core/src/services/csv_import.rs)).

---

## 1. Executive Reporting & Analytics

The reports engine computes workspace metrics across configurable date ranges:

* **Time & Attendance Summary**: Aggregates billable vs non-billable hours, overtime multipliers, and team utilization.
* **Financial Revenue Breakdown**: Calculates invoicing totals, pending client balances, and VAT distribution.
* **Care & Field Job Compliance**: Aggregates completed care visits, missed medication events, and inspection audit logs.

---

## 2. Chunked Batch CSV Data Import

To handle importing large datasets (10,000+ rows) without blocking the local SQLite thread or exceeding mobile RAM limits, `csv_import.rs` processes data in **chunked batches**:

```rust
// Stream and import CSV in 100-row transactional chunks
pub fn process_csv_import_chunk(
    workspace_id: &str,
    rows: &[Vec<String>],
    mapping: &HeaderMapping,
) -> Result<ImportResult, YntraError>
```

### Batch Execution Pipeline
1. **Validation Phase**: Validates data types, email formats, and foreign key references before execution.
2. **Transactional Batches**: Inserts rows in batches of **100 records** per SQLite transaction (`BEGIN TRANSACTION`).
3. **Progress Reporting**: Emits progress percentage updates to `DatabaseObserver` so host UI layers can display progress bars.
