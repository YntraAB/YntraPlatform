use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveInvoice {
    pub id: String,
    pub workspace_id: String,
    pub quote_id: String,
    pub customer_id: String,
    pub invoice_date: String,
    pub due_date: String,
    pub subtotal: f64,
    pub rut_deduction: f64,
    pub customer_amount: f64,
    pub tax_authority_amount: f64,
    pub status: String,
    pub currency: String,
    pub actual_hours: Option<f64>,
    pub additional_charges: Option<f64>,
    pub adjustment_notes: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct SwishPaymentSession {
    pub token: String,
    pub swish_url: String,
    pub qr_code_base64: String,
    pub amount: f64,
    pub status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct StripePaymentSession {
    pub session_id: String,
    pub checkout_url: String,
    pub client_secret: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct AdyenPaymentSession {
    pub session_id: String,
    pub session_data: String,
    pub amount: f64,
    pub currency: String,
    pub status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct RutInvoiceOverview {
    pub invoice_id: String,
    pub job_title: String,
    pub customer_name: String,
    pub customer_pnum: String,
    pub payment_date: String,
    pub rut_amount: f64,
    pub status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct SkatteverketSubmitResult {
    pub reference_number: String,
    pub total_claims: i32,
    pub total_amount: f64,
    pub status: String,
    pub message: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MobilePosTerminalSession {
    pub session_id: String,
    pub invoice_id: String,
    pub connection_token: String,
    pub payment_intent_id: String,
    pub reader_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub provider: String,
    pub status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct OnSitePaymentResult {
    pub success: bool,
    pub transaction_id: String,
    pub payment_method: String,
    pub amount_collected: f64,
    pub receipt_url: Option<String>,
    pub message: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct ErpSyncResult {
    pub success: bool,
    pub invoice_id: String,
    pub erp_provider: String,
    pub erp_invoice_number: String,
    pub ledger_account: String,
    pub synced_at: i64,
    pub message: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct ErpSyncOverview {
    pub id: String,
    pub invoice_id: String,
    pub erp_provider: String,
    pub erp_invoice_number: String,
    pub status: String,
    pub ledger_account: String,
    pub synced_at: i64,
    pub error_message: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct AccountingLedgerSummary {
    pub total_accounts_receivable: f64,
    pub total_revenue_ytd: f64,
    pub total_rut_tax_claims_pending: f64,
    pub total_payroll_liabilities: f64,
    pub primary_erp_provider: String,
    pub last_sync_timestamp: i64,
}
