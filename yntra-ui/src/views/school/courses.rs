use dioxus::prelude::*;
use crate::components;
use crate::state::AppState;
use yntra_core::{Course, StudentProfile, Assignment, Submission};

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
    let mut show_assignment_modal = props.show_assignment_modal;
    let mut show_grading_modal = props.show_grading_modal;
    let mut selected_submission_id = props.selected_submission_id;
    let mut grade_input = props.grade_input;
    let mut feedback_input = props.feedback_input;
    let _active_user_id = props.active_user_id.clone();
    let active_user_role = props.active_user_role.clone();

    let mut search_query = use_signal(String::new);
    let mut selected_subject = use_signal(|| "all".to_string());
    
    // Sub-navigation tab inside selected course (stream, classwork, people, gradebook, settings)
    let mut course_tab = use_signal(|| "stream".to_string());
    
    // Local state for class announcements
    let mut announcements = use_signal(|| vec![
        ("Ms. Andersson".to_string(), "Welcome to the new school term! Please check the Classwork tab for your first assignment.".to_string(), "2 hours ago".to_string()),
        ("Ms. Andersson".to_string(), "Reminder: Midterm review session will take place this Thursday in Room 304.".to_string(), "Yesterday".to_string()),
    ]);
    let mut new_announcement = use_signal(String::new);

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

    let get_subject_theme = |subj: &str| -> (&str, &str, &str) {
        match subj.to_lowercase().as_str() {
            "math" | "mathematics" => ("from-indigo-600 to-violet-500", "bg-indigo-500/10 text-indigo-400", "calculator"),
            "science" | "chemistry" | "physics" | "biology" => ("from-emerald-600 to-teal-500", "bg-emerald-500/10 text-emerald-400", "atom"),
            "english" | "literature" | "languages" | "swedish" => ("from-amber-500 to-orange-500", "bg-amber-500/10 text-amber-400", "book-open"),
            "history" | "social studies" => ("from-rose-600 to-red-500", "bg-rose-500/10 text-rose-400", "globe"),
            _ => ("from-blue-600 to-cyan-500", "bg-blue-500/10 text-blue-400", "graduation-cap")
        }
    };

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
    };

    let ws_default_late_label = match ws_default_late.as_str() {
        "none" => "None",
        "hard_deadline" => "Hard Deadline",
        "penalty_5" => "5% Daily Penalty",
        "penalty_10" => "10% Daily Penalty",
        _ => "None"
    };

    let is_teacher_or_admin = active_user_role == "admin" || active_user_role == "platform_admin" || active_user_role == "teacher";
    
    let is_authorized_to_post = is_teacher_or_admin || current_stream == "anyone";

    let save_course_settings = {
        let ws_opt = workspace.clone();
        let mut db_trigger = props.db_trigger;
        move |course_id: String, name: String, subject: String, classroom: Option<String>, grading: String, late: String, stream: String| {
            spawn(async move {
                let _ = yntra_core::update_course(props.active_user_id.clone(), course_id.clone(), name, subject, None, classroom).await;
                
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
                    let _ = yntra_core::update_workspace_block_settings(ws.id.clone(), block_str).await;
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
                                    class: "px-3 py-1.5 rounded-lg bg-white/20 hover:bg-white/30 backdrop-blur-md text-xs font-bold transition-all flex items-center gap-1.5 border border-white/10 text-white",
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

                        if *course_tab.read() == "stream" {
                            div { class: "grid gap-6 md:grid-cols-4",
                                // Left sidebar: Upcoming assignments list
                                div { class: "md:col-span-1 flex flex-col gap-4",
                                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                        h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Upcoming Tasks" }
                                        if assignments.is_empty() {
                                            p { class: "text-xs text-muted-foreground m-0", "Woohoo, no work due soon!" }
                                        } else {
                                            div { class: "flex flex-col gap-2.5",
                                                for a in assignments.iter().take(2) {
                                                    div { class: "text-xs flex flex-col gap-0.5",
                                                        span { class: "font-semibold text-foreground truncate", "{a.title}" }
                                                        span { class: "text-[10px] text-muted-foreground", "Due: {a.due_date}" }
                                                    }
                                                }
                                                button {
                                                    class: "text-[10px] font-bold text-primary hover:underline p-0 bg-transparent text-left mt-1",
                                                    onclick: move |_| course_tab.set("classwork".to_string()),
                                                    "View all assignments"
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Right side: Announcements post and list
                                div { class: "md:col-span-3 flex flex-col gap-4",
                                    // Create announcement card (only visible if allowed)
                                    if is_authorized_to_post {
                                        components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                            div { class: "flex items-start gap-3",
                                                div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                                                textarea {
                                                    class: "yntra-input py-2.5 px-3 h-16 w-full text-xs text-foreground bg-background border border-border/40 rounded-lg placeholder-muted-foreground/60 resize-none",
                                                    placeholder: "Announce something to your class...",
                                                    value: "{new_announcement}",
                                                    oninput: move |e| new_announcement.set(e.value()),
                                                }
                                            }
                                            div { class: "flex justify-end",
                                                button {
                                                    class: "yntra-btn text-xs font-bold py-1.5 px-3",
                                                    disabled: new_announcement.read().trim().is_empty(),
                                                    onclick: move |_| {
                                                        let text = new_announcement.read().trim().to_string();
                                                        if !text.is_empty() {
                                                            let mut list = announcements.read().clone();
                                                            list.insert(0, ("Ms. Andersson".to_string(), text, "Just now".to_string()));
                                                            announcements.set(list);
                                                            new_announcement.set(String::new());
                                                        }
                                                    },
                                                    "Post Announcement"
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Announcements Feed list
                                    for (author, content, time) in announcements.read().iter() {
                                        components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                            div { class: "flex justify-between items-center",
                                                div { class: "flex items-center gap-3",
                                                    div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                                                    div {
                                                        div { class: "text-xs font-bold text-foreground", "{author}" }
                                                        div { class: "text-[10px] text-muted-foreground", "{time}" }
                                                    }
                                                }
                                            }
                                            p { class: "text-xs text-muted-foreground/90 m-0 leading-relaxed", "{content}" }
                                        }
                                    }
                                }
                            }
                        }
                        else if *course_tab.read() == "people" {
                            div { class: "grid gap-6 md:grid-cols-2",
                                components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                    h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Teachers" }
                                    div { class: "flex items-center gap-3",
                                        div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                                        div {
                                            div { class: "text-xs font-bold text-foreground", "Ms. Andersson" }
                                            div { class: "text-[10px] text-muted-foreground", "Primary Instructor" }
                                        }
                                    }
                                }
                                
                                components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                    h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Students ({students.len()})" }
                                    if students.is_empty() {
                                        p { class: "text-xs text-muted-foreground m-0", "No students enrolled in this workspace." }
                                    } else {
                                        div { class: "flex flex-col gap-3 max-h-80 overflow-y-auto pr-1",
                                            for s in students.iter() {
                                                div { class: "flex items-center gap-3",
                                                    div { class: "w-7 h-7 rounded-full bg-sidebar flex items-center justify-center font-bold text-muted-foreground text-xs border border-border/30",
                                                        "{s.first_name.chars().next().unwrap_or('S')}"
                                                    }
                                                    div {
                                                        div { class: "text-xs font-bold text-foreground", "{s.first_name} {s.last_name}" }
                                                        div { class: "text-[10px] text-muted-foreground", "{s.grade_level}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        else if *course_tab.read() == "gradebook" {
                            components::Card { class: "p-5 border border-border/40 bg-sidebar/20 flex flex-col gap-4 overflow-hidden",
                                h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0", "Class Gradebook" }
                                
                                if students.is_empty() || assignments.is_empty() {
                                    div { class: "text-center p-8 text-muted-foreground text-xs", "Enrolled students and assignments required to display Gradebook." }
                                } else {
                                    div { class: "overflow-x-auto w-full border border-border/40 rounded-xl",
                                        table { class: "w-full border-collapse text-xs text-left",
                                            thead { class: "bg-sidebar border-b border-border/40",
                                                tr {
                                                    th { class: "p-3 font-bold text-muted-foreground min-w-[150px] border-r border-border/40", "Student" }
                                                    for a in assignments.iter() {
                                                        th { class: "p-3 font-bold text-muted-foreground text-center min-w-[120px] border-r border-border/40",
                                                            div { class: "truncate max-w-[120px]", "{a.title}" }
                                                            div { class: "text-[9px] font-normal text-primary/70 mt-0.5", "{a.max_points} pts" }
                                                        }
                                                    }
                                                }
                                            }
                                            tbody {
                                                for s in students.iter() {
                                                    tr { class: "border-b border-border/40 hover:bg-white/[0.01]",
                                                        td { class: "p-3 font-bold text-foreground border-r border-border/40", "{s.first_name} {s.last_name}" }
                                                        for a in assignments.iter() {
                                                            {
                                                                let submission = submissions.iter().find(|sub| sub.assignment_id == a.id && sub.student_id == s.id);
                                                                rsx! {
                                                                    td { class: "p-3 text-center border-r border-border/40",
                                                                        if let Some(sub) = submission {
                                                                            if let Some(g) = &sub.grade {
                                                                                span { class: "font-extrabold text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20", "{g}" }
                                                                            } else {
                                                                                span { class: "text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20 text-[10px] font-bold", "Ungraded" }
                                                                            }
                                                                        } else {
                                                                            span { class: "text-muted-foreground opacity-40", "-" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        else if *course_tab.read() == "settings" {
                            components::Card { class: "p-5 border border-border/40 bg-sidebar/20 flex flex-col gap-6 max-w-xl shadow-md",
                                h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Course settings" }
                                
                                div { class: "flex flex-col gap-4 text-xs",
                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Course Name" }
                                        input {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                                            placeholder: "e.g. Advanced Chemistry",
                                            value: "{edit_name}",
                                            oninput: move |e| edit_name.set(e.value()),
                                        }
                                    }
                                    
                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Subject" }
                                        input {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                                            placeholder: "e.g. Science",
                                            value: "{edit_subject}",
                                            oninput: move |e| edit_subject.set(e.value()),
                                        }
                                    }
                                    
                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Classroom Location" }
                                        input {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                                            placeholder: "e.g. Room 304",
                                            value: "{edit_classroom}",
                                            oninput: move |e| edit_classroom.set(e.value()),
                                        }
                                    }

                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Grading Scale Override" }
                                        select {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                                            value: "{edit_grading}",
                                            onchange: move |e| edit_grading.set(e.value()),
                                            option { value: "default", "Use Workspace Default ({ws_default_grading_label})" }
                                            option { value: "A-F", "A-F (Letter Grades)" }
                                            option { value: "1-10", "1-10 (Numeric Scale)" }
                                            option { value: "1-100", "0-100 (Percentage Scale)" }
                                            option { value: "U-G-VG", "U, G, VG (Swedish University Scale)" }
                                            option { value: "U-G", "U, G (Swedish Pass/Fail)" }
                                            option { value: "U-3-4-5", "U, 3, 4, 5 (Swedish Engineering)" }
                                        }
                                    }

                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Late Submission Policy Override" }
                                        select {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                                            value: "{edit_late}",
                                            onchange: move |e| edit_late.set(e.value()),
                                            option { value: "default", "Use Workspace Default ({ws_default_late_label})" }
                                            option { value: "none", "None (No Penalties)" }
                                            option { value: "hard_deadline", "Hard Deadline (Block Late Submissions)" }
                                            option { value: "penalty_5", "5% Daily Deduction Penalty" }
                                            option { value: "penalty_10", "10% Daily Deduction Penalty" }
                                        }
                                    }

                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Stream Posting Permissions" }
                                        select {
                                            class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                                            value: "{edit_stream}",
                                            onchange: move |e| edit_stream.set(e.value()),
                                            option { value: "anyone", "Students and Teachers can post" }
                                            option { value: "teachers_only", "Only Teachers can post" }
                                        }
                                    }

                                    button {
                                        class: "yntra-btn text-xs font-bold py-2 px-4 self-end mt-2 flex items-center gap-1.5",
                                        onclick: {
                                            let save_course_settings = save_course_settings.clone();
                                            let course_id = course.id.clone();
                                            move |_| {
                                                let save_course_settings = save_course_settings.clone();
                                                save_course_settings(
                                                    course_id.clone(),
                                                    (*edit_name.read()).clone(),
                                                    (*edit_subject.read()).clone(),
                                                    if edit_classroom.read().trim().is_empty() { None } else { Some((*edit_classroom.read()).trim().to_string()) },
                                                    (*edit_grading.read()).clone(),
                                                    (*edit_late.read()).clone(),
                                                    (*edit_stream.read()).clone()
                                                );
                                            }
                                        },
                                        components::LucideIcon { name: "save", size: "14" }
                                        "Save Settings"
                                    }
                                }
                            }
                        }
                        else {
                            // Classwork / Assignments
                            div { class: "grid gap-6 md:grid-cols-3",
                                // Left sidebar: posting details & create button
                                div { class: "flex flex-col gap-4 md:col-span-1",
                                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                        h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Classroom Details" }
                                        div { class: "flex flex-col gap-2.5 text-sm",
                                            div {
                                                div { class: "text-xs text-muted-foreground", "Classroom" }
                                                div { class: "font-bold text-foreground", "{course.classroom.clone().unwrap_or_else(|| \"N/A\".to_string())}" }
                                            }
                                            div {
                                                div { class: "text-xs text-muted-foreground", "Assigned Teacher ID" }
                                                div { class: "font-mono text-xs text-muted-foreground", "{course.teacher_id.clone().unwrap_or_else(|| \"-\".to_string())}" }
                                            }
                                        }
                                    }
                                    
                                    button {
                                        class: "yntra-btn text-xs font-bold flex items-center justify-center gap-1.5 py-2.5",
                                        onclick: move |_| show_assignment_modal.set(true),
                                        components::LucideIcon { name: "plus", size: "14" }
                                        "Post Assignment"
                                    }
                                }
                                
                                // Right sidebar: assignments list & submissions
                                div { class: "flex flex-col gap-4 md:col-span-2",
                                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                        h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Assignments" }
                                        
                                        if assignments.is_empty() {
                                            div { class: "text-center p-8 text-muted-foreground text-sm", "No assignments posted yet." }
                                        } else {
                                            div { class: "flex flex-col gap-2",
                                                for a in assignments.iter() {
                                                    div {
                                                        class: format!("border p-3 rounded-lg flex justify-between items-center hover:border-primary/50 transition-all cursor-pointer bg-white/[0.01] {}",
                                                            if Some(a.id.clone()) == *selected_assignment_id.read() { "border-primary/60 bg-primary/[0.02]" } else { "border-border/40" }
                                                        ),
                                                        onclick: {
                                                            let a_id = a.id.clone();
                                                            move |_| {
                                                                selected_assignment_id.set(Some(a_id.clone()));
                                                            }
                                                        },
                                                        div { class: "flex flex-col gap-0.5",
                                                            div { class: "font-bold text-foreground", "{a.title}" }
                                                            div { class: "text-xs text-muted-foreground", "{a.description}" }
                                                        }
                                                        div { class: "text-right flex flex-col gap-1 items-end",
                                                            span { class: "text-xs font-bold text-primary", "{a.max_points} Points" }
                                                            span { class: "text-[10px] text-muted-foreground", "Due: {a.due_date}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Submissions block if assignment selected
                                    if let Some(ref assignment) = selected_assignment {
                                        components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3 mt-2",
                                            h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Submissions for: {assignment.title}" }
                                            
                                            if submissions.is_empty() {
                                                div { class: "text-center p-8 text-muted-foreground text-sm", "No submissions for this assignment yet." }
                                            } else {
                                                div { class: "flex flex-col gap-2.5",
                                                    for s in submissions.iter() {
                                                        div { class: "border border-border/40 p-4 rounded-xl flex flex-col gap-3 bg-white/[0.01] hover:border-border transition-all",
                                                            div { class: "flex justify-between items-start",
                                                                div {
                                                                    div { class: "font-bold text-foreground text-xs",
                                                                        {
                                                                            let student = students.iter().find(|st| st.id == s.student_id);
                                                                            student.map(|st| format!("{} {}", st.first_name, st.last_name)).unwrap_or_else(|| "Unknown Student".to_string())
                                                                        }
                                                                    }
                                                                    div { class: "text-[10px] text-muted-foreground mt-0.5", "Submitted at: {s.submitted_at}" }
                                                                }
                                                                div { class: "flex gap-2 items-center",
                                                                    if let Some(ref g) = s.grade {
                                                                        span { class: "text-xs font-extrabold px-2.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20", "Grade: {g}" }
                                                                    } else {
                                                                        span { class: "text-xs font-extrabold px-2.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20", "Ungraded" }
                                                                    }
                                                                    button {
                                                                        class: "yntra-btn text-[10px] font-bold py-1 px-2.5",
                                                                        onclick: {
                                                                            let sub_id = s.id.clone();
                                                                            let curr_grade = s.grade.clone().unwrap_or_else(|| "A".to_string());
                                                                            let curr_feedback = s.feedback.clone().unwrap_or_default();
                                                                            move |_| {
                                                                                selected_submission_id.set(Some(sub_id.clone()));
                                                                                grade_input.set(curr_grade.clone());
                                                                                feedback_input.set(curr_feedback.clone());
                                                                                show_grading_modal.set(true);
                                                                            }
                                                                        },
                                                                        "Grade"
                                                                    }
                                                                }
                                                            }
                                                            
                                                            div { class: "text-xs bg-sidebar/50 p-3 rounded-lg border border-border/20 text-muted-foreground",
                                                                div { class: "font-bold text-foreground/80 mb-1", "Submission Text:" }
                                                                "{s.content}"
                                                            }
                                                            
                                                            if let Some(ref fb) = s.feedback {
                                                                div { class: "text-xs bg-primary/5 p-2.5 rounded-lg text-primary border border-primary/10",
                                                                    strong { "Feedback: " }
                                                                    "{fb}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // List of Courses
            div { class: "flex flex-col gap-6",
                // Search and Filters Bar
                div { class: "flex flex-col md:flex-row gap-4 justify-between items-center bg-sidebar/30 border border-border/40 p-4 rounded-xl shadow-sm backdrop-blur-md",
                    div { class: "relative w-full md:max-w-xs",
                        components::LucideIcon { name: "search", size: "16", class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground opacity-60" }
                        input {
                            class: "yntra-input pr-4 h-10 w-full rounded-lg bg-background border border-border/40 text-xs focus:ring-1 focus:ring-primary",
                            style: "padding-left: 2.5rem;",
                            placeholder: "Search courses...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
                    }
                    
                    // Subject Filter Pills
                    div { class: "flex gap-2 self-start md:self-auto overflow-x-auto max-w-full pb-1 md:pb-0",
                        for (slug, label) in &[("all", "All"), ("mathematics", "Math"), ("science", "Science"), ("languages", "Languages"), ("history", "History")] {
                            button {
                                class: format!("px-3.5 py-1.5 rounded-lg text-xs font-bold transition-all border bg-transparent cursor-pointer {}", 
                                    if *selected_subject.read() == *slug { "bg-primary text-primary-foreground border-primary shadow" } else { "bg-sidebar/50 text-muted-foreground border-border/30 hover:text-foreground hover:bg-sidebar" }
                                ),
                                onclick: move |_| selected_subject.set(slug.to_string()),
                                "{label}"
                            }
                        }
                    }
                }

                if filtered_courses.is_empty() {
                    div { class: "text-center p-12 text-muted-foreground border border-dashed border-border/40 rounded-xl",
                        components::LucideIcon { name: "book-open", size: "40", class: "opacity-20 mb-2 mx-auto" }
                        p { class: "text-sm font-semibold m-0", "No courses found." }
                    }
                } else {
                    div { class: "grid gap-6 md:grid-cols-3",
                        for c in filtered_courses.iter() {
                            {
                                let (gradient, _badge_style, icon) = get_subject_theme(&c.subject);
                                rsx! {
                                    div {
                                        class: "group relative overflow-hidden rounded-xl border border-border/40 bg-sidebar/20 hover:border-primary/40 hover:shadow-2xl hover:-translate-y-1 transition-all duration-300 flex flex-col justify-between h-48 cursor-pointer shadow-md",
                                        onclick: {
                                            let c_id = c.id.clone();
                                            move |_| {
                                                selected_course_id.set(Some(c_id.clone()));
                                                selected_assignment_id.set(None);
                                                course_tab.set("stream".to_string());
                                            }
                                        },
                                        // Header area with gradient
                                        div { class: format!("p-4 bg-gradient-to-r {} relative text-white flex flex-col justify-between h-28 overflow-hidden", gradient),
                                            // Subtle background icon overlay
                                            div { class: "absolute right-2 -bottom-4 text-white/10 select-none pointer-events-none scale-150 rotate-12",
                                                components::LucideIcon { name: icon, size: "80" }
                                            }
                                            div { class: "flex justify-between items-start z-10",
                                                span { class: "text-[10px] font-black uppercase tracking-widest px-2 py-0.5 bg-white/20 backdrop-blur-md rounded-md", "{c.subject}" }
                                                if let Some(ref room) = c.classroom {
                                                    span { class: "text-xs font-semibold flex items-center gap-1 opacity-90",
                                                        components::LucideIcon { name: "home", size: "12" }
                                                        "{room}"
                                                    }
                                                }
                                            }
                                            h3 { class: "text-lg font-black tracking-tight m-0 mb-1 z-10 drop-shadow-sm truncate text-white", "{c.name}" }
                                        }
                                        
                                        // Footer area
                                        div { class: "p-4 bg-sidebar/10 flex justify-between items-center text-xs text-muted-foreground border-t border-border/10",
                                            div { class: "flex items-center gap-1.5",
                                                div { class: "w-5 h-5 rounded-full bg-primary/25 flex items-center justify-center text-[10px] font-bold text-primary", "M" }
                                                span { "Ms. Andersson" }
                                            }
                                            span { class: "text-primary font-bold flex items-center gap-1 group-hover:translate-x-1 transition-all",
                                                "Enter Class"
                                                components::LucideIcon { name: "arrow-right", size: "12" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
