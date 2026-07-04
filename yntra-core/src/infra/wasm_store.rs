#![cfg(target_arch = "wasm32")]

use crate::{
    BlockItem, ClientProfile, DailyNote, JournalEntry, MedicationItem, MessageItem, ReportItem,
    Team, TeamEvent, TimeReport, TodoItem, Workspace, WorkspaceUser, BankIdAuthSession, JobTicket,
    AuditLogEntry, MoveInventoryItem, MoveQuote, StudentProfile, Course, Assignment, Submission, AttendanceRecord,
    TermGrade, ReportCard, TimetableSlot, HealthRecord, HealthIncident, SchoolInvoice, SchoolPayment, LibraryBook, LibraryLendingLog,
};
use std::sync::{Mutex, OnceLock};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Store {
    pub workspace: Workspace,
    pub users: Vec<WorkspaceUser>,
    pub teams: Vec<Team>,
    pub events: Vec<TeamEvent>,
    pub messages: Vec<MessageItem>,
    pub notes: Vec<DailyNote>,
    pub time_reports: Vec<TimeReport>,
    pub clients: Vec<ClientProfile>,
    pub medications: Vec<MedicationItem>,
    pub journals: Vec<JournalEntry>,
    pub todos: Vec<TodoItem>,
    pub blocks: Vec<BlockItem>,
    pub reports: Vec<ReportItem>,
    pub bankid_sessions: Vec<BankIdAuthSession>,
    pub passwords: std::collections::HashMap<String, String>,
    pub job_tickets: Vec<JobTicket>,
    pub audit_logs: Vec<AuditLogEntry>,
    pub move_inventories: Vec<MoveInventoryItem>,
    pub move_quotes: Vec<MoveQuote>,
    pub student_profiles: Vec<StudentProfile>,
    pub courses: Vec<Course>,
    pub assignments: Vec<Assignment>,
    pub submissions: Vec<Submission>,
    pub attendance_records: Vec<AttendanceRecord>,
    pub term_grades: Vec<TermGrade>,
    pub report_cards: Vec<ReportCard>,
    pub timetable_slots: Vec<TimetableSlot>,
    pub health_records: Vec<HealthRecord>,
    pub health_incidents: Vec<HealthIncident>,
    pub school_invoices: Vec<SchoolInvoice>,
    pub school_payments: Vec<SchoolPayment>,
    pub library_books: Vec<LibraryBook>,
    pub library_lending_logs: Vec<LibraryLendingLog>,
}

pub struct StoreGuard<'a>(std::sync::MutexGuard<'a, Store>);

impl<'a> std::ops::Deref for StoreGuard<'a> {
    type Target = Store;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> std::ops::DerefMut for StoreGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Drop for StoreGuard<'a> {
    fn drop(&mut self) {
        save_to_local_storage(&self.0);
    }
}

pub struct StoreMutex;

