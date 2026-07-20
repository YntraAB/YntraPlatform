use dioxus::prelude::*;
use crate::components::LucideIcon;
use crate::locales::t;
use super::SchoolViewProps;

pub mod utils;
pub mod student;
pub mod parent;
pub mod teacher;

pub use utils::{decrypt_field, decrypt_opt_field, AdvancedAttachment};
pub use student::StudentPortal;
pub use parent::ParentPortal;
pub use teacher::TeacherPortal;
pub use super::infer_subject_from_course;

#[component]
pub fn AcademicsView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let trigger_school_academics = state.trigger_school_academics;
    let trigger_school_directory = state.trigger_school_directory;
    let locale = props.locale.clone();
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    // View mode switcher: "teacher", "student", or "parent"
    let mut view_mode = use_signal(|| {
        let role = state.active_user_role.read().clone();
        let r = role.as_str();
        if r == "student" || r == "role-school-student" {
            "student".to_string()
        } else if r == "parent" || r == "role-school-parent" {
            "parent".to_string()
        } else {
            "teacher".to_string()
        }
    });

    let mut selected_student_profile_id = use_signal(|| "".to_string());
    let selected_course_id = use_signal(|| "".to_string());
    let mut show_course_modal = use_signal(|| false);
    let mut show_more_menu = use_signal(|| false);

    // Resources
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let courses_res = use_resource(move || {
        let _trig = trigger_school_academics.read();
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { yntra_core::get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone4 = user_id.clone();
    let ws_id_clone4 = ws_id.clone();
    let students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone4.clone();
        let ws = ws_id_clone4.clone();
        async move { yntra_core::get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone_s = user_id.clone();
    let can_manage_schedule_res = use_resource(move || {
        let uid = user_id_clone_s.clone();
        async move {
            yntra_core::check_school_permission(uid, "can_manage_schedule".to_string())
                .await
                .unwrap_or(false)
        }
    });

    let user_id_clone_p = user_id.clone();
    let ws_id_clone_p = ws_id.clone();
    let parent_students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone_p.clone();
        let ws = ws_id_clone_p.clone();
        async move {
            yntra_core::get_parent_students(uid.clone(), ws, uid).await.unwrap_or_default()
        }
    });

    use_effect(move || {
        let role = state.active_user_role.read().clone();
        if role == "student" || role == "role-school-student" {
            let uid = state.active_user_id.read().clone();
            let student_list = students_res.read().clone().unwrap_or_default();
            if let Some(profile) = student_list.iter().find(|s| s.user_id.as_ref() == Some(&uid)) {
                if selected_student_profile_id.read().as_str() != profile.id.as_str() {
                    selected_student_profile_id.set(profile.id.clone());
                }
            } else if !student_list.is_empty() {
                if selected_student_profile_id.read().is_empty() {
                    selected_student_profile_id.set(student_list[0].id.clone());
                }
            }
        } else if role == "parent" || role == "role-school-parent" {
            let student_list = parent_students_res.read().clone().unwrap_or_default();
            if !student_list.is_empty() && selected_student_profile_id.read().is_empty() {
                selected_student_profile_id.set(student_list[0].id.clone());
            }
        }
    });

    let courses = courses_res.read().clone().unwrap_or_default();
    let can_manage_schedule = can_manage_schedule_res.read().cloned().unwrap_or(false);
    let active_role = state.active_user_role.read().clone();
    let seed = state.get_passkey_seed();
    let students = {
        let raw = if active_role == "parent" || active_role == "role-school-parent" {
            parent_students_res.read().clone().unwrap_or_default()
        } else {
            students_res.read().clone().unwrap_or_default()
        };
        raw.into_iter()
            .map(|mut s| {
                s.first_name = decrypt_field(&seed, &s.first_name);
                s.last_name = decrypt_field(&seed, &s.last_name);
                s.grade_level = decrypt_field(&seed, &s.grade_level);
                s.parent_contact = decrypt_opt_field(&seed, s.parent_contact);
                s
            })
            .collect::<Vec<_>>()
    };

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            
            // Preview View Switcher
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "academics", class: "h-6 w-6 text-primary" }
                        if *view_mode.read() == "teacher" {
                            match locale.as_str() {
                                "sv" => "Kurser & Betygssättning",
                                _ => "Courses & Academics"
                            }
                        } else if *view_mode.read() == "parent" {
                            match locale.as_str() {
                                "sv" => "Föräldraportal & Studieresultat",
                                _ => "Parent Portal & Child Progress"
                            }
                        } else {
                            {t("school-student-portal-title", &locale)}
                        }
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", 
                        if *view_mode.read() == "teacher" {
                            match locale.as_str() {
                                "sv" => "Hantera akademiska kurser, schemaläggning och läxuppgifter.",
                                _ => "Manage academic courses, assignments, and class scheduling."
                            }
                        } else if *view_mode.read() == "parent" {
                            match locale.as_str() {
                                "sv" => "Följ dina barns närvaro, betyg, schema och hälsoincidenter.",
                                _ => "Track your children's attendance, report cards, schedules, and health records."
                            }
                        } else {
                            {t("school-student-portal-desc", &locale)}
                        }
                    }
                }
                
                div { class: "flex items-center gap-4",
                    // Tab toggle
                    if state.active_user_role.read().as_str() != "student" && state.active_user_role.read().as_str() != "parent" {
                        div { class: "flex bg-muted p-1 rounded-xl border border-border/40 text-xs font-bold w-max",
                            button {
                                class: format!(
                                    "px-3 py-1.5 rounded-lg transition-all {}",
                                    if *view_mode.read() == "teacher" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                                ),
                                onclick: move |_| view_mode.set("teacher".to_string()),
                                {t("school-teacher-view-tab", &locale)}
                            }
                            {
                                let students_toggle = students.clone();
                                rsx! {
                                    button {
                                        class: format!(
                                            "px-3 py-1.5 rounded-lg transition-all {}",
                                            if *view_mode.read() == "student" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                                        ),
                                        onclick: move |_| {
                                            view_mode.set("student".to_string());
                                            if selected_student_profile_id.read().is_empty() && !students_toggle.is_empty() {
                                                selected_student_profile_id.set(students_toggle[0].id.clone());
                                            }
                                        },
                                        {t("school-student-portal-tab", &locale)}
                                    }
                                }
                            }
                        }
                    }

                    if *view_mode.read() == "teacher" && can_manage_schedule {
                        div { class: "relative",
                            button {
                                class: "flex items-center justify-center h-9 w-9 p-0 rounded-xl bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground border border-border/40 transition-all duration-150 cursor-pointer",
                                onclick: move |_| {
                                    let current = *show_more_menu.read();
                                    show_more_menu.set(!current);
                                },
                                LucideIcon { name: "more-vertical", size: "16" }
                            }
                            if *show_more_menu.read() {
                                div { class: "absolute right-0 top-11 z-50 bg-popover text-popover-foreground border border-border rounded-xl shadow-lg p-1 min-w-[150px] animate-in fade-in slide-in-from-top-2 duration-150",
                                    button {
                                        class: "w-full text-left px-3 py-2 rounded-lg text-xs font-semibold hover:bg-muted transition-colors flex items-center gap-2",
                                        onclick: move |_| {
                                            show_more_menu.set(false);
                                            show_course_modal.set(true);
                                        },
                                        LucideIcon { name: "plus", size: "14" }
                                        match locale.as_str() {
                                            "sv" => "Skapa Kurs",
                                            _ => "Create Course"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            {
                if *view_mode.read() == "student" {
                    rsx! {
                        StudentPortal {
                            school_props: props.clone(),
                            students: students.clone(),
                            selected_student_profile_id: selected_student_profile_id,
                        }
                    }
                } else if *view_mode.read() == "parent" {
                    rsx! {
                        ParentPortal {
                            school_props: props.clone(),
                            students: students.clone(),
                            selected_student_profile_id: selected_student_profile_id,
                        }
                    }
                } else {
                    rsx! {
                        TeacherPortal {
                            school_props: props.clone(),
                            courses: courses.clone(),
                            students: students.clone(),
                            selected_course_id: selected_course_id,
                            show_course_modal: show_course_modal,
                            can_manage_schedule: can_manage_schedule,
                        }
                    }
                }
            }
        }
    }
}
