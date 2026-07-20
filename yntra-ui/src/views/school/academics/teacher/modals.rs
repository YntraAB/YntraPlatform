use dioxus::prelude::*;
use dioxus::html::HasFileData;
use crate::components::{Button, Dialog, Input, SuggestionInput, LucideIcon};
use yntra_core::{Course, Assignment, TermGrade, Submission, save_course, save_assignment, save_blob, save_term_grade, save_submission};
use super::super::infer_subject_from_course;
use super::super::utils::{format_file_size, compute_mock_hash, base64_encode, AdvancedAttachment};

#[component]
pub fn CreateCourseModal(
    mut show_course_modal: Signal<bool>,
    user_id: String,
    ws_id: String,
    locale: String,
    mut db_trigger: Signal<u32>,
    teacher_suggestions: Vec<String>,
) -> Element {
    let mut new_course_name = use_signal(String::new);
    let mut new_course_subject = use_signal(String::new);
    let mut new_course_teacher = use_signal(String::new);
    let state = use_context::<crate::state::AppState>();

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
                        suggestions: name_suggestions,
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
                        suggestions: subject_suggestions,
                        onchange: move |val| new_course_subject.set(val),
                    }
                }
                div { class: "grid gap-1.5",
                    span { class: "font-bold text-foreground text-xs", "Teacher Name" }
                    SuggestionInput {
                        placeholder: "e.g., Dr. Euler",
                        value: new_course_teacher.read().clone(),
                        suggestions: teacher_suggestions,
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
}

#[component]
pub fn CreateAssignmentModal(
    mut show_assignment_modal: Signal<bool>,
    user_id: String,
    ws_id: String,
    selected_course_id: String,
    mut db_trigger: Signal<u32>,
) -> Element {
    let mut new_assign_title = use_signal(String::new);
    let mut new_assign_desc = use_signal(String::new);
    let mut new_assign_due = use_signal(|| "2026-08-01".to_string());
    let mut new_assign_due_type = use_signal(|| "no_deadline".to_string());
    let mut new_assign_pts = use_signal(|| 100);
    let mut new_assign_file = use_signal(|| Option::<AdvancedAttachment>::None);
    let mut new_assign_drag_active = use_signal(|| false);
    let state = use_context::<crate::state::AppState>();

    let is_drag_over = *new_assign_drag_active.read();
    let staged_file = new_assign_file.read().clone();
    let drag_border_class = if is_drag_over { "border-primary bg-primary/5 shadow-md scale-[1.01]" } else { "border-border/60 bg-muted/20" };

    rsx! {
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
                div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                    Button {
                        class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                        onclick: move |_| show_assignment_modal.set(false),
                        "Cancel"
                    }
                    Button {
                        class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                        onclick: {
                            let active_c = selected_course_id.clone();
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
                                        "{}\n[Attachment: {} | {} | {} | {} | blob://{}]",
                                        new_assign_desc.read(), staged.filename, staged.size_str, staged.sha256, staged.e2ee, staged.sha256
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
                                let ws_blob = ws.clone();
                                let staged_blob = new_assign_file.read().clone();
                                spawn(async move {
                                    if let Some(staged) = staged_blob {
                                        let _ = yntra_core::save_blob(uid_c.clone(), staged.sha256, ws_blob, staged.dataurl).await;
                                    }
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
}

#[component]
pub fn TermGradeModal(
    mut show_grade_modal: Signal<bool>,
    grade_student_id: Signal<String>,
    mut grade_term: Signal<String>,
    mut grade_letter: Signal<String>,
    mut grade_points: Signal<i32>,
    mut grade_comments: Signal<String>,
    user_id: String,
    ws_id: String,
    selected_course_id: String,
    mut db_trigger: Signal<u32>,
) -> Element {
    let state = use_context::<crate::state::AppState>();

    rsx! {
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
                            let active_c = selected_course_id.clone();
                            let active_s = grade_student_id.read().clone();
                            let uid = user_id.clone();
                            let ws = ws_id.clone();
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
}

#[component]
pub fn HomeworkGradeModal(
    mut show_homework_grade_modal: Signal<bool>,
    selected_homework_sub: Signal<Option<Submission>>,
    mut homework_grade: Signal<String>,
    mut homework_feedback: Signal<String>,
    user_id: String,
    ws_id: String,
    mut db_trigger: Signal<u32>,
) -> Element {
    let state = use_context::<crate::state::AppState>();

    rsx! {
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
