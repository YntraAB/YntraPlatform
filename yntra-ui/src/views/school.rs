use crate::components;
use crate::locales;
use crate::state::AppState;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;

pub mod dashboard;
pub mod students;
pub mod courses;
pub mod attendance;
pub mod student_portal;
pub mod parent_portal;
pub mod report_cards;
pub mod health;
pub mod billing;
pub mod library;
pub mod modals;

#[derive(Props, Clone)]
pub struct SchoolViewProps {
    pub active_user: WorkspaceUser,
    pub db_trigger: Signal<u32>,
    pub locale: String,
    pub initial_tab: Option<String>,
}

impl PartialEq for SchoolViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn SchoolView(props: SchoolViewProps) -> Element {
    let active_user = props.active_user.clone();
    let db_trig = *props.db_trigger.read();
    let mut db_trigger = props.db_trigger;
    let state = use_context::<AppState>();
    // View toggle: true = Teacher/Admin view, false = Student/Kids portal view
    let mut is_teacher_view = use_signal(|| true);

    let initial = props.initial_tab.clone().unwrap_or_else(|| "dashboard".to_string());
    let mut active_tab = use_signal(|| initial);

    use_effect(use_reactive(&props.initial_tab, move |init_tab| {
        if let Some(tab) = init_tab {
            active_tab.set(tab);
        }
    }));

    let ws_opt = state.workspace.read().clone();
    let modules_active: serde_json::Value = ws_opt
        .as_ref()
        .and_then(|ws| serde_json::from_str(&ws.modules_active).ok())
        .unwrap_or_default();

    let is_academics_enabled = modules_active.get("academics").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_attendance_enabled = modules_active.get("attendance").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_finance_enabled = modules_active.get("finance").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_library_enabled = modules_active.get("library").and_then(|v| v.as_bool()).unwrap_or(false);

    let has_perm = |permission: &str| -> bool {
        if active_user.role == "platform_admin" || active_user.role == "admin" {
            return true;
        }
        if let Some(ref ws) = ws_opt {
            let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
            if let Some(roles) = settings_val.get("roles").and_then(|r| r.as_array()) {
                for role_val in roles {
                    if let Some(perms) = role_val.get("permissions").filter(|_| role_val.get("id").and_then(|i| i.as_str()) == Some(&active_user.role)) {
                        return perms.get(permission).and_then(|p| p.as_bool()).unwrap_or(false);
                    }
                }
            }
        }
        // Fallback for default unconfigured roles
        if active_user.role.contains("rektor") || active_user.role.contains("principal") {
            return true;
        }
        if active_user.role.contains("larare") || active_user.role.contains("teacher") {
            return matches!(permission, "can_manage_students" | "can_manage_grades" | "can_manage_schedule" | "can_manage_library" | "can_submit_reports");
        }
        if active_user.role.contains("syv") || active_user.role.contains("counselor") {
            return matches!(permission, "can_manage_students" | "can_submit_reports");
        }
        if active_user.role.contains("skoterska") || active_user.role.contains("nurse") {
            return matches!(permission, "can_access_health_records" | "can_submit_reports");
        }
        false
    };

    let can_access_teacher_view = has_perm("can_manage_students")
        || has_perm("can_manage_grades")
        || has_perm("can_manage_schedule")
        || has_perm("can_access_health_records")
        || has_perm("can_manage_billing")
        || has_perm("can_manage_library");

    if !can_access_teacher_view && *is_teacher_view.read() {
        is_teacher_view.set(false);
    }

    // 1. Fetch Students
    let uid_for_students = active_user.id.clone();
    let students_res = use_resource(move || {
        let _ = db_trig;
        let uid = uid_for_students.clone();
        async move {
            yntra_core::get_students(uid).await.unwrap_or_default()
        }
    });
    let students = students_res.read().clone().unwrap_or_default();

