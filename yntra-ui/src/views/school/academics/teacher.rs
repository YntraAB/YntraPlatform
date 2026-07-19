use dioxus::prelude::*;
use dioxus::html::HasFileData;
use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Dialog, Input, LucideIcon, SuggestionInput};
use crate::locales::t;
use super::SchoolViewProps;
use super::infer_subject_from_course;
use super::utils::{
    is_deadline_passed, parse_submission_content_and_advanced_attachment,
    format_file_size, compute_mock_hash, base64_encode, AdvancedAttachment
};
use yntra_core::{
    get_assignments, get_course_term_grades, save_assignment, save_course, save_term_grade,
    get_student_submissions, save_submission,
    Assignment, Course, TermGrade, StudentProfile, Submission
};

#[component]
pub fn TeacherPortal(
    school_props: SchoolViewProps,
    courses: Vec<Course>,
    students: Vec<StudentProfile>,
    mut selected_course_id: Signal<String>,
    mut show_course_modal: Signal<bool>,
) -> Element {
    let db_trigger = school_props.db_trigger;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();
    let state = use_context::<crate::state::AppState>();

    // Local states
    let mut active_menu_id = use_signal(|| "".to_string());
    let mut new_course_name = use_signal(String::new);
    let mut new_course_subject = use_signal(String::new);
    let mut new_course_teacher = use_signal(String::new);

    let mut show_assignment_modal = use_signal(|| false);
    let mut new_assign_title = use_signal(String::new);
    let mut new_assign_desc = use_signal(String::new);
    let mut new_assign_due = use_signal(|| "2026-08-01".to_string());
    let mut new_assign_due_type = use_signal(|| "no_deadline".to_string());
    let mut new_assign_pts = use_signal(|| 100);
    let mut new_assign_file = use_signal(|| Option::<AdvancedAttachment>::None);
    let mut new_assign_drag_active = use_signal(|| false);
    let mut assignment_to_delete = use_signal(|| Option::<String>::None);

    let mut sub_tab = use_signal(|| "stream".to_string());
    let mut new_announcement_text = use_signal(String::new);
    let mut comment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let mut show_grade_modal = use_signal(|| false);
    let mut grade_student_id = use_signal(String::new);
    let mut grade_term = use_signal(|| "Fall 2026".to_string());
    let mut grade_letter = use_signal(|| "A".to_string());
    let mut grade_points = use_signal(|| 90);
    let mut grade_comments = use_signal(String::new);

    let mut show_homework_grade_modal = use_signal(|| false);
    let mut selected_homework_sub = use_signal(|| Option::<Submission>::None);
    let mut homework_grade = use_signal(String::new);
    let mut homework_feedback = use_signal(String::new);

    // Resources
    
    let user_id_clone_users = user_id.clone();
    let users_res = use_resource(move || {
        let _trig = db_trigger.read();
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
                get_assignments(uid, ws, c_id).await.unwrap_or_default()
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
                get_course_term_grades(uid, ws, c_id).await.unwrap_or_default()
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
                if let Ok(mut subs) = get_student_submissions(uid.clone(), ws.clone(), s.id.clone()).await {
                    list.append(&mut subs);
                }
            }
            list
        }
    });

    let all_course_assignments = assignments_res.read().clone().unwrap_or_default();
    let announcements = all_course_assignments.iter().filter(|a| a.max_points == -1).cloned().collect::<Vec<_>>();
    let comments = all_course_assignments.iter().filter(|a| a.max_points == -2).cloned().collect::<Vec<_>>();
    let assignments = all_course_assignments.iter().filter(|a| a.max_points >= 0).cloned().collect::<Vec<_>>();
    let course_grades = course_grades_res.read().clone().unwrap_or_default();
    let all_submissions = all_submissions_res.read().clone().unwrap_or_default();
    let users_binding = users_res.read().clone().unwrap_or_default();
    let current_user = users_binding.iter().find(|u| u.id == user_id);
    let teacher_name = current_user
        .and_then(|u| u.full_name.clone())
        .unwrap_or_else(|| "Teacher".to_string());

    let name_suggestions = {
        let subj = new_course_subject.read().trim().to_lowercase();
        if subj == "matematik" || subj == "mathematics" || subj == "matematikk" || subj == "matematiikka" {
            match locale.as_str() {
                "sv" => vec!["Algebra I", "Algebra II", "Geometri", "Analys"],
                "no" => vec!["Algebra I", "Algebra II", "Geometri", "Kalkulus"],
                "da" => vec!["Algebra I", "Algebra II", "Geometri", "Kalkulus"],
                "fi" => vec!["Algebra I", "Algebra II", "Geometria", "Analyysi"],
                _ => vec!["Algebra I", "Algebra II", "Geometry", "Calculus"],
            }
        } else if subj == "naturvetenskap" || subj == "science" || subj == "naturfag" || subj == "luonnontiede" || subj == "fysik" || subj == "kemi" || subj == "biologi" || subj == "physics" || subj == "chemistry" || subj == "biology" || subj == "fysikk" || subj == "kjemi" || subj == "fysiikka" || subj == "kemia" || subj == "biologia" {
            match locale.as_str() {
                "sv" => vec!["Biologi I", "Avancerad biologi", "Grundläggande kemi", "Introduktion till fysik"],
                "no" => vec!["Biologi I", "Avansert biologi", "Kjemi 101", "Innføring i fysikk"],
                "da" => vec!["Biologi I", "Avanceret biologi", "Kemi 101", "Introduktion til fysik"],
                "fi" => vec!["Biologia I", "Syventävä biologia", "Kemia 101", "Johdatus fysiikkaan"],
                _ => vec!["Biology I", "Advanced Biology", "Chemistry 101", "Introduction to Physics"],
            }
        } else if subj == "historia" || subj == "history" || subj == "historie" || subj == "geografi" || subj == "geography" || subj == "maantieto" {
            match locale.as_str() {
                "sv" => vec!["Världshistoria", "Sveriges historia", "Geografi I", "Forntida civilisationer"],
                "no" => vec!["Verdenshistorie", "Norgeshistorie", "Geografi I", "Gamle sivilisasjoner"],
                "da" => vec!["Verdenshistorie", "Danmarks historie", "Geografi I", "Gamle civilisationer"],
                "fi" => vec!["Maailmanhistoria", "Suomen historia", "Maantieto I", "Muinaiset sivilisaatiot"],
                _ => vec!["World History", "US History", "Human Geography", "Ancient Civilizations"],
            }
        } else if subj == "engelska" || subj == "english" || subj == "engelsk" || subj == "englanti" {
            match locale.as_str() {
                "sv" => vec!["Engelsk litteratur", "Kreativt skrivande", "Akademiskt skrivande", "Engelska 101"],
                "no" => vec!["Engelsk litteratur", "Kreativ skriving", "Akademisk skriving", "Engelsk 101"],
                "da" => vec!["Engelsk litteratur", "Kreativ skrivning", "Akademisk skrivning", "Engelsk 101"],
                "fi" => vec!["Englanninkielinen kirjallisuus", "Luova kirjoittaminen", "Akateeminen kirjoittaminen", "Englanti 101"],
                _ => vec!["English Literature", "Creative Writing", "Academic Writing", "English 101"],
            }
        } else if subj == "bild" || subj == "art" || subj == "kunst og håndverk" || subj == "billedkunst" || subj == "kuvataide" {
            match locale.as_str() {
                "sv" => vec!["Teckning & målning", "Grafisk design", "Konsthistoria", "Keramik"],
                "no" => vec!["Tegning og maling", "Grafisk design", "Kunsthistorie", "Keramikk"],
                "da" => vec!["Tegning & maleri", "Grafisk design", "Kunsthistorie", "Keramik"],
                "fi" => vec!["Piirustus & maalaus", "Graafinen suunnittelu", "Taidehistoria", "Keramiikka"],
                _ => vec!["Drawing & Painting", "Graphic Design", "Art History", "Ceramics"],
            }
        } else if subj == "musik" || subj == "music" || subj == "musiikki" {
            match locale.as_str() {
                "sv" => vec!["Musikteori", "Körsång", "Orkester", "Gitarr för nybörjare"],
                "no" => vec!["Musikkteori", "Kor", "Orkester", "Gitar for nybegynnere"],
                "da" => vec!["Musikteori", "Kor", "Orkester", "Guitar for begyndere"],
                "fi" => vec!["Musiikin teoria", "Kuoro", "Orkesteri", "Kitaran alkeet"],
                _ => vec!["Music Theory", "Choir", "Band / Orchestra", "Beginner Guitar"],
            }
        } else if subj == "idrott och hälsa" || subj == "physical education" || subj == "kroppsøving" || subj == "idræt" || subj == "liikunta" {
            match locale.as_str() {
                "sv" => vec!["Lagsport", "Konditionsträning", "Hälsokunskap", "Yoga & välmående"],
                "no" => vec!["Lagsport", "Kondisjonstrening", "Helsefag", "Yoga og velvære"],
                "da" => vec!["Lagsport", "Konditionstræning", "Sundhedslære", "Yoga & være"],
                "fi" => vec!["Joukkuepelit", "Kuntosali & kuntoilu", "Terveystieto", "Jooga & hyvinvointi"],
                _ => vec!["Team Sports", "Fitness & Conditioning", "Health Education", "Yoga & Wellness"],
            }
        } else {
            match locale.as_str() {
                "sv" => vec!["Algebra I", "Världshistoria", "Grundläggande kemi", "Engelsk litteratur", "Bild"],
                "no" => vec!["Algebra I", "Verdenshistorie", "Kjemi 101", "Engelsk litteratur", "Kunst"],
                "da" => vec!["Algebra I", "Verdenshistorie", "Kemi 101", "Engelsk litteratur", "Billedkunst"],
                "fi" => vec!["Algebra I", "Maailmanhistoria", "Kemia 101", "Englanninkielinen kirjallisuus", "Kuvataide"],
                _ => vec!["Algebra I", "World History", "Chemistry 101", "English Literature", "Fine Arts"],
            }
        }
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
    };

    let subject_suggestions = match locale.as_str() {
        "sv" => vec![
            "Matematik".to_string(), "Naturvetenskap".to_string(), "Historia".to_string(),
            "Geografi".to_string(), "Fysik".to_string(), "Kemi".to_string(), "Biologi".to_string(),
            "Engelska".to_string(), "Idrott och hälsa".to_string(), "Bild".to_string(), "Musik".to_string(),
        ],
        "no" => vec![
            "Matematikk".to_string(), "Naturfag".to_string(), "Historie".to_string(),
            "Geografi".to_string(), "Fysikk".to_string(), "Kjemi".to_string(), "Biologi".to_string(),
            "Engelsk".to_string(), "Kroppsøving".to_string(), "Kunst og håndverk".to_string(), "Musikk".to_string(),
        ],
        "da" => vec![
            "Matematik".to_string(), "Naturfag".to_string(), "Historie".to_string(),
            "Geografi".to_string(), "Fysik".to_string(), "Kemi".to_string(), "Biologi".to_string(),
            "Engelsk".to_string(), "Idræt".to_string(), "Billedkunst".to_string(), "Musik".to_string(),
        ],
        "fi" => vec![
            "Matematiikka".to_string(), "Luonnontiede".to_string(), "Historia".to_string(),
            "Maantieto".to_string(), "Fysiikka".to_string(), "Kemia".to_string(), "Biologia".to_string(),
            "Englanti".to_string(), "Liikunta".to_string(), "Kuvataide".to_string(), "Musiikki".to_string(),
        ],
        _ => vec![
            "Mathematics".to_string(), "Science".to_string(), "History".to_string(),
            "Geography".to_string(), "Physics".to_string(), "Chemistry".to_string(), "Biology".to_string(),
            "English".to_string(), "Physical Education".to_string(), "Art".to_string(), "Music".to_string(),
        ],
    };

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            if selected_course_id.read().is_empty() {
                // Google Classroom style classroom grid view
                div { class: "space-y-6",
                    if courses.is_empty() {
                        div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                            LucideIcon { name: "library", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                            h4 { class: "text-sm font-bold text-foreground m-0", "No Classrooms Registered" }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-sm", "Click the options menu next to the view tabs and select 'Create Course' to start setting up your digital classrooms." }
                        }
                    } else {
                        div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6",
                            for c in courses.iter() {
                                {
                                    let c_id_dropdown = c.id.clone();
                                    let subject_lower = c.subject.trim().to_lowercase();
                                    let gradient_class = if subject_lower == "matematik" || subject_lower == "mathematics" || subject_lower == "matematikk" || subject_lower == "matematiikka" {
                                        "from-blue-600 to-indigo-600"
                                    } else if subject_lower == "naturvetenskap" || subject_lower == "science" || subject_lower == "naturfag" || subject_lower == "luonnontiede" {
                                        "from-teal-600 to-emerald-600"
                                    } else if subject_lower == "bild" || subject_lower == "art" || subject_lower == "billedkunst" || subject_lower == "kuvataide" {
                                        "from-purple-600 to-pink-600"
                                    } else if subject_lower == "musik" || subject_lower == "music" || subject_lower == "musiikki" {
                                        "from-rose-500 to-red-600"
                                    } else if subject_lower == "engelska" || subject_lower == "english" || subject_lower == "engelsk" || subject_lower == "englanti" {
                                        "from-amber-500 to-orange-600"
                                    } else if subject_lower == "historia" || subject_lower == "history" || subject_lower == "historie" {
                                        "from-cyan-600 to-sky-600"
                                    } else {
                                        "from-gray-600 to-slate-700"
                                    };
                                    rsx! {
                                        div {
                                            key: "{c.id}",
                                            class: "relative group flex flex-col rounded-2xl overflow-visible border border-border/85 hover:border-primary/45 transition-all hover:shadow-lg duration-200 bg-background cursor-pointer",
                                            onclick: {
                                                let c_id_select = c.id.clone();
                                                move |_| {
                                                    selected_course_id.set(c_id_select.clone());
                                                }
                                            },
                                            
                                            // Colorful header banner
                                            div { class: "h-28 bg-gradient-to-br {gradient_class} p-4 text-white relative flex flex-col justify-between shadow-inner rounded-t-2xl",
                                                div { class: "flex items-start justify-between w-full",
                                                    div { class: "space-y-0.5 max-w-[80%]",
                                                        h3 { class: "font-extrabold text-sm tracking-tight m-0 text-white truncate", "{c.name}" }
                                                        span { class: "text-[9px] font-bold text-white/90 uppercase tracking-wider", "{c.subject}" }
                                                    }
                                                    
                                                    // Safe settings button (tucked away)
                                                    button {
                                                        class: "p-1.5 rounded-full hover:bg-white/20 text-white/80 hover:text-white border-0 bg-transparent cursor-pointer transition-colors z-20",
                                                        r#type: "button",
                                                        onclick: {
                                                            let c_id_d = c_id_dropdown.clone();
                                                            move |e| {
                                                                e.stop_propagation();
                                                                if *active_menu_id.read() == c_id_d {
                                                                    active_menu_id.set("".to_string());
                                                                } else {
                                                                    active_menu_id.set(c_id_d.clone());
                                                                }
                                                            }
                                                        },
                                                        LucideIcon { name: "settings", class: "h-4 w-4" }
                                                    }
                                                }
                                                
                                                if let Some(ref room) = c.classroom {
                                                    span { class: "text-[10px] text-white/80 font-semibold", "Room: {room}" }
                                                }

                                                // destruct settings menu dropdown
                                                if *active_menu_id.read() == c_id_dropdown {
                                                    div { class: "absolute top-11 right-3 z-30 bg-popover border border-border rounded-xl shadow-2xl p-1 min-w-[130px] animate-in fade-in slide-in-from-top-2 duration-150",
                                                        button {
                                                            class: "flex w-full items-center gap-1.5 text-left px-2.5 py-1.5 text-xs font-semibold text-red-500 hover:bg-red-500/10 border-0 bg-transparent rounded-lg cursor-pointer transition-colors",
                                                            r#type: "button",
                                                            onclick: {
                                                                let uid = user_id.clone();
                                                                let cid = c.id.clone();
                                                                let mut db_trigger = db_trigger.clone();
                                                                move |e| {
                                                                    e.stop_propagation();
                                                                    let uid_c = uid.clone();
                                                                    let cid_c = cid.clone();
                                                                    spawn(async move {
                                                                        let _ = yntra_core::delete_course(uid_c, cid_c).await;
                                                                    });
                                                                    active_menu_id.set("".to_string());
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                }
                                                            },
                                                            LucideIcon { name: "trash", class: "h-3.5 w-3.5 text-red-500" }
                                                            "Delete Course"
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // Card content (Roster info)
                                            div { class: "p-4 flex flex-col justify-between flex-1 bg-card h-20 border-x border-b border-border/60 rounded-b-2xl",
                                                div { class: "text-[11px] text-muted-foreground flex items-center gap-1.5",
                                                    LucideIcon { name: "user", class: "h-3.5 w-3.5" }
                                                    "{c.teacher_id.clone().unwrap_or_default()}"
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
                // Active Course Classroom View (Google Classroom style)
                {
                    let selected_id = selected_course_id.read().clone();
                    let c = courses.iter().find(|item| item.id == selected_id).cloned().unwrap();
                    let subject_lower = c.subject.trim().to_lowercase();
                    let gradient_class = if subject_lower == "matematik" || subject_lower == "mathematics" || subject_lower == "matematikk" || subject_lower == "matematiikka" {
                        "from-blue-600 to-indigo-600"
                    } else if subject_lower == "naturvetenskap" || subject_lower == "science" || subject_lower == "naturfag" || subject_lower == "luonnontiede" {
                        "from-teal-600 to-emerald-600"
                    } else if subject_lower == "bild" || subject_lower == "art" || subject_lower == "billedkunst" || subject_lower == "kuvataide" {
                        "from-purple-600 to-pink-600"
                    } else if subject_lower == "musik" || subject_lower == "music" || subject_lower == "musiikki" {
                        "from-rose-500 to-red-600"
                    } else if subject_lower == "engelska" || subject_lower == "english" || subject_lower == "engelsk" || subject_lower == "englanti" {
                        "from-amber-500 to-orange-600"
                    } else if subject_lower == "historia" || subject_lower == "history" || subject_lower == "historie" {
                        "from-cyan-600 to-sky-600"
                    } else {
                        "from-gray-600 to-slate-700"
                    };
                    rsx! {
                        div { class: "space-y-6",
                            // Back button
                            button {
                                class: "flex items-center gap-1.5 text-xs font-bold text-muted-foreground hover:text-foreground cursor-pointer border-0 bg-transparent pb-1 transition-colors",
                                r#type: "button",
                                onclick: move |_| selected_course_id.set("".to_string()),
                                LucideIcon { name: "arrow-left", class: "h-4 w-4" }
                                "Back to Classrooms"
                            }

                            // Gorgeous Classroom banner card
                            div { class: "h-36 rounded-2xl bg-gradient-to-r {gradient_class} p-6 text-white flex flex-col justify-end shadow-md relative overflow-hidden shadow-inner",
                                h2 { class: "text-2xl font-extrabold m-0 text-white tracking-tight", "{c.name}" }
                                p { class: "text-xs font-bold text-white/90 m-0 mt-1.5 uppercase tracking-wider", 
                                    "Subject: {c.subject} • Room: {c.classroom.clone().unwrap_or_default()} • Teacher: {c.teacher_id.clone().unwrap_or_default()}"
                                }
                            }

                            // Horizontal Google Classroom tab selector
                            Card { class: "border-border shadow-sm rounded-2xl overflow-hidden",
                                CardContent { class: "p-6",
                                    div { class: "flex border-b border-border pb-2.5 mb-6 gap-6 text-xs font-extrabold tracking-wider",
                                        button {
                                            class: format!(
                                                "pb-2.5 transition-all border-b-2 {}",
                                                if *sub_tab.read() == "stream" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                                            ),
                                            r#type: "button",
                                            onclick: move |_| sub_tab.set("stream".to_string()),
                                            "Stream"
                                        }
                                        button {
                                            class: format!(
                                                "pb-2.5 transition-all border-b-2 {}",
                                                if *sub_tab.read() == "syllabus" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                                            ),
                                            r#type: "button",
                                            onclick: move |_| sub_tab.set("syllabus".to_string()),
                                            "Classwork"
                                        }
                                        button {
                                            class: format!(
                                                "pb-2.5 transition-all border-b-2 {}",
                                                if *sub_tab.read() == "grading" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                                            ),
                                            r#type: "button",
                                            onclick: move |_| sub_tab.set("grading".to_string()),
                                            "Grades"
                                        }
                                        button {
                                            class: format!(
                                                "pb-2.5 transition-all border-b-2 {}",
                                                if *sub_tab.read() == "submissions" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                                            ),
                                            r#type: "button",
                                            onclick: move |_| sub_tab.set("submissions".to_string()),
                                            "Submissions"
                                        }
                                    }

                                    if *sub_tab.read() == "stream" {
                                        div { class: "space-y-6",
                                            // Announcement editor box
                                            div { class: "p-4 border border-border/80 rounded-xl bg-muted/20 space-y-3",
                                                textarea {
                                                    class: "w-full min-h-[80px] p-3 text-xs bg-background border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/60 resize-none font-medium",
                                                    placeholder: "Announce something to your class...",
                                                    value: "{new_announcement_text}",
                                                    oninput: move |evt| new_announcement_text.set(evt.value().clone()),
                                                }
                                                div { class: "flex justify-end",
                                                    Button {
                                                        class: "px-4 h-8 text-xs font-semibold rounded-lg bg-primary text-primary-foreground flex items-center gap-1",
                                                        disabled: new_announcement_text.read().trim().is_empty(),
                                                        onclick: {
                                                            let cid = c.id.clone();
                                                            let ws = ws_id.clone();
                                                            let teacher_name = teacher_name.clone();
                                                            let uid = user_id.clone();
                                                            move |_| {
                                                                let text = new_announcement_text.read().clone();
                                                                let assignment = yntra_core::Assignment {
                                                                    id: uuid::Uuid::new_v4().to_string(),
                                                                    workspace_id: ws.clone(),
                                                                    course_id: cid.clone(),
                                                                    title: teacher_name.clone(),
                                                                    description: text,
                                                                    due_date: "teacher".to_string(),
                                                                    max_points: -1,
                                                                    updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                                };
                                                                let uid_c = uid.clone();
                                                                let mut trig = db_trigger;
                                                                spawn(async move {
                                                                    if yntra_core::save_assignment(uid_c, assignment, None).await.is_ok() {
                                                                        new_announcement_text.set(String::new());
                                                                        let current = *trig.read();
                                                                        trig.set(current + 1);
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        LucideIcon { name: "send", class: "h-3 w-3" }
                                                        "Post"
                                                    }
                                                }
                                            }

                                            // Stream Announcements Feed
                                            if announcements.is_empty() {
                                                div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic", "No announcements yet. Welcome your class with a post!" }
                                            } else {
                                                div { class: "space-y-4",
                                                    for ann in announcements.iter().rev() {
                                                        {
                                                            let ann_id = ann.id.clone();
                                                            let ann_comments = comments.iter().filter(|comm| comm.course_id == ann_id).collect::<Vec<_>>();
                                                            let comment_text = comment_inputs.read().get(&ann_id).cloned().unwrap_or_default();
                                                            rsx! {
                                                                div { key: "{ann.id}", class: "p-5 border border-border rounded-xl bg-background space-y-4 shadow-sm",
                                                                    div { class: "flex items-start justify-between gap-3",
                                                                        div { class: "flex items-center gap-2.5",
                                                                            div { class: "h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-xs",
                                                                                "{ann.title.chars().next().unwrap_or('?')}"
                                                                            }
                                                                            div {
                                                                                div { class: "text-xs font-bold text-foreground flex items-center gap-1.5", 
                                                                                    "{ann.title}"
                                                                                    span { class: "text-[9px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-extrabold uppercase tracking-wide", "Teacher" }
                                                                                }
                                                                                div { class: "text-[10px] text-muted-foreground",
                                                                                    {
                                                                                        let ms = ann.updated_at;
                                                                                        let formatted = format_timestamp(ms);
                                                                                        formatted
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        button {
                                                                            class: "p-1.5 rounded hover:bg-muted text-muted-foreground hover:text-red-500 border-0 cursor-pointer transition-colors",
                                                                            r#type: "button",
                                                                            onclick: {
                                                                                let ann_id_del = ann.id.clone();
                                                                                let uid_c = user_id.clone();
                                                                                let mut trig = db_trigger;
                                                                                move |_| {
                                                                                    let aid = ann_id_del.clone();
                                                                                    let uid_del = uid_c.clone();
                                                                                    spawn(async move {
                                                                                        if yntra_core::delete_assignment(uid_del, aid).await.is_ok() {
                                                                                            let current = *trig.read();
                                                                                            trig.set(current + 1);
                                                                                        }
                                                                                    });
                                                                                }
                                                                            },
                                                                            LucideIcon { name: "trash", size: "13" }
                                                                        }
                                                                    }
                                                                    div { class: "text-xs text-foreground font-medium whitespace-pre-line leading-relaxed", "{ann.description}" }
                                                                    
                                                                    div { class: "border-t border-border/40 pt-3 space-y-3",
                                                                        div { class: "text-[10px] font-bold text-muted-foreground flex items-center gap-1",
                                                                            LucideIcon { name: "message-square", size: "11" }
                                                                            "Class comments ({ann_comments.len()})"
                                                                        }
                                                                        
                                                                        if !ann_comments.is_empty() {
                                                                            div { class: "space-y-3 pl-3 border-l-2 border-muted",
                                                                                for comm in ann_comments.iter() {
                                                                                    div { key: "{comm.id}", class: "text-xs space-y-0.5",
                                                                                        div { class: "flex items-center gap-1.5",
                                                                                            span { class: "font-bold text-foreground", "{comm.title}" }
                                                                                            span { class: "text-[8px] font-bold px-1 rounded bg-muted text-muted-foreground uppercase", "{comm.due_date}" }
                                                                                            span { class: "text-[9px] text-muted-foreground/60",
                                                                                                {
                                                                                                    let ms = comm.updated_at;
                                                                                                    let formatted = format_timestamp(ms);
                                                                                                    formatted
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        div { class: "text-muted-foreground font-medium", "{comm.description}" }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        
                                                                        div { class: "flex items-center gap-2 pt-1.5",
                                                                            input {
                                                                                class: "flex-1 h-8 px-3 text-xs bg-muted/30 border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/50 font-medium",
                                                                                placeholder: "Add class comment...",
                                                                                value: "{comment_text}",
                                                                                oninput: {
                                                                                    let ann_id_c = ann_id.clone();
                                                                                    move |evt| {
                                                                                        comment_inputs.write().insert(ann_id_c.clone(), evt.value().clone());
                                                                                    }
                                                                                },
                                                                                onkeydown: {
                                                                                    let ann_id_c = ann_id.clone();
                                                                                    let ws = ws_id.clone();
                                                                                    let teacher_name = teacher_name.clone();
                                                                                    let uid = user_id.clone();
                                                                                    let text = comment_text.clone();
                                                                                    let mut trig = db_trigger;
                                                                                    move |evt| {
                                                                                        if evt.key() == Key::Enter && !text.trim().is_empty() {
                                                                                            let val = text.clone();
                                                                                            let aid = ann_id_c.clone();
                                                                                            let c_item = yntra_core::Assignment {
                                                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                                                workspace_id: ws.clone(),
                                                                                                course_id: aid,
                                                                                                title: teacher_name.clone(),
                                                                                                description: val,
                                                                                                due_date: "teacher".to_string(),
                                                                                                max_points: -2,
                                                                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                                                            };
                                                                                            let uid_c = uid.clone();
                                                                                            comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                                                            spawn(async move {
                                                                                                if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                                                    {
                                                                                                         let current = *trig.read();
                                                                                                         trig.set(current + 1);
                                                                                                     }
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            button {
                                                                                class: "h-8 w-8 rounded-lg bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground border-0 cursor-pointer flex items-center justify-center transition-colors",
                                                                                r#type: "button",
                                                                                onclick: {
                                                                                    let ann_id_c = ann_id.clone();
                                                                                    let ws = ws_id.clone();
                                                                                    let teacher_name = teacher_name.clone();
                                                                                    let uid = user_id.clone();
                                                                                    let text = comment_text.clone();
                                                                                    let mut trig = db_trigger;
                                                                                    move |_| {
                                                                                        if !text.trim().is_empty() {
                                                                                            let val = text.clone();
                                                                                            let aid = ann_id_c.clone();
                                                                                            let c_item = yntra_core::Assignment {
                                                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                                                workspace_id: ws.clone(),
                                                                                                course_id: aid,
                                                                                                title: teacher_name.clone(),
                                                                                                description: val,
                                                                                                due_date: "teacher".to_string(),
                                                                                                max_points: -2,
                                                                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                                                            };
                                                                                            let uid_c = uid.clone();
                                                                                            comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                                                            spawn(async move {
                                                                                                if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                                                    {
                                                                                                         let current = *trig.read();
                                                                                                         trig.set(current + 1);
                                                                                                     }
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                },
                                                                                LucideIcon { name: "arrow-right", class: "h-3.5 w-3.5" }
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
                                    } else if *sub_tab.read() == "syllabus" {
                                        div { class: "space-y-4",
                                            div { class: "flex items-center justify-between",
                                                h5 { class: "font-bold text-sm m-0 text-foreground", "Assignments" }
                                                Button {
                                                    class: "px-3 h-8 text-xs font-semibold rounded-lg flex items-center gap-1",
                                                    onclick: move |_| show_assignment_modal.set(true),
                                                    LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                                                    "Add Assignment"
                                                }
                                            }

                                            if assignments.is_empty() {
                                                div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No assignments registered yet." }
                                            } else {
                                                div { class: "grid gap-3 w-full",
                                                    for a in assignments.iter() {
                                                        {
                                                            let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&a.description);
                                                            let is_deleting = *assignment_to_delete.read() == Some(a.id.clone());
                                                            let aid = a.id.clone();
                                                            let uid = user_id.clone();
                                                            rsx! {
                                                                div { key: "{a.id}", class: "p-5 border border-border/80 bg-card/25 backdrop-blur-sm rounded-2xl flex items-start justify-between gap-4 transition-all hover:border-primary/40 hover:shadow-sm",
                                                                    div { class: "space-y-1.5 flex-1 min-w-0",
                                                                        div { class: "font-bold text-foreground text-sm truncate", "{a.title}" }
                                                                        div { class: "text-muted-foreground text-xs leading-relaxed", "{desc_text}" }
                                                                        if let Some(staged) = attachment {
                                                                            a {
                                                                                href: "{staged.dataurl}",
                                                                                download: "{staged.filename}",
                                                                                class: "flex items-center gap-2 p-2 bg-card/45 border border-primary/25 hover:bg-card/75 rounded-xl no-underline text-foreground cursor-pointer transition-all mt-2.5 w-fit max-w-sm",
                                                                                LucideIcon { name: "file-text", class: "h-4 w-4 text-primary shrink-0" }
                                                                                span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                                                LucideIcon { name: "download", class: "h-3.5 w-3.5 text-muted-foreground ml-1.5 shrink-0" }
                                                                            }
                                                                        }
                                                                        div { class: "text-muted-foreground/60 text-[10px] mt-2 flex items-center gap-1.5 font-medium", 
                                                                            LucideIcon { name: "calendar", class: "h-3 w-3" }
                                                                            "Due: {a.due_date}" 
                                                                        }
                                                                    }
                                                                    div { class: "flex items-center gap-3 shrink-0",
                                                                        span { class: "font-bold text-primary bg-primary/10 border border-primary/20 px-3 py-1 rounded-full text-[10px] uppercase tracking-wider", "{a.max_points} pts" }
                                                                        if is_deleting {
                                                                            div { class: "flex items-center gap-1.5 bg-muted/60 p-1 rounded-lg border border-border/40",
                                                                                Button {
                                                                                    class: "px-2 py-1 text-[9px] bg-red-600 hover:bg-red-700 text-white rounded font-bold uppercase tracking-wider",
                                                                                    onclick: {
                                                                                        let uid_c = uid.clone();
                                                                                        let aid_c = aid.clone();
                                                                                        let mut db_trigger = db_trigger.clone();
                                                                                        move |_| {
                                                                                            let u = uid_c.clone();
                                                                                            let d = aid_c.clone();
                                                                                            spawn(async move {
                                                                                                let _ = yntra_core::delete_assignment(u, d).await;
                                                                                            });
                                                                                            assignment_to_delete.set(None);
                                                                                            let current = *db_trigger.read();
                                                                                            db_trigger.set(current + 1);
                                                                                        }
                                                                                    },
                                                                                    "Confirm"
                                                                                }
                                                                                Button {
                                                                                    class: "px-2 py-1 text-[9px] bg-muted hover:bg-muted/80 text-foreground rounded font-bold uppercase tracking-wider",
                                                                                    onclick: move |_| assignment_to_delete.set(None),
                                                                                    "Cancel"
                                                                                }
                                                                            }
                                                                        } else {
                                                                            Button {
                                                                                class: "p-1.5 bg-transparent hover:bg-red-500/10 text-muted-foreground hover:text-red-500 rounded-lg transition-colors border border-border/40 cursor-pointer",
                                                                                onclick: {
                                                                                    let aid_c = aid.clone();
                                                                                    move |_| {
                                                                                        assignment_to_delete.set(Some(aid_c.clone()));
                                                                                    }
                                                                                },
                                                                                LucideIcon { name: "trash", class: "h-3.5 w-3.5" }
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
                                    } else if *sub_tab.read() == "grading" {
                                        div { class: "space-y-4",
                                            h5 { class: "font-bold text-sm m-0 text-foreground", "Class Gradebook Matrix" }
                                            
                                            if students.is_empty() {
                                                div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "Please enroll students in the directory to view the Gradebook." }
                                            } else {
                                                {
                                                    let gradable_assignments = assignments.iter().filter(|a| a.max_points > 0).collect::<Vec<_>>();
                                                    rsx! {
                                                        div { class: "border border-border rounded-xl overflow-x-auto bg-background shadow-sm",
                                                            table { class: "w-full text-left border-collapse text-xs min-w-[600px]",
                                                                thead {
                                                                    tr { class: "border-b border-border bg-muted/40",
                                                                        th { class: "p-3.5 font-extrabold text-foreground w-48 shrink-0", "Student Name" }
                                                                        for a in gradable_assignments.iter() {
                                                                            th { key: "{a.id}", class: "p-3.5 font-bold text-foreground text-center truncate max-w-[120px]", 
                                                                                span { title: "{a.title}", "{a.title}" }
                                                                            }
                                                                        }
                                                                        th { class: "p-3.5 font-extrabold text-primary text-center w-36 shrink-0", "Final Term Grade" }
                                                                    }
                                                                }
                                                                tbody { class: "divide-y divide-border",
                                                                    for s in students.iter() {
                                                                        {
                                                                            let student_id = s.id.clone();
                                                                            let current_grade = course_grades.iter().find(|g| g.student_id == student_id).cloned();
                                                                            let s_name = format!("{} {}", s.first_name, s.last_name);
                                                                            rsx! {
                                                                                tr { key: "{s.id}", class: "hover:bg-muted/10 transition-colors",
                                                                                    td { class: "p-3.5 font-semibold text-foreground align-middle", "{s_name}" }
                                                                                    for a in gradable_assignments.iter() {
                                                                                        {
                                                                                            let a_id = a.id.clone();
                                                                                            let sub_opt = all_submissions.iter().find(|sub| sub.assignment_id == a_id && sub.student_id == student_id).cloned();
                                                                                            rsx! {
                                                                                                td { key: "{a.id}", class: "p-3.5 text-center align-middle",
                                                                                                    if let Some(sub_rec) = sub_opt {
                                                                                                        {
                                                                                                            let is_graded = sub_rec.grade.is_some();
                                                                                                            let grade_val = sub_rec.grade.clone().unwrap_or_default();
                                                                                                            let sub_c = sub_rec.clone();
                                                                                                            rsx! {
                                                                                                                button {
                                                                                                                    class: format!(
                                                                                                                        "px-2.5 py-1 rounded-lg border font-bold text-[9px] cursor-pointer transition-all hover:scale-105 {}",
                                                                                                                        if is_graded { "bg-emerald-500/10 text-emerald-600 border-emerald-500/20" } else { "bg-amber-500/10 text-amber-600 border-amber-500/20" }
                                                                                                                    ),
                                                                                                                    r#type: "button",
                                                                                                                    onclick: move |_| {
                                                                                                                        selected_homework_sub.set(Some(sub_c.clone()));
                                                                                                                        homework_grade.set(sub_c.grade.clone().unwrap_or_else(|| "A".to_string()));
                                                                                                                        homework_feedback.set(sub_c.feedback.clone().unwrap_or_default());
                                                                                                                        show_homework_grade_modal.set(true);
                                                                                                                    },
                                                                                                                    if is_graded { "{grade_val}" } else { "PENDING" }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    } else {
                                                                                                        span { class: "text-muted-foreground/40 font-medium text-[10px]", "-" }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    td { class: "p-3.5 text-center align-middle",
                                                                                        div { class: "flex items-center justify-center gap-2",
                                                                                            if let Some(ref g) = current_grade {
                                                                                                span { class: "font-bold px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-[10px]",
                                                                                                    "{g.final_grade.clone().unwrap_or_else(|| \"-\".to_string())}"
                                                                                                }
                                                                                            } else {
                                                                                                span { class: "text-muted-foreground/30 italic text-[10px]", "-" }
                                                                                            }
                                                                                            button {
                                                                                                class: "p-1 rounded bg-muted/40 hover:bg-muted text-foreground border-0 cursor-pointer transition-colors",
                                                                                                r#type: "button",
                                                                                                onclick: {
                                                                                                    let sid = s.id.clone();
                                                                                                    let current_grade_c = current_grade.clone();
                                                                                                    move |_| {
                                                                                                        grade_student_id.set(sid.clone());
                                                                                                        if let Some(ref g) = current_grade_c {
                                                                                                            grade_letter.set(g.final_grade.clone().unwrap_or_else(|| "A".to_string()));
                                                                                                            grade_points.set(g.final_points.unwrap_or(90));
                                                                                                            grade_comments.set(g.teacher_comments.clone().unwrap_or_default());
                                                                                                        } else {
                                                                                                            grade_letter.set("A".to_string());
                                                                                                            grade_points.set(90);
                                                                                                            grade_comments.set(String::new());
                                                                                                        }
                                                                                                        show_grade_modal.set(true);
                                                                                                    }
                                                                                                },
                                                                                                LucideIcon { name: "edit-2", size: "11" }
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
                                        div { class: "space-y-4",
                                            h5 { class: "font-bold text-sm m-0 text-foreground", "Student Homework Submissions" }
                                            
                                            {
                                                let course_assignment_ids: std::collections::HashSet<String> = assignments.iter().map(|a| a.id.clone()).collect();
                                                let course_submissions = all_submissions.iter()
                                                    .filter(|sub| course_assignment_ids.contains(&sub.assignment_id))
                                                    .cloned()
                                                    .collect::<Vec<_>>();
                                                
                                                if course_submissions.is_empty() {
                                                    rsx! {
                                                        div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No homework submissions received yet for this course." }
                                                    }
                                                } else {
                                                    rsx! {
                                                        div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                                            for sub in course_submissions.into_iter() {
                                                                {
                                                                    let student_name = students.iter()
                                                                        .find(|s| s.id == sub.student_id)
                                                                        .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                                        .unwrap_or_else(|| "Unknown Student".to_string());
                                                                    let assignment_title = assignments.iter()
                                                                        .find(|a| a.id == sub.assignment_id)
                                                                        .map(|a| a.title.clone())
                                                                        .unwrap_or_else(|| "Unknown Assignment".to_string());
                                                                    let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&sub.content);
                                                                    let is_graded = sub.grade.is_some();
                                                                    let grade_str = sub.grade.clone().unwrap_or_default();
                                                                    let sub_c = sub.clone();
                                                                    rsx! {
                                                                        div { key: "{sub.id}", class: "p-4 flex flex-col md:flex-row md:items-center justify-between gap-4 text-xs hover:bg-muted/5 transition-colors",
                                                                            div { class: "space-y-1.5 flex-1 min-w-0",
                                                                                div { class: "flex items-center gap-2 flex-wrap",
                                                                                    span { class: "font-bold text-foreground text-sm", "{student_name}" }
                                                                                    span { class: "text-[10px] text-muted-foreground", "for" }
                                                                                    span { class: "font-semibold text-primary", "{assignment_title}" }
                                                                                }
                                                                                div { class: "text-muted-foreground break-words text-[11px] max-w-2xl bg-card/45 p-2.5 rounded-lg border border-border mt-1", "{desc_text}" }
                                                                                if let Some(staged) = attachment {
                                                                                    a {
                                                                                        href: "{staged.dataurl}",
                                                                                        download: "{staged.filename}",
                                                                                        class: "flex items-center gap-2 p-1.5 bg-card/45 border border-primary/20 hover:bg-card/75 rounded-lg no-underline text-foreground cursor-pointer transition-all mt-2 w-fit max-w-sm",
                                                                                        LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary shrink-0" }
                                                                                        span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                                                        LucideIcon { name: "download", class: "h-3 w-3 text-muted-foreground ml-1.5 shrink-0" }
                                                                                    }
                                                                                }
                                                                                div { class: "text-[10px] text-muted-foreground mt-1", "Submitted: {sub.submitted_at}" }
                                                                            }
                                                                            div { class: "flex items-center gap-3.5 self-start md:self-center shrink-0",
                                                                                if is_graded {
                                                                                    div { class: "flex flex-col items-end gap-1",
                                                                                        span { class: "font-bold px-2.5 py-1 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-[10px] uppercase",
                                                                                            "Graded: {grade_str}"
                                                                                        }
                                                                                        if let Some(ref f) = sub.feedback {
                                                                                            span { class: "text-[10px] text-muted-foreground italic max-w-[180px] truncate", "\"{f}\"" }
                                                                                        }
                                                                                    }
                                                                                } else {
                                                                                    span { class: "text-amber-600 font-bold bg-amber-500/10 border border-amber-500/20 px-2 py-0.5 rounded-full text-[10px] uppercase", "Pending Grade" }
                                                                                }
                                                                                Button {
                                                                                    class: "px-3 py-1.5 text-xs h-8 bg-primary text-primary-foreground rounded-lg font-semibold",
                                                                                    onclick: move |_| {
                                                                                        selected_homework_sub.set(Some(sub_c.clone()));
                                                                                        homework_grade.set(sub_c.grade.clone().unwrap_or_else(|| "A".to_string()));
                                                                                        homework_feedback.set(sub_c.feedback.clone().unwrap_or_default());
                                                                                        show_homework_grade_modal.set(true);
                                                                                    },
                                                                                    if is_graded { "Edit Grade" } else { "Grade Homework" }
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
                        }
                    }
                }
            }

            // Create Course Modal Dialog
            if *show_course_modal.read() {
                Dialog {
                    open: *show_course_modal.read(),
                    title: "Create New Course",
                    onclose: move |_| show_course_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Course Name" }
                            SuggestionInput {
                                placeholder: "e.g., Algebra II",
                                value: new_course_name.read().clone(),
                                suggestions: name_suggestions.clone(),
                                onchange: {
                                    let locale = locale.clone();
                                    move |val: String| {
                                        new_course_name.set(val.clone());
                                        if let Some(inferred) = infer_subject_from_course(&val, &locale) {
                                            new_course_subject.set(inferred);
                                        }
                                    }
                                },
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Subject" }
                            SuggestionInput {
                                placeholder: "e.g., Mathematics",
                                value: new_course_subject.read().clone(),
                                suggestions: subject_suggestions.clone(),
                                onchange: move |val| new_course_subject.set(val),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Teacher Name" }
                            SuggestionInput {
                                placeholder: "e.g., Dr. Euler",
                                value: new_course_teacher.read().clone(),
                                suggestions: teacher_suggestions.clone(),
                                onchange: move |val| new_course_teacher.set(val),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_course_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                     let uid = user_id.clone();
                                     let ws = ws_id.clone();
                                     let state = state;
                                     let mut db_trigger = db_trigger.clone();
                                     move |_| {
                                         let role = state.active_user_role.read().clone();
                                         let u_id = state.active_user_id.read().clone();
                                         let proof = yntra_core::ZkCryptoTrust::new()
                                             .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                             .ok();
                                         let c = Course {
                                             id: uuid::Uuid::new_v4().to_string(),
                                             name: new_course_name.read().clone(),
                                             subject: new_course_subject.read().clone(),
                                             teacher_id: Some(new_course_teacher.read().clone()),
                                             classroom: Some("Room 101".to_string()),
                                         };
                                         let uid_c = uid.clone();
                                         let ws_c = ws.clone();
                                         spawn(async move {
                                             let _ = save_course(uid_c, ws_c, c, proof).await;
                                         });
                                        new_course_name.set(String::new());
                                        new_course_subject.set(String::new());
                                        new_course_teacher.set(String::new());
                                        show_course_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                     }
                                 },
                                "Create Course"
                            }
                        }
                    }
                }
            }

            // Create Assignment Modal Dialog
            if *show_assignment_modal.read() {
                Dialog {
                    open: *show_assignment_modal.read(),
                    title: "Post Assignment",
                    onclose: move |_| show_assignment_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Assignment Title" }
                            Input {
                                placeholder: "e.g., Chapter 3 Problem Set",
                                value: new_assign_title.read().clone(),
                                oninput: move |evt: FormEvent| new_assign_title.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Description" }
                            Input {
                                placeholder: "e.g., Solve problems 1-15 on page 42.",
                                value: new_assign_desc.read().clone(),
                                oninput: move |evt: FormEvent| new_assign_desc.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Max Points" }
                            Input {
                                placeholder: "100",
                                value: format!("{}", *new_assign_pts.read()),
                                oninput: move |evt: FormEvent| {
                                    if let Ok(v) = evt.value().parse::<i32>() {
                                        new_assign_pts.set(v);
                                    }
                                },
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Deadline Mode" }
                            select {
                                class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                value: new_assign_due_type.read().clone(),
                                onchange: move |evt: FormEvent| new_assign_due_type.set(evt.value()),
                                option { value: "no_deadline", "No Due Date" }
                                option { value: "flexible", "Flexible / Soft Deadline (Allows late submissions)" }
                                option { value: "strict", "Strict Deadline (Closes submissions after due date)" }
                            }
                        }
                        if *new_assign_due_type.read() != "no_deadline" {
                            div { class: "grid gap-1.5",
                                span { class: "font-bold text-foreground text-xs", "Due Date" }
                                Input {
                                    r#type: "date",
                                    value: new_assign_due.read().clone(),
                                    oninput: move |evt: FormEvent| new_assign_due.set(evt.value()),
                                }
                            }
                        }
                        // Teacher assignment prompt document uploader
                        {
                            let is_drag_over = *new_assign_drag_active.read();
                            let staged_file = new_assign_file.read().clone();
                            let drag_border_class = if is_drag_over { "border-primary bg-primary/5 shadow-md scale-[1.01]" } else { "border-border/60 bg-muted/20" };

                            rsx! {
                                div { class: "grid gap-1.5",
                                    span { class: "font-bold text-foreground text-xs", "Assignment Document / Reference Sheet" }
                                    div {
                                        class: "flex flex-col gap-3 p-4 rounded-xl border border-dashed transition-all duration-200 {drag_border_class}",
                                        ondragover: move |evt: DragEvent| {
                                            evt.prevent_default();
                                            new_assign_drag_active.set(true);
                                        },
                                        ondragleave: move |evt: DragEvent| {
                                            evt.prevent_default();
                                            new_assign_drag_active.set(false);
                                        },
                                        ondrop: move |evt: DragEvent| {
                                            evt.prevent_default();
                                            new_assign_drag_active.set(false);
                                            let files = evt.files();
                                            let mut file_sig = new_assign_file;
                                            spawn(async move {
                                                if !files.is_empty() {
                                                    let file_name = files[0].name();
                                                    if let Ok(bytes) = files[0].read_bytes().await {
                                                        let size_str = format_file_size(bytes.len());
                                                        let sha256 = compute_mock_hash(&bytes);
                                                        let base64_str = base64_encode(bytes.as_ref());
                                                        let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                        file_sig.set(Some(AdvancedAttachment {
                                                            filename: file_name,
                                                            size_str,
                                                            sha256,
                                                            e2ee: true,
                                                            dataurl: data_url,
                                                        }));
                                                    }
                                                }
                                            });
                                        },

                                        // Drag and drop / file selector connector
                                        div { class: "border border-dashed border-border/85 rounded-lg p-4 flex flex-col items-center justify-center bg-background/50 hover:bg-background/85 transition-colors cursor-pointer relative",
                                            input {
                                                r#type: "file",
                                                class: "absolute inset-0 opacity-0 cursor-pointer z-10",
                                                onchange: move |evt| {
                                                    let files = evt.files();
                                                    spawn(async move {
                                                        if !files.is_empty() {
                                                            let file_name = files[0].name();
                                                            if let Ok(bytes) = files[0].read_bytes().await {
                                                                let size_str = format_file_size(bytes.len());
                                                                let sha256 = compute_mock_hash(&bytes);
                                                                let base64_str = base64_encode(bytes.as_ref());
                                                                let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                new_assign_file.set(Some(AdvancedAttachment {
                                                                    filename: file_name,
                                                                    size_str,
                                                                    sha256,
                                                                    e2ee: true,
                                                                    dataurl: data_url,
                                                                }));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                            LucideIcon { name: "upload-cloud", class: "h-5 w-5 text-primary mb-1.5" }
                                            span { class: "text-[10px] text-foreground font-semibold", "Drag & Drop document or click to upload" }
                                            span { class: "text-[9px] text-muted-foreground mt-0.5", "Syllabus, prompts, templates, or instructions" }
                                        }

                                        if let Some(ref staged) = staged_file {
                                            div { class: "flex items-center justify-between bg-primary/10 border border-primary/20 px-3 py-2 rounded-lg text-[10px] text-primary font-semibold",
                                                div { class: "flex items-center gap-1.5",
                                                    LucideIcon { name: "file-check", class: "h-4 w-4 text-primary" }
                                                    span { "{staged.filename} ({staged.size_str})" }
                                                }
                                                button {
                                                    class: "bg-transparent border-0 text-primary hover:text-red-500 cursor-pointer p-0.5 rounded transition-colors",
                                                    onclick: move |_| {
                                                        new_assign_file.set(None);
                                                    },
                                                    LucideIcon { name: "x", class: "h-3.5 w-3.5" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_assignment_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                     let active_c = selected_course_id.read().clone();
                                     let uid = user_id.clone();
                                     let ws = ws_id.clone();
                                     let state = state;
                                     let mut db_trigger = db_trigger.clone();
                                     move |_| {
                                         let due_val = match new_assign_due_type.read().as_str() {
                                             "flexible" => format!("{} (Flexible)", new_assign_due.read()),
                                             "strict" => format!("{} (Strict)", new_assign_due.read()),
                                             _ => "No due date".to_string(),
                                         };
                                         let final_desc = match new_assign_file.read().as_ref() {
                                             Some(staged) => format!(
                                                 "{}\n[Attachment: {} | {} | {} | {} | {}]",
                                                 new_assign_desc.read(), staged.filename, staged.size_str, staged.sha256, staged.e2ee, staged.dataurl
                                             ),
                                             None => new_assign_desc.read().clone(),
                                         };
                                         let role = state.active_user_role.read().clone();
                                         let u_id = state.active_user_id.read().clone();
                                         let proof = yntra_core::ZkCryptoTrust::new()
                                             .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                             .ok();
                                         let a = Assignment {
                                             id: uuid::Uuid::new_v4().to_string(),
                                             workspace_id: ws.clone(),
                                             course_id: active_c.clone(),
                                             title: new_assign_title.read().clone(),
                                             description: final_desc,
                                             due_date: due_val,
                                             max_points: *new_assign_pts.read(),
                                             updated_at: 0,
                                         };
                                         let uid_c = uid.clone();
                                         spawn(async move {
                                             let _ = save_assignment(uid_c, a, proof).await;
                                         });
                                        new_assign_title.set(String::new());
                                        new_assign_desc.set(String::new());
                                        new_assign_file.set(None);
                                        show_assignment_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                     }
                                 },
                                "Post Assignment"
                            }
                        }
                    }
                }
            }

            // Enter/Edit Term Grade Modal Dialog
            if *show_grade_modal.read() {
                Dialog {
                    open: *show_grade_modal.read(),
                    title: "Enter Term Grade",
                    onclose: move |_| show_grade_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Term Name" }
                            Input {
                                placeholder: "e.g., Fall 2026",
                                value: grade_term.read().clone(),
                                oninput: move |evt: FormEvent| grade_term.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Final Grade Letter" }
                            Input {
                                placeholder: "e.g., A, B+, C",
                                value: grade_letter.read().clone(),
                                oninput: move |evt: FormEvent| grade_letter.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Final Grade Points" }
                            Input {
                                value: format!("{}", *grade_points.read()),
                                oninput: move |evt: FormEvent| {
                                    if let Ok(v) = evt.value().parse::<i32>() {
                                        grade_points.set(v);
                                    }
                                },
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Teacher Comments" }
                            Input {
                                placeholder: "e.g., Great participant and excellent exam scores.",
                                value: grade_comments.read().clone(),
                                oninput: move |evt: FormEvent| grade_comments.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_grade_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                    let active_c = selected_course_id.read().clone();
                                    let active_s = grade_student_id.read().clone();
                                    let uid = user_id.clone();
                                    let ws = ws_id.clone();
                                    let state = state;
                                    let mut db_trigger = db_trigger.clone();
                                    move |_| {
                                        let role = state.active_user_role.read().clone();
                                        let u_id = state.active_user_id.read().clone();
                                        let proof = yntra_core::ZkCryptoTrust::new()
                                            .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                            .ok();
                                        let tg = TermGrade {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: ws.clone(),
                                            student_id: active_s.clone(),
                                            course_id: active_c.clone(),
                                            term_name: grade_term.read().clone(),
                                            final_grade: Some(grade_letter.read().clone()),
                                            final_points: Some(*grade_points.read()),
                                            teacher_comments: Some(grade_comments.read().clone()),
                                            updated_at: 0,
                                        };
                                        let uid_c = uid.clone();
                                        spawn(async move {
                                            let _ = save_term_grade(uid_c, tg, proof).await;
                                        });
                                        show_grade_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Save Grade"
                            }
                        }
                    }
                }
            }

            // Grade Homework Submission Modal Dialog
            if *show_homework_grade_modal.read() {
                Dialog {
                    open: *show_homework_grade_modal.read(),
                    title: "Grade Homework Submission",
                    onclose: move |_| show_homework_grade_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Grade (e.g. A, B+, Pass, 10/10)" }
                            Input {
                                placeholder: "e.g., A",
                                value: homework_grade.read().clone(),
                                oninput: move |evt: FormEvent| homework_grade.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Feedback / Comments" }
                            Input {
                                placeholder: "e.g., Well done, excellent analysis!",
                                value: homework_feedback.read().clone(),
                                oninput: move |evt: FormEvent| homework_feedback.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_homework_grade_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                    let uid = user_id.clone();
                                    let ws = ws_id.clone();
                                    let state = state;
                                    let mut db_trigger = db_trigger.clone();
                                    let sub_opt = selected_homework_sub.read().clone();
                                    move |_| {
                                        if let Some(sub_rec) = sub_opt.clone() {
                                            let role = state.active_user_role.read().clone();
                                            let u_id = state.active_user_id.read().clone();
                                            let proof = yntra_core::ZkCryptoTrust::new()
                                                .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                .ok();
                                            let mut updated_sub = sub_rec.clone();
                                            updated_sub.grade = Some(homework_grade.read().clone());
                                            updated_sub.feedback = Some(homework_feedback.read().clone());
                                            
                                            let uid_c = uid.clone();
                                            spawn(async move {
                                                let _ = save_submission(uid_c, updated_sub, proof).await;
                                            });
                                            show_homework_grade_modal.set(false);
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        }
                                    }
                                },
                                "Save Homework Grade"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_timestamp(ms: i64) -> String {
    let seconds = ms / 1000;
    if let Some(dt) = chrono::DateTime::from_timestamp(seconds, 0) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Just now".to_string()
    }
}
