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
pub struct TodoItem {
    pub id: String,
    pub workspace_id: String,
    pub text: String,
    pub completed: bool,
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
pub struct Team {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
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
pub struct TeamEvent {
    pub id: String,
    pub workspace_id: String,
    pub user_id: Option<String>,
    pub team_id: Option<String>,
    pub assignee_id: Option<String>,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub metadata: String,
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
pub struct MessageItem {
    pub id: String,
    pub workspace_id: String,
    pub sender_id: Option<String>,
    pub receiver_id: Option<String>,
    pub target_team_id: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub is_read: bool,
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
pub struct DailyNote {
    pub id: String,
    pub workspace_id: String,
    pub team_id: String,
    pub author_id: Option<String>,
    pub subject: String,
    pub content: String,
    pub edit_history: String,
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
pub struct TimeReport {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub team_id: Option<String>,
    pub date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub hours: f64,
    pub note: Option<String>,
    pub status: String,
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
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct ClientProfile {
    pub id: String,
    pub workspace_id: String,
    pub team_id: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub personal_number: Option<String>,
    pub care_level: Option<String>,
    pub message_settings: String,
    pub created_at: String,
    pub updated_at: i64,
    pub sync_status: String,
}

impl std::fmt::Debug for ClientProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProfile")
            .field("id", &self.id)
            .field("workspace_id", &self.workspace_id)
            .field("team_id", &self.team_id)
            .field("first_name", &self.first_name)
            .field("last_name", &self.last_name)
            .field(
                "personal_number",
                &self.personal_number.as_ref().map(|_| "***REDACTED***"),
            )
            .field("care_level", &self.care_level)
            .field("message_settings", &self.message_settings)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("sync_status", &self.sync_status)
            .finish()
    }
}
