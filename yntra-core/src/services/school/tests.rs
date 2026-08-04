use crate::database;
use super::academics::*;
use super::billing::*;
use super::conflicts::*;
use super::health::*;
use super::library::*;
use super::profiles::*;
use super::timetable::*;
use super::attendance::*;
use crate::{Course, TimetableSlot, StudentProfile, LibraryBook, SchoolInvoice, HealthRecord, HealthIncident, TermGrade, ReportCard, AttendanceRecord, Assignment, Submission, YntraError};

#[tokio::test]
async fn test_school_service_crud() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-school-test', 'School WS', '[]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-school-admin', 'ws-school-test', 'sch@admin.com', 'admin')", ()).await.unwrap();

    // 1. Student Profile CRUD
    let profile = StudentProfile {
        id: "stud-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        user_id: None,
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        grade_level: "10A".to_string(),
        parent_contact: Some("parent@smith.com".to_string()),
        updated_at: 0,
    };
    save_student_profile("u-school-admin".to_string(), profile.clone(), None).await.unwrap();

    let list = get_student_profiles("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].first_name, "Jane");

    // 2. Library CRUD
    let book = LibraryBook {
        id: "bk-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        title: "Physics I".to_string(),
        author: "Isaac Newton".to_string(),
        isbn: "111-222".to_string(),
        copies_available: 2,
        total_copies: 2,
        updated_at: 0,
    };
    save_library_book("u-school-admin".to_string(), book.clone(), None).await.unwrap();

    checkout_book("u-school-admin".to_string(), "ws-school-test".to_string(), "bk-1".to_string(), "stud-1".to_string(), "2026-08-01".to_string(), None).await.unwrap();
    let bk_list = get_library_books("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
    assert_eq!(bk_list[0].copies_available, 1);

    // 3. Billing CRUD
    let invoice = SchoolInvoice {
        id: "inv-s1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        student_id: "stud-1".to_string(),
        title: "Lab Fee".to_string(),
        amount: 50.0,
        due_date: "2026-08-01".to_string(),
        status: "unpaid".to_string(),
        paid_at: None,
        updated_at: 0,
    };
    create_school_invoice("u-school-admin".to_string(), invoice, None).await.unwrap();
    record_school_payment("u-school-admin".to_string(), "ws-school-test".to_string(), "inv-s1".to_string(), "Card".to_string(), None).await.unwrap();

    let invoices = get_school_invoices("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
    assert_eq!(invoices[0].status, "paid");
    assert!(invoices[0].paid_at.is_some());

    // 4. Parent Linking
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-parent-1', 'ws-school-test', 'parent@smith.com', 'parent')", ()).await.unwrap();
    link_parent_to_student("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string(), "u-parent-1".to_string(), None).await.unwrap();
    let parents = get_student_parents("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].id, "u-parent-1");

    // 5. Health Record & Incident CRUD
    let hr = HealthRecord {
        id: "hr-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        student_id: "stud-1".to_string(),
        vaccine_name: "MMR".to_string(),
        status: "administered".to_string(),
        administered_at: Some("2026-05-01".to_string()),
        updated_at: 0,
    };
    save_student_health_record("u-school-admin".to_string(), hr, None).await.unwrap();
    let hr_list = get_student_health_records("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
    assert_eq!(hr_list.len(), 1);
    assert_eq!(hr_list[0].vaccine_name, "MMR");

    let incident = HealthIncident {
        id: "inc-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        student_id: "stud-1".to_string(),
        visit_reason: "Fever".to_string(),
        treatment: "Paracetamol 500mg".to_string(),
        checked_in_at: "10:00".to_string(),
        checked_out_at: Some("10:30".to_string()),
        notes: Some("Slight headache".to_string()),
        updated_at: 0,
    };
    save_health_incident("u-school-admin".to_string(), incident, None).await.unwrap();
    let inc_list = get_health_incidents("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
    assert_eq!(inc_list.len(), 1);
    assert_eq!(inc_list[0].visit_reason, "Fever");

    // 6. Grading & Report Cards
    let test_course = Course {
        id: "crs-1".to_string(),
        name: "Math 101".to_string(),
        subject: "Math".to_string(),
        teacher_id: Some("u-school-admin".to_string()),
        classroom: Some("Room 101".to_string()),
    };
    save_course("u-school-admin".to_string(), "ws-school-test".to_string(), test_course, None).await.unwrap();

    let tg = TermGrade {
        id: "tg-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        student_id: "stud-1".to_string(),
        course_id: "crs-1".to_string(),
        term_name: "Fall 2026".to_string(),
        final_grade: Some("A".to_string()),
        final_points: Some(95),
        teacher_comments: Some("Excellent work".to_string()),
        updated_at: 0,
    };
    save_term_grade("u-school-admin".to_string(), tg, None).await.unwrap();
    let tg_list = get_term_grades("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
    assert_eq!(tg_list.len(), 1);
    assert_eq!(tg_list[0].final_grade, Some("A".to_string()));

    let c_tg_list = get_course_term_grades("u-school-admin".to_string(), "ws-school-test".to_string(), "crs-1".to_string()).await.unwrap();
    assert_eq!(c_tg_list.len(), 1);
    assert_eq!(c_tg_list[0].final_grade, Some("A".to_string()));

    let rc = ReportCard {
        id: "rc-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        student_id: "stud-1".to_string(),
        term_name: "Fall 2026".to_string(),
        gpa: 4.0,
        principal_comments: Some("Outstanding student".to_string()),
        status: "published".to_string(),
        updated_at: 0,
    };
    publish_report_card("u-school-admin".to_string(), rc, None).await.unwrap();
    let rc_list = get_report_cards("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
    assert_eq!(rc_list.len(), 1);
    assert_eq!(rc_list[0].gpa, 4.0);

    // 7. Submissions CRUD
    let test_assignment = Assignment {
        id: "assign-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        course_id: "crs-1".to_string(),
        title: "Math Homework 1".to_string(),
        description: "Solve page 10".to_string(),
        due_date: "2026-08-01".to_string(),
        max_points: 100,
        updated_at: 0,
    };
    save_assignment("u-school-admin".to_string(), test_assignment, None).await.unwrap();

    let sub = Submission {
        id: "sub-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        assignment_id: "assign-1".to_string(),
        student_id: "stud-1".to_string(),
        content: "My homework answer".to_string(),
        grade: None,
        feedback: None,
        submitted_at: "2026-07-16T12:00:00Z".to_string(),
        updated_at: 0,
    };
    save_submission("u-school-admin".to_string(), sub.clone(), None).await.unwrap();
    let sub_list = get_student_submissions("u-school-admin".to_string(), "ws-school-test".to_string(), "stud-1".to_string()).await.unwrap();
    assert_eq!(sub_list.len(), 1);
    assert_eq!(sub_list[0].content, "My homework answer");

    // 8. Timetable slots CRUD
    let slot = TimetableSlot {
        id: "slot-1".to_string(),
        workspace_id: "ws-school-test".to_string(),
        course_id: "crs-1".to_string(),
        day_of_week: 1,
        start_time: "08:30".to_string(),
        end_time: "09:45".to_string(),
        classroom: Some("Room 101".to_string()),
        updated_at: 0,
    };
    save_timetable_slot("u-school-admin".to_string(), slot.clone(), None).await.unwrap();
    let slots = get_timetable_slots("u-school-admin".to_string(), "ws-school-test".to_string()).await.unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].start_time, "08:30");

    // Cleanup
    conn.execute("DELETE FROM submissions WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM timetable_slots WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM assignments WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM report_cards WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM term_grades WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM health_incidents WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM health_records WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM student_parents WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM school_payments WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM school_invoices WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM library_books WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-school-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-school-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_school_permission_validation() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace with role templates in settings
    let settings_json = r#"{
        "roles": [
            {
                "id": "role-school-teacher",
                "name": "Teacher",
                "permissions": {
                    "can_manage_schedule": true,
                    "can_manage_grades": true
                }
            },
            {
                "id": "role-school-student",
                "name": "Student",
                "permissions": {
                    "can_manage_schedule": false,
                    "can_manage_grades": false
                }
            }
        ]
    }"#;

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-perm-test', 'Perm WS', '[]', ?1)", crate::params![settings_json]).await.unwrap();
    
    // Insert users
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-teacher', 'ws-perm-test', 'teacher@school.com', 'teacher')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student', 'ws-perm-test', 'student@school.com', 'student')", ()).await.unwrap();

    // Setup course to test
    let course = Course {
        id: "crs-p1".to_string(),
        name: "History".to_string(),
        subject: "History".to_string(),
        teacher_id: Some("u-teacher".to_string()),
        classroom: Some("Room 202".to_string()),
    };

    // 1. Teacher has can_manage_schedule, so save_course should succeed
    let save_res = save_course("u-teacher".to_string(), "ws-perm-test".to_string(), course.clone(), None).await;
    assert!(save_res.is_ok(), "Teacher should be authorized: {:?}", save_res.err());

    // 2. Student does NOT have can_manage_schedule, so save_course should fail
    let save_student_res = save_course("u-student".to_string(), "ws-perm-test".to_string(), course.clone(), None).await;
    assert!(save_student_res.is_err(), "Student should be denied");
    if let Err(YntraError::AuthError(msg)) = save_student_res {
        assert!(msg.contains("does not have permission 'can_manage_schedule'"));
    } else {
        panic!("Expected AuthError for student course save");
    }

    // Cleanup
    conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-perm-test'", ()).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-perm-test'", ()).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-perm-test'", ()).await.unwrap();
}

#[tokio::test]
async fn test_grading_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-grade-test";
    let settings_json = r#"{
        "roles": [
            {
                "id": "role-school-admin",
                "name": "Admin",
                "permissions": {
                    "can_manage_grades": true
                }
            }
        ]
    }"#;

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Grade WS', '[]', ?2)", crate::params![ws_id, settings_json]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-g', ?1, 'admin@g.com', 'admin')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-g', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-g', 'Math', 'Math', 'Room 1', ?1, 0)", crate::params![ws_id]).await.unwrap();

    // 1. Initial grade (v1)
    let grade_v1 = TermGrade {
        id: "tg-g".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-g".to_string(),
        course_id: "crs-g".to_string(),
        term_name: "Fall 2026".to_string(),
        final_grade: Some("B".to_string()),
        final_points: Some(80),
        teacher_comments: Some("Good progress".to_string()),
        updated_at: 1000,
    };

    save_term_grade("u-admin-g".to_string(), grade_v1, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // Fetch to find the actual updated_at saved in the DB
    let saved_time_a: i64 = conn.query_row(
        "SELECT updated_at FROM term_grades WHERE id = 'tg-g'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 2. Editor A updates the grade (having read the initial grade version)
    let grade_v2_a = TermGrade {
        id: "tg-g".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-g".to_string(),
        course_id: "crs-g".to_string(),
        term_name: "Fall 2026".to_string(),
        final_grade: Some("A-".to_string()),
        final_points: Some(90),
        teacher_comments: Some("Excellent progress".to_string()),
        updated_at: saved_time_a, // read version matches
    };
    save_term_grade("u-admin-g".to_string(), grade_v2_a, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // Fetch updated_at after A's write
    let _saved_time_b: i64 = conn.query_row(
        "SELECT updated_at FROM term_grades WHERE id = 'tg-g'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 3. Editor B concurrent offline update (holds stale parent updated_at)
    let grade_v2_b = TermGrade {
        id: "tg-g".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-g".to_string(),
        course_id: "crs-g".to_string(),
        term_name: "Fall 2026".to_string(),
        final_grade: Some("B+".to_string()),
        final_points: Some(85),
        teacher_comments: Some("Steady progress".to_string()),
        updated_at: saved_time_a, // stale! (points to v1, but database is now at v2a)
    };

    // This save should trigger conflict detection!
    save_term_grade("u-admin-g".to_string(), grade_v2_b, None).await.unwrap();

    // 4. Assert conflict encoding
    let (final_grade, final_points, teacher_comments): (String, Option<i64>, String) = conn.query_row(
        "SELECT final_grade, final_points, teacher_comments FROM term_grades WHERE id = 'tg-g'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).await.unwrap();

    // The main table fields remain clean under Last-Write-Wins (which keeps Editor A's version as A wrote a newer version saved_time_b > saved_time_a)
    assert_eq!(final_grade, "A-");
    assert_eq!(final_points, Some(90));
    assert_eq!(teacher_comments, "Excellent progress");

    // The conflict table should record details of both versions
    let conflict_json: String = conn.query_row(
        "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'term_grades' AND entity_id = 'tg-g'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    assert!(conflict_json.contains("\"conflict\":true"));
    assert!(conflict_json.contains("A-"));
    assert!(conflict_json.contains("B+"));

    // Cleanup
    conn.execute("DELETE FROM term_grades WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_student_profile_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-profile-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Profile WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-p', ?1, 'admin@p.com', 'admin')", crate::params![ws_id]).await.unwrap();

    // 1. Initial version
    let profile_v1 = StudentProfile {
        id: "stud-p".to_string(),
        workspace_id: ws_id.to_string(),
        user_id: None,
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        grade_level: "10A".to_string(),
        parent_contact: Some("parent@doe.com".to_string()),
        updated_at: 1000,
    };
    save_student_profile("u-admin-p".to_string(), profile_v1, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    let saved_time_a: i64 = conn.query_row(
        "SELECT updated_at FROM student_profiles WHERE id = 'stud-p'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 2. Editor A updates
    let profile_v2_a = StudentProfile {
        id: "stud-p".to_string(),
        workspace_id: ws_id.to_string(),
        user_id: None,
        first_name: "Johnny".to_string(),
        last_name: "Doe".to_string(),
        grade_level: "10A".to_string(),
        parent_contact: Some("parent-new@doe.com".to_string()),
        updated_at: saved_time_a,
    };
    save_student_profile("u-admin-p".to_string(), profile_v2_a, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // 3. Editor B updates concurrently (using stale v1 time)
    let profile_v2_b = StudentProfile {
        id: "stud-p".to_string(),
        workspace_id: ws_id.to_string(),
        user_id: None,
        first_name: "John-Boy".to_string(),
        last_name: "Doe".to_string(),
        grade_level: "10B".to_string(),
        parent_contact: Some("parent-stale@doe.com".to_string()),
        updated_at: saved_time_a, // stale!
    };
    save_student_profile("u-admin-p".to_string(), profile_v2_b, None).await.unwrap();

    // 4. Assert conflict
    let (first_name, last_name, grade_level, parent_contact): (String, String, String, String) = conn.query_row(
        "SELECT first_name, last_name, grade_level, parent_contact FROM student_profiles WHERE id = 'stud-p'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).await.unwrap();

    assert_eq!(first_name, "Johnny");
    assert_eq!(last_name, "Doe");
    assert_eq!(grade_level, "10A");
    assert_eq!(parent_contact, "parent-new@doe.com");

    let conflict_json: String = conn.query_row(
        "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'student_profiles' AND entity_id = 'stud-p'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    assert!(conflict_json.contains("\"conflict\":true"));
    assert!(conflict_json.contains("Johnny"));
    assert!(conflict_json.contains("John-Boy"));

    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_attendance_record_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-att-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Attendance WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-a', ?1, 'admin@a.com', 'admin')", crate::params![ws_id]).await.unwrap();

    // Insert required relations to satisfy FOREIGN KEY checks
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-a', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-a', 'Math', 'Math', 'Room A', ?1, 0)", crate::params![ws_id]).await.unwrap();

    // 1. Initial version
    let rec_v1 = AttendanceRecord {
        id: "att-1".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-a".to_string(),
        course_id: "crs-a".to_string(),
        date: "2026-07-19".to_string(),
        status: "present".to_string(),
        notes: Some("On time".to_string()),
        updated_at: 1000,
    };
    save_attendance_record("u-admin-a".to_string(), rec_v1, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    let saved_time_a: i64 = conn.query_row(
        "SELECT updated_at FROM attendance_records WHERE id = 'att-1'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 2. Editor A updates
    let rec_v2_a = AttendanceRecord {
        id: "att-1".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-a".to_string(),
        course_id: "crs-a".to_string(),
        date: "2026-07-19".to_string(),
        status: "late".to_string(),
        notes: Some("Late 5 minutes".to_string()),
        updated_at: saved_time_a,
    };
    save_attendance_record("u-admin-a".to_string(), rec_v2_a, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // 3. Editor B updates concurrently
    let rec_v2_b = AttendanceRecord {
        id: "att-1".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-a".to_string(),
        course_id: "crs-a".to_string(),
        date: "2026-07-19".to_string(),
        status: "excused".to_string(),
        notes: Some("Parent called".to_string()),
        updated_at: saved_time_a, // stale!
    };
    save_attendance_record("u-admin-a".to_string(), rec_v2_b, None).await.unwrap();

    // 4. Assert conflict
    let (status, notes): (String, String) = conn.query_row(
        "SELECT status, notes FROM attendance_records WHERE id = 'att-1'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).await.unwrap();

    assert_eq!(status, "late");
    assert_eq!(notes, "Late 5 minutes");

    let conflict_json: String = conn.query_row(
        "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'attendance_records' AND entity_id = 'att-1'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    assert!(conflict_json.contains("\"conflict\":true"));
    assert!(conflict_json.contains("late"));
    assert!(conflict_json.contains("excused"));

    conn.execute("DELETE FROM attendance_records WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_timetable_slot_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-slot-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Slot WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-s', ?1, 'admin@s.com', 'admin')", crate::params![ws_id]).await.unwrap();

    // Insert required relations to satisfy FOREIGN KEY checks
    conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-slot-t', 'Math', 'Math', 'Room A', ?1, 0)", crate::params![ws_id]).await.unwrap();

    // 1. Initial version
    let slot_v1 = TimetableSlot {
        id: "slot-t".to_string(),
        workspace_id: ws_id.to_string(),
        course_id: "crs-slot-t".to_string(),
        day_of_week: 1,
        start_time: "09:00".to_string(),
        end_time: "10:00".to_string(),
        classroom: Some("Room A".to_string()),
        updated_at: 1000,
    };
    save_timetable_slot("u-admin-s".to_string(), slot_v1, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    let saved_time_a: i64 = conn.query_row(
        "SELECT updated_at FROM timetable_slots WHERE id = 'slot-t'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 2. Editor A updates
    let slot_v2_a = TimetableSlot {
        id: "slot-t".to_string(),
        workspace_id: ws_id.to_string(),
        course_id: "crs-slot-t".to_string(),
        day_of_week: 1,
        start_time: "09:00".to_string(),
        end_time: "10:00".to_string(),
        classroom: Some("Room B".to_string()),
        updated_at: saved_time_a,
    };
    save_timetable_slot("u-admin-s".to_string(), slot_v2_a, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // 3. Editor B updates concurrently
    let slot_v2_b = TimetableSlot {
        id: "slot-t".to_string(),
        workspace_id: ws_id.to_string(),
        course_id: "crs-slot-t".to_string(),
        day_of_week: 1,
        start_time: "09:00".to_string(),
        end_time: "10:00".to_string(),
        classroom: Some("Room C".to_string()),
        updated_at: saved_time_a, // stale!
    };
    save_timetable_slot("u-admin-s".to_string(), slot_v2_b, None).await.unwrap();

    // 4. Assert conflict
    let (course_id, day_of_week, start_time, end_time, classroom): (String, i64, String, String, String) = conn.query_row(
        "SELECT course_id, day_of_week, start_time, end_time, classroom FROM timetable_slots WHERE id = 'slot-t'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).await.unwrap();

    assert_eq!(course_id, "crs-slot-t");
    assert_eq!(day_of_week, 1);
    assert_eq!(start_time, "09:00");
    assert_eq!(end_time, "10:00");
    assert_eq!(classroom, "Room B");

    let conflict_json: String = conn.query_row(
        "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'timetable_slots' AND entity_id = 'slot-t'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    assert!(conflict_json.contains("\"conflict\":true"));
    assert!(conflict_json.contains("Room B"));
    assert!(conflict_json.contains("Room C"));

    conn.execute("DELETE FROM timetable_slots WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_health_incident_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-health-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Health WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-h', ?1, 'admin@h.com', 'admin')", crate::params![ws_id]).await.unwrap();

    // Insert required relations to satisfy FOREIGN KEY checks
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-h', ?1, 'John', 'Doe', 'Grade 10', 0)", crate::params![ws_id]).await.unwrap();

    // 1. Initial version
    let inc_v1 = HealthIncident {
        id: "inc-t".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-h".to_string(),
        visit_reason: "Cough".to_string(),
        treatment: "Cough Syrup".to_string(),
        checked_in_at: "09:00".to_string(),
        checked_out_at: Some("09:15".to_string()),
        notes: Some("Slight cold".to_string()),
        updated_at: 1000,
    };
    save_health_incident("u-admin-h".to_string(), inc_v1, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    let saved_time_a: i64 = conn.query_row(
        "SELECT updated_at FROM health_incidents WHERE id = 'inc-t'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    // 2. Editor A updates
    let inc_v2_a = HealthIncident {
        id: "inc-t".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-h".to_string(),
        visit_reason: "Cough".to_string(),
        treatment: "Cough Syrup + Tea".to_string(),
        checked_in_at: "09:00".to_string(),
        checked_out_at: Some("09:15".to_string()),
        notes: Some("Rest advised".to_string()),
        updated_at: saved_time_a,
    };
    save_health_incident("u-admin-h".to_string(), inc_v2_a, None).await.unwrap();
    crate::infra::time::sleep_ms(10).await;

    // 3. Editor B updates concurrently
    let inc_v2_b = HealthIncident {
        id: "inc-t".to_string(),
        workspace_id: ws_id.to_string(),
        student_id: "stud-h".to_string(),
        visit_reason: "Fever".to_string(),
        treatment: "Paracetamol".to_string(),
        checked_in_at: "09:10".to_string(),
        checked_out_at: Some("09:30".to_string()),
        notes: Some("Temp 38.5C".to_string()),
        updated_at: saved_time_a, // stale!
    };
    save_health_incident("u-admin-h".to_string(), inc_v2_b, None).await.unwrap();

    // 4. Assert conflict
    let (reason, treatment, checked_in_at, notes): (String, String, String, String) = conn.query_row(
        "SELECT visit_reason, treatment, checked_in_at, notes FROM health_incidents WHERE id = 'inc-t'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).await.unwrap();

    assert_eq!(reason, "Cough");
    assert_eq!(treatment, "Cough Syrup + Tea");
    assert_eq!(checked_in_at, "09:00");
    assert_eq!(notes, "Rest advised");

    let conflict_json: String = conn.query_row(
        "SELECT conflict_json FROM school_conflicts WHERE entity_table = 'health_incidents' AND entity_id = 'inc-t'",
        (),
        |r| r.get(0),
    ).await.unwrap();

    assert!(conflict_json.contains("\"conflict\":true"));
    assert!(conflict_json.contains("Cough"));
    assert!(conflict_json.contains("Fever"));

    conn.execute("DELETE FROM health_incidents WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_school_row_level_sync() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace and users
    let ws_id = "ws-sync-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Sync WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    
    // Unprivileged student user
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-student-sync', ?1, 'std@school.com', 'student')", crate::params![ws_id]).await.unwrap();
    
    // Admins and other students
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-other-student', ?1, 'other@school.com', 'student')", crate::params![ws_id]).await.unwrap();

    // 2. Setup Student Profiles
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-sync-1', ?1, 'SyncStudent', 'One', '10A', 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-sync-2', ?1, 'OtherStudent', 'Two', '10A', 0)", crate::params![ws_id]).await.unwrap();

    // Map users to students so partitioning knows which students belong to the requester
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, role_signature) VALUES ('u-student-sync', ?1, 'std@school.com', 'student', 'valid-sig')", crate::params![ws_id]).await.unwrap();
    // Insert student profile link for student-sync
    conn.execute("UPDATE student_profiles SET user_id = 'u-student-sync' WHERE id = 'stud-sync-1'", ()).await.unwrap();

    // Insert Course and Assignment to satisfy foreign key constraints
    conn.execute("INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at) VALUES ('crs-sync-1', ?1, 'Math', 'MATH101', 'u-teacher', 'Room 101', 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at) VALUES ('assign-sync-1', ?1, 'crs-sync-1', 'HW1', 'Homework 1', '2026-08-01', 100, 0)", crate::params![ws_id]).await.unwrap();

    // 3. Create a pending local write on submissions (e.g. submitting an assignment)
    conn.execute("INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, submitted_at, updated_at, sync_status) VALUES ('sub-sync-1', ?1, 'assign-sync-1', 'stud-sync-1', 'My Homework Content', '2026-07-20', 0, 'pending')", crate::params![ws_id]).await.unwrap();

    // 4. Configure database sync to enable sync loop (local/mock mode)
    crate::database::sync::configure_database_sync("local_url".to_string(), "local_token".to_string());

    // 5. Trigger sync
    crate::database::sync::sync_database().await.unwrap();

    // 6. Verify that submissions is marked as synced locally
    let sync_status: String = conn.query_row(
        "SELECT sync_status FROM submissions WHERE id = 'sub-sync-1'",
        (),
        |r| r.get(0),
    ).await.unwrap();
    assert_eq!(sync_status, "synced");

    // Clean up
    conn.execute("DELETE FROM submissions WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM assignments WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}

#[tokio::test]
async fn test_school_conflict_resolution() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    let ws_id = "ws-conflict-test";
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Conflict WS', '[]', '{}')", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-conflict-admin', ?1, 'admin@conf.com', 'admin')", crate::params![ws_id]).await.unwrap();

    // 1. Setup a conflict record in school_conflicts
    let conflict_json = serde_json::json!({
        "conflict": true,
        "versions": [
            {
                "status": "absent",
                "notes": "Original note",
                "by": "Concurrent Editor",
                "updated_at": 100
            },
            {
                "status": "present",
                "notes": "New note",
                "by": "u-conflict-admin",
                "updated_at": 200
            }
        ]
    });

    // Insert a dummy student_profile, course, and attendance_record to satisfy foreign keys
    conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-c', ?1, NULL, 'John', 'Doe', '10', 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-c', 'Math', 'Math', 'Room 1', ?1, 0)", crate::params![ws_id]).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at) VALUES ('att-c', ?1, 'stud-c', 'crs-c', '2026-07-20', 'absent', 'Original note', 100)", crate::params![ws_id]).await.unwrap();

    let conflict_id = "attendance_records-att-c-12345";
    conn.execute(
        "INSERT INTO school_conflicts (id, workspace_id, entity_table, entity_id, conflict_json, updated_at) VALUES (?1, ?2, 'attendance_records', 'att-c', ?3, 200)",
        crate::params![conflict_id, ws_id, conflict_json.to_string()]
    ).await.unwrap();

    // 2. Verify get_school_conflicts retrieves it
    let conflicts = get_school_conflicts("u-conflict-admin".to_string(), ws_id.to_string()).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].id, conflict_id);
    assert_eq!(conflicts[0].entity_table, "attendance_records");

    // 3. Resolve using version choice "1" (present, New note)
    resolve_school_conflict(
        "u-conflict-admin".to_string(),
        ws_id.to_string(),
        conflict_id.to_string(),
        "1".to_string(),
        None,
        None
    ).await.unwrap();

    // 4. Verify record in database has been updated and conflict has been removed
    let (resolved_status, resolved_notes): (String, Option<String>) = conn.query_row(
        "SELECT status, notes FROM attendance_records WHERE id = 'att-c'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.unwrap();
    assert_eq!(resolved_status, "present");
    assert_eq!(resolved_notes.unwrap(), "New note");

    let remaining = get_school_conflicts("u-conflict-admin".to_string(), ws_id.to_string()).await.unwrap();
    assert_eq!(remaining.len(), 0);

    // 5. Clean up
    conn.execute("DELETE FROM attendance_records WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM courses WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM student_profiles WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = ?1", crate::params![ws_id]).await.unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", crate::params![ws_id]).await.unwrap();
}
