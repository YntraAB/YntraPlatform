use dioxus::prelude::*;
use crate::components;
use crate::state::AppState;
use yntra_core::{Course, StudentProfile, Assignment, Submission};

mod list;
mod stream;
mod classwork;
mod people;
mod gradebook;
mod settings;

pub use list::CourseList;
pub use stream::CourseStream;
pub use classwork::CourseClasswork;
pub use people::CoursePeople;
pub use gradebook::CourseGradebook;
pub use settings::CourseSettings;

pub fn get_subject_theme(subj: &str) -> (&'static str, &'static str, &'static str) {
    match subj.to_lowercase().as_str() {
        "math" | "mathematics" => ("from-indigo-600 to-violet-500", "bg-indigo-500/10 text-indigo-400", "calculator"),
        "science" | "chemistry" | "physics" | "biology" => ("from-emerald-600 to-teal-500", "bg-emerald-500/10 text-emerald-400", "atom"),
        "english" | "literature" | "languages" | "swedish" => ("from-amber-500 to-orange-500", "bg-amber-500/10 text-amber-400", "book-open"),
        "history" | "social studies" => ("from-rose-600 to-red-500", "bg-rose-500/10 text-rose-400", "globe"),
        _ => ("from-blue-600 to-cyan-500", "bg-blue-500/10 text-blue-400", "graduation-cap")
    }
}

