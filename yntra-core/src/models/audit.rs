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
pub struct AuditLogEntry {
    pub id: String,
    pub workspace_id: String,
    pub actor_id: String,
    pub target_client_id: Option<String>,
    pub action_type: String,
    pub timestamp: i64,
    pub prev_hash: String,
    pub curr_hash: String,
    pub seq: i64,
    pub signature: Option<String>,
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
pub struct BlockItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub category: String,
    pub dependencies: String, // JSON array of strings
    pub fields_schema: Option<String>,
    pub navigation_items: Option<String>,
    pub ui_config: Option<String>,
    pub tier: Option<String>,
    pub compliance_standards: Option<String>, // JSON array of strings
    pub supported_protocols: Option<String>,   // JSON array of strings
    pub enterprise_connectors: Option<String>, // JSON array of strings
    pub jurisdiction: Option<String>,
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
pub struct DynamicEntity {
    pub id: String,
    pub workspace_id: String,
    pub block_id: String,
    pub entity_type: String,
    pub data: String,
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
pub struct ReportItem {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub type_name: String,
    pub is_anonymous: bool,
    pub content: String, // JSON string
    pub status: String,
    pub created_at: String,
    pub updated_at: i64,
    pub sync_status: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditHistoryEntry {
    pub edited_by: String,
    pub edited_at: String,
    pub old_subject: Option<String>,
    pub new_subject: Option<String>,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
}
