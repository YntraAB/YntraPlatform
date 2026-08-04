use super::SchoolViewProps;
use super::utils::AdvancedAttachment;
use dioxus::prelude::*;
use yntra_core::{Course, StudentProfile, Submission};

mod classroom;
mod classwork;
mod course_grid;
mod grades;
mod modals;
mod stream;
mod submissions;

pub use classroom::ClassroomContainer;
pub use classwork::CourseClassworkTab;
pub use course_grid::CourseGrid;
pub use grades::CourseGradesTab;
pub use modals::{CreateAssignmentModal, CreateCourseModal, HomeworkGradeModal, TermGradeModal};
pub use stream::CourseStreamTab;
pub use submissions::CourseSubmissionsTab;

#[component]
pub fn TeacherPortal(
    school_props: SchoolViewProps,
    courses: Vec<Course>,
    students: Vec<StudentProfile>,
    mut selected_course_id: Signal<String>,
    mut show_course_modal: Signal<bool>,
    can_manage_schedule: bool,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let db_trigger = state.trigger_school_academics;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();

    // Local states
    let active_menu_id = use_signal(|| "".to_string());

    let show_assignment_modal = use_signal(|| false);
    let assignment_to_delete = use_signal(|| Option::<String>::None);

    let sub_tab = use_signal(|| "stream".to_string());
    let new_announcement_text = use_signal(String::new);
    let comment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let show_grade_modal = use_signal(|| false);
    let grade_student_id = use_signal(String::new);
    let grade_term = use_signal(|| "Fall 2026".to_string());
    let grade_letter = use_signal(|| "A".to_string());
    let grade_points = use_signal(|| 90);
    let grade_comments = use_signal(String::new);

    let show_homework_grade_modal = use_signal(|| false);
    let selected_homework_sub = use_signal(|| Option::<Submission>::None);
    let homework_grade = use_signal(String::new);
    let homework_feedback = use_signal(String::new);

    // Resources
    let user_id_clone_users = user_id.clone();
    let users_res = use_resource(move || {
        let _trig = state.trigger_users.read();
        let uid = user_id_clone_users.clone();
        async move { yntra_core::get_users(uid).await.unwrap_or_default() }
    });

    let teachers = users_res
        .read()
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|u| u.role == "teacher")
        .collect::<Vec<_>>();

    let teacher_suggestions = teachers
        .iter()
        .map(|u| u.full_name.clone().unwrap_or_else(|| u.email.clone()))
        .collect::<Vec<String>>();

    let active_course = selected_course_id.read().clone();
    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let assignments_res = use_resource(move || {
        let _trig = db_trigger.read();
        let c_id = active_course.clone();
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move {
            if c_id.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_assignments(uid, ws, c_id)
                    .await
                    .unwrap_or_default()
            }
        }
    });

    let active_course_g = selected_course_id.read().clone();
    let user_id_clone3 = user_id.clone();
    let ws_id_clone3 = ws_id.clone();
    let course_grades_res = use_resource(move || {
        let _trig = db_trigger.read();
        let c_id = active_course_g.clone();
        let uid = user_id_clone3.clone();
        let ws = ws_id_clone3.clone();
        async move {
            if c_id.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_course_term_grades(uid, ws, c_id)
                    .await
                    .unwrap_or_default()
            }
        }
    });

    let students_c = students.clone();
    let user_id_clone_sub = user_id.clone();
    let ws_id_clone_sub = ws_id.clone();
    let all_submissions_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = user_id_clone_sub.clone();
        let ws = ws_id_clone_sub.clone();
        let students_list = students_c.clone();
        async move {
            let mut list = Vec::new();
            for s in students_list {
                if let Ok(mut subs) =
                    yntra_core::get_student_submissions(uid.clone(), ws.clone(), s.id.clone()).await
                {
                    list.append(&mut subs);
                }
            }
            list
        }
    });

    let all_course_assignments = assignments_res.read().clone().unwrap_or_default();
    let announcements = all_course_assignments
        .iter()
        .filter(|a| a.max_points == -1)
        .cloned()
        .collect::<Vec<_>>();
    let comments = all_course_assignments
        .iter()
        .filter(|a| a.max_points == -2)
        .cloned()
        .collect::<Vec<_>>();
    let assignments = all_course_assignments
        .iter()
        .filter(|a| a.max_points >= 0)
        .cloned()
        .collect::<Vec<_>>();
    let course_grades = course_grades_res.read().clone().unwrap_or_default();
    let all_submissions = all_submissions_res.read().clone().unwrap_or_default();
    let users_binding = users_res.read().clone().unwrap_or_default();
    let current_user = users_binding.iter().find(|u| u.id == user_id);
    let teacher_name = current_user
        .and_then(|u| u.full_name.clone())
        .unwrap_or_else(|| "Teacher".to_string());

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            if selected_course_id.read().is_empty() {
                CourseGrid {
                    courses: courses,
                    can_manage_schedule: can_manage_schedule,
                    active_menu_id: active_menu_id,
                    selected_course_id: selected_course_id,
                    db_trigger: db_trigger,
                    user_id: user_id.clone(),
                }
            } else {
                {
                    let selected_id = selected_course_id.read().clone();
                    if let Some(c) = courses.iter().find(|item| item.id == selected_id).cloned() {
                        let active_tab_view = match sub_tab.read().as_str() {
                            "stream" => rsx! {
                                CourseStreamTab {
                                    announcements: announcements,
                                    comments: comments,
                                    course_id: c.id.clone(),
                                    workspace_id: ws_id.clone(),
                                    teacher_name: teacher_name,
                                    active_user_id: user_id.clone(),
                                    db_trigger: db_trigger,
                                    new_announcement_text: new_announcement_text,
                                    comment_inputs: comment_inputs,
                                }
                            },
                            "syllabus" => rsx! {
                                CourseClassworkTab {
                                    assignments: assignments,
                                    show_assignment_modal: show_assignment_modal,
                                    assignment_to_delete: assignment_to_delete,
                                    active_user_id: user_id.clone(),
                                    db_trigger: db_trigger,
                                }
                            },
                            "grading" => rsx! {
                                CourseGradesTab {
                                    students: students.clone(),
                                    assignments: assignments,
                                    all_submissions: all_submissions.clone(),
                                    course_grades: course_grades,
                                    grade_student_id: grade_student_id,
                                    grade_term: grade_term,
                                    grade_letter: grade_letter,
                                    grade_points: grade_points,
                                    grade_comments: grade_comments,
                                    show_grade_modal: show_grade_modal,
                                    selected_homework_sub: selected_homework_sub,
                                    homework_grade: homework_grade,
                                    homework_feedback: homework_feedback,
                                    show_homework_grade_modal: show_homework_grade_modal,
                                }
                            },
                            _ => rsx! {
                                CourseSubmissionsTab {
                                    assignments: assignments,
                                    all_submissions: all_submissions,
                                    students: students.clone(),
                                    selected_homework_sub: selected_homework_sub,
                                    homework_grade: homework_grade,
                                    homework_feedback: homework_feedback,
                                    show_homework_grade_modal: show_homework_grade_modal,
                                }
                            },
                        };
                        rsx! {
                            ClassroomContainer {
                                course: c,
                                selected_course_id: selected_course_id,
                                sub_tab: sub_tab,
                                {active_tab_view}
                            }
                        }
                    } else {
                        rsx! { div {} }
                    }
                }
            }

            // Modals
            if *show_course_modal.read() {
                CreateCourseModal {
                    show_course_modal: show_course_modal,
                    user_id: user_id.clone(),
                    ws_id: ws_id.clone(),
                    locale: locale.clone(),
                    db_trigger: db_trigger,
                    teacher_suggestions: teacher_suggestions,
                }
            }

            if *show_assignment_modal.read() {
                CreateAssignmentModal {
                    show_assignment_modal: show_assignment_modal,
                    user_id: user_id.clone(),
                    ws_id: ws_id.clone(),
                    selected_course_id: selected_course_id.read().clone(),
                    db_trigger: db_trigger,
                }
            }

            if *show_grade_modal.read() {
                TermGradeModal {
                    show_grade_modal: show_grade_modal,
                    grade_student_id: grade_student_id,
                    grade_term: grade_term,
                    grade_letter: grade_letter,
                    grade_points: grade_points,
                    grade_comments: grade_comments,
                    user_id: user_id.clone(),
                    ws_id: ws_id.clone(),
                    selected_course_id: selected_course_id.read().clone(),
                    db_trigger: db_trigger,
                }
            }

            if *show_homework_grade_modal.read() {
                HomeworkGradeModal {
                    show_homework_grade_modal: show_homework_grade_modal,
                    selected_homework_sub: selected_homework_sub,
                    homework_grade: homework_grade,
                    homework_feedback: homework_feedback,
                    user_id: user_id,
                    ws_id: ws_id,
                    db_trigger: db_trigger,
                }
            }
        }
    }
}
