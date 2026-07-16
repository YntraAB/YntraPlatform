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
pub struct JobTicket {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub description: String,
    pub location_address: String,
    pub priority: String,
    pub status: String,
    pub assigned_user_id: Option<String>,
    pub scheduled_date: String,
    pub checklist_json: String,
    pub completion_report: Option<String>,
    pub created_at: String,
    pub updated_at: i64,
    pub sync_status: String,
    pub origin_address: Option<String>,
    pub destination_address: Option<String>,
    pub origin_floor: i32,
    pub destination_floor: i32,
    pub origin_has_elevator: bool,
    pub destination_has_elevator: bool,
    pub origin_parking_permit_needed: bool,
    pub destination_parking_permit_needed: bool,
    pub assigned_vehicle_id: Option<String>,
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
pub struct MoveInventoryItem {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_category: String,
    pub item_name: String,
    pub quantity: i32,
    pub estimated_volume_m3: f64,
    pub handling_notes: Option<String>,
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
pub struct MoveQuote {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub base_price: i64,
    pub distance_fee: i64,
    pub stairs_surcharge: i64,
    pub packing_supplies_fee: i64,
    pub total_price: i64,
    pub status: String,
    pub accepted_at: Option<i64>,
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
pub struct AcademicOverview {
    pub course_count: i32,
    pub slot_count: i32,
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
pub struct LibraryOverview {
    pub total_books: i32,
    pub available_copies: i32,
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
pub struct FinanceOverview {
    pub unpaid_invoice_count: i32,
    pub total_due_amount: f64,
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
pub struct Course {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub teacher_id: Option<String>,
    pub classroom: Option<String>,
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
pub struct MoveVehicle {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub license_plate: String,
    pub capacity_m3: f64,
    pub status: String,
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
pub struct MoveSignature {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub signer_name: String,
    pub signature_data_base64: String,
    pub signed_at: i64,
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
pub struct StudentProfile {
    pub id: String,
    pub workspace_id: String,
    pub user_id: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub grade_level: String,
    pub parent_contact: Option<String>,
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
pub struct Assignment {
    pub id: String,
    pub workspace_id: String,
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub due_date: String,
    pub max_points: i32,
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
pub struct Submission {
    pub id: String,
    pub workspace_id: String,
    pub assignment_id: String,
    pub student_id: String,
    pub content: String,
    pub grade: Option<String>,
    pub feedback: Option<String>,
    pub submitted_at: String,
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
pub struct AttendanceRecord {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub course_id: String,
    pub date: String,
    pub status: String,
    pub notes: Option<String>,
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
pub struct TermGrade {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub course_id: String,
    pub term_name: String,
    pub final_grade: Option<String>,
    pub final_points: Option<i32>,
    pub teacher_comments: Option<String>,
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
pub struct ReportCard {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub term_name: String,
    pub gpa: f64,
    pub principal_comments: Option<String>,
    pub status: String,
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
pub struct TimetableSlot {
    pub id: String,
    pub workspace_id: String,
    pub course_id: String,
    pub day_of_week: i32,
    pub start_time: String,
    pub end_time: String,
    pub classroom: Option<String>,
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
pub struct LibraryBook {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub copies_available: i32,
    pub total_copies: i32,
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
pub struct LibraryLendingLog {
    pub id: String,
    pub workspace_id: String,
    pub book_id: String,
    pub student_id: String,
    pub checked_out_at: String,
    pub due_date: String,
    pub returned_at: Option<String>,
    pub status: String,
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
pub struct SchoolInvoice {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub title: String,
    pub amount: f64,
    pub due_date: String,
    pub status: String,
    pub paid_at: Option<String>,
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
pub struct SchoolPayment {
    pub id: String,
    pub workspace_id: String,
    pub invoice_id: String,
    pub amount: f64,
    pub payment_method: String,
    pub paid_at: String,
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
pub struct HealthRecord {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub vaccine_name: String,
    pub status: String,
    pub administered_at: Option<String>,
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
pub struct HealthIncident {
    pub id: String,
    pub workspace_id: String,
    pub student_id: String,
    pub visit_reason: String,
    pub treatment: String,
    pub checked_in_at: String,
    pub checked_out_at: Option<String>,
    pub notes: Option<String>,
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
pub struct LibraryLendingLogInfo {
    pub id: String,
    pub book_title: String,
    pub student_name: String,
    pub checked_out_at: String,
    pub due_date: String,
    pub returned_at: Option<String>,
    pub status: String,
}


