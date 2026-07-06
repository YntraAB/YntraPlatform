use dioxus::prelude::*;
use crate::components;
use crate::locales;
use yntra_core::{WorkspaceUser, StudentProfile, Course, Submission};

pub mod academics;
pub mod attendance;
pub mod staff;
pub mod transcripts;
pub mod health;
pub mod billing;
pub mod library;

use academics::AcademicsTab;
use attendance::AttendanceTab;
use staff::StaffTab;
use transcripts::TranscriptsTab;
use health::HealthTab;
use billing::BillingTab;
use library::LibraryTab;

#[derive(Props, Clone)]
pub struct ParentPortalProps {
    pub active_user: WorkspaceUser,
    pub students: Vec<StudentProfile>,
    pub courses: Vec<Course>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for ParentPortalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ParentPortal(props: ParentPortalProps) -> Element {
    let active_user = props.active_user.clone();
    let students = props.students.clone();
    let courses = props.courses.clone();
    let db_trigger = props.db_trigger;
    let locale = props.locale.clone();

    // 1. Filter students linked to this parent's email
    let linked_children: Vec<StudentProfile> = students
        .iter()
        .filter(|s| {
            s.parent_contact
                .as_ref()
                .map(|email| email.to_lowercase() == active_user.email.to_lowercase())
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    // 2. Selected child state
    let mut selected_child_id = use_signal(|| linked_children.first().map(|s| s.id.clone()));
    let selected_student = linked_children
        .iter()
        .find(|s| Some(s.id.clone()) == *selected_child_id.read())
        .or(linked_children.first())
        .cloned();

    // 3. Fetch attendance records for selected child
    let db_trig = *db_trigger.read();
    let child_id_for_att = selected_child_id.read().clone();
    let req_att = active_user.id.clone();
    let attendance_res = use_resource(move || {
        let _ = db_trig;
        let c_id = child_id_for_att.clone();
        let r_id = req_att.clone();
        async move {
            if let Some(id) = c_id {
                yntra_core::get_student_attendance(r_id, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let attendance_records = attendance_res.read().clone().unwrap_or_default();

    // 3.5 Fetch report cards and term grades for selected child
    let selected_term = use_signal(|| "Fall 2026".to_string());
    let child_id_for_rc = selected_child_id.read().clone();
    let req_rc = active_user.id.clone();
    let report_cards_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_rc.clone();
        let r_id = req_rc.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_report_cards(r_id, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let report_cards = report_cards_res.read().clone().unwrap_or_default();

    let term_for_tg = selected_term.read().clone();
    let child_id_for_tg = selected_child_id.read().clone();
    let req_tg = active_user.id.clone();
    let term_grades_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_tg.clone();
        let term = term_for_tg.clone();
        let r_id = req_tg.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_term_grades(r_id, id, term).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let term_grades = term_grades_res.read().clone().unwrap_or_default();

    // 3.8 Fetch timetable slots
    let req_id_timetable = active_user.id.clone();
    let timetable_res = use_resource(move || {
        let _ = db_trig;
        let r_id = req_id_timetable.clone();
        async move {
            yntra_core::get_timetable_slots(r_id).await.unwrap_or_default()
        }
    });
    let timetable_slots = timetable_res.read().clone().unwrap_or_default();

    // 3.9 Fetch child's health records
    let child_id_for_health = selected_child_id.read().clone();
    let req_id_records = active_user.id.clone();
    let health_records_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_health.clone();
        let r_id = req_id_records.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_health_records(r_id, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let health_records = health_records_res.read().clone().unwrap_or_default();

    // 3.95 Fetch child's health incidents
    let child_id_for_inc = selected_child_id.read().clone();
    let req_id_incidents = active_user.id.clone();
    let health_incidents_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_inc.clone();
        let r_id = req_id_incidents.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_health_incidents(r_id, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let health_incidents = health_incidents_res.read().clone().unwrap_or_default();

    // 3.96 Fetch child's invoices
    let child_id_for_billing = selected_child_id.read().clone();
    let req_billing = active_user.id.clone();
    let invoices_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_billing.clone();
        let r_id = req_billing.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_school_invoices(r_id, id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let invoices = invoices_res.read().clone().unwrap_or_default();

    // 3.97 Fetch child's library loans
    let child_id_for_library = selected_child_id.read().clone();
    let req_library = active_user.id.clone();
    let library_logs_res = use_resource(move || {
        let _ = db_trig;
        let s_id = child_id_for_library.clone();
        let r_id = req_library.clone();
        async move {
            if let Some(id) = s_id {
                yntra_core::get_library_lending_logs(r_id, Some(id)).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    });
    let library_logs = library_logs_res.read().clone().unwrap_or_default();

    // 3.98 Fetch all library books for title lookup
    let req_id_lib_books = active_user.id.clone();
    let library_books_res = use_resource(move || {
        let _ = db_trig;
        let r_id = req_id_lib_books.clone();
        async move {
            yntra_core::get_library_books(r_id).await.unwrap_or_default()
        }
    });
    let library_books = library_books_res.read().clone().unwrap_or_default();

    // 4. Fetch assignments and submissions
    let uid_for_acad = active_user.id.clone();
    let academic_data_res = use_resource(move || {
        let _ = db_trig;
        let uid = uid_for_acad.clone();
        async move {
            let courses_list = yntra_core::get_courses(uid.clone()).await.unwrap_or_default();
            let mut all_assigns = Vec::new();
            for c in courses_list.iter() {
                if let Ok(assigns) = yntra_core::get_assignments(uid.clone(), c.id.clone()).await {
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
    let (all_assignments, all_submissions) = academic_data_res.read().clone().unwrap_or_else(|| (Vec::new(), Vec::new()));

    // Active sub-tab inside Parent Portal
    let mut active_tab = use_signal(|| "academics".to_string()); // academics, attendance, staff, transcripts, health, billing, library

    // Messaging modal state
    let mut show_message_modal = use_signal(|| false);
    let mut target_teacher_id = use_signal(String::new);
    let mut target_teacher_name = use_signal(String::new);
    let mut message_subject = use_signal(String::new);
    let mut message_body = use_signal(String::new);
    let mut msg_success = use_signal(|| false);

    // Fetch workspace ID
    let state = use_context::<crate::state::AppState>();
    let workspace_id = state.workspace.read().as_ref().map(|w| w.id.clone()).unwrap_or_else(|| "workspace-1".to_string());

    // Calculate metrics
    let total_attendance = attendance_records.len();
    let present_attendance = attendance_records.iter().filter(|r| r.status == "present" || r.status == "excused").count();
    let attendance_rate = if total_attendance > 0 {
        ((present_attendance as f64) / (total_attendance as f64) * 100.0) as i32
    } else {
        100
    };

    let child_subs: Vec<Submission> = if let Some(ref child) = selected_student {
        all_submissions.iter().filter(|s| s.student_id == child.id).cloned().collect()
    } else {
        Vec::new()
    };
    let completed_tasks = child_subs.len();
    let pending_tasks = if selected_student.is_some() {
        all_assignments.len().saturating_sub(child_subs.len())
    } else {
        0
    };

    rsx! {
        if linked_children.is_empty() {
            div { class: "p-8 text-center border border-dashed border-border/40 rounded-2xl bg-sidebar/20",
                components::LucideIcon { name: "users", size: "48", class: "mx-auto text-muted-foreground opacity-30 mb-3" }
                h2 { class: "text-lg font-bold text-foreground mb-1", {locales::t("school-parent-no-children", &locale)} }
                p { class: "text-sm text-muted-foreground max-w-md mx-auto m-0", 
                    "To use the Parent Portal, your user email must match the parent contact field of an enrolled student." 
                }
            }
        } else if let Some(ref student) = selected_student {
            div { class: "flex flex-col gap-6",
                
                // Welcome Hero Header
                div {
                    class: "relative p-6 rounded-2xl overflow-hidden border border-primary/20 shadow-lg flex flex-col md:flex-row justify-between items-start md:items-center gap-4 bg-gradient-to-r from-primary/10 via-purple-500/5 to-indigo-500/10",
                    div { class: "flex items-center gap-4",
                        div { class: "p-3 rounded-xl bg-primary/10 text-primary",
                            components::LucideIcon { name: "shield", size: "32", class: "text-primary" }
                        }
                        div { class: "flex flex-col gap-1",
                            h2 { class: "text-2xl font-black text-foreground m-0 flex items-center gap-2",
                                {locales::t("school-parent-title", &locale)}
                            }
                            p { class: "text-xs text-muted-foreground m-0 font-medium",
                                {locales::t("school-parent-desc", &locale)}
                            }
                        }
                    }

                    // Children selector tabs
                    if linked_children.len() > 1 {
                        div { class: "flex bg-sidebar border border-border/80 p-0.5 rounded-lg",
                            for s in linked_children.iter() {
                                {
                                    let s_id = s.id.clone();
                                    let is_active = Some(s_id.clone()) == *selected_child_id.read();
                                    rsx! {
                                        button {
                                            key: "{s_id}",
                                            class: format!("px-3 py-1.5 rounded text-xs font-bold transition-all {}", if is_active { "bg-primary text-primary-foreground shadow" } else { "text-muted-foreground hover:text-foreground" }),
                                            onclick: move |_| selected_child_id.set(Some(s_id.clone())),
                                            "{s.first_name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Metrics Cards Grid
                div { class: "grid gap-6 md:grid-cols-3",
                    // Card 1: Attendance Rate
                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col justify-between gap-3",
                        div { class: "flex justify-between items-start",
                            div { class: "flex flex-col gap-1",
                                span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-parent-attendance", &locale)} }
                                h3 { class: "text-2xl font-black text-foreground m-0", "{attendance_rate}%" }
                            }
                            div { class: "p-2 rounded-lg bg-emerald-500/10 text-emerald-400",
                                components::LucideIcon { name: "check-circle", size: "20" }
                            }
                        }
                        div { class: "w-full bg-border rounded-full h-2 overflow-hidden",
                            div { 
                                class: format!("h-full rounded-full transition-all {}", if attendance_rate > 90 { "bg-emerald-500" } else if attendance_rate > 75 { "bg-yellow-500" } else { "bg-red-500" }),
                                style: format!("width: {}%;", attendance_rate)
                            }
                        }
                        span { class: "text-[10px] text-muted-foreground font-medium", "{present_attendance} of {total_attendance} school days logged" }
                    }

                    // Card 2: Academic Submissions
                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col justify-between gap-3",
                        div { class: "flex justify-between items-start",
                            div { class: "flex flex-col gap-1",
                                span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-parent-completed", &locale)} }
                                h3 { class: "text-2xl font-black text-foreground m-0", "{completed_tasks} / {all_assignments.len()}" }
                            }
                            div { class: "p-2 rounded-lg bg-primary/10 text-primary",
                                components::LucideIcon { name: "book-open", size: "20" }
                            }
                        }
                        div { class: "w-full bg-border rounded-full h-2 overflow-hidden",
                            div { 
                                class: "h-full bg-primary rounded-full transition-all",
                                style: format!("width: {}%;", if !all_assignments.is_empty() { completed_tasks * 100 / all_assignments.len() } else { 100 })
                            }
                        }
                        span { class: "text-[10px] text-muted-foreground font-medium", "{pending_tasks} assignments currently pending" }
                    }

                    // Card 3: Grade Level info
                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col justify-between gap-3",
                        div { class: "flex justify-between items-start",
                            div { class: "flex flex-col gap-1",
                                span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", {locales::t("school-parent-grade-level", &locale)} }
                                h3 { class: "text-2xl font-black text-foreground m-0", "{student.grade_level}" }
                            }
                            div { class: "p-2 rounded-lg bg-indigo-500/10 text-indigo-400",
                                components::LucideIcon { name: "award", size: "20" }
                            }
                        }
                        div { class: "flex gap-1.5 flex-wrap",
                            span { class: "px-2 py-0.5 rounded text-[10px] bg-sidebar/80 border border-border/40 text-foreground font-bold", "Workspace: {workspace_id}" }
                            span { class: "px-2 py-0.5 rounded text-[10px] bg-primary/10 text-primary font-bold", "Linked Profile" }
                        }
                        span { class: "text-[10px] text-muted-foreground font-medium", "Active student: {student.first_name} {student.last_name}" }
                    }
                }

                // Sub-tabs navigation bar
                div { class: "flex border-b border-border/60 gap-4 mt-2",
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "academics" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("academics".to_string()),
                        "Academic Performance"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "attendance" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("attendance".to_string()),
                        "Attendance & Safe Reports"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "staff" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("staff".to_string()),
                        "Contact Teachers"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "transcripts" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("transcripts".to_string()),
                        "Report Cards"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "health" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("health".to_string()),
                        "Medical & Health"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "billing" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("billing".to_string()),
                        "Tuition & Fees"
                    }
                    button {
                        class: format!("pb-3 text-sm font-bold border-b-2 transition-all px-1 {}", if *active_tab.read() == "library" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }),
                        onclick: move |_| active_tab.set("library".to_string()),
                        "Library Catalog"
                    }
                }

                // Render Content Tabs
                match active_tab.read().as_str() {
                    "academics" => rsx! {
                        AcademicsTab {
                            courses: courses.clone(),
                            child_subs: child_subs,
                            all_assignments: all_assignments.clone(),
                            timetable_slots: timetable_slots.clone(),
                            locale: locale.clone(),
                        }
                    },
                    "attendance" => rsx! {
                        AttendanceTab {
                            active_user: active_user.clone(),
                            student_id: student.id.clone(),
                            courses: courses.clone(),
                            attendance_records: attendance_records.clone(),
                            db_trigger,
                            workspace_id: workspace_id.clone(),
                            locale: locale.clone(),
                        }
                    },
                    "staff" => rsx! {
                        StaffTab {
                            courses: courses.clone(),
                            on_message: move |(teacher_id, course_name): (String, String)| {
                                target_teacher_id.set(teacher_id.clone());
                                target_teacher_name.set(format!("Teacher for {}", course_name));
                                message_subject.set(format!("Inquiry regarding {}", course_name));
                                message_body.set(String::new());
                                msg_success.set(false);
                                show_message_modal.set(true);
                            }
                        }
                    },
                    "transcripts" => rsx! {
                        TranscriptsTab {
                            student: student.clone(),
                            report_cards: report_cards.clone(),
                            term_grades: term_grades.clone(),
                            courses: courses.clone(),
                            selected_term,
                        }
                    },
                    "health" => rsx! {
                        HealthTab {
                            health_records: health_records.clone(),
                            health_incidents: health_incidents.clone(),
                        }
                    },
                    "billing" => rsx! {
                        BillingTab {
                            invoices: invoices.clone(),
                            active_user: active_user.clone(),
                            workspace_id: workspace_id.clone(),
                            db_trigger,
                        }
                    },
                    "library" => rsx! {
                        LibraryTab {
                            library_logs: library_logs.clone(),
                            library_books: library_books.clone(),
                        }
                    },
                    _ => rsx! {}
                }
            }

            // teacher/staff contact modal
            if *show_message_modal.read() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in duration-200",
                    div { class: "w-full max-w-lg p-6 bg-sidebar border border-border/50 rounded-2xl shadow-xl flex flex-col gap-4 animate-in scale-in duration-200",
                        div { class: "flex justify-between items-center",
                            h3 { class: "text-base font-black text-foreground m-0", "Send Message to Teacher" }
                            button {
                                class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground",
                                onclick: move |_| show_message_modal.set(false),
                                components::LucideIcon { name: "x", size: "18" }
                            }
                        }

                        if *msg_success.read() {
                            div { class: "p-6 rounded-xl border border-emerald-500/20 bg-emerald-500/5 text-emerald-400 text-xs font-bold flex flex-col gap-2.5 items-center text-center",
                                components::LucideIcon { name: "check-circle", size: "32" }
                                span { class: "text-sm", "Message Sent Successfully!" }
                                button {
                                    class: "yntra-btn text-xs mt-2 py-1.5 px-4",
                                    onclick: move |_| show_message_modal.set(false),
                                    "Close Modal"
                                }
                            }
                        } else {
                            div { class: "flex flex-col gap-3",
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Recipient" }
                                    input {
                                        r#type: "text",
                                        class: "yntra-input text-xs w-full opacity-60",
                                        readonly: true,
                                        value: "{target_teacher_name}",
                                    }
                                }

                                div { class: "flex flex-col gap-1",
                                    label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Subject" }
                                    input {
                                        r#type: "text",
                                        class: "yntra-input text-xs w-full",
                                        placeholder: "Subject",
                                        value: "{message_subject}",
                                        oninput: move |e| message_subject.set(e.value()),
                                    }
                                }

                                div { class: "flex flex-col gap-1",
                                    label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Message Body" }
                                    textarea {
                                        class: "yntra-input text-xs w-full min-h-[140px]",
                                        placeholder: "Write your message here...",
                                        value: "{message_body}",
                                        oninput: move |e| message_body.set(e.value()),
                                    }
                                }

                                div { class: "flex gap-2 justify-end mt-2",
                                    button {
                                        class: "yntra-btn-secondary text-xs font-bold py-2 px-4 rounded-lg",
                                        onclick: move |_| show_message_modal.set(false),
                                        "Cancel"
                                    }
                                    button {
                                        class: "yntra-btn text-xs font-bold py-2 px-5 rounded-lg flex items-center gap-1.5 shadow-md",
                                        onclick: {
                                            let ws = workspace_id.clone();
                                            let sender = active_user.id.clone();
                                            move |_| {
                                                let ws_clone = ws.clone();
                                                let sender_clone = sender.clone();
                                                let receiver_clone = target_teacher_id.read().clone();
                                                let subject_clone = message_subject.read().clone();
                                                let body_clone = message_body.read().clone();
                                                
                                                spawn(async move {
                                                    if yntra_core::send_message(
                                                        sender_clone.clone(),
                                                        ws_clone,
                                                        sender_clone,
                                                        Some(receiver_clone),
                                                        None,
                                                        subject_clone,
                                                        body_clone
                                                    ).await.is_ok() {
                                                        msg_success.set(true);
                                                        message_body.set(String::new());
                                                    }
                                                });
                                            }
                                        },
                                        components::LucideIcon { name: "check", size: "14" }
                                        "Send Message"
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
