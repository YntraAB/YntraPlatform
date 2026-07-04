use dioxus::prelude::*;
use crate::components;
use crate::locales;
use yntra_core::{StudentProfile, Assignment, Submission, Course};

#[derive(Props, Clone)]
pub struct StudentPortalProps {
    pub active_user_id: String,
    pub students: Vec<StudentProfile>,
    pub selected_student: Option<StudentProfile>,
    pub impersonated_student_id: Signal<Option<String>>,
    pub student_unsubmitted_assigns: Vec<Assignment>,
    pub student_subs: Vec<Submission>,
    pub all_assignments: Vec<Assignment>,
    pub courses: Vec<Course>,
    pub submit_assignment_id: Signal<Option<String>>,
    pub submission_text: Signal<String>,
    pub show_submit_modal: Signal<bool>,
    pub locale: String,
}

impl PartialEq for StudentPortalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn StudentPortal(props: StudentPortalProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let requester_user_id = state.active_user_id.read().clone();
    let students = props.students.clone();
    let selected_student = props.selected_student.clone();
    let mut impersonated_student_id = props.impersonated_student_id;
    let student_unsubmitted_assigns = props.student_unsubmitted_assigns.clone();
    let student_subs = props.student_subs.clone();
    let all_assignments = props.all_assignments.clone();
    let courses = props.courses.clone();
    let mut submit_assignment_id = props.submit_assignment_id;
    let mut submission_text = props.submission_text;
    let mut show_submit_modal = props.show_submit_modal;
    let locale = props.locale.clone();

    // Fetch report cards for selected student
    let s_id_for_rc = selected_student.as_ref().map(|s| s.id.clone());
    let req_rc = requester_user_id.clone();
    let report_cards_res = use_resource(move || {
        let s_id = s_id_for_rc.clone();
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

    // Fetch timetable slots
    let timetable_res = use_resource(move || {
        async move {
            yntra_core::get_timetable_slots().await.unwrap_or_default()
        }
    });
    let timetable_slots = timetable_res.read().clone().unwrap_or_default();

    // Fetch student's health records
    let s_id_for_health = selected_student.as_ref().map(|s| s.id.clone());
    let req_id_records = requester_user_id.clone();
    let health_records_res = use_resource(move || {
        let s_id = s_id_for_health.clone();
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

    // Fetch student's health incidents
    let s_id_for_inc = selected_student.as_ref().map(|s| s.id.clone());
    let req_id_incidents = requester_user_id.clone();
    let health_incidents_res = use_resource(move || {
        let s_id = s_id_for_inc.clone();
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

    // Fetch student's invoices
    let s_id_for_billing = selected_student.as_ref().map(|s| s.id.clone());
    let req_billing = requester_user_id.clone();
    let invoices_res = use_resource(move || {
        let s_id = s_id_for_billing.clone();
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

    // Fetch student's library loans
    let s_id_for_library = selected_student.as_ref().map(|s| s.id.clone());
    let req_library = requester_user_id.clone();
    let library_logs_res = use_resource(move || {
        let s_id = s_id_for_library.clone();
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

    // Fetch all library books to lookup titles
    let library_books_res = use_resource(move || {
        async move {
            yntra_core::get_library_books().await.unwrap_or_default()
        }
    });
    let library_books = library_books_res.read().clone().unwrap_or_default();

    rsx! {
        if let Some(ref student) = selected_student {
            div { class: "flex flex-col gap-6",
                // Superstar Welcome Header with Impersonator Dropdown
                div {
                    class: "relative p-6 rounded-2xl overflow-hidden border border-primary/20 shadow-lg flex flex-col md:flex-row justify-between items-start md:items-center gap-4 bg-gradient-to-r from-primary/10 via-purple-500/5 to-indigo-500/10",
                    
                    div { class: "flex items-center gap-4",
                        div { class: "p-3 rounded-xl bg-primary/10 text-primary",
                            components::LucideIcon { name: "sparkles", size: "32", class: "animate-pulse text-amber-400" }
                        }
                        div { class: "flex flex-col gap-1",
                            h2 { class: "text-2xl font-black text-foreground m-0 flex items-center gap-2",
                                { locales::t_with_args("school-welcome", &locale, &[("name", &student.first_name)]) }
                            }
                            p { class: "text-xs text-muted-foreground m-0 font-medium",
                                { locales::t("school-ready-msg", &locale) }
                            }
                        }
                    }

                    // Student Picker Dropdown
                    div { class: "flex flex-col gap-1 self-stretch md:self-auto",
                        label { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider",
                            { locales::t("school-change-profile", &locale) }
                        }
                        select {
                            class: "yntra-input py-1.5 px-3 text-xs bg-sidebar border border-border text-foreground rounded-lg w-full md:w-56 font-bold shadow-sm",
                            value: student.id.clone(),
                            onchange: move |e| impersonated_student_id.set(Some(e.value())),
                            for s in students.iter() {
                                option { value: "{s.id}", "{s.first_name} {s.last_name}" }
                            }
                        }
                    }
                }

                // Dashboard Columns
                div { class: "grid gap-6 md:grid-cols-3",
                    // Column 1: Daily Schedule & Achievements
                    div { class: "flex flex-col gap-6 md:col-span-1",
                        // Timetable / Schema
                        // Timetable / Schema
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            div { class: "flex justify-between items-center",
                                h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                    components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                                    { locales::t("school-my-timetable", &locale) }
                                }
                                
                                button {
                                    class: "p-1 rounded hover:bg-muted text-primary transition-all flex items-center justify-center",
                                    title: "Sync to Calendar",
                                    onclick: {
                                        let ws = student.workspace_id.clone();
                                        let s_id = student.id.clone();
                                        move |_| {
                                            let ws_clone = ws.clone();
                                            let s_clone = s_id.clone();
                                            spawn(async move {
                                                let _ = yntra_core::sync_timetable_to_calendar(ws_clone, s_clone).await;
                                            });
                                        }
                                    },
                                    components::LucideIcon { name: "refresh-cw", size: "14" }
                                }
                            }
                            
                            if timetable_slots.is_empty() {
                                p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", {locales::t("school-timetable-no-slots", &locale)} }
                            } else {
                                div { class: "flex flex-col gap-3.5",
                                    for s in timetable_slots.iter().take(5) {
                                        div { class: "flex items-start gap-3 border-l-4 border-primary pl-3 py-0.5",
                                            div { class: "text-xs font-bold text-muted-foreground w-12", "{s.start_time}" }
                                            div { class: "flex flex-col gap-0.5",
                                                div { class: "text-sm font-extrabold text-foreground flex items-center gap-1.5",
                                                    {
                                                        let course = courses.iter().find(|c| c.id == s.course_id);
                                                        course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                    }
                                                    components::LucideIcon { name: "book-open", size: "14", class: "text-primary" }
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

                        // Medical & Immunization overview
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "heart", size: "16", class: "text-primary" }
                                { locales::t("school-health-title", &locale) }
                            }
                            
                            if health_records.is_empty() {
                                p { class: "text-xs text-muted-foreground italic text-center p-2 m-0", "No immunization data recorded." }
                            } else {
                                div { class: "flex flex-col gap-2.5",
                                    for r in health_records.iter().take(3) {
                                        div { class: "flex justify-between items-center bg-sidebar/40 p-2 rounded-lg border border-border/20 text-xs font-semibold",
                                            span { "{r.vaccine_name}" }
                                            match r.status.as_str() {
                                                "completed" => rsx! { span { class: "text-[10px] text-emerald-400 font-bold", "Completed" } },
                                                "exempted" => rsx! { span { class: "text-[10px] text-indigo-400 font-bold", "Exempt" } },
                                                _ => rsx! { span { class: "text-[10px] text-amber-400 font-bold", "Pending" } }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if !health_incidents.is_empty() {
                                div { class: "flex flex-col gap-2 border-t border-border/30 pt-3",
                                    span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Recent Nurse Visits" }
                                    for inc in health_incidents.iter().take(2) {
                                        div { class: "flex flex-col gap-0.5 pl-2 border-l-2 border-primary/50 text-xs",
                                            div { class: "flex justify-between items-center text-[10px] font-black text-foreground",
                                                span { "{inc.visit_reason}" }
                                                span { class: "text-[9px] text-muted-foreground font-normal", "{inc.checked_in_at}" }
                                            }
                                            span { class: "text-[9px] text-muted-foreground", "Treatment: {inc.treatment}" }
                                        }
                                    }
                                }
                            }
                        }

                        // Tuition & Fees overview
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "credit-card", size: "16", class: "text-primary" }
                                { locales::t("school-billing-title", &locale) }
                            }
                            
                            {
                                let student_balance: f64 = invoices.iter()
                                    .filter(|i| i.status == "unpaid")
                                    .map(|i| i.amount)
                                    .sum();
                                rsx! {
                                    div { class: "flex justify-between items-center bg-sidebar/40 p-3 rounded-xl border border-border/20",
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", {locales::t("school-billing-outstanding", &locale)} }
                                            span { class: "text-lg font-black text-foreground", "${student_balance:.2}" }
                                        }
                                        components::LucideIcon { name: "wallet", size: "24", class: "text-primary" }
                                    }
                                }
                            }
                            
                            if !invoices.is_empty() {
                                div { class: "flex flex-col gap-2 border-t border-border/30 pt-3",
                                    span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Recent Invoices" }
                                    for inv in invoices.iter().take(2) {
                                        div { class: "flex justify-between items-center text-xs font-semibold pl-2 border-l-2 border-primary/50 py-0.5",
                                            div { class: "flex flex-col gap-0.5",
                                                span { "{inv.title}" }
                                                span { class: "text-[9px] text-muted-foreground font-normal", "Due: {inv.due_date}" }
                                            }
                                            span { class: "text-foreground", "${inv.amount:.2}" }
                                        }
                                    }
                                }
                            }
                        }

                        // Library Loans overview
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                                { locales::t("school-library-title", &locale) }
                            }
                            
                            {
                                let active_loans: Vec<_> = library_logs.iter().filter(|l| l.status != "returned").collect();
                                rsx! {
                                    if active_loans.is_empty() {
                                        p { class: "text-xs text-muted-foreground italic text-center p-2 m-0", "No books currently checked out." }
                                    } else {
                                        div { class: "flex flex-col gap-2.5",
                                            for loan in active_loans.iter() {
                                                {
                                                    let title = library_books.iter()
                                                        .find(|b| b.id == loan.book_id)
                                                        .map(|b| b.title.clone())
                                                        .unwrap_or_else(|| "Unknown Book".to_string());
                                                    let is_overdue = loan.status == "overdue";
                                                    let status_color = if is_overdue { "text-rose-400 font-bold" } else { "text-muted-foreground font-semibold" };
                                                    
                                                    rsx! {
                                                        div { key: "{loan.id}", class: "flex flex-col gap-1 bg-sidebar/40 p-2.5 rounded-lg border border-border/20 text-xs",
                                                            div { class: "flex justify-between items-start font-semibold",
                                                                span { class: "font-black text-foreground", "{title}" }
                                                                if is_overdue {
                                                                    span { class: "text-[9px] text-rose-400 font-bold tracking-wider animate-pulse", "OVERDUE" }
                                                                }
                                                            }
                                                            div { class: "flex justify-between items-center text-[10px] {status_color}",
                                                                span { "Due: {loan.due_date}" }
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

                        // Study Stars & Badges
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "award", size: "16", class: "text-primary" }
                                { locales::t("school-my-stars", &locale) }
                            }

                            div { class: "grid gap-3 grid-cols-2",
                                div { class: "p-3 rounded-xl border border-primary/20 bg-primary/5 flex flex-col items-center text-center gap-1.5",
                                    components::LucideIcon { name: "rocket", size: "28", class: "text-primary" }
                                    div { class: "text-xs font-black text-foreground", "Rocket Reader" }
                                    div { class: "text-[9px] text-muted-foreground leading-tight", "Read 5 books this month!" }
                                }
                                div { class: "p-3 rounded-xl border border-amber-500/20 bg-amber-500/5 flex flex-col items-center text-center gap-1.5",
                                    components::LucideIcon { name: "star", size: "28", class: "text-amber-400 animate-pulse" }
                                    div { class: "text-xs font-black text-foreground", "Math Whiz" }
                                    div { class: "text-[9px] text-muted-foreground leading-tight", "Perfect division quiz!" }
                                }
                                div { class: "p-3 rounded-xl border border-emerald-500/20 bg-emerald-500/5 flex flex-col items-center text-center gap-1.5",
                                    components::LucideIcon { name: "palette", size: "28", class: "text-emerald-400" }
                                    div { class: "text-xs font-black text-foreground", "Creative Mind" }
                                    div { class: "text-[9px] text-muted-foreground leading-tight", "Great watercolor work!" }
                                }
                                div { class: "p-3 rounded-xl border border-purple-500/20 bg-purple-500/5 flex flex-col items-center text-center gap-1.5 opacity-50",
                                    components::LucideIcon { name: "trophy", size: "28", class: "text-purple-400" }
                                    div { class: "text-xs font-black text-foreground", "Homework Hero" }
                                    div { class: "text-[9px] text-muted-foreground leading-tight", "Complete next 2 tasks" }
                                }
                            }
                        }
                    }

                    // Column 2 & 3: Active Homework & Graded Tasks
                    div { class: "flex flex-col gap-6 md:col-span-2",
                        // Homework list
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "book-open", size: "16", class: "text-primary" }
                                { locales::t("school-my-homework", &locale) }
                            }

                            if student_unsubmitted_assigns.is_empty() {
                                div { class: "text-center p-8 bg-emerald-500/5 border border-emerald-500/10 rounded-xl text-emerald-400 text-sm font-bold flex flex-col items-center gap-1.5",
                                    components::LucideIcon { name: "check-circle", size: "32", class: "text-emerald-400" }
                                    { locales::t("school-all-homework-done", &locale) }
                                }
                            } else {
                                div { class: "flex flex-col gap-3",
                                    for a in student_unsubmitted_assigns.iter() {
                                        div { class: "border border-border/40 p-4 rounded-xl flex justify-between items-center hover:border-primary/45 transition-all bg-white/[0.01]",
                                            div { class: "flex flex-col gap-1",
                                                span { class: "text-[9px] font-black uppercase tracking-wider text-primary px-2 py-0.5 bg-primary/10 rounded self-start",
                                                    {
                                                        let course = courses.iter().find(|c| c.id == a.course_id);
                                                        course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                    }
                                                }
                                                div { class: "font-bold text-foreground mt-1", "{a.title}" }
                                                div { class: "text-xs text-muted-foreground", "{a.description}" }
                                            }
                                            div { class: "text-right flex flex-col gap-2 items-end",
                                                span { class: "text-xs font-bold text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded", "Due: {a.due_date}" }
                                                button {
                                                    class: "yntra-btn text-xs font-bold py-1 px-3 flex items-center gap-1",
                                                    onclick: {
                                                        let assign_id = a.id.clone();
                                                        move |_| {
                                                            submit_assignment_id.set(Some(assign_id.clone()));
                                                            submission_text.set(String::new());
                                                            show_submit_modal.set(true);
                                                        }
                                                    },
                                                    components::LucideIcon { name: "plus", size: "12" }
                                                    { locales::t("school-submit-homework", &locale) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Graded Homework & Teacher Feedback
                        components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                            h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                components::LucideIcon { name: "reporting", size: "16", class: "text-primary" }
                                { locales::t("school-submitted-answers", &locale) }
                            }

                            if student_subs.is_empty() {
                                div { class: "text-center p-8 text-muted-foreground text-sm",
                                    { locales::t("school-no-homework-submitted", &locale) }
                                }
                            } else {
                                div { class: "flex flex-col gap-3.5",
                                    for s in student_subs.iter() {
                                        div { class: "border border-border/40 p-4 rounded-xl flex flex-col gap-2 bg-white/[0.01]",
                                            div { class: "flex justify-between items-start",
                                                div {
                                                    div { class: "font-bold text-foreground",
                                                        {
                                                            let assignment = all_assignments.iter().find(|a| a.id == s.assignment_id);
                                                            assignment.map(|a| a.title.clone()).unwrap_or_else(|| "Assignment".to_string())
                                                        }
                                                    }
                                                    div { class: "text-xs text-muted-foreground mt-0.5", "Submitted: {s.submitted_at}" }
                                                }
                                                
                                                div { class: "flex items-center gap-2",
                                                    if let Some(ref g) = s.grade {
                                                        span { class: "text-xs font-black px-2.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/15", "Grade: {g}" }
                                                    } else {
                                                        span { class: "text-xs font-black px-2.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/15 animate-pulse", { locales::t("school-waiting-grade", &locale) } }
                                                    }
                                                }
                                            }
                                            
                                            // Answer text
                                            div { class: "text-xs bg-sidebar/50 p-2.5 rounded border border-border/20 text-muted-foreground font-medium",
                                                div { class: "font-bold text-foreground/80 mb-0.5", { locales::t("school-my-answer", &locale) } }
                                                "{s.content}"
                                            }

                                            // Teacher Feedback Note
                                            if let Some(ref fb) = s.feedback {
                                                div {
                                                    class: "relative p-3 rounded-lg border border-yellow-500/20 bg-yellow-500/5 text-xs text-amber-200 mt-1 flex gap-2 items-start",
                                                    components::LucideIcon { name: "pen-tool", size: "16", class: "text-amber-400 mt-0.5" }
                                                    div {
                                                        div { class: "font-black text-amber-400/90 mb-0.5", { locales::t("school-teacher-feedback", &locale) } }
                                                        "\"{fb}\""
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Report Cards & Transcripts Section
                        if !report_cards.is_empty() {
                            components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "award", size: "16", class: "text-primary" }
                                        "My Term Report Cards"
                                    }
                                    
                                    div { class: "flex flex-col gap-3",
                                        for rc in report_cards.iter().filter(|r| r.status == "published") {
                                            div { 
                                                key: "{rc.id}",
                                                class: "border border-yellow-500/20 p-4 rounded-xl bg-gradient-to-br from-yellow-500/5 via-sidebar/20 to-primary/5 flex justify-between items-center gap-4 flex-wrap",
                                                div { class: "flex flex-col gap-1",
                                                    span { class: "text-xs font-black uppercase text-yellow-500 tracking-wider", "{rc.term_name}" }
                                                    span { class: "text-sm font-bold text-foreground", "GPA: {rc.gpa:.2}" }
                                                    if let Some(ref comment) = rc.principal_comments {
                                                        span { class: "text-xs text-muted-foreground italic mt-1", "\"{comment}\"" }
                                                    }
                                                }
                                                
                                                button {
                                                    class: "yntra-btn-secondary text-xs font-bold py-1.5 px-3.5 flex items-center gap-1.5 shadow-sm",
                                                    onclick: move |_| {
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            if let Some(w) = web_sys::window() {
                                                                    let _ = w.print();
                                                            }
                                                        }
                                                    },
                                                    components::LucideIcon { name: "printer", size: "12" }
                                                    "Print Transcript"
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
            div { class: "text-center p-12 text-muted-foreground border border-dashed border-border/40 rounded-xl",
                components::LucideIcon { name: "directory", size: "40", class: "opacity-20 mb-2 mx-auto" }
                p { class: "text-sm font-semibold m-0", { locales::t("school-no-enrolled-students", &locale) } }
            }
        }
    }
}
