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
pub struct DataImportRecord {
    pub id: String,
    pub workspace_id: String,
    pub entity_type: String,
    pub file_name: String,
    pub file_format: String,
    pub records_total: u32,
    pub records_imported: u32,
    pub records_failed: u32,
    pub status: String,
    pub summary_json: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: String,
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
pub struct DataImportPreviewResult {
    pub entity_type: String,
    pub detected_delimiter: String,
    pub total_rows: u32,
    pub valid_rows: u32,
    pub invalid_rows: u32,
    pub columns_detected: Vec<String>,
    pub mapped_fields: Vec<String>,
    pub sample_preview_json: String,
    pub validation_errors: Vec<String>,
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
pub struct DataImportExecutionResult {
    pub import_id: String,
    pub entity_type: String,
    pub records_total: u32,
    pub records_imported: u32,
    pub records_failed: u32,
    pub status: String,
    pub error_details: Vec<String>,
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
pub struct CalendarIntegration {
    pub id: String,
    pub workspace_id: String,
    pub provider: String,
    pub account_email: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: i64,
    pub sync_direction: String,
    pub auto_sync_enabled: bool,
    pub last_synced_at: i64,
    pub sync_status: String,
    pub error_message: Option<String>,
    pub sync_token: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
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
pub struct CalendarSyncResult {
    pub integration_id: String,
    pub provider: String,
    pub events_pulled: u32,
    pub events_pushed: u32,
    pub conflicts_resolved: u32,
    pub status: String,
    pub error_message: Option<String>,
    pub synced_at: i64,
    pub new_sync_token: Option<String>,
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
pub struct WebhookEndpoint {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub target_url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub consecutive_failures: u32,
    pub circuit_state: String,
    pub created_at: i64,
    pub updated_at: i64,
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
pub struct WebhookDeliveryLog {
    pub id: String,
    pub endpoint_id: String,
    pub workspace_id: String,
    pub event_type: String,
    pub payload_json: String,
    pub status: String,
    pub response_code: i32,
    pub response_body: Option<String>,
    pub attempt_count: u32,
    pub idempotency_key: Option<String>,
    pub next_retry_at: i64,
    pub created_at: i64,
}