    // 2. Fetch Courses
    let uid_for_courses = active_user.id.clone();
    let courses_res = use_resource(move || {
        let _ = db_trig;
        let uid = uid_for_courses.clone();
        async move {
            yntra_core::get_courses(uid).await.unwrap_or_default()
        }
    });
    let courses = courses_res.read().clone().unwrap_or_default();

    // Selected Course ID for detail view
    let mut selected_course_id = use_signal(|| Option::<String>::None);

    // Fetch assignments for selected course
    let assignments_res = use_resource(move || {
        let _ = db_trig;
        let c_id = selected_course_id.read().clone();
        async move {
            if let Some(id) = c_id {
                yntra_core::get_assignments(id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let assignments = assignments_res.read().clone().unwrap_or_default();

    // Selected Assignment ID for grading view
    let selected_assignment_id = use_signal(|| Option::<String>::None);

    // Fetch submissions for selected assignment
    let uid_for_submissions = active_user.id.clone();
    let submissions_res = use_resource(move || {
        let _ = db_trig;
        let a_id = selected_assignment_id.read().clone();
        let uid = uid_for_submissions.clone();
        async move {
            if let Some(id) = a_id {
                yntra_core::get_submissions(uid, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let submissions = submissions_res.read().clone().unwrap_or_default();

    // Impersonated Student for portal view
    let impersonated_student_id = use_signal(|| Option::<String>::None);
    let selected_student = students
        .iter()
        .find(|s| Some(s.id.clone()) == *impersonated_student_id.read())
        .or(students.first())
        .cloned();

    // Student Portal Data Resource (fetches all assignments and submissions for portal view)
    let uid_for_portal = active_user.id.clone();
    let student_portal_data_res = use_resource(move || {
        let _ = db_trig;
        let uid = uid_for_portal.clone();
        async move {
            let courses_list = yntra_core::get_courses(uid.clone()).await.unwrap_or_default();
            let mut all_assigns = Vec::new();
            for c in courses_list.iter() {
                if let Ok(assigns) = yntra_core::get_assignments(c.id.clone()).await {
                    all_assigns.extend(assigns);
                }
            }
            let mut all_subs = Vec::new();
            for a in all_assigns.iter() {
                if let Ok(subs) = yntra_core::get_submissions(uid.clone(), a.id.clone()).await {
                    all_subs.extend(subs);
                }
            }
            (all_assigns, all_subs)
        }
    });

    let show_school_setup = use_signal(|| {
        if let Some(ref ws) = ws_opt {
            if active_user.role == "admin" || active_user.role == "platform_admin" {
                let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                !settings_val.get("school_setup_completed").and_then(|v| v.as_bool()).unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    });

    let handle_school_setup = {
        let ws_opt = ws_opt.clone();
        let mut db_trigger = db_trigger;
        let mut show_school_setup = show_school_setup;
        let active_user_id = active_user.id.clone();
        move |(grading, late, country, school_type): (String, String, String, String)| {
            if let Some(ref ws) = ws_opt {
                let ws_id = ws.id.clone();
                let mut settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                settings_val["grading_system"] = serde_json::json!(grading);
                settings_val["late_policy"] = serde_json::json!(late);
                settings_val["country"] = serde_json::json!(country);
                settings_val["school_type"] = serde_json::json!(school_type);
                settings_val["school_setup_completed"] = serde_json::json!(true);
                
                if country == "SE" {
                    settings_val["language"] = serde_json::json!("sv");
                    settings_val["timezone"] = serde_json::json!("Europe/Stockholm");
                    settings_val["week_start"] = serde_json::json!(1);
                }
                
                let settings_str = serde_json::to_string(&settings_val).unwrap_or_default();
                let requester_uid = active_user_id.clone();
                spawn(async move {
                    let _ = yntra_core::update_workspace_settings(requester_uid, ws_id, settings_str).await;
                    show_school_setup.set(false);
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                });
            }
        }
    };
    let (all_assignments, all_submissions) = student_portal_data_res.read().clone().unwrap_or_else(|| (Vec::new(), Vec::new()));

    // Enrollment Modal Signals
    let mut show_enroll_modal = use_signal(|| false);
    let mut new_student_first_name = use_signal(String::new);
    let mut new_student_last_name = use_signal(String::new);
    let new_student_grade = use_signal(|| "Grade 10".to_string());
    let mut new_student_contact = use_signal(String::new);

    // Course Creation Modal Signals
    let mut show_course_modal = use_signal(|| false);
    let mut new_course_name = use_signal(String::new);
    let mut new_course_subject = use_signal(String::new);
    let mut new_course_classroom = use_signal(String::new);

    // Assignment Creation Modal Signals
    let mut show_assignment_modal = use_signal(|| false);
    let mut new_assign_title = use_signal(String::new);
    let mut new_assign_desc = use_signal(String::new);
    let new_assign_due = use_signal(|| "2026-07-15".to_string());
    let new_assign_points = use_signal(|| "100".to_string());

    // Attendance Date and Course Signals
    let attendance_course_id = use_signal(|| Option::<String>::None);
    let attendance_date = use_signal(|| "2026-07-04".to_string());

    // Fetch Attendance records
    let uid_for_attendance = active_user.id.clone();
    let attendance_res = use_resource(move || {
        let _ = db_trig;
        let c_id = attendance_course_id.read().clone();
        let date_str = attendance_date.read().clone();
        let uid = uid_for_attendance.clone();
        async move {
            if let Some(id) = c_id {
                yntra_core::get_attendance(uid, id, date_str).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let attendance_records = attendance_res.read().clone().unwrap_or_default();

    // Grading Modal Signals
    let mut show_grading_modal = use_signal(|| false);
    let selected_submission_id = use_signal(|| Option::<String>::None);
    let grade_input = use_signal(|| "A".to_string());
    let feedback_input = use_signal(String::new);

    // Student Homework Submission Modal Signals
    let mut show_submit_modal = use_signal(|| false);
    let submit_assignment_id = use_signal(|| Option::<String>::None);
    let mut submission_text = use_signal(String::new);

    let mut tab_items = vec![
        components::tabs::TabItem {
            value: "dashboard".to_string(),
            label: "Dashboard".to_string(),
            icon: Some("dashboard".to_string()),
        },
    ];

    if has_perm("can_manage_students") && (is_academics_enabled || is_attendance_enabled) {
        tab_items.push(components::tabs::TabItem {
            value: "students".to_string(),
            label: "Students".to_string(),
            icon: Some("directory".to_string()),
        });
    }
    if has_perm("can_manage_grades") && is_academics_enabled {
        tab_items.push(components::tabs::TabItem {
            value: "courses".to_string(),
            label: "Courses & Grading".to_string(),
            icon: Some("book-open".to_string()),
        });
    }
    if has_perm("can_manage_schedule") && is_attendance_enabled {
        tab_items.push(components::tabs::TabItem {
            value: "attendance".to_string(),
            label: "Attendance".to_string(),
            icon: Some("scheduling".to_string()),
        });
    }
    if (has_perm("can_publish_report_cards") || has_perm("can_manage_grades")) && is_academics_enabled {
        tab_items.push(components::tabs::TabItem {
            value: "report_cards".to_string(),
            label: "Report Cards".to_string(),
            icon: Some("award".to_string()),
        });
    }

    if has_perm("can_access_health_records") && (is_academics_enabled || is_attendance_enabled) {
        tab_items.push(components::tabs::TabItem {
            value: "health".to_string(),
            label: "Medical & Health".to_string(),
            icon: Some("heart".to_string()),
        });
    }
    if has_perm("can_manage_billing") && is_finance_enabled && props.locale != "se" {
        tab_items.push(components::tabs::TabItem {
            value: "billing".to_string(),
            label: "Billing & Fees".to_string(),
            icon: Some("credit-card".to_string()),
        });
    }
    if has_perm("can_manage_library") && is_library_enabled {
        tab_items.push(components::tabs::TabItem {
            value: "library".to_string(),
            label: "Library Catalog".to_string(),
            icon: Some("book-open".to_string()),
        });
    }

    // Redirect unauthorized tab selection
    let active_tab_val = active_tab.read().clone();
    if !tab_items.iter().any(|t| t.value == active_tab_val) {
        active_tab.set("dashboard".to_string());
    }

    let workspace_id = use_signal(|| {
        state.workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string())
    });

    // Helper views calculated outside of rsx! to avoid nested blocks
    let student_unsubmitted_assigns: Vec<yntra_core::Assignment> = if let Some(ref student) = selected_student {
        all_assignments.iter().filter(|a| {
            !all_submissions.iter().any(|s| s.assignment_id == a.id && s.student_id == student.id)
        }).cloned().collect()
    } else {
        Vec::new()
    };

    let student_subs: Vec<yntra_core::Submission> = if let Some(ref student) = selected_student {
        all_submissions.iter().filter(|s| s.student_id == student.id).cloned().collect()
    } else {
        Vec::new()
    };

    let locale = props.locale.clone();
    let is_parent = active_user.role == "parent";

    let active_user_id_for_submit = active_user.id.clone();
    let active_user_id_for_course = active_user.id.clone();
    let active_user_id_for_assign = active_user.id.clone();
    let active_user_id_for_grade = active_user.id.clone();



    rsx! {
        div {
            class: "mx-auto w-full max-w-6xl animate-in fade-in slide-in-from-top-4 duration-300",
            style: "padding: 2rem; display: flex; flex-direction: column; gap: 1.5rem; box-sizing: border-box;",
            
            if is_parent {
                parent_portal::ParentPortal {
                    active_user: active_user.clone(),
                    students: students.clone(),
                    courses: courses.clone(),
                    db_trigger: db_trigger,
                    locale: locale.clone(),
                }
            } else {
                // Header Section
                div { class: "flex justify-between items-center mb-2",
                if selected_course_id.read().is_some() {
                    button {
                        class: "yntra-btn secondary text-xs flex items-center gap-2 px-3 py-1.5 cursor-pointer rounded-lg",
                        onclick: move |_| {
                            selected_course_id.set(None);
                        },
                        components::LucideIcon { name: "arrow-left", size: "14" }
                        "Back to Courses"
                    }
                } else {
                    div {}
                }
                
                div { class: "flex gap-2 items-center",
                    // Segmented Control for Portal view
                    if can_access_teacher_view {
                        div { class: "flex bg-sidebar border border-border/80 p-0.5 rounded-lg mr-2",
                            button {
                                class: format!("px-3 py-1 rounded text-xs font-bold transition-all {}", if *is_teacher_view.read() { "bg-primary text-primary-foreground shadow" } else { "text-muted-foreground hover:text-foreground" }),
                                onclick: move |_| is_teacher_view.set(true),
                                { locales::t("school-teacher-view-tab", &locale) }
                            }
                            button {
                                class: format!("px-3 py-1 rounded text-xs font-bold transition-all {}", if !*is_teacher_view.read() { "bg-primary text-primary-foreground shadow" } else { "text-muted-foreground hover:text-foreground" }),
                                onclick: move |_| is_teacher_view.set(false),
                                { locales::t("school-student-portal-tab", &locale) }
                            }
                        }
                    }

                    if *is_teacher_view.read() {
                        if *active_tab.read() == "students" && has_perm("can_manage_students") {
                            button {
                                class: "yntra-btn text-xs font-bold flex items-center gap-1.5",
                                onclick: move |_| show_enroll_modal.set(true),
                                components::LucideIcon { name: "plus", size: "14" }
                                "Enroll Student"
                            }
                        }
                        if *active_tab.read() == "courses" && selected_course_id.read().is_none() && has_perm("can_manage_workspace") {
                            button {
                                class: "yntra-btn text-xs font-bold flex items-center gap-1.5",
                                onclick: move |_| show_course_modal.set(true),
                                components::LucideIcon { name: "plus", size: "14" }
                                "Add Course"
                            }
                        }
                    }
                }
                }

                if *is_teacher_view.read() {
                    // Tab contents
                if *active_tab.read() == "dashboard" {
                    dashboard::TeacherDashboard {
                        students: students.clone(),
                        courses: courses.clone(),
                        active_tab: active_tab,
                    }
                } else if *active_tab.read() == "students" {
                    students::StudentsList {
                        students: students.clone(),
                    }
                } else if *active_tab.read() == "courses" {
                    courses::CoursesGrading {
                        courses: courses.clone(),
                        students: students.clone(),
                        selected_course_id: selected_course_id,
                        selected_assignment_id: selected_assignment_id,
                        assignments: assignments.clone(),
                        submissions: submissions.clone(),
                        show_assignment_modal: show_assignment_modal,
                        show_grading_modal: show_grading_modal,
                        selected_submission_id: selected_submission_id,
                        grade_input: grade_input,
                        feedback_input: feedback_input,
                        active_user_id: active_user.id.clone(),
                        active_user_role: active_user.role.clone(),
                        workspace_id: workspace_id,
                        db_trigger: db_trigger,
                    }
                } else if *active_tab.read() == "attendance" {
                    attendance::AttendanceTracker {
                        active_user_id: active_user.id.clone(),
                        courses: courses.clone(),
                        students: students.clone(),
                        attendance_records: attendance_records.clone(),
                        attendance_course_id: attendance_course_id,
                        attendance_date: attendance_date,
                        workspace_id: workspace_id.read().clone(),
                        db_trigger: db_trigger,
                    }
                } else if *active_tab.read() == "report_cards" {
                    report_cards::ReportCardsGrading {
                        active_user_id: active_user.id.clone(),
                        students: students.clone(),
                        courses: courses.clone(),
                        workspace_id: workspace_id.read().clone(),
                        db_trigger: db_trigger,
                        locale: locale.clone(),
                    }

                } else if *active_tab.read() == "health" {
                    health::HealthRegistry {
                        students: students.clone(),
                        workspace_id: workspace_id.read().clone(),
                        db_trigger: db_trigger,
                        locale: locale.clone(),
                    }
                } else if *active_tab.read() == "billing" {
                    billing::BillingDashboard {
                        active_user_id: active_user.id.clone(),
                        students: students.clone(),
                        workspace_id: workspace_id.read().clone(),
                        db_trigger: db_trigger,
                        locale: locale.clone(),
                    }
                } else if *active_tab.read() == "library" {
                    library::LibraryDashboard {
                        active_user_id: active_user.id.clone(),
                        students: students.clone(),
                        workspace_id: workspace_id.read().clone(),
                        db_trigger: db_trigger,
                        locale: locale.clone(),
                    }
                }
            } else {
                student_portal::StudentPortal {
                    active_user_id: active_user.id.clone(),
                    students: students.clone(),
                    selected_student: selected_student.clone(),
                    impersonated_student_id: impersonated_student_id,
                    student_unsubmitted_assigns: student_unsubmitted_assigns.clone(),
                    student_subs: student_subs.clone(),
                    all_assignments: all_assignments.clone(),
                    courses: courses.clone(),
                    submit_assignment_id: submit_assignment_id,
                    submission_text: submission_text,
                    show_submit_modal: show_submit_modal,
                    locale: locale.clone(),
                }
            }
        }

        // --- Decoupled Modal Dialogs ---

        // Student Homework Submission Modal
        {
            let a_id = submit_assignment_id.read().clone().unwrap_or_default();
            let target_assign = all_assignments.iter().find(|a| a.id == a_id);
            let title = target_assign.map(|a| a.title.clone()).unwrap_or_default();
            let desc = target_assign.map(|a| a.description.clone()).unwrap_or_default();
            let due_date = target_assign.map(|a| a.due_date.clone()).unwrap_or_default();
            let s_id = selected_student.clone().map(|s| s.id).unwrap_or_default();
            let ws = workspace_id.read().clone();
            
            let resolved_late_policy = {
                let ws_opt = state.workspace.read().clone();
                let course_id = target_assign.map(|a| a.course_id.clone());
                
                if let (Some(c_id), Some(ws)) = (course_id, ws_opt) {
                    let block_settings: serde_json::Value = serde_json::from_str(&ws.block_settings).unwrap_or_default();
                    let override_l = block_settings.get("course_settings")
                        .and_then(|c| c.get(&c_id))
                        .and_then(|cs| cs.get("late_policy"))
                        .and_then(|l| l.as_str())
                        .map(|l| l.to_string());
                    
                    if let Some(ol) = override_l {
                        if ol != "default" {
                            ol
                        } else {
                            let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                            settings_val.get("late_policy").and_then(|v| v.as_str()).unwrap_or("none").to_string()
                        }
                    } else {
                        let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                        settings_val.get("late_policy").and_then(|v| v.as_str()).unwrap_or("none").to_string()
                    }
                } else {
                    "none".to_string()
                }
            };
            
            rsx! {
                modals::HomeworkSubmitModal {
                    open: *show_submit_modal.read(),
                    title: title,
                    desc: desc,
                    submission_text: submission_text,
                    due_date: due_date,
                    late_policy: resolved_late_policy,
                    onclose: move |_| show_submit_modal.set(false),
                    onsubmit: move |_| {
                        let ws_clone = ws.clone();
                        let aid_clone = a_id.clone();
                        let sid_clone = s_id.clone();
                        let content = submission_text.read().clone();
                        let uid = active_user_id_for_submit.clone();
                        
                        spawn(async move {
                            if yntra_core::add_submission(uid, ws_clone, aid_clone, sid_clone, content, None, None).await.is_ok() {
                                show_submit_modal.set(false);
                                submission_text.set(String::new());
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            }
                        });
                    }
                }
            }
        }

        // Student Enrollment Modal
        modals::StudentEnrollModal {
            open: *show_enroll_modal.read(),
            first_name: new_student_first_name,
            last_name: new_student_last_name,
            grade: new_student_grade,
            contact: new_student_contact,
            onclose: move |_| show_enroll_modal.set(false),
            onsubmit: move |_| {
                let first = new_student_first_name.read().clone();
                let last = new_student_last_name.read().clone();
                let grade = new_student_grade.read().clone();
                let contact = new_student_contact.read().clone();
                let contact_opt = if contact.is_empty() { None } else { Some(contact) };
                let ws = workspace_id.read().clone();
                let requester_user_id = active_user.id.clone();
                
                spawn(async move {
                    if yntra_core::add_student(requester_user_id, ws, None, first, last, grade, contact_opt).await.is_ok() {
                        show_enroll_modal.set(false);
                        new_student_first_name.set(String::new());
                        new_student_last_name.set(String::new());
                        new_student_contact.set(String::new());
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                    }
                });
            }
        }

        // Course Creation Modal
        modals::CourseCreateModal {
            open: *show_course_modal.read(),
            name: new_course_name,
            subject: new_course_subject,
            classroom: new_course_classroom,
            onclose: move |_| show_course_modal.set(false),
            onsubmit: move |_| {
                let name = new_course_name.read().clone();
                let subj = new_course_subject.read().clone();
                let room = new_course_classroom.read().clone();
                let room_opt = if room.is_empty() { None } else { Some(room) };
                let teacher_id = Some(active_user_id_for_course.clone());
                let ws = workspace_id.read().clone();
                let uid = active_user_id_for_course.clone();
                
                spawn(async move {
                    if yntra_core::add_course(uid, ws, name, subj, teacher_id, room_opt).await.is_ok() {
                        show_course_modal.set(false);
                        new_course_name.set(String::new());
                        new_course_subject.set(String::new());
                        new_course_classroom.set(String::new());
                        let current = *db_trigger.read();
                        db_trigger.set(current + 1);
                    }
                });
            }
        }

        // Assignment Creation Modal
        {
            let ws = workspace_id.read().clone();
            let c_id = selected_course_id.read().clone().unwrap_or_default();
            
            rsx! {
                modals::AssignmentCreateModal {
                    open: *show_assignment_modal.read(),
                    title: new_assign_title,
                    desc: new_assign_desc,
                    due: new_assign_due,
                    points: new_assign_points,
                    onclose: move |_| show_assignment_modal.set(false),
                    onsubmit: move |_| {
                        let title = new_assign_title.read().clone();
                        let desc = new_assign_desc.read().clone();
                        let due = new_assign_due.read().clone();
                        let max_pts = new_assign_points.read().parse::<i32>().unwrap_or(100);
                        let cid = c_id.clone();
                        let ws_clone = ws.clone();
                        let uid = active_user_id_for_assign.clone();
                        
                        spawn(async move {
                            if yntra_core::add_assignment(uid, ws_clone, cid, title, desc, due, max_pts).await.is_ok() {
                                show_assignment_modal.set(false);
                                new_assign_title.set(String::new());
                                new_assign_desc.set(String::new());
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            }
                        });
                    }
                }
            }
        }

        // Submission Grading Modal
        {
            let sub_id = selected_submission_id.read().clone().unwrap_or_default();
            let resolved_grading_system = {
                let submissions_list = submissions.clone();
                let assignments_list = assignments.clone();
                let ws_opt = state.workspace.read().clone();
                
                let course_id = submissions_list.iter()
                    .find(|s| s.id == sub_id)
                    .and_then(|s| assignments_list.iter().find(|a| a.id == s.assignment_id))
                    .map(|a| a.course_id.clone());
                
                if let (Some(c_id), Some(ws)) = (course_id, ws_opt) {
                    let block_settings: serde_json::Value = serde_json::from_str(&ws.block_settings).unwrap_or_default();
                    let override_g = block_settings.get("course_settings")
                        .and_then(|c| c.get(&c_id))
                        .and_then(|cs| cs.get("grading_system"))
                        .and_then(|g| g.as_str())
                        .map(|g| g.to_string());
                    
                    if let Some(og) = override_g {
                        if og != "default" {
                            og
                        } else {
                            let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                            settings_val.get("grading_system").and_then(|v| v.as_str()).unwrap_or("A-F").to_string()
                        }
                    } else {
                        let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
                        settings_val.get("grading_system").and_then(|v| v.as_str()).unwrap_or("A-F").to_string()
                    }
                } else {
                    "A-F".to_string()
                }
            };
            
            rsx! {
                modals::GradeSubmissionModal {
                    open: *show_grading_modal.read(),
                    grade: grade_input,
                    feedback: feedback_input,
                    grading_system: resolved_grading_system,
                    onclose: move |_| show_grading_modal.set(false),
                    onsubmit: move |_| {
                        let grade = Some(grade_input.read().clone());
                        let feedback = feedback_input.read().clone();
                        let feedback_opt = if feedback.is_empty() { None } else { Some(feedback) };
                        let sid = sub_id.clone();
                        let uid = active_user_id_for_grade.clone();
                        
                        spawn(async move {
                            if yntra_core::update_submission_grade(uid, sid, grade, feedback_opt).await.is_ok() {
                                show_grading_modal.set(false);
                                let current = *db_trigger.read();
                                db_trigger.set(current + 1);
                            }
                        });
                    }
                }
            }
        }

        modals::SchoolSetupWizardModal {
            open: *show_school_setup.read(),
            onsubmit: handle_school_setup,
        }
        }
    }
}


