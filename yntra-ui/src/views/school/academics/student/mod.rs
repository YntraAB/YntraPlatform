use dioxus::prelude::*;
use crate::components::LucideIcon;
use crate::locales::t;
use super::SchoolViewProps;
use super::utils::{decrypt_field, decrypt_opt_field, AdvancedAttachment};
use yntra_core::{
    get_assignments, get_student_submissions, get_timetable_slots,
    get_student_attendance_records, get_library_lending_logs, StudentProfile
};

mod profile_selector;
mod stream;
mod dashboard;
mod classwork;

pub use profile_selector::ProfileSelector;
pub use stream::StudentStreamTab;
pub use dashboard::StudentDashboardTab;
pub use classwork::StudentClassworkTab;

#[component]
pub fn StudentPortal(
    school_props: SchoolViewProps,
    students: Vec<StudentProfile>,
    mut selected_student_profile_id: Signal<String>,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let db_trigger_academics = state.trigger_school_academics;
    let db_trigger_attendance = state.trigger_school_attendance;
    let db_trigger_library = state.trigger_school_library;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();

    // Local states specific to student submissions and files
    let assignment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let submission_files = use_signal(std::collections::HashMap::<String, AdvancedAttachment>::new);
    let drag_active = use_signal(std::collections::HashMap::<String, bool>::new);
    let homework_filter = use_signal(|| "todo".to_string());
    let mut active_tab = use_signal(|| "stream".to_string());
    let new_announcement_text = use_signal(String::new);
    let comment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let editing_item_id = use_signal(|| Option::<String>::None);
    let edit_text = use_signal(String::new);
    let submitting_map = use_signal(std::collections::HashSet::<String>::new);

    let user_id_clone_att = user_id.clone();
    let ws_id_clone_att = ws_id.clone();
    let student_attendance_res = use_resource(move || {
        let _trig = db_trigger_attendance.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_att.clone();
        let ws = ws_id_clone_att.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_attendance_records(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Dynamic data fetching resource for student submissions
    let user_id_clone6 = user_id.clone();
    let ws_id_clone6 = ws_id.clone();
    let submissions_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone6.clone();
        let ws = ws_id_clone6.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_submissions(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Timetable slots resource
    let user_id_clone7 = user_id.clone();
    let ws_id_clone7 = ws_id.clone();
    let timetable_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone7.clone();
        let ws = ws_id_clone7.clone();
        async move {
            get_timetable_slots(uid, ws).await.unwrap_or_default()
        }
    });

    // Courses resource to look up course names in the timetable
    let user_id_clone_courses = user_id.clone();
    let ws_id_clone_courses = ws_id.clone();
    let courses_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone_courses.clone();
        let ws = ws_id_clone_courses.clone();
        async move { yntra_core::get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

    // Helper resource to load all workspace assignments for the student dashboard preview
    let courses = courses_res.read().clone().unwrap_or_default();
    let courses_clone = courses.clone();
    let user_id_clone5 = user_id.clone();
    let ws_id_clone5 = ws_id.clone();
    let all_assignments_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone5.clone();
        let ws = ws_id_clone5.clone();
        let courses_list = courses_clone.clone();
        async move {
            let mut list = Vec::new();
            for c in courses_list {
                if let Ok(mut assign) = get_assignments(uid.clone(), ws.clone(), c.id.clone()).await {
                    list.append(&mut assign);
                }
            }
            list
        }
    });

    // Helper resource to load library logs to dynamically compute the "Avid Reader" badge
    let user_id_clone_lib = user_id.clone();
    let ws_id_clone_lib = ws_id.clone();
    let library_logs_res = use_resource(move || {
        let _trig = db_trigger_library.read();
        let uid = user_id_clone_lib.clone();
        let ws = ws_id_clone_lib.clone();
        async move { get_library_lending_logs(uid, ws).await.unwrap_or_default() }
    });

    if students.is_empty() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                LucideIcon { name: "users", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                h4 { class: "text-sm font-bold text-foreground m-0", {t("school-student-no-students", &locale)} }
                p { class: "text-xs text-muted-foreground mt-1 max-w-sm", {t("school-no-enrolled-students", &locale)} }
            }
        };
    }

    let current_student_id = selected_student_profile_id.read().clone();
    let current_student = students.iter().find(|s| s.id == current_student_id).cloned().unwrap_or_else(|| students[0].clone());
    let student_name = format!("{} {}", current_student.first_name, current_student.last_name);

    let seed = state.get_passkey_seed();
    let all_assignments = all_assignments_res.read().clone().unwrap_or_default();
    let student_submissions = submissions_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut s| {
            s.content = decrypt_field(&seed, &s.content);
            s.grade = decrypt_opt_field(&seed, s.grade);
            s.feedback = decrypt_opt_field(&seed, s.feedback);
            s
        })
        .collect::<Vec<_>>();

    let announcements = all_assignments.iter().filter(|a| a.max_points == -1).cloned().collect::<Vec<_>>();
    let comments = all_assignments.iter().filter(|a| a.max_points == -2).cloned().collect::<Vec<_>>();

    let homework_filter_val = homework_filter.read().clone();
    let filtered_assignments = all_assignments.iter().filter(|a| {
        if a.max_points < 0 {
            return false;
        }
        let has_sub = student_submissions.iter().any(|sub| sub.assignment_id == a.id);
        if homework_filter_val == "todo" {
            a.max_points > 0 && !has_sub
        } else if homework_filter_val == "done" {
            has_sub
        } else if homework_filter_val == "materials" {
            a.max_points == 0
        } else {
            true
        }
    }).cloned().collect::<Vec<_>>();

    let timetable = timetable_res.read().clone().unwrap_or_default();
    let attendance = student_attendance_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut a| {
            a.notes = decrypt_opt_field(&seed, a.notes);
            a
        })
        .collect::<Vec<_>>();
    let library_logs = library_logs_res.read().clone().unwrap_or_default();

    let enrolled_course_ids: std::collections::HashSet<String> = {
        let mut ids = std::collections::HashSet::new();
        for att in attendance.iter() {
            ids.insert(att.course_id.clone());
        }
        for sub in student_submissions.iter() {
            if let Some(assign) = all_assignments.iter().find(|a| a.id == sub.assignment_id) {
                ids.insert(assign.course_id.clone());
            }
        }
        ids
    };

    let filtered_timetable: Vec<_> = timetable.iter()
        .filter(|s| enrolled_course_ids.contains(&s.course_id))
        .cloned()
        .collect();

    // Stats calculations
    let total_days = attendance.len();
    let present_days = attendance.iter().filter(|a| a.status == "present" || a.status == "late").count();
    let attendance_rate = if total_days > 0 {
        (present_days as f32 / total_days as f32) * 100.0
    } else {
        100.0
    };

    // SVG circle math
    let stroke_dasharray = 188.49; // 2 * PI * 30
    let stroke_dashoffset = stroke_dasharray - (attendance_rate as f64 / 100.0) * stroke_dasharray;

    let is_active_scholar = total_days > 0 && attendance_rate >= 95.0;
    let is_avid_reader = library_logs.iter().any(|log| {
        log.student_name == student_name && (log.status == "borrowed" || log.status == "returned")
    });

    let graded_submissions_count = student_submissions.iter()
        .filter(|sub| {
            if let Some(ref g) = sub.grade {
                g == "A" || g == "B" || g == "C" || g == "B+" || g == "A+"
            } else {
                false
            }
        })
        .count();
    let stars_count = std::cmp::max(1, graded_submissions_count);

    let active_tab_val = active_tab.read().clone();
    let user_role = state.active_user_role.read().clone();

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            ProfileSelector {
                selected_student_profile_id: selected_student_profile_id,
                students: students.clone(),
                locale: locale.clone(),
                role: user_role.clone(),
            }

            // Horizontal Google Classroom-style tab selector for students
            div { class: "flex border-b border-border pb-2.5 gap-6 text-xs font-extrabold tracking-wider",
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if active_tab_val == "stream" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("stream".to_string()),
                    "Stream"
                }
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if active_tab_val == "dashboard" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("dashboard".to_string()),
                    "Dashboard"
                }
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if active_tab_val == "classwork" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("classwork".to_string()),
                    "Classwork"
                }
            }

            match active_tab_val.as_str() {
                "stream" => rsx! {
                    StudentStreamTab {
                        announcements: announcements,
                        comments: comments,
                        courses: courses,
                        student_name: student_name,
                        current_student: current_student,
                        workspace_id: ws_id,
                        active_user_id: user_id,
                        db_trigger: db_trigger_academics,
                        new_announcement_text: new_announcement_text,
                        comment_inputs: comment_inputs,
                        editing_item_id: editing_item_id,
                        edit_text: edit_text,
                        locale: locale,
                    }
                },
                "dashboard" => rsx! {
                    StudentDashboardTab {
                        student_name: student_name,
                        locale: locale,
                        stars_count: stars_count,
                        is_active_scholar: is_active_scholar,
                        is_avid_reader: is_avid_reader,
                        attendance_rate: attendance_rate,
                        stroke_dasharray: stroke_dasharray,
                        stroke_dashoffset: stroke_dashoffset,
                        filtered_timetable: filtered_timetable,
                        courses: courses,
                        students: students,
                        selected_student_profile_id: current_student_id,
                    }
                },
                _ => rsx! {
                    StudentClassworkTab {
                        homework_filter: homework_filter,
                        filtered_assignments: filtered_assignments,
                        student_submissions: student_submissions,
                        submitting_map: submitting_map,
                        drag_active: drag_active,
                        submission_files: submission_files,
                        assignment_inputs: assignment_inputs,
                        locale: locale,
                        active_user_id: user_id,
                        workspace_id: ws_id,
                        student_profile_id: current_student_id,
                        db_trigger: db_trigger_academics,
                    }
                }
            }
        }
    }
}
