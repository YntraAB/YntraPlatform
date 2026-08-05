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
    pub edfi_payload: Option<String>,
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
    pub student_id: String,
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
pub struct SchoolConflict {
    pub id: String,
    pub workspace_id: String,
    pub entity_table: String,
    pub entity_id: String,
    pub conflict_json: String,
    pub updated_at: i64,
}
