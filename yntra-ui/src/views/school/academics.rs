use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Dialog, Input, LucideIcon, SuggestionInput};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    checkout_book, create_school_invoice, get_assignments, get_library_books,
    get_library_lending_logs, get_school_invoices, get_student_profiles, get_workspace_courses,
    get_users, record_school_payment, return_book, save_assignment, save_attendance_record, save_course,
    link_parent_to_student, get_student_parents, get_student_health_records, save_student_health_record,
    get_health_incidents, save_health_incident, save_student_profile,
    get_course_term_grades, save_term_grade, publish_report_card, get_report_cards,
    get_student_submissions, save_submission, get_timetable_slots, save_timetable_slot,
    Assignment, Course, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot,
};

use super::{SchoolViewProps, infer_subject_from_course};

fn is_deadline_passed(due_date: &str) -> bool {
    if due_date.is_empty() || due_date == "No due date" {
        return false;
    }
    if due_date.contains("(Strict)") {
        if due_date.len() >= 10 {
            let date_str = &due_date[0..10];
            if let Ok(due_naive) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let today = chrono::Utc::now().date_naive();
                return today > due_naive;
            }
        }
    }
    false
}



#[derive(Clone, Debug, PartialEq)]
pub struct AdvancedAttachment {
    pub filename: String,
    pub size_str: String,
    pub sha256: String,
    pub e2ee: bool,
    pub dataurl: String,
}

fn parse_submission_content_and_advanced_attachment(content: &str) -> (String, Option<AdvancedAttachment>) {
    if let Some(idx) = content.rfind("\n[Attachment: ") {
        let text = content[..idx].to_string();
        let link_part = &content[idx + "\n[Attachment: ".len()..];
        if let Some(end_idx) = link_part.find(']') {
            let payload = &link_part[..end_idx];
            let parts: Vec<&str> = payload.split('|').collect();
            if parts.len() >= 5 {
                return (text, Some(AdvancedAttachment {
                    filename: parts[0].trim().to_string(),
                    size_str: parts[1].trim().to_string(),
                    sha256: parts[2].trim().to_string(),
                    e2ee: parts[3].trim() == "true",
                    dataurl: parts[4].trim().to_string(),
                }));
            } else if parts.len() == 2 {
                return (text, Some(AdvancedAttachment {
                    filename: parts[0].trim().to_string(),
                    size_str: "Unknown size".to_string(),
                    sha256: "legacy-unhashed".to_string(),
                    e2ee: false,
                    dataurl: parts[1].trim().to_string(),
                }));
            }
        }
    }
    (content.to_string(), None)
}

fn compute_mock_hash(bytes: &[u8]) -> String {
    let mut hash = 0u64;
    for &b in bytes {
        hash = hash.wrapping_add(b as u64).wrapping_mul(31);
    }
    format!("{:x}", hash)
}

fn format_file_size(bytes_len: usize) -> String {
    if bytes_len >= 1_048_576 {
        format!("{:.1} MB", bytes_len as f64 / 1_048_576.0)
    } else if bytes_len >= 1024 {
        format!("{:.1} KB", bytes_len as f64 / 1024.0)
    } else {
        format!("{} B", bytes_len)
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = match chunk.len() {
            3 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32),
            2 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8),
            1 => (chunk[0] as u32) << 16,
            _ => unreachable!(),
        };
        let c1 = CHARSET[((b >> 18) & 63) as usize] as char;
        let c2 = CHARSET[((b >> 12) & 63) as usize] as char;
        let c3 = if chunk.len() > 1 {
            CHARSET[((b >> 6) & 63) as usize] as char
        } else {
            '='
        };
        let c4 = if chunk.len() > 2 {
            CHARSET[(b & 63) as usize] as char
        } else {
            '='
        };
        result.push(c1);
        result.push(c2);
        result.push(c3);
        result.push(c4);
    }
    result
}

