use dioxus::prelude::*;
use crate::components;
use crate::locales;
use yntra_core::{WorkspaceUser, StudentProfile, Course, Submission};

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
    let mut db_trigger = props.db_trigger;
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
    let mut selected_term = use_signal(|| "Fall 2026".to_string());
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
    let timetable_res = use_resource(move || {
        let _ = db_trig;
        async move {
            yntra_core::get_timetable_slots().await.unwrap_or_default()
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
    let library_books_res = use_resource(move || {
        let _ = db_trig;
        async move {
            yntra_core::get_library_books().await.unwrap_or_default()
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
                if let Ok(assigns) = yntra_core::get_assignments(c.id.clone()).await {
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
    let mut active_tab = use_signal(|| "academics".to_string()); // academics, attendance, staff

    // Absence Reporting Form state
    let mut absence_date = use_signal(|| "2026-07-04".to_string());
    let mut absence_course_id = use_signal(|| courses.first().map(|c| c.id.clone()).unwrap_or_default());
    let mut absence_notes = use_signal(String::new);
    let mut absence_status = use_signal(|| "excused".to_string());
    let mut form_success = use_signal(|| false);

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

                // SOTA Metrics Cards Grid
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

                // Sub-tabs navigation
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
                if *active_tab.read() == "academics" {
                    div { class: "grid gap-6 md:grid-cols-3 items-start",
                        // Left 2 columns: Assignment grades
                        div { class: "md:col-span-2 flex flex-col gap-4",
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "book-open", size: "18", class: "text-primary" }
                                    "Assignment Grades & Teacher Feedback"
                                }
                                
                                if child_subs.is_empty() {
                                    div { class: "text-center p-8 text-muted-foreground text-sm",
                                        "No assignments have been completed yet."
                                    }
                                } else {
                                    div { class: "flex flex-col gap-4",
                                        for s in child_subs.iter() {
                                            div { class: "border border-border/40 p-4 rounded-xl flex flex-col gap-3 bg-white/[0.01]",
                                                div { class: "flex justify-between items-start",
                                                    div {
                                                        div { class: "font-bold text-foreground text-sm",
                                                            {
                                                                let assignment = all_assignments.iter().find(|a| a.id == s.assignment_id);
                                                                assignment.map(|a| a.title.clone()).unwrap_or_else(|| "Assignment".to_string())
                                                            }
                                                        }
                                                        div { class: "text-xs text-muted-foreground mt-0.5", "Submitted on: {s.submitted_at}" }
                                                    }
                                                    
                                                    div { class: "flex items-center gap-2",
                                                        if let Some(ref g) = s.grade {
                                                            span { class: "text-xs font-black px-2.5 py-1 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Grade: {g}" }
                                                        } else {
                                                            span { class: "text-xs font-black px-2.5 py-1 rounded bg-amber-500/10 text-amber-400 border border-amber-500/15 animate-pulse", "Awaiting Review" }
                                                        }
                                                    }
                                                }

                                                // Student Submission
                                                div { class: "text-xs bg-sidebar/50 p-2.5 rounded border border-border/20 text-muted-foreground font-medium",
                                                    div { class: "font-bold text-foreground/80 mb-1", "Submitted Answer:" }
                                                    "{s.content}"
                                                }

                                                // Teacher Feedback
                                                if let Some(ref fb) = s.feedback {
                                                    div {
                                                        class: "relative p-3 rounded-lg border border-yellow-500/20 bg-yellow-500/5 text-xs text-amber-200 flex gap-2.5 items-start",
                                                        components::LucideIcon { name: "pen-tool", size: "16", class: "text-amber-400 mt-0.5" }
                                                        div {
                                                            div { class: "font-black text-amber-400/90 mb-0.5", "Teacher Feedback Note:" }
                                                            "\"{fb}\""
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right 1 column: Student Schedule
                        div { class: "md:col-span-1 flex flex-col gap-4",
                            components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                                    "Student Class Schedule"
                                }
                                
                                if timetable_slots.is_empty() {
                                    p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", {locales::t("school-timetable-no-slots", &locale)} }
                                } else {
                                    div { class: "flex flex-col gap-3.5",
                                        for s in timetable_slots.iter().take(6) {
                                            div { class: "flex items-start gap-3 border-l-4 border-primary pl-3 py-0.5",
                                                div { class: "text-xs font-bold text-muted-foreground w-12", "{s.start_time}" }
                                                div { class: "flex flex-col gap-0.5",
                                                    div { class: "text-sm font-extrabold text-foreground flex items-center gap-1.5",
                                                        {
                                                            let course = courses.iter().find(|c| c.id == s.course_id);
                                                            course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                        }
                                                    }
                                                    div { class: "text-xs text-muted-foreground", 
                                                        "Room: {s.classroom.clone().unwrap_or_else(|| \"TBD\".to_string())} • {s.start_time} - {s.end_time}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if *active_tab.read() == "attendance" {
                    div { class: "grid gap-6 md:grid-cols-3 items-start",
                        // Left: Attendance logs
                        div { class: "md:col-span-2 flex flex-col gap-6",
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "scheduling", size: "18", class: "text-primary" }
                                    "Attendance Timeline"
                                }

                                if attendance_records.is_empty() {
                                    div { class: "text-center p-8 text-muted-foreground text-sm",
                                        "No attendance records logged for this student."
                                    }
                                } else {
                                    div { class: "flex flex-col gap-3.5",
                                        for r in attendance_records.iter() {
                                            div { class: "flex items-center justify-between border border-border/30 p-3 rounded-xl bg-white/[0.01]",
                                                div { class: "flex items-center gap-3.5",
                                                    {
                                                        let icon_name = match r.status.as_str() {
                                                            "present" => "check-circle",
                                                            "absent" => "x-circle",
                                                            "late" => "clock",
                                                            _ => "shield", // excused
                                                        };
                                                        let icon_color = match r.status.as_str() {
                                                            "present" => "text-emerald-400 bg-emerald-500/10",
                                                            "absent" => "text-red-400 bg-red-500/10",
                                                            "late" => "text-amber-400 bg-amber-500/10",
                                                            _ => "text-indigo-400 bg-indigo-500/10", // excused
                                                        };
                                                        rsx! {
                                                            div { class: format!("p-2 rounded-full {}", icon_color),
                                                                components::LucideIcon { name: icon_name, size: "16" }
                                                            }
                                                        }
                                                    }
                                                    div { class: "flex flex-col",
                                                        span { class: "text-sm font-bold text-foreground capitalize", "{r.status}" }
                                                        if let Some(ref note) = r.notes {
                                                            span { class: "text-xs text-muted-foreground", "{note}" }
                                                        }
                                                    }
                                                }
                                                div { class: "text-right flex flex-col gap-0.5",
                                                    span { class: "text-xs font-black text-foreground", "{r.date}" }
                                                    span { class: "text-[10px] text-muted-foreground",
                                                        {
                                                            let course = courses.iter().find(|c| c.id == r.course_id);
                                                            course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right: Report Absence Form
                        div { class: "md:col-span-1",
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "reporting", size: "18", class: "text-primary" }
                                    "Report Absence"
                                }
                                p { class: "text-xs text-muted-foreground m-0 -mt-1 leading-normal",
                                    "Submit an official excused absence log for your child. Teachers will be notified instantly."
                                }

                                if *form_success.read() {
                                    div { class: "p-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 text-emerald-400 text-xs font-bold flex flex-col gap-2 items-center text-center",
                                        components::LucideIcon { name: "check-circle", size: "24" }
                                        span { "Absence Report Submitted Successfully!" }
                                        button {
                                            class: "yntra-btn text-[10px] py-1 px-3 mt-1",
                                            onclick: move |_| form_success.set(false),
                                            "Report Another"
                                        }
                                    }
                                } else {
                                    div { class: "flex flex-col gap-3",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Absence Date" }
                                            input {
                                                r#type: "date",
                                                class: "yntra-input text-xs w-full",
                                                value: "{absence_date}",
                                                oninput: move |e| absence_date.set(e.value()),
                                            }
                                        }

                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Target Course" }
                                            select {
                                                class: "yntra-input text-xs w-full bg-sidebar",
                                                value: "{absence_course_id}",
                                                onchange: move |e| absence_course_id.set(e.value()),
                                                for c in courses.iter() {
                                                    option { value: "{c.id}", "{c.name}" }
                                                }
                                            }
                                        }

                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Status Category" }
                                            select {
                                                class: "yntra-input text-xs w-full bg-sidebar",
                                                value: "{absence_status}",
                                                onchange: move |e| absence_status.set(e.value()),
                                                option { value: "excused", "Excused Absence" }
                                                option { value: "late", "Excused Late Arrival" }
                                            }
                                        }

                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Reason / Notes" }
                                            textarea {
                                                class: "yntra-input text-xs w-full min-h-[80px]",
                                                placeholder: "e.g., Dentist appointment, family illness...",
                                                value: "{absence_notes}",
                                                oninput: move |e| absence_notes.set(e.value()),
                                            }
                                        }

                                        button {
                                            class: "yntra-btn text-xs font-bold w-full py-2 flex items-center justify-center gap-1.5 shadow-md",
                                            onclick: {
                                                let ws = workspace_id.clone();
                                                let stud_id = student.id.clone();
                                                let uid = active_user.id.clone();
                                                move |_| {
                                                    let ws_clone = ws.clone();
                                                    let stud_clone = stud_id.clone();
                                                    let course_clone = absence_course_id.read().clone();
                                                    let date_clone = absence_date.read().clone();
                                                    let status_clone = absence_status.read().clone();
                                                    let notes_clone = Some(absence_notes.read().clone());
                                                    let uid_clone = uid.clone();
                                                    
                                                    spawn(async move {
                                                        if yntra_core::save_attendance_record(
                                                            uid_clone,
                                                            ws_clone,
                                                            stud_clone,
                                                            course_clone,
                                                            date_clone,
                                                            status_clone,
                                                            notes_clone
                                                        ).await.is_ok() {
                                                            form_success.set(true);
                                                            absence_notes.set(String::new());
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "check", size: "14" }
                                            "Submit Absence Log"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if *active_tab.read() == "staff" {
                    components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                        h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                            components::LucideIcon { name: "users", size: "18", class: "text-primary" }
                            "Course Teachers & Advisors"
                        }
                        
                        div { class: "grid gap-4 md:grid-cols-2",
                            for c in courses.iter() {
                                div { class: "border border-border/30 p-4 rounded-xl flex justify-between items-center bg-white/[0.01] hover:border-primary/30 transition-all",
                                    div { class: "flex flex-col gap-1.5",
                                        span { class: "text-[10px] font-black uppercase bg-primary/10 text-primary px-2 py-0.5 rounded self-start", "{c.subject}" }
                                        div { class: "font-bold text-foreground text-sm", "{c.name}" }
                                        div { class: "text-xs text-muted-foreground flex items-center gap-1",
                                            components::LucideIcon { name: "home", size: "12" }
                                            "Classroom: {c.classroom.clone().unwrap_or_else(|| \"TBD\".to_string())}"
                                        }
                                    }
                                    
                                    button {
                                        class: "yntra-btn text-xs font-bold py-1.5 px-3.5 flex items-center gap-1.5 shadow-sm",
                                        onclick: {
                                            let teacher_id = c.teacher_id.clone().unwrap_or_else(|| "user-1".to_string());
                                            let course_name = c.name.clone();
                                            move |_| {
                                                target_teacher_id.set(teacher_id.clone());
                                                target_teacher_name.set(format!("Teacher for {}", course_name));
                                                message_subject.set(format!("Inquiry regarding {}", course_name));
                                                message_body.set(String::new());
                                                msg_success.set(false);
                                                show_message_modal.set(true);
                                            }
                                        },
                                        components::LucideIcon { name: "message-square", size: "12" }
                                        "Message"
                                    }
                                }
                            }
                        }
                    }
                } else if *active_tab.read() == "transcripts" {
                    div { class: "flex flex-col gap-6",
                        components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            div { class: "flex justify-between items-center flex-wrap gap-4",
                                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "award", size: "18", class: "text-primary" }
                                    "Official Term Report Cards"
                                }
                                
                                div { class: "flex items-center gap-2",
                                    span { class: "text-xs font-bold text-muted-foreground", "Academic Period:" }
                                    select {
                                        class: "yntra-input text-xs bg-sidebar py-1.5 px-3 border border-border/60 rounded-lg",
                                        value: "{selected_term}",
                                        onchange: move |e| selected_term.set(e.value()),
                                        option { value: "Fall 2026", "Fall 2026" }
                                        option { value: "Spring 2027", "Spring 2027" }
                                    }
                                }
                            }
                            
                            {
                                let term = selected_term.read().clone();
                                let rc = report_cards.iter().find(|r| r.term_name == term && r.status == "published");
                                
                                if let Some(card) = rc {
                                    rsx! {
                                        div { class: "border border-yellow-500/20 p-6 rounded-2xl bg-gradient-to-br from-yellow-500/5 via-sidebar/20 to-primary/5 flex flex-col gap-5 relative overflow-hidden shadow-inner",
                                            div { class: "absolute right-[-20px] top-[-20px] opacity-5 pointer-events-none",
                                                components::LucideIcon { name: "award", size: "120", class: "text-yellow-500" }
                                            }
                                            
                                            div { class: "flex justify-between items-start border-b border-border/30 pb-4",
                                                div { class: "flex flex-col gap-1",
                                                    span { class: "text-[10px] font-black uppercase text-yellow-500 tracking-widest", "Official Academic Record" }
                                                    h4 { class: "text-lg font-black text-foreground m-0", "Yntra Academy Term Transcript" }
                                                    span { class: "text-xs text-muted-foreground", "Student: {student.first_name} {student.last_name} | {student.grade_level}" }
                                                }
                                                
                                                div { class: "text-right flex flex-col items-end gap-1",
                                                    span { class: "text-2xl font-black text-primary", "{card.gpa:.2}" }
                                                    span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Calculated GPA" }
                                                }
                                            }
                                            
                                            div { class: "flex flex-col gap-3",
                                                h5 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0", "Term Courses & Evaluations" }
                                                if term_grades.is_empty() {
                                                    p { class: "text-xs text-muted-foreground italic m-0", "No final course grades loaded for this term period." }
                                                } else {
                                                    div { class: "flex flex-col gap-2.5",
                                                        for tg in term_grades.iter() {
                                                            div { class: "flex flex-col md:flex-row md:items-center justify-between border border-border/20 p-3 rounded-xl bg-sidebar/30 gap-2",
                                                                div { class: "flex flex-col",
                                                                    span { class: "text-sm font-bold text-foreground",
                                                                        {
                                                                            let course = courses.iter().find(|c| c.id == tg.course_id);
                                                                            course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                                        }
                                                                    }
                                                                    if let Some(ref comment) = tg.teacher_comments {
                                                                        span { class: "text-xs text-muted-foreground italic mt-0.5", "\"{comment}\"" }
                                                                    }
                                                                }
                                                                
                                                                div { class: "flex items-center gap-3 self-end md:self-auto",
                                                                    if let Some(ref points) = tg.final_points {
                                                                        span { class: "text-xs font-semibold text-muted-foreground", "Points: {points}" }
                                                                    }
                                                                    span { class: "text-xs font-black px-2.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/15",
                                                                        "{tg.final_grade.clone().unwrap_or_else(|| \"-\".to_string())}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            if let Some(ref comm) = card.principal_comments {
                                                div { class: "border-t border-border/30 pt-4 flex flex-col gap-1.5",
                                                    span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Principal Advisory Remarks" }
                                                    p { class: "text-xs text-foreground/80 m-0 italic bg-sidebar/40 p-3 rounded-lg border border-border/20",
                                                        "\"{comm}\""
                                                    }
                                                }
                                            }
                                            
                                            div { class: "flex justify-end border-t border-border/30 pt-4 mt-1",
                                                button {
                                                    class: "yntra-btn-secondary text-xs font-bold py-2 px-4 flex items-center gap-1.5 shadow-sm",
                                                    onclick: move |_| {
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            if let Some(w) = web_sys::window() {
                                                                let _ = w.print();
                                                            }
                                                        }
                                                    },
                                                    components::LucideIcon { name: "printer", size: "14" }
                                                    "Print Official Record"
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "p-8 text-center border border-dashed border-border/30 rounded-xl bg-white/[0.01] text-muted-foreground text-xs",
                                            components::LucideIcon { name: "award", size: "32", class: "mx-auto opacity-30 mb-2" }
                                            "No published report card is available for {term}."
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if *active_tab.read() == "health" {
                    div { class: "grid gap-6 md:grid-cols-2 items-start",
                        // Left: Vaccine Schedule Card
                        components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "award", size: "18", class: "text-primary" }
                                "Official Immunization Card"
                            }
                            
                            if health_records.is_empty() {
                                p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No immunization data recorded." }
                            } else {
                                div { class: "flex flex-col gap-3.5",
                                    for r in health_records.iter() {
                                        div { class: "flex items-center justify-between border border-border/30 p-3 rounded-xl bg-white/[0.01]",
                                            div { class: "flex flex-col gap-0.5",
                                                span { class: "text-xs font-black text-foreground", "{r.vaccine_name}" }
                                                if let Some(ref date) = r.administered_at {
                                                    span { class: "text-[10px] text-muted-foreground font-semibold", "Administered: {date}" }
                                                } else {
                                                    span { class: "text-[10px] text-muted-foreground italic", "Pending schedule" }
                                                }
                                            }
                                            match r.status.as_str() {
                                                "completed" => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Completed" } },
                                                "exempted" => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-indigo-500/10 text-indigo-400 border border-indigo-500/15 shadow-sm", "Exempt" } },
                                                _ => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-amber-500/10 text-amber-400 border border-amber-500/15 animate-pulse", "Pending" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right: Nurse Log History Card
                        components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "heart", size: "18", class: "text-primary" }
                                "School Nurse Check-in Logs"
                            }
                            
                            if health_incidents.is_empty() {
                                p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No nurse visit logs recorded for this student." }
                            } else {
                                div { class: "flex flex-col gap-3.5",
                                    for inc in health_incidents.iter() {
                                        div { class: "border border-border/30 p-3 rounded-xl bg-white/[0.01] flex flex-col gap-2",
                                            div { class: "flex justify-between items-center",
                                                span { class: "text-xs font-black text-foreground", "{inc.visit_reason}" }
                                                span { class: "text-[10px] text-muted-foreground font-semibold", "{inc.checked_in_at}" }
                                            }
                                            div { class: "text-[10px] text-muted-foreground", "Treatment: {inc.treatment}" }
                                            if let Some(ref note) = inc.notes {
                                                div { class: "text-[10px] bg-sidebar/50 p-2 rounded border border-border/20 text-muted-foreground italic",
                                                    "\"{note}\""
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if *active_tab.read() == "billing" {
                    {
                        let total_due: f64 = invoices.iter()
                            .filter(|i| i.status == "unpaid")
                            .map(|i| i.amount)
                            .sum();
                        
                        rsx! {
                            div { class: "grid gap-6 md:grid-cols-3 items-start",
                                // Left: Balance & Payment Simulator Card
                                div { class: "md:col-span-1 flex flex-col gap-4",
                                    components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "wallet", size: "16", class: "text-primary" }
                                            "Tuition Balance"
                                        }
                                        
                                        div { class: "p-4 rounded-xl bg-sidebar/40 border border-border/20 flex flex-col gap-1 text-center",
                                            span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Total Outstanding" }
                                            span { class: "text-2xl font-black text-foreground", "${total_due:.2}" }
                                        }

                                        // Swish Payment simulator
                                        if total_due > 0.0 {
                                            div { class: "flex flex-col gap-3 pt-3 border-t border-border/30",
                                                span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Simulated Payment Gateway" }
                                                
                                                button {
                                                    class: "w-full py-2.5 rounded-xl text-xs font-black bg-emerald-500 text-white hover:bg-emerald-600 transition-all shadow-md flex items-center justify-center gap-2",
                                                    onclick: {
                                                        let ws = workspace_id.clone();
                                                        let unpaid_invs = invoices.iter().filter(|i| i.status == "unpaid").cloned().collect::<Vec<_>>();
                                                        let uid = active_user.id.clone();
                                                        move |_| {
                                                            let ws_c = ws.clone();
                                                            let invs = unpaid_invs.clone();
                                                            let uid_c = uid.clone();
                                                            spawn(async move {
                                                                // Pay all unpaid invoices
                                                                for inv in invs.iter() {
                                                                    let now_str = std::time::SystemTime::now()
                                                                        .duration_since(std::time::UNIX_EPOCH)
                                                                        .map(|d| {
                                                                            let secs = d.as_secs() as i64;
                                                                            let mut days = secs / 86400;
                                                                            let mut year = 1970;
                                                                            loop {
                                                                                let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                                let days_in_year = if leap { 366 } else { 365 };
                                                                                if days >= days_in_year {
                                                                                    days -= days_in_year;
                                                                                    year += 1;
                                                                                } else {
                                                                                    break;
                                                                                }
                                                                            }
                                                                            let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                            let month_lengths = if leap {
                                                                                [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                            } else {
                                                                                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                                            };
                                                                            let mut month = 1;
                                                                            for &length in month_lengths.iter() {
                                                                                if days >= length {
                                                                                    days -= length;
                                                                                    month += 1;
                                                                                } else {
                                                                                    break;
                                                                                }
                                                                            }
                                                                            let day = days + 1;
                                                                            let tod_secs = secs % 86400;
                                                                            let hour = tod_secs / 3600;
                                                                            let min = (tod_secs % 3600) / 60;
                                                                            format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hour, min)
                                                                        })
                                                                        .unwrap_or_else(|_| "2026-07-04 12:00".to_string());
                                                                    let _ = yntra_core::record_school_payment(uid_c.clone(), ws_c.clone(), inv.id.clone(), inv.amount, "card_simulation".to_string(), now_str).await;
                                                                }
                                                                let current = *db_trigger.read();
                                                                db_trigger.set(current + 1);
                                                            });
                                                        }
                                                    },
                                                    components::LucideIcon { name: "credit-card", size: "14" }
                                                    "Pay Full Balance (Simulated)"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Right: Invoices and Invoicing History Table
                                div { class: "md:col-span-2 flex flex-col gap-4",
                                    components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "credit-card", size: "16", class: "text-primary" }
                                            "Invoice Registry & Payments"
                                        }

                                        if invoices.is_empty() {
                                            p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No billing invoices found." }
                                        } else {
                                            div { class: "flex flex-col gap-4",
                                                for invoice in invoices.iter() {
                                                    div { key: "{invoice.id}", class: "border border-border/30 p-4 rounded-xl bg-white/[0.01] flex flex-col gap-2.5",
                                                        div { class: "flex justify-between items-center",
                                                            div { class: "flex flex-col gap-0.5",
                                                                span { class: "text-sm font-black text-foreground", "{invoice.title}" }
                                                                span { class: "text-[10px] text-muted-foreground font-semibold", "Due Date: {invoice.due_date}" }
                                                            }
                                                            div { class: "flex items-center gap-2.5",
                                                                span { class: "text-sm font-black text-foreground", "${invoice.amount:.2}" }
                                                                match invoice.status.as_str() {
                                                                    "paid" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Paid" } },
                                                                    _ => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-amber-500/10 text-amber-400 border border-amber-500/15 shadow-sm animate-pulse", "Unpaid" } }
                                                                }
                                                            }
                                                        }
                                                        
                                                        if let Some(ref paid_at) = invoice.paid_at {
                                                            div { class: "text-[10px] text-muted-foreground bg-sidebar/30 p-2 rounded border border-border/20 font-medium",
                                                                "Receipt: Paid via Simulator on {paid_at}"
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
                } else if *active_tab.read() == "library" {
                    div { class: "grid gap-6 md:grid-cols-3 items-start",
                        // Left: Active loans count summary
                        div { class: "md:col-span-1 flex flex-col gap-4",
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                                h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                                    "Library Status"
                                }
                                
                                {
                                    let active_count = library_logs.iter().filter(|l| l.status != "returned").count();
                                    let overdue_count = library_logs.iter().filter(|l| l.status == "overdue").count();
                                    
                                    rsx! {
                                        div { class: "flex flex-col gap-3",
                                            div { class: "p-4 rounded-xl bg-sidebar/40 border border-border/20 flex flex-col gap-1 text-center",
                                                span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Active Loans" }
                                                span { class: "text-2xl font-black text-foreground", "{active_count}" }
                                            }
                                            
                                            if overdue_count > 0 {
                                                div { class: "p-3 rounded-xl border border-rose-500/20 bg-rose-500/5 flex flex-col items-center text-center gap-1 text-rose-400 font-bold",
                                                    components::LucideIcon { name: "time", size: "18", class: "animate-pulse" }
                                                    span { class: "text-[10px] uppercase tracking-wider", "Overdue Books" }
                                                    span { class: "text-base font-black", "{overdue_count} OVERDUE" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right: Book loans registry
                        div { class: "md:col-span-2 flex flex-col gap-4",
                            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4 shadow-sm",
                                h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                                    "Asset Loan Ledger"
                                }

                                if library_logs.is_empty() {
                                    p { class: "text-xs text-muted-foreground italic text-center p-8 m-0", "No books borrowed by this student." }
                                } else {
                                    div { class: "flex flex-col gap-4",
                                        for loan in library_logs.iter() {
                                            {
                                                let book_title = library_books.iter()
                                                    .find(|b| b.id == loan.book_id)
                                                    .map(|b| b.title.clone())
                                                    .unwrap_or_else(|| "Unknown Book".to_string());
                                                let is_returned = loan.status == "returned";
                                                
                                                rsx! {
                                                    div { key: "{loan.id}", class: "border border-border/30 p-4 rounded-xl bg-white/[0.01] flex flex-col gap-2.5",
                                                        div { class: "flex justify-between items-center",
                                                            div { class: "flex flex-col gap-0.5",
                                                                span { class: "text-sm font-black text-foreground", "{book_title}" }
                                                                span { class: "text-[10px] text-muted-foreground font-semibold", "Borrowed: {loan.checked_out_at} | Due: {loan.due_date}" }
                                                            }
                                                            div { class: "flex items-center gap-2.5",
                                                                match loan.status.as_str() {
                                                                    "returned" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Returned" } },
                                                                    "overdue" => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-rose-500/10 text-rose-400 border border-rose-500/15 shadow-sm animate-pulse", "Overdue" } },
                                                                    _ => rsx! { span { class: "px-2.5 py-0.5 rounded text-[10px] font-black bg-primary/10 text-primary border border-primary/15 shadow-sm", "Active" } }
                                                                }
                                                            }
                                                        }
                                                        
                                                        if is_returned {
                                                            if let Some(ref ret_at) = loan.returned_at {
                                                                div { class: "text-[10px] text-muted-foreground bg-sidebar/30 p-2 rounded border border-border/20 font-medium",
                                                                    "Returned and checked in on {ret_at}"
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
