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
pub struct JournalEntry {
    pub id: String,
    pub client_id: String,
    pub workspace_id: String,
    pub content: String,
    pub author_id: Option<String>,
    pub created_at: String,
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
pub struct MedicationItem {
    pub id: String,
    pub client_id: String,
    pub workspace_id: String,
    pub name: String,
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub instructions: Option<String>,
    pub updated_at: i64,
    pub sync_status: String,
}
