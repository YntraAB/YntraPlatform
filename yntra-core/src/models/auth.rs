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
pub struct BankIdAuthSession {
    pub id: String,
    pub token: String,
    pub target_role: String,
    pub provider: String,
    pub status: String,
    pub error_message: Option<String>,
    pub qr_data: String,
    pub progress: f64,
    pub authenticated_user_id: Option<String>,
    pub created_at: String,
    pub challenge: Option<String>,
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
pub struct OauthAuthSession {
    pub id: String,
    pub provider: String,
    pub token: String,
    pub status: String,
    pub error_message: Option<String>,
    pub authenticated_user_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
