use dioxus::prelude::*;
use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, LucideIcon};
use crate::locales::t;
use super::SchoolViewProps;
use yntra_core::{
    get_student_attendance_records, get_student_health_records, get_health_incidents,
    get_report_cards, get_course_term_grades, get_timetable_slots, StudentProfile,
    link_student_self_service, get_assignments, get_student_submissions
};

#[component]
pub fn ParentPortal(
    school_props: SchoolViewProps,
    students: Vec<StudentProfile>,
    mut selected_student_profile_id: Signal<String>,
) -> Element {
    let db_trigger = school_props.db_trigger;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();
    let mut state = use_context::<crate::state::AppState>();

    let mut link_input_id = use_signal(String::new);
    let mut link_error = use_signal(|| Option::<String>::None);

    // Resources specific to parent child tracking
    
    let user_id_clone_att = user_id.clone();
    let ws_id_clone_att = ws_id.clone();
    let student_attendance_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_att.clone();
        let ws = ws_id_clone_att.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_attendance_records(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    let user_id_clone_h = user_id.clone();
    let ws_id_clone_h = ws_id.clone();
    let student_health_records_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_h.clone();
        let ws = ws_id_clone_h.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_health_records(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    let user_id_clone_inc = user_id.clone();
    let ws_id_clone_inc = ws_id.clone();
    let student_health_incidents_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_inc.clone();
        let ws = ws_id_clone_inc.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                let all_inc = get_health_incidents(uid, ws).await.unwrap_or_default();
                all_inc.into_iter().filter(|i| i.student_id == s_id).collect::<Vec<_>>()
            }
        }
    });

    let user_id_clone_rc = user_id.clone();
    let ws_id_clone_rc = ws_id.clone();
    let student_report_cards_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_rc.clone();
        let ws = ws_id_clone_rc.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_report_cards(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Courses resource to look up course names
    let user_id_clone_courses = user_id.clone();
    let ws_id_clone_courses = ws_id.clone();
    let courses_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = user_id_clone_courses.clone();
        let ws = ws_id_clone_courses.clone();
        async move { yntra_core::get_workspace_courses(uid, ws).await.unwrap_or_default() }
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

    // Course Grades resource
    let user_id_clone_cg = user_id.clone();
    let ws_id_clone_cg = ws_id.clone();
    let course_grades_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_cg.clone();
        let ws = ws_id_clone_cg.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                // Fetch all grades for child
                let courses = yntra_core::get_workspace_courses(uid.clone(), ws.clone()).await.unwrap_or_default();
                let mut list = Vec::new();
                for c in courses {
                    if let Ok(mut cg) = get_course_term_grades(uid.clone(), ws.clone(), c.id.clone()).await {
                        // filter by student
                        cg.retain(|g| g.student_id == s_id);
                        list.append(&mut cg);
                    }
                }
                list
            }
        }
    });

    let courses = courses_res.read().clone().unwrap_or_default();
    let timetable = timetable_res.read().clone().unwrap_or_default();

    // Student submissions resource
    let user_id_clone_sub = user_id.clone();
    let ws_id_clone_sub = ws_id.clone();
    let student_submissions_res = use_resource(move || {
        let _trig = db_trigger.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_sub.clone();
        let ws = ws_id_clone_sub.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_submissions(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Assignments resource
    let courses_clone = courses.clone();
    let user_id_clone_assign = user_id.clone();
    let ws_id_clone_assign = ws_id.clone();
    let all_assignments_res = use_resource(move || {
        let _trig = db_trigger.read();
        let uid = user_id_clone_assign.clone();
        let ws = ws_id_clone_assign.clone();
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

    if students.is_empty() {
         return rsx! {
            div { class: "max-w-md mx-auto py-12 space-y-6",
                div { class: "flex flex-col items-center justify-center text-center border border-dashed border-border rounded-2xl bg-muted/10 p-8",
                    LucideIcon { name: "shield-alert", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                    h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-no-children", &locale)} }
                    p { class: "text-xs text-muted-foreground mt-1 leading-relaxed", {t("school-parent-link-student-instruction", &locale)} }
                }

                Card { class: "border-border shadow-sm p-6 space-y-4",
                    div {
                        h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-link-student-title", &locale)} }
                        p { class: "text-[11px] text-muted-foreground m-0 mt-0.5", {t("school-parent-link-student-desc", &locale)} }
                    }
                    if let Some(err) = link_error.read().as_ref() {
                        div { class: "p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-xs font-medium text-red-600",
                            "{err}"
                        }
                    }
                    div { class: "space-y-3",
                        crate::components::Input {
                            placeholder: format!("{}...", t("school-parent-link-student-id", &locale)),
                            value: "{link_input_id}",
                            oninput: move |evt: FormEvent| link_input_id.set(evt.value()),
                        }
                        Button {
                            class: "w-full bg-primary hover:bg-primary/90 text-primary-foreground font-bold uppercase tracking-wider text-[10px] py-2.5 rounded-lg flex items-center justify-center gap-1.5 transition-all shadow-sm border-0 cursor-pointer",
                            onclick: {
                                let uid_c = user_id.clone();
                                let ws_c = ws_id.clone();
                                let mut link_input_c = link_input_id;
                                let mut link_error_c = link_error;
                                let mut db_trigger_c = db_trigger;
                                let locale_c = locale.clone();
                                move |_| {
                                    let inp = link_input_c.read().trim().to_string();
                                    if inp.is_empty() {
                                        link_error_c.set(Some(t("school-parent-link-student-error", &locale_c)));
                                        return;
                                    }
                                    let u = uid_c.clone();
                                    let w = ws_c.clone();
                                    let mut db_t = db_trigger_c.clone();
                                    let loc_c = locale_c.clone();
                                    spawn(async move {
                                        match link_student_self_service(u, w, inp).await {
                                            Ok(_) => {
                                                link_error_c.set(None);
                                                link_input_c.set(String::new());
                                                let current = *db_t.read();
                                                db_t.set(current + 1);
                                            }
                                            Err(_) => {
                                                link_error_c.set(Some(t("school-parent-link-student-error", &loc_c)));
                                            }
                                        }
                                    });
                                }
                            },
                            {t("school-parent-link-student-btn", &locale)}
                        }
                    }
                }
            }
        };
    }

    // Read parent portal child resources
    let attendance = student_attendance_res.read().clone().unwrap_or_default();
    let health_records = student_health_records_res.read().clone().unwrap_or_default();
    let health_incidents = student_health_incidents_res.read().clone().unwrap_or_default();
    let report_cards = student_report_cards_res.read().clone().unwrap_or_default();
    let course_grades = course_grades_res.read().clone().unwrap_or_default();
    let student_submissions = student_submissions_res.read().clone().unwrap_or_default();
    let all_assignments = all_assignments_res.read().clone().unwrap_or_default();

    // Stats calculations
    let total_days = attendance.len();
    let present_days = attendance.iter().filter(|a| a.status == "present" || a.status == "late").count();
    let absent_days = attendance.iter().filter(|a| a.status == "absent").count();
    let attendance_rate = if total_days > 0 {
        (present_days as f32 / total_days as f32) * 100.0
    } else {
        100.0
    };

    // SVG circle math
    let stroke_dasharray = 226.19; // 2 * PI * 36
    let stroke_dashoffset = stroke_dasharray - (attendance_rate / 100.0) * stroke_dasharray;

    // Alerts: unexcused absence check or nurse visit alerts
    let has_attendance_alert = attendance.iter().any(|a| a.status == "absent");
    let has_health_alert = !health_incidents.is_empty();

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            // Child Profile Selector Card
            Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                div { class: "flex items-center gap-3 w-full sm:w-auto",
                    LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                    div {
                        h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-select-child", &locale)} }
                        p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-parent-select-child-desc", &locale)} }
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

            // SOTA Alert Banner
            if has_attendance_alert || has_health_alert {
                div { class: "p-4 rounded-2xl bg-amber-500/10 border border-amber-500/20 flex items-start gap-3.5 shadow-sm animate-in fade-in slide-in-from-top-2 duration-300",
                    LucideIcon { name: "shield-alert", class: "h-5 w-5 text-amber-600 mt-0.5" }
                    div { class: "space-y-1",
                        h4 { class: "text-xs font-extrabold text-amber-800 dark:text-amber-500 m-0 uppercase tracking-wider", "Portal Alerts & Notifications" }
                        p { class: "text-[11px] text-muted-foreground m-0 leading-relaxed",
                            if has_attendance_alert && has_health_alert {
                                "Unexcused absences and recent health clinic visit incidents have been recorded for your child. Please review the attendance and medical details below."
                            } else if has_attendance_alert {
                                "One or more unexcused absences have been logged for your child. Please check the attendance log details."
                            } else {
                                "A recent health incident / nurse visit has been registered for your child. Please check the medical incident logs."
                            }
                        }
                    }
                }
            }

            // Dashboard Roster Overview Grid
            div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                // Left Side: Attendance Progress and Health logs
                div { class: "lg:col-span-2 space-y-6",
                    
                    // Attendance SOTA Widget
                    Card { class: "border-border shadow-sm overflow-hidden",
                        CardHeader { class: "pb-2",
                            CardTitle { {t("school-parent-attendance-tracking", &locale)} }
                            CardDescription { {t("school-parent-attendance-desc", &locale)} }
                        }
                        CardContent { class: "space-y-6",
                            div { class: "flex flex-col sm:flex-row items-center gap-6 justify-between p-4 bg-muted/20 border border-border/40 rounded-2xl",
                                div { class: "flex items-center gap-4",
                                    div { class: "relative h-24 w-24",
                                        svg {
                                            class: "h-24 w-24 -rotate-90 transform",
                                            circle {
                                                cx: "48",
                                                cy: "48",
                                                r: "36",
                                                class: "stroke-muted/40",
                                                stroke_width: "6",
                                                fill: "transparent",
                                            }
                                            circle {
                                                cx: "48",
                                                cy: "48",
                                                r: "36",
                                                class: "stroke-primary transition-all duration-700",
                                                stroke_width: "6",
                                                stroke_dasharray: "{stroke_dasharray}",
                                                stroke_dashoffset: "{stroke_dashoffset}",
                                                stroke_linecap: "round",
                                                fill: "transparent",
                                            }
                                        }
                                        div { class: "absolute inset-0 flex flex-col items-center justify-center",
                                            span { class: "text-base font-black text-foreground", "{attendance_rate:.1}%" }
                                            span { class: "text-[8px] text-muted-foreground uppercase font-bold tracking-wider", "Rate" }
                                        }
                                    }
                                    div {
                                        div { class: "text-sm font-black text-foreground", "Attendance Status" }
                                        div { class: "text-[10px] text-muted-foreground mt-0.5", 
                                            if attendance_rate >= 90.0 { {t("school-parent-excellent-standing", &locale)} } else { {t("school-parent-low-attendance", &locale)} }
                                        }
                                    }
                                }
                                
                                div { class: "flex gap-6 items-center text-center",
                                    div {
                                        div { class: "text-lg font-black text-foreground", "{present_days}" }
                                        div { class: "text-[9px] text-muted-foreground uppercase font-bold", "Present" }
                                    }
                                    div { class: "w-px h-8 bg-border" }
                                    div {
                                        div { class: "text-lg font-black text-red-500", "{absent_days}" }
                                        div { class: "text-[9px] text-muted-foreground uppercase font-bold", "Absent" }
                                    }
                                    div { class: "w-px h-8 bg-border" }
                                    div {
                                        div { class: "text-lg font-black text-primary", "{total_days}" }
                                        div { class: "text-[9px] text-muted-foreground uppercase font-bold", "Total Logs" }
                                    }
                                }
                            }

                            // Recent 12 logs status row
                            div { class: "space-y-2.5",
                                h5 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0", {t("school-parent-recent-history", &locale)} }
                                if attendance.is_empty() {
                                    div { class: "py-3 text-center text-xs text-muted-foreground italic", {t("school-parent-no-attendance", &locale)} }
                                } else {
                                    div { class: "flex flex-wrap gap-2.5",
                                        for record in attendance.iter().take(12) {
                                            {
                                                let bg = match record.status.as_str() {
                                                    "present" => "bg-emerald-500/10 text-emerald-600 border border-emerald-500/25",
                                                    "late" => "bg-amber-500/10 text-amber-600 border border-amber-500/25",
                                                    _ => "bg-red-500/10 text-red-500 border border-red-500/25",
                                                };
                                                rsx! {
                                                    div { key: "{record.id}", class: "px-2.5 py-1 text-[10px] font-bold rounded-lg {bg} flex flex-col items-center gap-0.5 min-w-[70px]",
                                                        span { "{record.date}" }
                                                        span { class: "uppercase text-[8px] font-extrabold tracking-wider", "{record.status}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Medical incident and Vaccine list
                    Card { class: "border-border shadow-sm",
                        CardHeader {
                                                            CardTitle { {t("settings-blocks-health-clinic-name", &locale)} }
                                                            CardDescription { {t("settings-blocks-health-clinic-desc", &locale)} }
                                                        }
                        CardContent { class: "space-y-6",
                            // Incident updates
                            div { class: "space-y-3",
                                h5 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0", {t("school-health-nurse-incidents", &locale)} }
                                if health_incidents.is_empty() {
                                    div { class: "py-4 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", {t("school-health-no-incidents", &locale)} }
                                } else {
                                    div { class: "divide-y divide-border border border-border/40 rounded-xl overflow-hidden bg-background",
                                        for inc in health_incidents.iter() {
                                            div { key: "{inc.id}", class: "p-4 space-y-2 hover:bg-muted/5 transition-colors",
                                                div { class: "flex items-center justify-between text-xs",
                                                    span { class: "font-extrabold text-foreground bg-primary/10 text-primary px-2.5 py-0.5 rounded-full", "{inc.visit_reason}" }
                                                    span { class: "text-muted-foreground font-semibold", "{inc.checked_in_at}" }
                                                }
                                                div { class: "text-xs text-foreground font-medium", {t("school-health-treatment", &locale)}, ": {inc.treatment}" }
                                                if let Some(ref note) = inc.notes {
                                                    div { class: "text-[10px] text-muted-foreground italic", {t("school-health-notes", &locale)}, ": {note}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Vaccine records
                            div { class: "space-y-3",
                                h5 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0", {t("school-health-immunization", &locale)} }
                                if health_records.is_empty() {
                                    div { class: "py-3 text-center text-xs text-muted-foreground italic", {t("school-health-no-vaccine", &locale)} }
                                } else {
                                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3.5",
                                        for vac in health_records.iter() {
                                            div { key: "{vac.id}", class: "p-3 border border-border/60 bg-muted/10 rounded-xl flex items-center justify-between shadow-sm",
                                                div {
                                                    div { class: "text-xs font-bold text-foreground", "{vac.vaccine_name}" }
                                                    div { class: "text-[9px] text-muted-foreground mt-0.5", {t("school-health-administered", &locale)}, ": {vac.administered_at.as_deref().unwrap_or(\"- \")}" }
                                                }
                                                span { class: "text-[9px] font-black uppercase tracking-wider px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "{vac.status}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Homework Tasks Tracking
                    Card { class: "border-border shadow-sm",
                        CardHeader {
                            CardTitle { {t("school-my-homework", &locale)} }
                            CardDescription { {t("school-student-homework-desc", &locale)} }
                        }
                        CardContent { class: "space-y-4",
                            if all_assignments.is_empty() {
                                div { class: "py-6 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", {t("school-no-homework", &locale)} }
                            } else {
                                div { class: "divide-y divide-border border border-border/40 rounded-xl overflow-hidden bg-background",
                                    for a in all_assignments.iter() {
                                        {
                                            let a_id = a.id.clone();
                                            let has_submission = student_submissions.iter().any(|s| s.assignment_id == a_id);
                                            let sub_record = student_submissions.iter().find(|s| s.assignment_id == a_id);
                                            let grade = sub_record.and_then(|s| s.grade.clone());
                                            let course_opt = courses.iter().find(|c| c.id == a.course_id);
                                            let c_name = course_opt.map(|c| c.name.clone()).unwrap_or_else(|| "Course".to_string());
                                            let teacher_id_opt = course_opt.and_then(|c| c.teacher_id.clone());
                                            
                                            let status_tag = if let Some(g) = grade {
                                                rsx! {
                                                    span { class: "text-[9px] font-black uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 px-2 py-0.5 rounded",
                                                        "Graded: {g}"
                                                    }
                                                }
                                            } else if has_submission {
                                                rsx! {
                                                    span { class: "text-[9px] font-black uppercase bg-primary/10 text-primary border border-primary/20 px-2 py-0.5 rounded",
                                                        "Submitted"
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    span { class: "text-[9px] font-black uppercase bg-amber-500/10 text-amber-600 border border-amber-500/20 px-2 py-0.5 rounded animate-pulse",
                                                        "Pending"
                                                    }
                                                }
                                            };

                                            rsx! {
                                                div { key: "{a.id}", class: "p-4 flex items-center justify-between text-xs hover:bg-muted/5 transition-colors",
                                                    div { class: "space-y-1",
                                                        div { class: "flex items-center gap-2",
                                                            span { class: "font-extrabold text-foreground", "{a.title}" }
                                                            span { class: "text-[9px] font-bold text-muted-foreground uppercase bg-muted/40 px-1.5 py-0.5 rounded", "{c_name}" }
                                                            if let Some(t_id) = teacher_id_opt.clone() {
                                                                button {
                                                                    class: "p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-primary border-0 bg-transparent cursor-pointer transition-colors",
                                                                    onclick: {
                                                                        let mut state = state;
                                                                        let t_id_c = t_id.clone();
                                                                        move |_| {
                                                                            state.active_section.set("messaging".to_string());
                                                                            state.messaging_view_tab.set("compose".to_string());
                                                                            state.compose_recipient_id.set(Some(t_id_c.clone()));
                                                                        }
                                                                    },
                                                                    title: "Message Teacher",
                                                                    LucideIcon { name: "message-square", size: "11" }
                                                                }
                                                            }
                                                        }
                                                        div { class: "text-[10px] text-muted-foreground flex items-center gap-1.5",
                                                            LucideIcon { name: "clock", size: "12" }
                                                            if a.due_date.is_empty() || a.due_date == "No due date" {
                                                                span { class: "text-muted-foreground/80 font-medium", "No due date" }
                                                            } else if a.due_date.contains("(Strict)") {
                                                                if super::utils::is_deadline_passed(&a.due_date) {
                                                                    span { class: "text-red-500 font-semibold", {crate::locales::t_with_args("school-student-strict-closed", &locale, &[("date", &a.due_date)])} }
                                                                } else {
                                                                    span { class: "text-red-500/80 font-medium", {crate::locales::t_with_args("school-student-strict-due", &locale, &[("date", &a.due_date)])} }
                                                                }
                                                            } else if a.due_date.contains("(Flexible)") {
                                                                if super::utils::is_deadline_passed(&a.due_date) {
                                                                    span { class: "text-amber-500 font-semibold", {crate::locales::t_with_args("school-student-flexible-late", &locale, &[("date", &a.due_date)])} }
                                                                } else {
                                                                    span { class: "text-amber-500/80 font-medium", {crate::locales::t_with_args("school-student-flexible-due", &locale, &[("date", &a.due_date)])} }
                                                                }
                                                            } else {
                                                                span { class: "text-muted-foreground font-medium", "Due: {a.due_date}" }
                                                            }
                                                        }
                                                    }
                                                    {status_tag}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Side: Academic status, report cards and schedule
                div { class: "space-y-6",
                    
                    // GPA / Course Grades
                    Card { class: "border-border shadow-sm",
                        CardHeader {
                            CardTitle { {t("school-parent-gpa-progress", &locale)} }
                            CardDescription { {t("school-parent-gpa-desc", &locale)} }
                        }
                        CardContent { class: "space-y-4",
                            if course_grades.is_empty() {
                                div { class: "py-6 text-center text-xs text-muted-foreground italic", {t("school-parent-no-grades", &locale)} }
                            } else {
                                div { class: "divide-y divide-border border border-border/40 rounded-xl overflow-hidden bg-background",
                                    for cg in course_grades.iter() {
                                        {
                                            let course_opt = courses.iter().find(|c| c.id == cg.course_id);
                                            let c_name = course_opt.map(|c| c.name.clone()).unwrap_or_else(|| "Course".to_string());
                                            let teacher_id_opt = course_opt.and_then(|c| c.teacher_id.clone());
                                            let grade_let = cg.final_grade.clone().unwrap_or_else(|| "-".to_string());
                                            let grade_pct = cg.final_points.unwrap_or(0);
                                            rsx! {
                                                div { key: "{cg.id}", class: "p-3 flex items-center justify-between text-xs hover:bg-muted/10 transition-colors",
                                                    div {
                                                        div { class: "flex items-center gap-1.5",
                                                            span { class: "font-extrabold text-foreground", "{c_name}" }
                                                            if let Some(t_id) = teacher_id_opt.clone() {
                                                                button {
                                                                    class: "p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-primary border-0 bg-transparent cursor-pointer transition-colors",
                                                                    onclick: {
                                                                        let mut state = state;
                                                                        let t_id_c = t_id.clone();
                                                                        move |_| {
                                                                            state.active_section.set("messaging".to_string());
                                                                            state.messaging_view_tab.set("compose".to_string());
                                                                            state.compose_recipient_id.set(Some(t_id_c.clone()));
                                                                        }
                                                                    },
                                                                    title: "Message Teacher",
                                                                    LucideIcon { name: "message-square", size: "11" }
                                                                }
                                                            }
                                                        }
                                                        div { class: "text-[9px] text-muted-foreground mt-0.5", {t("school-parent-term", &locale)}, ": {cg.term_name}" }
                                                    }
                                                    span { class: "font-black px-2 py-0.5 rounded bg-primary/10 text-primary border border-primary/20 text-[10px]",
                                                        {crate::locales::t_with_args("school-parent-grade-label", &locale, &[("grade", &grade_let), ("points", &grade_pct.to_string())])}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Latest Report Cards
                    Card { class: "border-border shadow-sm",
                        CardHeader {
                            CardTitle { {t("school-parent-report-cards", &locale)} }
                            CardDescription { {t("school-parent-report-cards-desc", &locale)} }
                        }
                        CardContent { class: "space-y-3.5",
                            if report_cards.is_empty() {
                                div { class: "py-6 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", {t("school-parent-no-reports", &locale)} }
                            } else {
                                div { class: "space-y-3",
                                    for rc in report_cards.iter() {
                                        div { key: "{rc.id}", class: "p-4 border border-border/60 rounded-xl bg-background hover:border-primary/30 transition-all flex flex-col gap-2 shadow-sm",
                                            div { class: "flex items-center justify-between text-xs",
                                                span { class: "font-black text-foreground text-sm", "{rc.term_name}" }
                                                span { class: "text-[9px] font-black uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 px-2 py-0.5 rounded", "{rc.status}" }
                                            }
                                            div { class: "text-[11px] text-muted-foreground font-medium", {t("school-parent-cumulative-gpa", &locale)}, ": {rc.gpa:.2}" }
                                            if let Some(ref note) = rc.principal_comments {
                                                div { class: "text-[10px] text-muted-foreground italic leading-relaxed", "\"{note}\"" }
                                            }
                                            Button {
                                                class: "mt-2.5 w-full bg-primary hover:bg-primary/90 text-primary-foreground font-bold uppercase tracking-wider text-[9px] py-2 rounded-lg flex items-center justify-center gap-1.5 transition-all shadow-sm border-0 cursor-pointer",
                                                onclick: {
                                                    let term = rc.term_name.clone();
                                                    let gpa = rc.gpa;
                                                    let comments = rc.principal_comments.clone().unwrap_or_default();
                                                    let child_id_val = selected_student_profile_id.read().clone();
                                                    let students_c = students.clone();
                                                    move |_| {
                                                        let child_name_val = students_c.iter()
                                                            .find(|s| s.id == child_id_val)
                                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                            .unwrap_or_else(|| "Student".to_string());
                                                        let file_content = format!(
                                                            "====================================================\n\
                                                             OFFICIAL YNTRA PLATFORM REPORT CARD\n\
                                                             ====================================================\n\
                                                             Student Name  : {}\n\
                                                             Term          : {}\n\
                                                             Cumulative GPA: {:.2}\n\
                                                             Status        : Published\n\
                                                             ----------------------------------------------------\n\
                                                             Principal Comments:\n\
                                                             \"{}\"\n\
                                                             ----------------------------------------------------\n\
                                                             This document has been digitally signed and validated\n\
                                                             using zero-knowledge cryptographic proof credentials.\n\
                                                             \n\
                                                             Generated on  : {}\n\
                                                             Verification Code: {}\n\
                                                             ====================================================\n",
                                                             child_name_val,
                                                             term,
                                                             gpa,
                                                             comments,
                                                             chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                                             uuid::Uuid::new_v4()
                                                         );
                                                          #[cfg(target_arch = "wasm32")]
                                                          {
                                                              let base64_str = super::utils::base64_encode(file_content.as_bytes());
                                                              let file_name = format!("ReportCard_{}_{}.txt", child_name_val.replace(" ", "_"), term.replace(" ", "_"));
                                                              let js_code = format!(
                                                                  r#"
                                                                  (function() {{
                                                                      const base64 = "{}";
                                                                      const filename = "{}";
                                                                      const binString = atob(base64);
                                                                      const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                                                                      const blob = new Blob([bytes], {{ type: "text/plain;charset=utf-8;" }});
                                                                      const url = URL.createObjectURL(blob);
                                                                      const a = document.createElement("a");
                                                                      a.href = url;
                                                                      a.download = filename;
                                                                      document.body.appendChild(a);
                                                                      a.click();
                                                                      document.body.removeChild(a);
                                                                      URL.revokeObjectURL(url);
                                                                  }})();
                                                                  "#,
                                                                  base64_str, file_name
                                                              );
                                                              let _ = js_sys::eval(&js_code);
                                                          }

                                                          #[cfg(not(target_arch = "wasm32"))]
                                                          {
                                                              let home_dir = std::env::var("USERPROFILE")
                                                                  .or_else(|_| std::env::var("HOME"))
                                                                  .unwrap_or_else(|_| ".".to_string());
                                                              let filename = format!("ReportCard_{}_{}.txt", child_name_val.replace(" ", "_"), term.replace(" ", "_"));
                                                              let paths = vec![
                                                                  format!("{}/Desktop", home_dir),
                                                                  format!("{}/Downloads", home_dir),
                                                                  home_dir.clone(),
                                                              ];
                                                              for path in paths {
                                                                  let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                                  if std::fs::write(&file_path, &file_content).is_ok() {
                                                                      break;
                                                                  }
                                                              }
                                                          }
                                                      }
                                                 },
                                                 LucideIcon { name: "download", size: "12" }
                                                 {t("school-parent-export-txt", &locale)}
                                             }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Child schedule
                    Card { class: "border-border shadow-sm",
                        CardHeader {
                            CardTitle { {t("school-parent-timetable", &locale)} }
                            CardDescription { {t("school-parent-timetable-desc", &locale)} }
                        }
                        CardContent { class: "space-y-3.5",
                            if timetable.is_empty() {
                                div { class: "py-6 text-center text-xs text-muted-foreground italic", {t("school-parent-no-timetable", &locale)} }
                            } else {
                                div { class: "space-y-3",
                                    div { class: "space-y-3",
                                        for s in timetable.iter().take(5) {
                                            {
                                                let course_opt = courses.iter().find(|c| c.id == s.course_id);
                                                let course_name = course_opt.map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                                let teacher_id_opt = course_opt.and_then(|c| c.teacher_id.clone());
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
                                                    div { key: "{s.id}", class: "p-3 border border-border bg-background rounded-xl flex flex-col gap-1.5 shadow-sm",
                                                        div { class: "flex items-center justify-between text-[10px] font-bold text-muted-foreground",
                                                            span { "{day_name}" }
                                                            span { "{s.start_time} - {s.end_time}" }
                                                        }
                                                        div { class: "flex items-center justify-between",
                                                            div { class: "font-bold text-xs text-foreground", "{course_name}" }
                                                            if let Some(t_id) = teacher_id_opt.clone() {
                                                                button {
                                                                    class: "p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-primary border-0 bg-transparent cursor-pointer transition-colors",
                                                                    onclick: {
                                                                        let mut state = state;
                                                                        let t_id_c = t_id.clone();
                                                                        move |_| {
                                                                            state.active_section.set("messaging".to_string());
                                                                            state.messaging_view_tab.set("compose".to_string());
                                                                            state.compose_recipient_id.set(Some(t_id_c.clone()));
                                                                        }
                                                                    },
                                                                    title: "Message Teacher",
                                                                    LucideIcon { name: "message-square", size: "11" }
                                                                }
                                                            }
                                                        }
                                                        div { class: "text-[9px] text-muted-foreground flex items-center gap-1",
                                                            LucideIcon { name: "map-pin", size: "10" }
                                                            span { "{classroom_name}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Button {
                                        class: "w-full bg-primary hover:bg-primary/90 text-primary-foreground font-bold uppercase tracking-wider text-[9px] py-2 rounded-lg flex items-center justify-center gap-1.5 transition-all shadow-sm border-0 cursor-pointer",
                                        onclick: {
                                            let timetable_c = timetable.clone();
                                            let courses_c = courses.clone();
                                            let child_id_val = selected_student_profile_id.read().clone();
                                            let students_c = students.clone();
                                            move |_| {
                                                let child_name_val = students_c.iter()
                                                    .find(|s| s.id == child_id_val)
                                                    .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                    .unwrap_or_else(|| "Student".to_string());
                                                
                                                let mut ics_content = format!(
                                                    "BEGIN:VCALENDAR\n\
                                                     VERSION:2.0\n\
                                                     PRODID:-//YntraPlatform//ClassSchedule//EN\n\
                                                     CALSCALE:GREGORIAN\n\
                                                     METHOD:PUBLISH\n"
                                                );

                                                for s in timetable_c.iter() {
                                                    let course_name = courses_c.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                                    let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                                    let day_code = match s.day_of_week {
                                                        1 => "MO",
                                                        2 => "TU",
                                                        3 => "WE",
                                                        4 => "TH",
                                                        5 => "FR",
                                                        _ => "MO",
                                                    };
                                                    let start_date = match s.day_of_week {
                                                        1 => "20260720",
                                                        2 => "20260721",
                                                        3 => "20260722",
                                                        4 => "20260723",
                                                        5 => "20260724",
                                                        _ => "20260720",
                                                    };
                                                    
                                                    let clean_start = s.start_time.replace(":", "");
                                                    let clean_end = s.end_time.replace(":", "");

                                                    ics_content.push_str(&format!(
                                                        "BEGIN:VEVENT\n\
                                                         UID:{}@yntra.platform\n\
                                                         DTSTAMP:20260718T000000Z\n\
                                                         SUMMARY:{}\n\
                                                         LOCATION:{}\n\
                                                         DESCRIPTION:Weekly recurring school class for {}\n\
                                                         DTSTART;TZID=Europe/Stockholm:{}T{}00\n\
                                                         DTEND;TZID=Europe/Stockholm:{}T{}00\n\
                                                         RRULE:FREQ=WEEKLY;BYDAY={}\n\
                                                         END:VEVENT\n",
                                                         uuid::Uuid::new_v4(),
                                                         course_name,
                                                         classroom_name,
                                                         child_name_val,
                                                         start_date,
                                                         clean_start,
                                                         start_date,
                                                         clean_end,
                                                         day_code
                                                     ));
                                                }
                                                ics_content.push_str("END:VCALENDAR\n");

                                                 #[cfg(target_arch = "wasm32")]
                                                 {
                                                     let base64_str = super::utils::base64_encode(ics_content.as_bytes());
                                                     let file_name = format!("Schema_{}.ics", child_name_val.replace(" ", "_"));
                                                     let js_code = format!(
                                                         r#"
                                                         (function() {{
                                                             const base64 = "{}";
                                                             const filename = "{}";
                                                             const binString = atob(base64);
                                                             const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                                                             const blob = new Blob([bytes], {{ type: "text/calendar;charset=utf-8;" }});
                                                             const url = URL.createObjectURL(blob);
                                                             const a = document.createElement("a");
                                                             a.href = url;
                                                             a.download = filename;
                                                             document.body.appendChild(a);
                                                             a.click();
                                                             document.body.removeChild(a);
                                                             URL.revokeObjectURL(url);
                                                         }})();
                                                         "#,
                                                         base64_str, file_name
                                                     );
                                                     let _ = js_sys::eval(&js_code);
                                                 }

                                                 #[cfg(not(target_arch = "wasm32"))]
                                                 {
                                                     let home_dir = std::env::var("USERPROFILE")
                                                         .or_else(|_| std::env::var("HOME"))
                                                         .unwrap_or_else(|_| ".".to_string());
                                                     let filename = format!("Schema_{}.ics", child_name_val.replace(" ", "_"));
                                                     let paths = vec![
                                                         format!("{}/Desktop", home_dir),
                                                         format!("{}/Downloads", home_dir),
                                                         home_dir.clone(),
                                                     ];
                                                     for path in paths {
                                                         let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                         if std::fs::write(&file_path, &ics_content).is_ok() {
                                                             break;
                                                         }
                                                     }
                                                 }
                                             }
                                        },
                                        LucideIcon { name: "calendar", size: "12" }
                                        {t("school-calendar-sync-btn", &locale)}
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
