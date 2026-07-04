use dioxus::prelude::*;
use crate::components;
use crate::locales;
use yntra_core::{StudentProfile, Course};

#[derive(Props, Clone)]
pub struct ReportCardsGradingProps {
    pub active_user_id: String,
    pub students: Vec<StudentProfile>,
    pub courses: Vec<Course>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for ReportCardsGradingProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ReportCardsGrading(props: ReportCardsGradingProps) -> Element {
    let active_user_id = props.active_user_id.clone();
    let students = props.students.clone();
    let courses = props.courses.clone();
    let workspace_id = props.workspace_id.clone();
    let mut db_trigger = props.db_trigger;
    let locale = props.locale.clone();

    // Active Term filter state
    let mut active_term = use_signal(|| "Fall 2026".to_string());

    // Selected Student State
    let mut selected_student_id = use_signal(|| students.first().map(|s| s.id.clone()));
    let selected_student = students
        .iter()
        .find(|s| Some(s.id.clone()) == *selected_student_id.read())
        .or(students.first())
        .cloned();

    // Fetch term grades for selected student
    let db_trig = *db_trigger.read();
    let stud_id_for_grades = selected_student_id.read().clone();
    let term_for_grades = active_term.read().clone();
    let req_uid = active_user_id.clone();
    let term_grades_res = use_resource(move || {
        let _ = db_trig;
        let s_id = stud_id_for_grades.clone();
        let t_name = term_for_grades.clone();
        let uid = req_uid.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_term_grades(uid, id, t_name).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let term_grades = term_grades_res.read().clone().unwrap_or_default();

    let req_rc_uid = active_user_id.clone();
    let report_cards_res = use_resource(move || {
        let _ = db_trig;
        let uid = req_rc_uid.clone();
        async move {
            let mut list = Vec::new();
            let students_list = yntra_core::get_students(uid.clone()).await.unwrap_or_default();
            for s in students_list {
                if let Ok(cards) = yntra_core::get_report_cards(uid.clone(), s.id.clone()).await {
                    list.extend(cards);
                }
            }
            list
        }
    });
    let all_report_cards = report_cards_res.read().clone().unwrap_or_default();

    // Form inputs state for selected course
    let mut selected_course_id = use_signal(|| courses.first().map(|c| c.id.clone()).unwrap_or_default());
    let mut form_grade = use_signal(|| "A".to_string());
    let mut form_points = use_signal(|| "90".to_string());
    let mut form_comments = use_signal(String::new);
    let mut save_success = use_signal(|| false);

    // Report card advisor remarks state
    let active_report_card = selected_student.as_ref().and_then(|s| {
        all_report_cards.iter().find(|rc| rc.student_id == s.id && rc.term_name == *active_term.read()).cloned()
    });
    let mut advisor_comments = use_signal(|| {
        active_report_card.as_ref().and_then(|rc| rc.principal_comments.clone()).unwrap_or_default()
    });
    
    // Sync advisor comments state when student or report card changes
    use_effect(use_reactive(&active_report_card, move |rc| {
        advisor_comments.set(rc.and_then(|c| c.principal_comments.clone()).unwrap_or_default());
    }));

    rsx! {
        div { class: "flex flex-col gap-6",
            // Hero Title Card
            div {
                class: "relative p-6 rounded-2xl overflow-hidden border border-primary/20 shadow-lg flex flex-col md:flex-row justify-between items-start md:items-center gap-4 bg-gradient-to-r from-primary/10 via-purple-500/5 to-indigo-500/10",
                div { class: "flex items-center gap-4",
                    div { class: "p-3 rounded-xl bg-primary/10 text-primary",
                        components::LucideIcon { name: "award", size: "32", class: "text-primary" }
                    }
                    div { class: "flex flex-col gap-1",
                        h2 { class: "text-2xl font-black text-foreground m-0 flex items-center gap-2",
                            {locales::t("school-report-title", &locale)}
                        }
                        p { class: "text-xs text-muted-foreground m-0 font-medium",
                            {locales::t("school-report-desc", &locale)}
                        }
                    }
                }

                // Term Selector Dropdown
                div { class: "flex items-center gap-2.5",
                    label { class: "text-xs font-bold text-muted-foreground", "Select Term:" }
                    select {
                        class: "yntra-input text-xs bg-sidebar py-1.5 px-3 border border-border/60 rounded-lg",
                        value: "{active_term}",
                        onchange: move |e| active_term.set(e.value()),
                        option { value: "Fall 2026", "Fall 2026" }
                        option { value: "Spring 2027", "Spring 2027" }
                    }
                }
            }

            if students.is_empty() {
                div { class: "p-8 text-center border border-dashed border-border/40 rounded-2xl bg-sidebar/20",
                    components::LucideIcon { name: "users", size: "48", class: "mx-auto text-muted-foreground opacity-30 mb-3" }
                    h3 { class: "text-lg font-bold text-foreground mb-1", "No enrolled students" }
                    p { class: "text-sm text-muted-foreground max-w-md mx-auto m-0", 
                        "Please enroll students in the 'Students' directory tab first to manage report cards." 
                    }
                }
            } else {
                div { class: "grid gap-6 md:grid-cols-3 items-start",
                    // Left Column: Student roster list
                    div { class: "md:col-span-1 flex flex-col gap-3",
                        h3 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0 px-1", {locales::t("school-report-roster", &locale)} }
                        div { class: "flex flex-col gap-2 max-h-[600px] overflow-y-auto pr-1",
                            for s in students.iter() {
                                {
                                    let s_id = s.id.clone();
                                    let is_selected = Some(s_id.clone()) == *selected_student_id.read();
                                    
                                    // Find student's report card to show current GPA badge
                                    let rc = all_report_cards.iter().find(|r| r.student_id == s_id && r.term_name == *active_term.read());
                                    let gpa_str = rc.map(|r| format!("{:.2}", r.gpa)).unwrap_or_else(|| "N/A".to_string());
                                    let is_published = rc.map(|r| r.status == "published").unwrap_or(false);
                                    
                                    rsx! {
                                        div {
                                            key: "{s_id}",
                                            class: format!("border p-4 rounded-xl flex justify-between items-center cursor-pointer transition-all hover:bg-sidebar/40 {}", if is_selected { "bg-sidebar border-primary/50 shadow-md ring-1 ring-primary/20" } else { "bg-sidebar/10 border-border/40" }),
                                            onclick: move |_| {
                                                selected_student_id.set(Some(s_id.clone()));
                                                save_success.set(false);
                                            },
                                            div { class: "flex flex-col gap-1",
                                                span { class: "text-sm font-bold text-foreground", "{s.first_name} {s.last_name}" }
                                                span { class: "text-[10px] text-muted-foreground font-semibold", "{s.grade_level}" }
                                            }
                                            div { class: "flex flex-col items-end gap-1.5",
                                                span { class: "text-xs font-black px-2 py-0.5 rounded bg-primary/10 text-primary border border-primary/15", "GPA: {gpa_str}" }
                                                if is_published {
                                                    span { class: "text-[9px] font-black uppercase text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/15 tracking-wider", "Published" }
                                                } else if rc.is_some() {
                                                    span { class: "text-[9px] font-black uppercase text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/15 tracking-wider", "Draft" }
                                                } else {
                                                    span { class: "text-[9px] font-semibold text-muted-foreground/60 tracking-wider", "Uncompiled" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Right Column: Student detailed grading sheet
                    if let Some(ref student) = selected_student {
                        div { class: "md:col-span-2 flex flex-col gap-6",
                            // Student Header Summary Card
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex justify-between items-center",
                                div { class: "flex flex-col gap-1",
                                    span { class: "text-[10px] font-black uppercase text-primary tracking-wider", "Selected Grading Sheet" }
                                    h3 { class: "text-xl font-black text-foreground m-0", "{student.first_name} {student.last_name}" }
                                    span { class: "text-xs text-muted-foreground", "Parent Contact: {student.parent_contact.clone().unwrap_or_else(|| \"None\".to_string())}" }
                                }
                                
                                div { class: "flex gap-2.5",
                                    button {
                                        class: "yntra-btn text-xs font-bold py-2 px-4 flex items-center gap-1.5 shadow-sm",
                                        onclick: {
                                            let ws = workspace_id.clone();
                                            let s_id = student.id.clone();
                                            let term = active_term.read().clone();
                                            let uid = active_user_id.clone();
                                            move |_| {
                                                let ws_clone = ws.clone();
                                                let s_clone = s_id.clone();
                                                let t_clone = term.clone();
                                                let uid_clone = uid.clone();
                                                spawn(async move {
                                                    if yntra_core::calculate_and_save_gpa(uid_clone, ws_clone, s_clone, t_clone).await.is_ok() {
                                                        let current = *db_trigger.read();
                                                        db_trigger.set(current + 1);
                                                    }
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "award", size: "14" }
                                        {locales::t("school-report-calculate", &locale)}
                                    }
                                }
                            }

                            // 1. Course Term Grade Form & Table
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                h4 { class: "text-sm font-black uppercase text-foreground tracking-wider m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                                    {locales::t("school-report-course-grades", &locale)}
                                }

                                // Quick Grading Inputs Row
                                div { class: "grid gap-4 md:grid-cols-4 items-end bg-sidebar/40 p-4 rounded-xl border border-border/30",
                                    div { class: "flex flex-col gap-1",
                                        label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Course" }
                                        select {
                                            class: "yntra-input text-xs w-full bg-sidebar",
                                            value: "{selected_course_id}",
                                            onchange: move |e| selected_course_id.set(e.value()),
                                            for c in courses.iter() {
                                                option { value: "{c.id}", "{c.name}" }
                                            }
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Final Grade" }
                                        select {
                                            class: "yntra-input text-xs w-full bg-sidebar",
                                            value: "{form_grade}",
                                            onchange: move |e| form_grade.set(e.value()),
                                            option { value: "A", "A (Excellent)" }
                                            option { value: "B", "B (Very Good)" }
                                            option { value: "C", "C (Good)" }
                                            option { value: "D", "D (Pass)" }
                                            option { value: "F", "F (Fail)" }
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Final Points (0-100)" }
                                        input {
                                            r#type: "number",
                                            min: "0",
                                            max: "100",
                                            class: "yntra-input text-xs w-full",
                                            value: "{form_points}",
                                            oninput: move |e| form_points.set(e.value()),
                                        }
                                    }
                                    div {
                                        button {
                                            class: "yntra-btn text-xs font-bold w-full py-2 flex items-center justify-center gap-1.5 shadow-sm",
                                            onclick: {
                                                let ws = workspace_id.clone();
                                                let s_id = student.id.clone();
                                                let uid = active_user_id.clone();
                                                move |_| {
                                                    let ws_clone = ws.clone();
                                                    let s_clone = s_id.clone();
                                                    let c_clone = selected_course_id.read().clone();
                                                    let term_clone = active_term.read().clone();
                                                    let grade_clone = Some(form_grade.read().clone());
                                                    let points_clone = Some(form_points.read().parse::<i32>().unwrap_or(90));
                                                    let comm_clone = Some(form_comments.read().clone());
                                                    let uid_clone = uid.clone();
                                                    
                                                    spawn(async move {
                                                        if yntra_core::save_term_grade(
                                                            uid_clone,
                                                            ws_clone,
                                                            s_clone,
                                                            c_clone,
                                                            term_clone,
                                                            grade_clone,
                                                            points_clone,
                                                            comm_clone
                                                        ).await.is_ok() {
                                                            save_success.set(true);
                                                            form_comments.set(String::new());
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "check", size: "14" }
                                            "Save Grade"
                                        }
                                    }
                                }

                                // Additional Comment Text Area
                                div { class: "flex flex-col gap-1.5 -mt-2 bg-sidebar/40 p-4 rounded-xl border border-border/30 border-t-0 -t-none",
                                    label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Teacher Report Comments" }
                                    textarea {
                                        class: "yntra-input text-xs w-full min-h-[60px]",
                                        placeholder: "Write specific feedback remarks for this course...",
                                        value: "{form_comments}",
                                        oninput: move |e| form_comments.set(e.value()),
                                    }
                                    if *save_success.read() {
                                        span { class: "text-[10px] font-bold text-emerald-400 mt-1 flex items-center gap-1",
                                            components::LucideIcon { name: "check-circle", size: "12" }
                                            "Course grade saved successfully!"
                                        }
                                    }
                                }

                                // Active Term Grades Table
                                if term_grades.is_empty() {
                                    div { class: "text-center py-6 text-xs text-muted-foreground border border-dashed border-border/30 rounded-xl bg-white/[0.01]",
                                        "No term grades assigned yet for this student in {active_term}."
                                    }
                                } else {
                                    div { class: "overflow-x-auto border border-border/30 rounded-xl bg-white/[0.01]",
                                        table { class: "w-full border-collapse text-left text-xs",
                                            thead { class: "bg-sidebar/45 text-muted-foreground font-black uppercase border-b border-border/30",
                                                tr {
                                                    th { class: "p-3 font-semibold", "Course" }
                                                    th { class: "p-3 font-semibold w-24 text-center", "Grade" }
                                                    th { class: "p-3 font-semibold w-24 text-center", "Points" }
                                                    th { class: "p-3 font-semibold", "Teacher Comments" }
                                                }
                                            }
                                            tbody { class: "divide-y divide-border/20",
                                                for tg in term_grades.iter() {
                                                    tr {
                                                        key: "{tg.id}",
                                                        class: "hover:bg-white/[0.01] transition-all",
                                                        td { class: "p-3 font-bold text-foreground",
                                                            {
                                                                let course = courses.iter().find(|c| c.id == tg.course_id);
                                                                course.map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string())
                                                            }
                                                        }
                                                        td { class: "p-3 text-center",
                                                            span { class: "font-black px-2 py-0.5 rounded bg-primary/10 text-primary border border-primary/15",
                                                                "{tg.final_grade.clone().unwrap_or_else(|| \"-\".to_string())}"
                                                            }
                                                        }
                                                        td { class: "p-3 text-center font-bold text-foreground/80",
                                                            "{tg.final_points.clone().unwrap_or(0)}"
                                                        }
                                                        td { class: "p-3 text-muted-foreground italic",
                                                            "\"{tg.teacher_comments.clone().unwrap_or_else(|| \"No comments.\".to_string())}\""
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 2. Term Summary & Publication
                            if let Some(rc) = active_report_card {
                                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                    h4 { class: "text-sm font-black uppercase text-foreground tracking-wider m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "award", size: "16", class: "text-primary" }
                                        {locales::t("school-report-card-summary", &locale)}
                                    }
                                    
                                    div { class: "grid gap-6 md:grid-cols-3 bg-sidebar/40 p-4 rounded-xl border border-border/30",
                                        div { class: "flex flex-col gap-1",
                                            span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-report-final-gpa", &locale)} }
                                            span { class: "text-2xl font-black text-foreground", "{rc.gpa:.2}" }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Status" }
                                            if rc.status == "published" {
                                                span { class: "text-xs font-black text-emerald-400 bg-emerald-500/10 border border-emerald-500/15 px-2.5 py-0.5 rounded self-start mt-0.5 uppercase tracking-wider", "Published to Parents" }
                                            } else {
                                                span { class: "text-xs font-black text-amber-400 bg-amber-500/10 border border-amber-500/15 px-2.5 py-0.5 rounded self-start mt-0.5 uppercase tracking-wider", "Draft Mode" }
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Last Updated" }
                                            span { class: "text-xs font-bold text-foreground/80 mt-1.5", "Date Code: {rc.updated_at}" }
                                        }
                                    }

                                    div { class: "flex flex-col gap-1.5",
                                        label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-report-advisor-remarks", &locale)} }
                                        textarea {
                                            class: "yntra-input text-xs w-full min-h-[80px]",
                                            placeholder: "Provide cumulative remarks for the report card certificate...",
                                            value: "{advisor_comments}",
                                            oninput: move |e| advisor_comments.set(e.value()),
                                        }
                                    }

                                    div { class: "flex gap-2 justify-end",
                                        button {
                                            class: "yntra-btn text-xs font-bold py-2 px-5 flex items-center gap-1.5 shadow-md",
                                            onclick: {
                                                let r_id = rc.id.clone();
                                                let comm_clone = Some(advisor_comments.read().clone());
                                                let uid = active_user_id.clone();
                                                move |_| {
                                                    let r_clone = r_id.clone();
                                                    let uid_clone = uid.clone();
                                                    let c_clone = comm_clone.clone();
                                                    spawn(async move {
                                                        if yntra_core::publish_report_card(uid_clone, r_clone, c_clone).await.is_ok() {
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "check-circle", size: "14" }
                                            {locales::t("school-report-publish", &locale)}
                                        }
                                    }
                                }
                            } else {
                                div { class: "p-6 text-center border border-dashed border-border/30 rounded-xl bg-white/[0.01]",
                                    p { class: "text-xs text-muted-foreground m-0 mb-3", "GPA record hasn't been calculated for this student in {active_term} yet." }
                                    button {
                                        class: "yntra-btn text-xs py-1.5 px-4 font-bold shadow-sm",
                                        onclick: {
                                            let ws = workspace_id.clone();
                                            let s_id = student.id.clone();
                                            let term = active_term.read().clone();
                                            let uid = active_user_id.clone();
                                            move |_| {
                                                let ws_clone = ws.clone();
                                                let s_clone = s_id.clone();
                                                let t_clone = term.clone();
                                                let uid_clone = uid.clone();
                                                spawn(async move {
                                                    if yntra_core::calculate_and_save_gpa(uid_clone, ws_clone, s_clone, t_clone).await.is_ok() {
                                                        let current = *db_trigger.read();
                                                        db_trigger.set(current + 1);
                                                    }
                                                });
                                            }
                                        },
                                        "Run GPA Calculation"
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
