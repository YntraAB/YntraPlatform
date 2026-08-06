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
pub struct EhrIntegrationConfig {
    pub id: String,
    pub workspace_id: String,
    pub provider: String,
    pub fhir_endpoint_url: String,
    pub account_id: Option<String>,
    pub api_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: i64,
    pub mtls_client_cert_pem: Option<String>,
    pub mtls_client_key_pem: Option<String>,
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
pub struct EhrSyncResult {
    pub integration_id: String,
    pub provider: String,
    pub records_pulled: u32,
    pub records_pushed: u32,
    pub conflicts_resolved: u32,
    pub status: String,
    pub error_message: Option<String>,
    pub synced_at: i64,
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
pub struct SisIntegrationConfig {
    pub id: String,
    pub workspace_id: String,
    pub provider: String,
    pub edfi_endpoint_url: String,
    pub client_key: Option<String>,
    pub client_secret: Option<String>,
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
pub struct SisSyncResult {
    pub integration_id: String,
    pub provider: String,
    pub records_pulled: u32,
    pub records_pushed: u32,
    pub conflicts_resolved: u32,
    pub status: String,
    pub error_message: Option<String>,
    pub synced_at: i64,
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
pub struct MllpListenerConfig {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub port: u16,
    pub bind_address: String,
    pub tls_enabled: bool,
    pub status: String,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for MllpListenerConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            workspace_id: String::new(),
            name: "HL7 ADT MLLP Listener".to_string(),
            port: 2575,
            bind_address: "0.0.0.0".to_string(),
            tls_enabled: false,
            status: "stopped".to_string(),
            last_active_at: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7MessageRecord {
    pub id: String,
    pub workspace_id: String,
    pub listener_id: Option<String>,
    pub message_type: String,
    pub trigger_event: String,
    pub sending_app: Option<String>,
    pub sending_facility: Option<String>,
    pub message_control_id: String,
    pub patient_mrn: Option<String>,
    pub patient_name: Option<String>,
    pub encounter_id: Option<String>,
    pub raw_payload: String,
    pub parsed_json: String,
    pub ack_status: String,
    pub ack_payload: Option<String>,
    pub status: String,
    pub received_at: i64,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FhirExportResult {
    pub resource_type: String,
    pub fhir_id: String,
    pub json_payload: String,
    pub validation_passed: bool,
    pub warnings: Vec<String>,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FhirImportResult {
    pub resource_type: String,
    pub internal_entity_id: String,
    pub success: bool,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FhirResourceMappingRecord {
    pub id: String,
    pub workspace_id: String,
    pub resource_type: String,
    pub fhir_id: String,
    pub internal_entity_type: String,
    pub internal_entity_id: String,
    pub raw_fhir_json: String,
    pub last_synced_at: i64,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct NcpdpPrescriptionRecord {
    pub id: String,
    pub workspace_id: String,
    pub client_id: String,
    pub prescriber_id: String,
    pub prescriber_npi: String,
    pub pharmacy_npi: String,
    pub pharmacy_name: Option<String>,
    pub drug_name: String,
    pub rxnorm_code: Option<String>,
    pub ndc_code: Option<String>,
    pub quantity: f64,
    pub days_supply: u32,
    pub refills: u32,
    pub sig_instructions: String,
    pub transaction_type: String,
    pub status: String,
    pub surescripts_tx_id: Option<String>,
    pub raw_xml_payload: String,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct NcpdpTransmitResult {
    pub prescription_id: String,
    pub transaction_type: String,
    pub surescripts_tx_id: String,
    pub status: String,
    pub xml_payload: String,
    pub success: bool,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FdaDualSignatureRecord {
    pub id: String,
    pub workspace_id: String,
    pub target_record_type: String,
    pub target_record_id: String,
    pub primary_signer_id: String,
    pub primary_signer_name: String,
    pub primary_intent: String,
    pub primary_ed25519_sig: String,
    pub primary_pubkey: String,
    pub secondary_signer_id: Option<String>,
    pub secondary_signer_name: Option<String>,
    pub secondary_intent: Option<String>,
    pub secondary_ed25519_sig: Option<String>,
    pub secondary_pubkey: Option<String>,
    pub dual_sign_completed: bool,
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FdaSignatureResult {
    pub signature_id: String,
    pub target_record_id: String,
    pub dual_sign_completed: bool,
    pub is_valid: bool,
    pub primary_verified: bool,
    pub secondary_verified: bool,
    pub message: String,
}
