use rkyv::{Archive, Deserialize, Serialize};

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq)]
pub enum WorkspaceTemplateType {
    Care,
    MovingCompany,
    School,
    General,
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
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub modules_active: String, // JSON
    pub settings: String,       // JSON
    pub brand_color: String,
    pub logo_url: Option<String>,
    pub block_settings: String, // JSON
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
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DbPoolMetrics {
    pub active_connections: u32,
    pub max_pool_size: u32,
    pub available_permits: u32,
    pub total_acquisitions: u64,
    pub total_exhaustions: u64,
    pub peak_active_connections: u32,
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
pub struct OpfsStorageQuota {
    pub quota_bytes: u64,
    pub usage_bytes: u64,
    pub remaining_bytes: u64,
    pub usage_percent: f64,
    pub is_storage_low: bool,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct WorkspaceUser {
    pub id: String,
    pub workspace_id: Option<String>,
    pub email: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub role: String,
    pub preferences: String,
    pub siths_card_id: Option<String>,
    pub nfc_badge_uid: Option<String>,
    pub updated_at: i64,
    pub sync_status: String,
    pub personal_number: Option<String>,
    pub public_key: Option<String>,
}

impl std::fmt::Debug for WorkspaceUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceUser")
            .field("id", &self.id)
            .field("workspace_id", &self.workspace_id)
            .field("email", &self.email)
            .field("full_name", &self.full_name)
            .field("phone", &self.phone)
            .field("role", &self.role)
            .field("preferences", &self.preferences)
            .field("siths_card_id", &self.siths_card_id)
            .field("nfc_badge_uid", &self.nfc_badge_uid)
            .field("updated_at", &self.updated_at)
            .field("sync_status", &self.sync_status)
            .field(
                "personal_number",
                &self.personal_number.as_ref().map(|_| "***REDACTED***"),
            )
            .finish()
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
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct SettingDefinition {
    pub key: String,
    pub label: String,
    pub category: String,
    pub value_type: String,
    pub default_value: String,
    pub tooltip: String,
}