impl StoreMutex {
    pub fn lock(&self) -> Result<StoreGuard<'static>, std::sync::PoisonError<std::sync::MutexGuard<'static, Store>>> {
        let mutex = STORE.get_or_init(|| {
            Mutex::new(load_from_local_storage().unwrap_or_else(default_store))
        });
        match mutex.lock() {
            Ok(guard) => Ok(StoreGuard(guard)),
            Err(poisoned) => Err(poisoned),
        }
    }
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();

pub fn get_store() -> StoreMutex {
    StoreMutex
}

fn load_from_local_storage() -> Option<Store> {
    let window = web_sys::window()?;
    let local_storage = window.local_storage().ok()??;
    let serialized = local_storage.get_item("yntra_store").ok()??;
    serde_json::from_str(&serialized).ok()
}

fn save_to_local_storage(store: &Store) {
    let save = || -> Option<()> {
        let window = web_sys::window()?;
        let local_storage = window.local_storage().ok()??;
        let serialized = serde_json::to_string(store).ok()?;
        local_storage.set_item("yntra_store", &serialized).ok()?;
        Some(())
    };
    let _ = save();
}

fn default_store() -> Store {
    Store {
        workspace: Workspace {
            id: "workspace-1".to_string(),
            name: "Yntra Care (WASM Store)".to_string(),
            modules_active: "{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}".to_string(),
            settings: "{}".to_string(),
            brand_color: "#3b82f6".to_string(),
            logo_url: None,
            block_settings: "{}".to_string(),
        },
        users: Vec::new(),
        teams: Vec::new(),
        events: Vec::new(),
        messages: Vec::new(),
        notes: Vec::new(),
        time_reports: Vec::new(),
        clients: Vec::new(),
        medications: Vec::new(),
        journals: Vec::new(),
        todos: Vec::new(),
        blocks: vec![
            BlockItem { id: "dashboard".to_string(), name: "Dashboard".to_string(), description: Some("Central overview and command center".to_string()), icon: "LayoutGrid".to_string(), category: "Core".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "messaging".to_string(), name: "Messaging".to_string(), description: Some("Internal messaging system".to_string()), icon: "MessageSquare".to_string(), category: "Communication".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "scheduling".to_string(), name: "Scheduling".to_string(), description: Some("Calendar and scheduling management".to_string()), icon: "Calendar".to_string(), category: "Operations".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "notes".to_string(), name: "Notes".to_string(), description: Some("Team notes and documentation".to_string()), icon: "FileText".to_string(), category: "Communication".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "directory".to_string(), name: "Directory".to_string(), description: Some("Team and user directory".to_string()), icon: "Users".to_string(), category: "Core".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "assistance".to_string(), name: "Assistance".to_string(), description: Some("Client assistance and overview".to_string()), icon: "Heart".to_string(), category: "Care".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "journals".to_string(), name: "Care Journals".to_string(), description: Some("Document client care diaries, daily reports, and support logs".to_string()), icon: "BookOpen".to_string(), category: "Care".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "medications".to_string(), name: "Medications".to_string(), description: Some("Log active medication plans, instructions, and dosages".to_string()), icon: "Activity".to_string(), category: "Care".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "time".to_string(), name: "Time".to_string(), description: Some("Time management and reporting".to_string()), icon: "Clock".to_string(), category: "Operations".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "reporting".to_string(), name: "Reporting".to_string(), description: Some("Incident and deviation reporting".to_string()), icon: "AlertTriangle".to_string(), category: "Operations".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "jobs".to_string(), name: "Jobs".to_string(), description: Some("Job Tickets & Work Orders".to_string()), icon: "Wrench".to_string(), category: "Operations".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "academics".to_string(), name: "Academics".to_string(), description: Some("Academics, courses and grading management".to_string()), icon: "BookOpen".to_string(), category: "Education".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "attendance".to_string(), name: "Attendance".to_string(), description: Some("Student attendance tracking".to_string()), icon: "UserCheck".to_string(), category: "Education".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "finance".to_string(), name: "Finance".to_string(), description: Some("Finance, invoices and fee management".to_string()), icon: "CreditCard".to_string(), category: "Operations".to_string(), dependencies: "[]".to_string() },
            BlockItem { id: "library".to_string(), name: "Library".to_string(), description: Some("Library catalog and lending log".to_string()), icon: "BookOpen".to_string(), category: "Education".to_string(), dependencies: "[]".to_string() },
        ],
        reports: Vec::new(),
        bankid_sessions: Vec::new(),
        passwords: std::collections::HashMap::new(),
        job_tickets: Vec::new(),
        audit_logs: Vec::new(),
        move_inventories: Vec::new(),
        move_quotes: Vec::new(),
        student_profiles: Vec::new(),
        courses: Vec::new(),
        assignments: Vec::new(),
        submissions: Vec::new(),
        attendance_records: Vec::new(),
        term_grades: Vec::new(),
        report_cards: Vec::new(),
        timetable_slots: Vec::new(),
        health_records: Vec::new(),
        health_incidents: Vec::new(),
        school_invoices: Vec::new(),
        school_payments: Vec::new(),
        library_books: Vec::new(),
        library_lending_logs: Vec::new(),
    }
}

#[allow(dead_code)]
fn hash_password_pbkdf2(password: &str, salt: &[u8]) -> String {
    let mut out_hash = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, 10_000, &mut out_hash);
    
    let mut s = String::with_capacity(salt.len() * 2 + 1 + out_hash.len() * 2);
    for &b in salt {
        s.push_str(&format!("{:02x}", b));
    }
    s.push(':');
    for &b in &out_hash {
        s.push_str(&format!("{:02x}", b));
    }
    s
}