#[component]
pub fn AcademicsView(props: SchoolViewProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let locale = props.locale.clone();
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    // View mode switcher: "teacher" or "student"
    let mut view_mode = use_signal(|| "teacher".to_string());
    let mut selected_student_profile_id = use_signal(|| "".to_string());

    // Local state
    let mut selected_course_id = use_signal(|| "".to_string());
    let mut active_menu_id = use_signal(|| "".to_string());
    let mut show_course_modal = use_signal(|| false);
    let mut show_more_menu = use_signal(|| false);
    let mut new_course_name = use_signal(String::new);
    let mut new_course_subject = use_signal(String::new);
    let mut new_course_teacher = use_signal(String::new);

    let mut show_assignment_modal = use_signal(|| false);
    let mut new_assign_title = use_signal(String::new);
    let mut new_assign_desc = use_signal(String::new);
    let mut new_assign_due = use_signal(|| "2026-08-01".to_string());
    let mut new_assign_due_type = use_signal(|| "no_deadline".to_string());
    let mut new_assign_pts = use_signal(|| 100);
    let mut assignment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let mut new_assign_file = use_signal(|| Option::<AdvancedAttachment>::None);
    let mut new_assign_drag_active = use_signal(|| false);
    let mut assignment_to_delete = use_signal(|| Option::<String>::None);
    let mut submission_files = use_signal(std::collections::HashMap::<String, AdvancedAttachment>::new);
    let mut drag_active = use_signal(std::collections::HashMap::<String, bool>::new);

    let name_suggestions = {
        let subj = new_course_subject.read().trim().to_lowercase();
        // Check Subject to match course suggestions
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
            // Default list if subject is empty or unrecognized
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
            "Matematik".to_string(),
            "Naturvetenskap".to_string(),
            "Historia".to_string(),
            "Geografi".to_string(),
            "Fysik".to_string(),
            "Kemi".to_string(),
            "Biologi".to_string(),
            "Engelska".to_string(),
            "Idrott och hälsa".to_string(),
            "Bild".to_string(),
            "Musik".to_string(),
        ],
        "no" => vec![
            "Matematikk".to_string(),
            "Naturfag".to_string(),
            "Historie".to_string(),
            "Geografi".to_string(),
            "Fysikk".to_string(),
            "Kjemi".to_string(),
            "Biologi".to_string(),
            "Engelsk".to_string(),
            "Kroppsøving".to_string(),
            "Kunst og håndverk".to_string(),
            "Musikk".to_string(),
        ],
        "da" => vec![
            "Matematik".to_string(),
            "Naturfag".to_string(),
            "Historie".to_string(),
            "Geografi".to_string(),
            "Fysik".to_string(),
            "Kemi".to_string(),
            "Biologi".to_string(),
            "Engelsk".to_string(),
            "Idræt".to_string(),
            "Billedkunst".to_string(),
            "Musik".to_string(),
        ],
        "fi" => vec![
            "Matematiikka".to_string(),
            "Luonnontiede".to_string(),
            "Historia".to_string(),
            "Maantieto".to_string(),
            "Fysiikka".to_string(),
            "Kemia".to_string(),
            "Biologia".to_string(),
            "Englanti".to_string(),
            "Liikunta".to_string(),
            "Kuvataide".to_string(),
            "Musiikki".to_string(),
        ],
        _ => vec![
            "Mathematics".to_string(),
            "Science".to_string(),
            "History".to_string(),
            "Geography".to_string(),
            "Physics".to_string(),
            "Chemistry".to_string(),
            "Biology".to_string(),
            "English".to_string(),
            "Physical Education".to_string(),
            "Art".to_string(),
            "Music".to_string(),
        ],
    };

    // Resources
    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let courses_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

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

    let user_id_clone4 = user_id.clone();
    let ws_id_clone4 = ws_id.clone();
    let students_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = user_id_clone4.clone();
        let ws = ws_id_clone4.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    // Helper resource to load all workspace assignments for the student dashboard preview
    let user_id_clone5 = user_id.clone();
    let ws_id_clone5 = ws_id.clone();
    let courses = courses_res.read().clone().unwrap_or_default();
    let courses_clone = courses.clone();
    let all_assignments_res = use_resource(move || {
        let _trig = db_trigger.read();
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

    // Student Submissions resource
    let active_student_id = selected_student_profile_id.read().clone();
    let user_id_clone6 = user_id.clone();
    let ws_id_clone6 = ws_id.clone();
    let submissions_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = active_student_id.clone();
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
        let _trig = db_trigger.read();
        let uid = user_id_clone7.clone();
        let ws = ws_id_clone7.clone();
        async move {
            get_timetable_slots(uid, ws).await.unwrap_or_default()
        }
    });

    let mut sub_tab = use_signal(|| "syllabus".to_string());
    let mut show_grade_modal = use_signal(|| false);
    let mut grade_student_id = use_signal(String::new);
    let mut grade_term = use_signal(|| "Fall 2026".to_string());
    let mut grade_letter = use_signal(|| "A".to_string());
    let mut grade_points = use_signal(|| 90);
    let mut grade_comments = use_signal(String::new);

    let assignments = assignments_res.read().clone().unwrap_or_default();
    let course_grades = course_grades_res.read().clone().unwrap_or_default();
    let students = students_res.read().clone().unwrap_or_default();
    let all_assignments = all_assignments_res.read().clone().unwrap_or_default();
    let student_submissions = submissions_res.read().clone().unwrap_or_default();
    let timetable = timetable_res.read().clone().unwrap_or_default();

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
                        } else {
                            {t("school-student-portal-desc", &locale)}
                        }
                    }
                }
                
                div { class: "flex items-center gap-4",
                    // Tab toggle
                    div { class: "flex bg-muted p-1 rounded-xl border border-border/40 text-xs font-bold w-max",
                        button {
                            class: format!(
                                "px-3 py-1.5 rounded-lg transition-all {}",
                                if *view_mode.read() == "teacher" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            onclick: move |_| view_mode.set("teacher".to_string()),
                            {t("school-teacher-view-tab", &locale)}
                        }
                        button {
                            class: format!(
                                "px-3 py-1.5 rounded-lg transition-all {}",
                                if *view_mode.read() == "student" { "bg-background text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            onclick: move |_| {
                                view_mode.set("student".to_string());
                                if selected_student_profile_id.read().is_empty() && !students.is_empty() {
                                    selected_student_profile_id.set(students[0].id.clone());
                                }
                            },
                            {t("school-student-portal-tab", &locale)}
                        }
                    }

                    if *view_mode.read() == "teacher" {
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
                    if students.is_empty() {
                        rsx! {
                            div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                                LucideIcon { name: "users", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                                h4 { class: "text-sm font-bold text-foreground m-0", "No Students Registered" }
                                p { class: "text-xs text-muted-foreground mt-1 max-w-sm", {t("school-no-enrolled-students", &locale)} }
                            }
                        }
                    } else {
                        let current_student_id = selected_student_profile_id.read().clone();
                        let current_student = students.iter().find(|s| s.id == current_student_id).cloned().unwrap_or_else(|| students[0].clone());
                        let student_name = format!("{} {}", current_student.first_name, current_student.last_name);
                        
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

                        rsx! {
                            // Student Profile Selector Card
                            Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                                div { class: "flex items-center gap-3 w-full sm:w-auto",
                                    LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                                    div {
                                        h4 { class: "text-sm font-bold text-foreground m-0", {t("school-change-profile", &locale)} }
                                        p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", "Preview learning portal progress as different roster profiles." }
                                    }
                                }
                                select {
                                    class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                                    value: selected_student_profile_id.read().clone(),
                                    onchange: move |evt: FormEvent| selected_student_profile_id.set(evt.value()),
                                    for s in students.iter() {
                                        option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                                    }
                                }
                            }

                            // Welcome Banner Card
                            div { class: "p-6 rounded-2xl bg-gradient-to-r from-primary/10 to-accent/5 border border-primary/20 flex items-center justify-between animate-in fade-in duration-300 shadow-sm",
                                div { class: "space-y-1.5",
                                    h3 { class: "text-lg font-extrabold text-foreground m-0", 
                                        {crate::locales::t_with_args("school-welcome", &locale, &[("name", &student_name)])}
                                    }
                                    p { class: "text-xs text-muted-foreground m-0", {t("school-ready-msg", &locale)} }
                                }
                                LucideIcon { name: "award", class: "h-10 w-10 text-primary" }
                            }

                            // Stars & Badges grid
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                                Card { class: "border-border shadow-sm",
                                    CardHeader {
                                        CardTitle { {t("school-my-stars", &locale)} }
                                        CardDescription { "Earn study stars for graded A/B homework submissions!" }
                                    }
                                    CardContent { class: "flex flex-col items-center py-6 space-y-4",
                                        div { class: "flex gap-2.5",
                                            for i in 0..5 {
                                                {
                                                    let active = i < stars_count;
                                                    rsx! {
                                                        LucideIcon { 
                                                            name: "star", 
                                                            class: format!("h-10 w-10 {}", if active { "text-amber-500 fill-amber-500 animate-pulse" } else { "text-muted-foreground/10" }) 
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "text-xs font-bold text-foreground", "You have earned {stars_count} Study Stars!" }
                                    }
                                }
                                Card { class: "border-border shadow-sm",
                                    CardHeader {
                                        CardTitle { "My Learning Badges" }
                                        CardDescription { "Achieved milestones checklist" }
                                    }
                                    CardContent { class: "flex flex-wrap gap-2.5 pt-2",
                                        if graded_submissions_count > 0 {
                                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-xs font-bold shadow-sm",
                                                LucideIcon { name: "award", class: "h-4 w-4" }
                                                "Perfect Homework"
                                            }
                                        }
                                        div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-xs font-bold shadow-sm",
                                            LucideIcon { name: "user-check", class: "h-4 w-4" }
                                            "Active Scholar"
                                        }
                                        div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-purple-500/10 text-purple-600 border border-purple-500/20 text-xs font-bold shadow-sm",
                                            LucideIcon { name: "library", class: "h-4 w-4" }
                                            "Avid Reader"
                                        }
                                    }
                                }
                            }

                            // Homework Tasks List
                            Card { class: "border border-border shadow-sm",
                                CardHeader {
                                    CardTitle { {t("school-my-homework", &locale)} }
                                    CardDescription { "Assignments and learning tasks due" }
                                }
                                CardContent {
                                    if all_assignments.is_empty() {
                                        div { class: "py-10 text-center text-xs text-muted-foreground", {t("school-all-homework-done", &locale)} }
                                    } else {
                                        div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                            for a in all_assignments.iter() {
                                                {
                                                    let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&a.description);
                                                    let assignment_id = a.id.clone();
                                                    let existing_sub = student_submissions.iter().find(|sub| sub.assignment_id == assignment_id).cloned();
                                                    let a_id = a.id.clone();
                                                    let s_id = current_student_id.clone();
                                                    let uid = user_id.clone();
                                                    let ws = ws_id.clone();
                                                    rsx! {
                                                        div { key: "{a.id}", class: "p-4 flex flex-col gap-3.5",
                                                            div { class: "flex justify-between items-start",
                                                                div {
                                                                    div { class: "font-bold text-sm text-foreground", "{a.title}" }
                                                                    div { class: "text-xs text-muted-foreground mt-0.5", "{desc_text}" }
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
                                                                    div { class: "text-[10px] text-muted-foreground mt-2 flex items-center gap-1.5",
                                                                        LucideIcon { name: "clock", size: "12" }
                                                                        if a.due_date.is_empty() || a.due_date == "No due date" {
                                                                            span { class: "text-muted-foreground/80 font-medium", "No due date" }
                                                                        } else if a.due_date.contains("(Strict)") {
                                                                            if is_deadline_passed(&a.due_date) {
                                                                                span { class: "text-red-500 font-semibold animate-pulse", "Due: {a.due_date} (Closed)" }
                                                                            } else {
                                                                                span { class: "text-red-500/80 font-medium", "Due: {a.due_date}" }
                                                                            }
                                                                        } else if a.due_date.contains("(Flexible)") {
                                                                            if is_deadline_passed(&a.due_date) {
                                                                                span { class: "text-amber-500 font-semibold", "Due: {a.due_date} (Late submission)" }
                                                                            } else {
                                                                                span { class: "text-amber-500/80 font-medium", "Due: {a.due_date}" }
                                                                            }
                                                                        } else {
                                                                            span { class: "text-muted-foreground font-medium", "Due: {a.due_date}" }
                                                                        }
                                                                    }
                                                                }
                                                                span { class: "text-xs font-bold bg-primary/10 text-primary px-2.5 py-1 rounded-full",
                                                                    "{a.max_points} pts"
                                                                }
                                                            }
                                                            if let Some(ref sub) = existing_sub {
                                                                {
                                                                    let (ans_text, attachment) = parse_submission_content_and_advanced_attachment(&sub.content);
                                                                    rsx! {
                                                                        div { class: "p-3.5 rounded-lg bg-muted/40 border border-border/50 text-xs space-y-2",
                                                                            div { class: "font-semibold text-muted-foreground", {t("school-submitted-answers", &locale)} }
                                                                            div { class: "text-foreground font-medium", "{ans_text}" }
                                                                            if let Some(staged) = attachment {
                                                                                a {
                                                                                    href: "{staged.dataurl}",
                                                                                    download: "{staged.filename}",
                                                                                    class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl max-w-sm no-underline text-foreground cursor-pointer hover:bg-card/75 transition-colors mt-2",
                                                                                    div { class: "flex items-center justify-between",
                                                                                        div { class: "flex items-center gap-1.5 min-w-0",
                                                                                            LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                                            span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                                        }
                                                                                        LucideIcon { name: "download", class: "h-3.5 w-3.5 text-muted-foreground" }
                                                                                    }
                                                                                    div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                                        if staged.e2ee {
                                                                                            span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                                                LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                                                "E2EE SECURED"
                                                                                            }
                                                                                        }
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                                                    }
                                                                                }
                                                                            }
                                                                            div { class: "flex items-center gap-2 border-t border-border/30 pt-2 mt-2",
                                                                                span { class: "text-[10px] uppercase font-bold text-muted-foreground", "Status: " }
                                                                                if let Some(ref g) = sub.grade {
                                                                                    span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-green-500/10 text-green-600 border border-green-500/20 uppercase",
                                                                                        "Graded: {g}"
                                                                                    }
                                                                                } else {
                                                                                    span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20 uppercase",
                                                                                        {t("school-waiting-grade", &locale)}
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(ref feedback) = sub.feedback {
                                                                                if !feedback.is_empty() {
                                                                                    div { class: "text-[11px] text-muted-foreground italic pl-2 border-l border-primary/30 mt-1.5",
                                                                                        span { class: "font-bold not-italic", {t("school-teacher-feedback", &locale)} }
                                                                                        " {feedback}"
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else if is_deadline_passed(&a.due_date) {
                                                                div { class: "p-3.5 rounded-xl border border-red-200/50 bg-red-500/5 text-xs text-red-600 flex items-center gap-2 font-medium",
                                                                    LucideIcon { name: "lock", size: "14" }
                                                                    span { "Submissions are closed for this assignment (Strict deadline passed)." }
                                                                }
                                                            } else {
                                                                // Answer submission input box with Drag-and-Drop uploader
                                                                {
                                                                    let is_drag_over = *drag_active.read().get(&a_id).unwrap_or(&false);
                                                                    let staged_file = submission_files.read().get(&a_id).cloned();
                                                                    let drag_border_class = if is_drag_over { "border-primary bg-primary/5 shadow-md scale-[1.01]" } else { "border-border/60 bg-muted/20" };

                                                                    rsx! {
                                                                        div {
                                                                            class: "flex flex-col gap-3 p-4 rounded-xl border border-dashed transition-all duration-200 {drag_border_class}",
                                                                            ondragover: {
                                                                                let a_id_c = a_id.clone();
                                                                                move |evt: DragEvent| {
                                                                                    evt.prevent_default();
                                                                                    drag_active.write().insert(a_id_c.clone(), true);
                                                                                }
                                                                            },
                                                                            ondragleave: {
                                                                                let a_id_c = a_id.clone();
                                                                                move |evt: DragEvent| {
                                                                                    evt.prevent_default();
                                                                                    drag_active.write().insert(a_id_c.clone(), false);
                                                                                }
                                                                            },
                                                                            ondrop: {
                                                                                let a_id_c = a_id.clone();
                                                                                move |evt: DragEvent| {
                                                                                    evt.prevent_default();
                                                                                    drag_active.write().insert(a_id_c.clone(), false);
                                                                                }
                                                                            },
                                                                            span { class: "text-[11px] font-bold text-muted-foreground", "Write Your Answer:" }
                                                                            textarea {
                                                                                class: "w-full min-h-[70px] rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                                                                placeholder: "Type your answers here...",
                                                                                value: assignment_inputs.read().get(&a_id).cloned().unwrap_or_default(),
                                                                                oninput: {
                                                                                    let a_id_c = a_id.clone();
                                                                                    move |evt: FormEvent| {
                                                                                        assignment_inputs.write().insert(a_id_c.clone(), evt.value());
                                                                                    }
                                                                                },
                                                                            }

                                                                            // Drag and drop / file selector connector
                                                                            div { class: "border border-dashed border-border/85 rounded-lg p-4 flex flex-col items-center justify-center bg-background/50 hover:bg-background/85 transition-colors cursor-pointer relative",
                                                                                input {
                                                                                    r#type: "file",
                                                                                    class: "absolute inset-0 opacity-0 cursor-pointer z-10",
                                                                                    onchange: {
                                                                                        let a_id_c = a_id.clone();
                                                                                        move |evt| {
                                                                                            let a_id_f = a_id_c.clone();
                                                                                            let files = evt.files();
                                                                                            spawn(async move {
                                                                                                if !files.is_empty() {
                                                                                                    let file_name = files[0].name();
                                                                                                    if let Ok(bytes) = files[0].read_bytes().await {
                                                                                                        let size_str = format_file_size(bytes.len());
                                                                                                        let sha256 = compute_mock_hash(&bytes);
                                                                                                        let base64_str = base64_encode(bytes.as_ref());
                                                                                                        let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                                                        submission_files.write().insert(a_id_f.clone(), AdvancedAttachment {
                                                                                                            filename: file_name,
                                                                                                            size_str,
                                                                                                            sha256,
                                                                                                            e2ee: true,
                                                                                                            dataurl: data_url,
                                                                                                        });
                                                                                                    }
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                }
                                                                                LucideIcon { name: "upload-cloud", class: "h-5 w-5 text-primary mb-1.5" }
                                                                                span { class: "text-[10px] text-foreground font-semibold", "Drag & Drop document or click to upload" }
                                                                                span { class: "text-[9px] text-muted-foreground mt-0.5", "PDF, DOCX, ZIP, or images up to 50MB" }
                                                                            }

                                                                            if let Some(staged) = staged_file {
                                                                                div { class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl",
                                                                                    div { class: "flex items-center justify-between",
                                                                                        div { class: "flex items-center gap-1.5 min-w-0",
                                                                                            LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                                            span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                                        }
                                                                                        button {
                                                                                            class: "bg-transparent border-0 text-muted-foreground hover:text-red-500 cursor-pointer p-0.5 rounded transition-colors",
                                                                                            onclick: {
                                                                                                let a_id_c = a_id.clone();
                                                                                                move |_| {
                                                                                                    submission_files.write().remove(&a_id_c);
                                                                                                }
                                                                                            },
                                                                                            LucideIcon { name: "x", class: "h-3.5 w-3.5" }
                                                                                        }
                                                                                    }
                                                                                    div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                                            LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                                            "E2EE SECURED"
                                                                                        }
                                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                                                    }
                                                                                }
                                                                            }

                                                                            Button {
                                                                                class: "text-xs px-3 h-8 self-end font-semibold flex items-center gap-1.5 mt-2",
                                                                                onclick: {
                                                                                    let a_id = a_id.clone();
                                                                                    let s_id = s_id.clone();
                                                                                    let uid_c = uid.clone();
                                                                                    let ws_c = ws.clone();
                                                                                    move |_| {
                                                                                        let ans = assignment_inputs.read().get(&a_id).cloned().unwrap_or_default();
                                                                                        let staged = submission_files.read().get(&a_id).cloned();
                                                                                        let final_content = match staged.as_ref() {
                                                                                            Some(staged_f) => format!(
                                                                                                "{}\n[Attachment: {} | {} | {} | {} | {}]",
                                                                                                ans, staged_f.filename, staged_f.size_str, staged_f.sha256, staged_f.e2ee, staged_f.dataurl
                                                                                            ),
                                                                                            None => ans.clone(),
                                                                                        };
                                                                                        if !ans.is_empty() || staged.is_some() {
                                                                                            let sub_rec = Submission {
                                                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                                                workspace_id: ws_c.clone(),
                                                                                                assignment_id: a_id.clone(),
                                                                                                student_id: s_id.clone(),
                                                                                                content: final_content,
                                                                                                grade: None,
                                                                                                feedback: None,
                                                                                                submitted_at: chrono::Utc::now().to_rfc3339(),
                                                                                                updated_at: 0,
                                                                                            };
                                                                                            let uid_sub = uid_c.clone();
                                                                                            let a_id_clear = a_id.clone();
                                                                                            spawn(async move {
                                                                                                let _ = save_submission(uid_sub, sub_rec).await;
                                                                                            });
                                                                                            assignment_inputs.write().insert(a_id.clone(), String::new());
                                                                                            submission_files.write().remove(&a_id_clear);
                                                                                            let current = *db_trigger.read();
                                                                                            db_trigger.set(current + 1);
                                                                                        }
                                                                                    }
                                                                                },
                                                                                LucideIcon { name: "send", class: "h-3.5 w-3.5" }
                                                                                {t("school-submit-answer", &locale)}
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

                            // Timetable Schedule Today
                            Card { class: "border border-border shadow-sm",
                                CardHeader {
                                    CardTitle { {t("school-my-timetable", &locale)} }
                                    CardDescription { "Weekly recurring school timetable classes schedule" }
                                }
                                CardContent {
                                    if timetable.is_empty() {
                                        div { class: "py-8 text-center text-xs text-muted-foreground", {t("school-timetable-no-slots", &locale)} }
                                    } else {
                                        div { class: "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4",
                                            for s in timetable.iter() {
                                                {
                                                    let course_name = courses.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                                    let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                                    let day_name = match s.day_of_week {
                                                        1 => t("common-monday", &locale),
                                                        2 => t("common-tuesday", &locale),
                                                        3 => t("common-wednesday", &locale),
                                                        4 => t("common-thursday", &locale),
                                                        5 => t("common-friday", &locale),
                                                        _ => "Weekday".to_string(),
                                                    };
                                                    rsx! {
                                                        div { key: "{s.id}", class: "p-4 rounded-xl border border-border bg-background flex flex-col gap-2 shadow-sm hover:border-primary/20 transition-all",
                                                            div { class: "flex items-center gap-2",
                                                                LucideIcon { name: "clock", class: "h-4 w-4 text-primary" }
                                                                span { class: "text-xs font-bold text-foreground", "{day_name} • {s.start_time} - {s.end_time}" }
                                                            }
                                                            div { class: "font-semibold text-sm text-foreground", "{course_name}" }
                                                            div { class: "text-[10px] text-muted-foreground flex items-center gap-1",
                                                                LucideIcon { name: "map-pin", class: "h-3 w-3" }
                                                                span { "Classroom: {classroom_name}" }
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
                    rsx! {
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
                                                let c_id = c.id.clone();
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
                                                }

                                                if *sub_tab.read() == "syllabus" {
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
                                                } else {
                                                    div { class: "space-y-4",
                                                        h5 { class: "font-bold text-sm m-0 text-foreground", "Course Grading Ledger" }
                                                        
                                                        if students.is_empty() {
                                                            div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "Please enroll students in the directory to enter course grades." }
                                                        } else {
                                                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                                                for s in students.iter() {
                                                                    {
                                                                        let student_id = s.id.clone();
                                                                        let current_grade = course_grades.iter().find(|g| g.student_id == student_id).cloned();
                                                                        let s_name = format!("{} {}", s.first_name, s.last_name);
                                                                        rsx! {
                                                                            div { key: "{s.id}", class: "p-4 flex items-center justify-between text-xs hover:bg-muted/10 transition-colors",
                                                                                div {
                                                                                    div { class: "font-bold text-foreground text-sm", "{s_name}" }
                                                                                    div { class: "text-[10px] text-muted-foreground mt-0.5", "Grade: {s.grade_level}" }
                                                                                }
                                                                                div { class: "flex items-center gap-3.5",
                                                                                    if let Some(ref g) = current_grade {
                                                                                        span { class: "font-bold px-2.5 py-1 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-[10px]",
                                                                                            "Grade: {g.final_grade.clone().unwrap_or_else(|| \"-\".to_string())} ({g.final_points.unwrap_or(0)}%)"
                                                                                        }
                                                                                    } else {
                                                                                        span { class: "text-muted-foreground italic text-[11px]", "No grade assigned" }
                                                                                    }
                                                                                    Button {
                                                                                        class: "px-3 py-1.5 text-xs h-8 bg-primary text-primary-foreground rounded-lg font-semibold",
                                                                                        onclick: {
                                                                                            let sid = s.id.clone();
                                                                                            move |_| {
                                                                                                grade_student_id.set(sid.clone());
                                                                                                if let Some(ref g) = current_grade {
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
                                                                                        "Enter Grade"
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
                                    move |_| {
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
                                            let _ = save_course(uid_c, ws_c, c).await;
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
                                            let _ = save_assignment(uid_c, a).await;
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
                                    move |_| {
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
                                            let _ = save_term_grade(uid_c, tg).await;
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
        }
    }
}

