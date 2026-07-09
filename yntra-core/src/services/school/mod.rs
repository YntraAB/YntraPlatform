#![allow(unused)]

use crate::{
    Assignment, AttendanceRecord, Course, HealthIncident, HealthRecord, LibraryBook,
    LibraryLendingLog, ReportCard, SchoolInvoice, SchoolPayment,
    StudentPortalData, StudentProfile, TermGrade, TimetableSlot, YntraError,
};

pub(crate) fn check_permission_for_auth(
    auth: &crate::AuthContext,
    _permission: &str,
) -> bool {
    auth.role == "platform_admin" || auth.role == "admin"
}

#[allow(dead_code)]
pub(crate) async fn check_permission(
    conn: &crate::database::DbConnection,
    user_id: &str,
    permission: &str,
) -> Result<bool, YntraError> {
    let auth = match crate::AuthContext::authorize(conn, user_id).await {
        Ok(a) => a,
        Err(_) => return Ok(false),
    };
    Ok(check_permission_for_auth(&auth, permission))
}

#[uniffi::export]
pub async fn get_courses(requester_user_id: String) -> Result<Vec<Course>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn add_course(
    requester_user_id: String,
    workspace_id: String,
    name: String,
    subject: String,
    teacher_id: Option<String>,
    classroom: Option<String>,
) -> Result<Course, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn update_course(
    requester_user_id: String,
    course_id: String,
    name: String,
    subject: String,
    teacher_id: Option<String>,
    classroom: Option<String>,
) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_assignments(requester_user_id: String, course_id: String) -> Result<Vec<Assignment>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn add_assignment(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
    title: String,
    description: String,
    due_date: String,
    max_points: i32,
) -> Result<Assignment, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_submissions(
    requester_user_id: String,
    assignment_id: String,
) -> Result<Vec<crate::Submission>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn add_submission(
    requester_user_id: String,
    workspace_id: String,
    assignment_id: String,
    student_id: String,
    content: String,
    submitted_at: Option<String>,
    grade: Option<String>,
) -> Result<crate::Submission, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn update_submission_grade(
    requester_user_id: String,
    submission_id: String,
    grade: Option<String>,
    feedback: Option<String>,
) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_term_grades(
    requester_user_id: String,
    student_id: String,
    course_id: String,
) -> Result<Vec<TermGrade>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_term_grade(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    course_id: String,
    term_name: String,
    final_grade: Option<String>,
    final_points: Option<i32>,
    teacher_comments: Option<String>,
) -> Result<TermGrade, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_report_cards(requester_user_id: String, student_id: String) -> Result<Vec<ReportCard>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn publish_report_card(
    requester_user_id: String,
    report_card_id: String,
    comments: Option<String>,
) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn calculate_and_save_gpa(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    term_name: String,
) -> Result<ReportCard, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_student_portal_data(requester_user_id: String) -> Result<StudentPortalData, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_attendance(
    requester_user_id: String,
    course_id: String,
    date: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_attendance_record(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    course_id: String,
    date: String,
    status: String,
    notes: Option<String>,
) -> Result<AttendanceRecord, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_student_attendance(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<AttendanceRecord>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn get_school_invoices(
    requester_user_id: String,
    student_id: String,
) -> Result<Vec<SchoolInvoice>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_school_invoice(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    title: String,
    amount: f64,
    due_date: String,
    status: String,
    paid_at: Option<String>,
) -> Result<SchoolInvoice, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_school_payments(
    requester_user_id: String,
    invoice_id: String,
) -> Result<Vec<SchoolPayment>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn record_school_payment(
    requester_user_id: String,
    workspace_id: String,
    invoice_id: String,
    amount: f64,
    payment_method: String,
    paid_at: String,
) -> Result<SchoolPayment, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_health_records(requester_user_id: String, student_id: String) -> Result<Vec<HealthRecord>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_health_record(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    vaccine_name: String,
    status: String,
    administered_at: Option<String>,
) -> Result<HealthRecord, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_health_incidents(requester_user_id: String, student_id: String) -> Result<Vec<HealthIncident>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_health_incident(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    student_id: String,
    visit_reason: String,
    treatment: String,
    checked_in_at: String,
    checked_out_at: Option<String>,
    notes: Option<String>,
) -> Result<HealthIncident, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_library_books(requester_user_id: String) -> Result<Vec<LibraryBook>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_library_book(
    requester_user_id: String,
    id: Option<String>,
    workspace_id: String,
    title: String,
    author: String,
    isbn: String,
    copies_available: i32,
    total_copies: i32,
) -> Result<LibraryBook, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_library_lending_logs(
    requester_user_id: String,
    student_id: Option<String>,
) -> Result<Vec<LibraryLendingLog>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn checkout_library_book(
    requester_user_id: String,
    workspace_id: String,
    book_id: String,
    student_id: String,
    checked_out_at: String,
    due_date: String,
) -> Result<LibraryLendingLog, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn return_library_book(
    requester_user_id: String,
    lending_log_id: String,
    returned_at: String,
) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_timetable_slots(requester_user_id: String) -> Result<Vec<TimetableSlot>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn save_timetable_slot(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
    day_of_week: i32,
    start_time: String,
    end_time: String,
    classroom: Option<String>,
) -> Result<TimetableSlot, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn delete_timetable_slot(requester_user_id: String, id: String) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn sync_timetable_to_calendar(workspace_id: String, user_id: String) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_students(requester_user_id: String) -> Result<Vec<StudentProfile>, YntraError> {
    Ok(vec![])
}

#[uniffi::export]
pub async fn add_student(
    requester_user_id: String,
    workspace_id: String,
    user_id: Option<String>,
    first_name: String,
    last_name: String,
    grade_level: String,
    parent_contact: Option<String>,
) -> Result<StudentProfile, YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn associate_parent_student(
    requester_user_id: String,
    student_id: String,
    parent_user_id: String,
) -> Result<(), YntraError> {
    Err(YntraError::AuthError("School vertical is deprecated".to_string()))
}

#[uniffi::export]
pub async fn get_parent_students(requester_user_id: String) -> Result<Vec<StudentProfile>, YntraError> {
    Ok(vec![])
}
