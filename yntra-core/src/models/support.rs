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
pub struct SupportTicketMessage {
    pub sender_id: String,
    pub sender_name: String,
    pub is_staff: bool,
    pub message: String,
    pub timestamp: i64,
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
pub struct SupportTicket {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
    pub messages_json: String,
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
pub struct HelpdeskArticle {
    pub id: String,
    pub title: String,
    pub category: String,
    pub summary: String,
    pub content_markdown: String,
    pub tags_json: String,
    pub views_count: u32,
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
pub struct ProductTourStep {
    pub step_number: u32,
    pub tour_name: String,
    pub target_element_id: String,
    pub title: String,
    pub description: String,
    pub position: String,
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
pub struct UserTourProgress {
    pub workspace_id: String,
    pub user_id: String,
    pub tour_name: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub completed: bool,
    pub updated_at: i64,
}