#[derive(Props, Clone)]
pub struct CoursesGradingProps {
    pub courses: Vec<Course>,
    pub students: Vec<StudentProfile>,
    pub selected_course_id: Signal<Option<String>>,
    pub selected_assignment_id: Signal<Option<String>>,
    pub assignments: Vec<Assignment>,
    pub submissions: Vec<Submission>,
    pub show_assignment_modal: Signal<bool>,
    pub show_grading_modal: Signal<bool>,
    pub selected_submission_id: Signal<Option<String>>,
    pub grade_input: Signal<String>,
    pub feedback_input: Signal<String>,
    pub active_user_id: String,
    pub active_user_role: String,
    pub workspace_id: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for CoursesGradingProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CoursesGrading(props: CoursesGradingProps) -> Element {
    let courses = props.courses.clone();
    let students = props.students.clone();
    let mut selected_course_id = props.selected_course_id;
    let mut selected_assignment_id = props.selected_assignment_id;
    let assignments = props.assignments.clone();
    let submissions = props.submissions.clone();
    let show_assignment_modal = props.show_assignment_modal;
    let show_grading_modal = props.show_grading_modal;
    let selected_submission_id = props.selected_submission_id;
    let grade_input = props.grade_input;
    let feedback_input = props.feedback_input;
    let active_user_id = props.active_user_id.clone();
    let active_user_role = props.active_user_role.clone();
    let db_trigger = props.db_trigger;

    let search_query = use_signal(String::new);
    let selected_subject = use_signal(|| "all".to_string());
    
    // Sub-navigation tab inside selected course (stream, classwork, people, gradebook, settings)
    let mut course_tab = use_signal(|| "stream".to_string());
    
    // Local state for class announcements
    let announcements = use_signal(|| vec![
        ("Ms. Andersson".to_string(), "Welcome to the new school term! Please check the Classwork tab for your first assignment.".to_string(), "2 hours ago".to_string()),
        ("Ms. Andersson".to_string(), "Reminder: Midterm review session will take place this Thursday in Room 304.".to_string(), "Yesterday".to_string()),
    ]);
    let new_announcement = use_signal(String::new);

    // Context for block settings
    let state = use_context::<AppState>();
    let workspace = state.workspace.read().clone();

    // Helper calculated variables
    let selected_course = selected_course_id.read().clone().and_then(|id| {
        courses.iter().find(|c| c.id == id).cloned()
    });

    let selected_assignment = selected_assignment_id.read().clone().and_then(|id| {
        assignments.iter().find(|a| a.id == id).cloned()
    });

    let filtered_courses: Vec<Course> = courses.iter().filter(|c| {
        let matches_search = c.name.to_lowercase().contains(&search_query.read().to_lowercase())
            || c.subject.to_lowercase().contains(&search_query.read().to_lowercase());
        let matches_subject = if *selected_subject.read() == "all" {
            true
        } else {
            let sub_low = c.subject.to_lowercase();
            if *selected_subject.read() == "mathematics" {
                sub_low.contains("math") || sub_low.contains("algebra")
            } else if *selected_subject.read() == "science" {
                sub_low.contains("science") || sub_low.contains("physics") || sub_low.contains("chemistry") || sub_low.contains("biology")
            } else if *selected_subject.read() == "languages" {
                sub_low.contains("english") || sub_low.contains("literature") || sub_low.contains("swedish") || sub_low.contains("languages")
            } else if *selected_subject.read() == "history" {
                sub_low.contains("history") || sub_low.contains("social")
            } else {
                sub_low.contains(selected_subject.read().as_str())
            }
        };
        matches_search && matches_subject
    }).cloned().collect();

    let course_theme = selected_course.as_ref().map(|c| get_subject_theme(&c.subject));

    // Teacher course setting overrides local signals
    let current_course_id = selected_course.as_ref().map(|c| c.id.clone()).unwrap_or_default();
    let mut last_course_id = use_signal(String::new);
    let mut edit_name = use_signal(String::new);
    let mut edit_subject = use_signal(String::new);
    let mut edit_classroom = use_signal(String::new);
    let mut edit_grading = use_signal(|| "default".to_string());
    let mut edit_late = use_signal(|| "default".to_string());
    let mut edit_stream = use_signal(|| "anyone".to_string());

    let (current_grading, current_late, current_stream) = {
        let c_id = current_course_id.clone();
        if let Some(ref ws) = workspace {
            let block_settings: serde_json::Value = serde_json::from_str(&ws.block_settings).unwrap_or_default();
            let c_settings = block_settings.get("course_settings").and_then(|c| c.get(&c_id));
            let g = c_settings.and_then(|cs| cs.get("grading_system")).and_then(|v| v.as_str()).unwrap_or("default").to_string();
            let l = c_settings.and_then(|cs| cs.get("late_policy")).and_then(|v| v.as_str()).unwrap_or("default").to_string();
            let s = c_settings.and_then(|cs| cs.get("stream_post")).and_then(|v| v.as_str()).unwrap_or("anyone").to_string();
            (g, l, s)
        } else {
            ("default".to_string(), "default".to_string(), "anyone".to_string())
        }
    };

    if current_course_id != *last_course_id.read() {
        last_course_id.set(current_course_id.clone());
        if let Some(ref c) = selected_course {
            edit_name.set(c.name.clone());
            edit_subject.set(c.subject.clone());
            edit_classroom.set(c.classroom.clone().unwrap_or_default());
            edit_grading.set(current_grading.clone());
            edit_late.set(current_late.clone());
            edit_stream.set(current_stream.clone());
        }
    }

    let (ws_default_grading, ws_default_late) = {
        if let Some(ref ws) = workspace {
            let settings_val: serde_json::Value = serde_json::from_str(&ws.settings).unwrap_or_default();
            let g = settings_val.get("grading_system").and_then(|v| v.as_str()).unwrap_or("A-F").to_string();
            let l = settings_val.get("late_policy").and_then(|v| v.as_str()).unwrap_or("none").to_string();
            (g, l)
        } else {
            ("A-F".to_string(), "none".to_string())
        }
    };
    
    let ws_default_grading_label = match ws_default_grading.as_str() {
        "A-F" => "A-F",
        "1-10" => "1-10",
        "1-100" => "0-100",
        "U-G-VG" => "U, G, VG",
        "U-G" => "U, G",
        "U-3-4-5" => "U, 3, 4, 5",
        _ => "A-F"
    }.to_string();

    let ws_default_late_label = match ws_default_late.as_str() {
        "none" => "None",
        "hard_deadline" => "Hard Deadline",
        "penalty_5" => "5% Daily Penalty",
        "penalty_10" => "10% Daily Penalty",
        _ => "None"
    }.to_string();

    let is_teacher_or_admin = active_user_role == "admin" || active_user_role == "platform_admin" || active_user_role == "teacher";
    
    let is_authorized_to_post = is_teacher_or_admin || current_stream == "anyone";

    let save_course_settings = {
        let ws_opt = workspace.clone();
        let db_trigger = db_trigger;
        let active_user_id = active_user_id.clone();
        move |course_id: String, name: String, subject: String, classroom: Option<String>, grading: String, late: String, stream: String| {
            let active_user_id = active_user_id.clone();
            let ws_opt = ws_opt.clone();
            let mut db_trigger = db_trigger;
            spawn(async move {
                let _ = yntra_core::update_course(active_user_id.clone(), course_id.clone(), name, subject, None, classroom).await;
                
                if let Some(ref ws) = ws_opt {
                    let mut block_settings: serde_json::Value = serde_json::from_str(&ws.block_settings).unwrap_or_default();
                    if block_settings.get("course_settings").is_none() {
                        block_settings["course_settings"] = serde_json::json!({});
                    }
                    block_settings["course_settings"][&course_id] = serde_json::json!({
                        "grading_system": grading,
                        "late_policy": late,
                        "stream_post": stream
                    });
                    
                    let block_str = serde_json::to_string(&block_settings).unwrap_or_default();
                    let requester_uid = active_user_id.clone();
                    let _ = yntra_core::update_workspace_block_settings(requester_uid, ws.id.clone(), block_str).await;
                }
                
                let current = *db_trigger.read();
                db_trigger.set(current + 1);
            });
        }
    };

    rsx! {
        if let Some(ref course) = selected_course {
            {
                let (gradient, _badge_style, icon) = course_theme.unwrap();
                rsx! {
                    div { class: "flex flex-col gap-6",
                        // Premium Course Banner
                        div { class: format!("p-6 rounded-2xl bg-gradient-to-r {} relative text-white flex flex-col justify-between h-36 overflow-hidden shadow-lg", gradient),
                            div { class: "absolute right-6 -bottom-6 text-white/10 select-none pointer-events-none scale-[2.0] rotate-12",
                                components::LucideIcon { name: icon, size: "100" }
                            }
                            div { class: "flex justify-between items-start z-10",
                                button {
                                    class: "yntra-btn text-xs font-bold py-1.5 px-3 bg-white/20 hover:bg-white/30 border border-white/10 text-white flex items-center gap-1.5",
                                    onclick: move |_| {
                                        selected_course_id.set(None);
                                        selected_assignment_id.set(None);
                                    },
                                    components::LucideIcon { name: "arrow-left", size: "14" }
                                    "All Courses"
                                }
                                span { class: "text-xs font-black uppercase tracking-wider px-2.5 py-1 bg-white/20 backdrop-blur-md rounded-lg border border-white/10", "{course.subject}" }
                            }
                            div { class: "z-10 flex flex-col gap-1",
                                h2 { class: "text-2xl font-black m-0 tracking-tight drop-shadow-sm text-white", "{course.name}" }
                                p { class: "text-xs opacity-90 m-0", "Classroom: {course.classroom.clone().unwrap_or_else(|| \"N/A\".to_string())} • Instructor: Ms. Andersson" }
                            }
                        }
                        
                        // Course Sub-tab Navigation
                        div { class: "flex border-b border-border/40 pb-px gap-6 text-sm font-bold text-muted-foreground",
                            for (slug, label) in &[("stream", "Stream"), ("classwork", "Classwork"), ("people", "People"), ("gradebook", "Gradebook")] {
                                button {
                                    class: format!("pb-3 px-1 transition-all border-b-2 hover:text-foreground bg-transparent {}",
                                        if *course_tab.read() == *slug { "border-primary text-foreground" } else { "border-transparent" }
                                    ),
                                    onclick: move |_| course_tab.set(slug.to_string()),
                                    "{label}"
                                }
                            }
                            if is_teacher_or_admin {
                                button {
                                    class: format!("pb-3 px-1 transition-all border-b-2 hover:text-foreground bg-transparent {}",
                                        if *course_tab.read() == "settings" { "border-primary text-foreground" } else { "border-transparent" }
                                    ),
                                    onclick: move |_| course_tab.set("settings".to_string()),
                                    "Settings"
                                }
                            }
                        }

                        {
                            match course_tab.read().as_str() {
                                "stream" => {
                                    rsx! {
                                        CourseStream {
                                            assignments: assignments.clone(),
                                            course_tab: course_tab,
                                            announcements: announcements,
                                            new_announcement: new_announcement,
                                            is_authorized_to_post: is_authorized_to_post,
                                        }
                                    }
                                }
                                "people" => {
                                    rsx! {
                                        CoursePeople {
                                            students: students.clone(),
                                        }
                                    }
                                }
                                "gradebook" => {
                                    rsx! {
                                        CourseGradebook {
                                            students: students.clone(),
                                            assignments: assignments.clone(),
                                            submissions: submissions.clone(),
                                        }
                                    }
                                }
                                "settings" => {
                                    rsx! {
                                        CourseSettings {
                                            course_id: course.id.clone(),
                                            edit_name: edit_name,
                                            edit_subject: edit_subject,
                                            edit_classroom: edit_classroom,
                                            edit_grading: edit_grading,
                                            edit_late: edit_late,
                                            edit_stream: edit_stream,
                                            ws_default_grading_label: ws_default_grading_label.clone(),
                                            ws_default_late_label: ws_default_late_label.clone(),
                                            save_course_settings: Callback::new({
                                                let save_course_settings = save_course_settings.clone();
                                                move |(c_id, name, subject, classroom, grading, late, stream)| {
                                                    save_course_settings(c_id, name, subject, classroom, grading, late, stream);
                                                }
                                            }),
                                        }
                                    }
                                }
                                _ => {
                                    rsx! {
                                        CourseClasswork {
                                            course: course.clone(),
                                            assignments: assignments.clone(),
                                            submissions: submissions.clone(),
                                            students: students.clone(),
                                            selected_assignment: selected_assignment.clone(),
                                            selected_assignment_id: selected_assignment_id,
                                            selected_submission_id: selected_submission_id,
                                            grade_input: grade_input,
                                            feedback_input: feedback_input,
                                            show_assignment_modal: show_assignment_modal,
                                            show_grading_modal: show_grading_modal,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            CourseList {
                filtered_courses: filtered_courses,
                selected_course_id: selected_course_id,
                selected_assignment_id: selected_assignment_id,
                course_tab: course_tab,
                search_query: search_query,
                selected_subject: selected_subject,
            }
        }
    }
}
