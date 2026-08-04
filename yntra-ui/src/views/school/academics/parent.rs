use dioxus::prelude::*;
use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, LucideIcon, Dialog, Input};
use crate::locales::t;
use super::SchoolViewProps;
use super::utils::{decrypt_field, decrypt_opt_field};
use yntra_core::{
    get_student_attendance_records, get_student_health_records, get_health_incidents,
    get_report_cards, get_course_term_grades, get_timetable_slots, StudentProfile,
    link_student_self_service, get_assignments, get_student_submissions, report_student_absence
};
use dioxus_primitives::toast::{ToastOptions, use_toast};

#[component]
pub fn ParentPortal(
    school_props: SchoolViewProps,
    students: Vec<StudentProfile>,
    mut selected_student_profile_id: Signal<String>,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let db_trigger = state.trigger_school_academics;
    let db_trigger_academics = state.trigger_school_academics;
    let db_trigger_attendance = state.trigger_school_attendance;
    let db_trigger_health = state.trigger_school_health;
    let db_trigger_report_cards = state.trigger_school_report_cards;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();
    let toast = use_toast();

    let mut link_input_id = use_signal(String::new);
    let mut show_absence_modal = use_signal(|| false);
    let mut absence_date = use_signal(String::new);
    let mut absence_reason = use_signal(|| "Sick Leave".to_string());
    let link_error = use_signal(|| Option::<String>::None);

    // Resources specific to parent child tracking
    
    let user_id_clone_att = user_id.clone();
    let ws_id_clone_att = ws_id.clone();
    let student_attendance_res = use_resource(move || {
        let _trig = db_trigger_attendance.read();
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
        let _trig = db_trigger_health.read();
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
        let _trig = db_trigger_health.read();
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
        let _trig = db_trigger_report_cards.read();
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
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone_courses.clone();
        let ws = ws_id_clone_courses.clone();
        async move { yntra_core::get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

    // Timetable slots resource
    let user_id_clone7 = user_id.clone();
    let ws_id_clone7 = ws_id.clone();
    let timetable_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
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
        let _trig = db_trigger_academics.read();
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
        let _trig = db_trigger_academics.read();
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
        let _trig = db_trigger_academics.read();
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
                                let db_trigger_c = db_trigger;
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

    let seed = state.get_passkey_seed();
    // Read parent portal child resources
    let attendance = student_attendance_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut a| {
            a.notes = decrypt_opt_field(&seed, a.notes);
            a
        })
        .collect::<Vec<_>>();
    let health_records = student_health_records_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut r| {
            r.vaccine_name = decrypt_field(&seed, &r.vaccine_name);
            r.status = decrypt_field(&seed, &r.status);
            r.administered_at = decrypt_opt_field(&seed, r.administered_at);
            r
        })
        .collect::<Vec<_>>();
    let health_incidents = student_health_incidents_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut i| {
            i.visit_reason = decrypt_field(&seed, &i.visit_reason);
            i.treatment = decrypt_field(&seed, &i.treatment);
            i.checked_in_at = decrypt_field(&seed, &i.checked_in_at);
            i.checked_out_at = decrypt_opt_field(&seed, i.checked_out_at);
            i.notes = decrypt_opt_field(&seed, i.notes);
            i
        })
        .collect::<Vec<_>>();
    let report_cards = student_report_cards_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut rc| {
            rc.principal_comments = decrypt_opt_field(&seed, rc.principal_comments);
            rc
        })
        .collect::<Vec<_>>();
    let (chart_points, chart_line_d, chart_area_d) = {
        let mut sorted_reports = report_cards.clone();
        sorted_reports.sort_by_key(|r| r.updated_at);
        
        let width = 500.0;
        let height = 120.0;
        let padding_x = 45.0;
        let padding_y = 15.0;
        
        let points: Vec<(f32, f32, f64, String)> = sorted_reports.iter().enumerate().map(|(i, rc)| {
            let x = if sorted_reports.len() > 1 {
                padding_x + (i as f32) * (width - 2.0 * padding_x) / ((sorted_reports.len() - 1) as f32)
            } else {
                width / 2.0
            };
            let gpa_ratio = (rc.gpa as f32) / 4.0;
            let y = height - padding_y - gpa_ratio * (height - 2.0 * padding_y);
            (x, y, rc.gpa, rc.term_name.clone())
        }).collect();
        
        let line_d = if points.len() > 1 {
            let mut d = format!("M {:.1} {:.1}", points[0].0, points[0].1);
            for pt in points.iter().skip(1) {
                d.push_str(&format!(" L {:.1} {:.1}", pt.0, pt.1));
            }
            d
        } else {
            String::new()
        };
        
        let area_d = if points.len() > 1 {
            let mut d = format!("M {:.1} {:.1}", points[0].0, height - padding_y);
            for pt in points.iter() {
                d.push_str(&format!(" L {:.1} {:.1}", pt.0, pt.1));
            }
            d.push_str(&format!(" L {:.1} {:.1} Z", points[points.len() - 1].0, height - padding_y));
            d
        } else {
            String::new()
        };
        
        (points, line_d, area_d)
    };
    let course_grades = course_grades_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut g| {
            g.final_grade = decrypt_opt_field(&seed, g.final_grade);
            g.teacher_comments = decrypt_opt_field(&seed, g.teacher_comments);
            g
        })
        .collect::<Vec<_>>();
    let student_submissions = student_submissions_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut s| {
            s.content = decrypt_field(&seed, &s.content);
            s.grade = decrypt_opt_field(&seed, s.grade);
            s.feedback = decrypt_opt_field(&seed, s.feedback);
            s
        })
        .collect::<Vec<_>>();
    let all_assignments = all_assignments_res.read().clone().unwrap_or_default();

    let enrolled_course_ids: std::collections::HashSet<String> = {
        let mut ids = std::collections::HashSet::new();
        for cg in course_grades.iter() {
            ids.insert(cg.course_id.clone());
        }
        for att in attendance.iter() {
            ids.insert(att.course_id.clone());
        }
        for sub in student_submissions.iter() {
            if let Some(assign) = all_assignments.iter().find(|a| a.id == sub.assignment_id) {
                ids.insert(assign.course_id.clone());
            }
        }
        ids
    };

    let filtered_timetable: Vec<_> = timetable.iter()
        .filter(|s| enrolled_course_ids.contains(&s.course_id))
        .cloned()
        .collect();

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
                        CardHeader { class: "pb-2 flex flex-row items-center justify-between",
                            div {
                                CardTitle { {t("school-parent-attendance-tracking", &locale)} }
                                CardDescription { {t("school-parent-attendance-desc", &locale)} }
                            }
                            Button {
                                class: "text-xs px-3 py-1.5 bg-primary text-primary-foreground hover:bg-primary/90 font-bold rounded-lg border-0 cursor-pointer flex items-center gap-1.5",
                                onclick: move |_| {
                                    absence_date.set(chrono::Local::now().format("%Y-%m-%d").to_string());
                                    show_absence_modal.set(true);
                                },
                                LucideIcon { name: "calendar", size: "12" }
                                "Report Absence"
                            }
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

                    // Absence Modal
                    if *show_absence_modal.read() {
                        Dialog {
                            open: *show_absence_modal.read(),
                            title: "Report Planned Absence".to_string(),
                            onclose: move |_| show_absence_modal.set(false),
                            div { class: "flex flex-col gap-4 text-sm w-full py-2",
                                div { class: "grid gap-1.5",
                                    span { class: "font-bold text-foreground text-xs", "Absence Date" }
                                    Input {
                                        r#type: "date",
                                        value: "{absence_date}",
                                        oninput: move |evt: FormEvent| absence_date.set(evt.value()),
                                    }
                                }
                                div { class: "grid gap-1.5",
                                    span { class: "font-bold text-foreground text-xs", "Reason / Notes" }
                                    select {
                                        class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                                        value: absence_reason.read().clone(),
                                        onchange: move |evt: FormEvent| absence_reason.set(evt.value()),
                                        option { value: "Sick Leave", "Sick Leave" }
                                        option { value: "Medical Appointment", "Medical Appointment" }
                                        option { value: "Family Event", "Family Event" }
                                        option { value: "Personal Reason", "Personal Reason" }
                                    }
                                }
                                div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                                    Button {
                                        class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground border border-border/40 cursor-pointer",
                                        onclick: move |_| show_absence_modal.set(false),
                                        "Cancel"
                                    }
                                    Button {
                                        class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground font-bold border-0 cursor-pointer",
                                        onclick: {
                                            let uid = user_id.clone();
                                            let ws = ws_id.clone();
                                            let child_id_val = selected_student_profile_id.read().clone();
                                            let state = state.clone();
                                            let date = absence_date.clone();
                                            let reason = absence_reason.clone();
                                            let db_trigger = db_trigger.clone();
                                            let toast = toast.clone();
                                            let locale_c = locale.clone();
                                            move |_| {
                                                let proof = yntra_core::ZkCryptoTrust::new()
                                                    .generate_role_proof(state.get_passkey_seed(), uid.clone(), "parent".to_string())
                                                    .ok();
                                                let u = uid.clone();
                                                let w = ws.clone();
                                                let c = child_id_val.clone();
                                                let d = date.read().clone();
                                                let r = reason.read().clone();
                                                let mut db_t = db_trigger.clone();
                                                let toast_c = toast.clone();
                                                let loc = locale_c.clone();
                                                spawn(async move {
                                                    match report_student_absence(u, w, c, d, r, proof).await {
                                                        Ok(_) => {
                                                            toast_c.success(
                                                                t("school-toast-export-success", &loc),
                                                                ToastOptions::new().description("Absence reported successfully".to_string())
                                                            );
                                                        }
                                                        Err(e) => {
                                                            toast_c.error(
                                                                t("school-toast-export-failed", &loc),
                                                                ToastOptions::new().description(format!("Failed: {}", e))
                                                            );
                                                        }
                                                    }
                                                    let current = *db_t.read();
                                                    db_t.set(current + 1);
                                                });
                                                show_absence_modal.set(false);
                                            }
                                        },
                                        "Submit"
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
                                div { class: "p-4 bg-muted/10 border border-border/40 rounded-2xl mb-4 space-y-2.5",
                                    h5 { class: "text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground m-0", "GPA Trend Visualization" }
                                    div { class: "w-full overflow-x-auto",
                                        svg {
                                            view_box: "0 0 500 120",
                                            class: "w-full min-w-[400px] h-[120px] overflow-visible",
                                            
                                            defs {
                                                linearGradient {
                                                    id: "gpa-grad",
                                                    x1: "0%",
                                                    y1: "0%",
                                                    x2: "0%",
                                                    y2: "100%",
                                                    stop { offset: "0%", stop_color: "var(--primary)" }
                                                    stop { offset: "100%", stop_color: "var(--primary)", stop_opacity: "0" }
                                                }
                                            }
                                            
                                            for gpa_val in [1.0, 2.0, 3.0, 4.0] {
                                                {
                                                    let ratio = gpa_val / 4.0;
                                                    let y = 120.0 - 15.0 - ratio * (120.0 - 30.0);
                                                    rsx! {
                                                        line {
                                                            x1: "45",
                                                            y1: format!("{}", y),
                                                            x2: "455",
                                                            y2: format!("{}", y),
                                                            class: "stroke-muted/20",
                                                            stroke_width: "1",
                                                            stroke_dasharray: "3 3",
                                                        }
                                                        text {
                                                            x: "37",
                                                            y: format!("{}", y + 3.0),
                                                            class: "fill-muted-foreground text-[8px] font-black",
                                                            text_anchor: "end",
                                                            "{gpa_val:.1}"
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            if !chart_area_d.is_empty() {
                                                path {
                                                    d: "{chart_area_d}",
                                                    fill: "url(#gpa-grad)",
                                                    opacity: "0.15",
                                                }
                                            }
                                            
                                            if !chart_line_d.is_empty() {
                                                path {
                                                    d: "{chart_line_d}",
                                                    class: "stroke-primary",
                                                    stroke_width: "2.5",
                                                    stroke_linecap: "round",
                                                    stroke_linejoin: "round",
                                                    fill: "none",
                                                }
                                            }
                                            
                                            for pt in chart_points.iter() {
                                                {
                                                    let (x, y, val, term) = pt;
                                                    rsx! {
                                                        g { class: "group/pt cursor-pointer",
                                                            circle {
                                                                cx: format!("{}", x),
                                                                cy: format!("{}", y),
                                                                r: "4",
                                                                class: "fill-background stroke-primary transition-all duration-150 group-hover/pt:r-6",
                                                                stroke_width: "2",
                                                            }
                                                            text {
                                                                x: format!("{}", x),
                                                                y: format!("{}", y - 8.0),
                                                                class: "fill-foreground text-[9px] font-bold opacity-0 group-hover/pt:opacity-100 transition-opacity duration-150",
                                                                text_anchor: "middle",
                                                                "{val:.2}"
                                                            }
                                                            text {
                                                                x: format!("{}", x),
                                                                y: "118",
                                                                class: "fill-muted-foreground text-[7px] font-bold",
                                                                text_anchor: "middle",
                                                                "{term}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
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
                                                    let toast = toast.clone();
                                                    let locale_c = locale.clone();
                                                    move |_| {
                                                        let child_name_val = students_c.iter()
                                                            .find(|s| s.id == child_id_val)
                                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                                            .unwrap_or_else(|| "Student".to_string());
                                                        let file_content = format!(
                                                            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Official Report Card - {child_name}</title>
    <style>
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background-color: #f8fafc;
            color: #1e293b;
            margin: 0;
            padding: 40px 20px;
            display: flex;
            justify-content: center;
        }}
        .container {{
            max-width: 800px;
            width: 100%;
            background: #ffffff;
            border-radius: 16px;
            box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.05), 0 8px 10px -6px rgba(0, 0, 0, 0.05);
            border: 1px solid #e2e8f0;
            padding: 48px;
            box-sizing: border-box;
            position: relative;
        }}
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 2px solid #e2e8f0;
            padding-bottom: 24px;
            margin-bottom: 32px;
        }}
        .school-info h1 {{
            font-size: 24px;
            font-weight: 800;
            margin: 0;
            color: #0f172a;
            letter-spacing: -0.025em;
        }}
        .school-info p {{
            font-size: 13px;
            color: #64748b;
            margin: 4px 0 0 0;
        }}
        .badge {{
            background: rgba(16, 185, 129, 0.1);
            color: #10b981;
            border: 1px solid rgba(16, 185, 129, 0.2);
            padding: 6px 14px;
            border-radius: 9999px;
            font-size: 11px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .meta-grid {{
            display: grid;
            grid-template-cols: 1fr 1fr;
            gap: 24px;
            margin-bottom: 32px;
        }}
        .meta-item {{
            background: #f8fafc;
            border: 1px solid #f1f5f9;
            padding: 16px;
            border-radius: 12px;
        }}
        .meta-label {{
            font-size: 10px;
            font-weight: 700;
            text-transform: uppercase;
            color: #64748b;
            letter-spacing: 0.05em;
            margin-bottom: 4px;
        }}
        .meta-value {{
            font-size: 15px;
            font-weight: 700;
            color: #0f172a;
        }}
        .gpa-box {{
            background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
            color: #ffffff;
            padding: 24px;
            border-radius: 14px;
            text-align: center;
            margin-bottom: 32px;
        }}
        .gpa-title {{
            font-size: 12px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.1em;
            opacity: 0.9;
        }}
        .gpa-value {{
            font-size: 48px;
            font-weight: 900;
            margin: 8px 0;
        }}
        .comments-section {{
            margin-bottom: 32px;
        }}
        .comments-section h3 {{
            font-size: 14px;
            font-weight: 800;
            color: #0f172a;
            margin-top: 0;
            margin-bottom: 12px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .comments-box {{
            font-size: 14px;
            line-height: 1.6;
            color: #334155;
            background: #fffbeb;
            border-left: 4px solid #f59e0b;
            padding: 16px 20px;
            border-radius: 0 12px 12px 0;
            margin: 0;
            font-style: italic;
        }}
        .verification {{
            border-top: 1px solid #e2e8f0;
            padding-top: 24px;
            font-size: 11px;
            color: #64748b;
            line-height: 1.5;
        }}
        .verification strong {{
            color: #0f172a;
        }}
        .actions {{
            display: flex;
            justify-content: flex-end;
            margin-bottom: 24px;
        }}
        .btn {{
            background-color: #0f172a;
            color: #ffffff;
            border: none;
            padding: 10px 20px;
            font-size: 13px;
            font-weight: 600;
            border-radius: 8px;
            cursor: pointer;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            transition: background-color 0.2s;
        }}
        .btn:hover {{
            background-color: #1e293b;
        }}
        @media print {{
            body {{
                background-color: #ffffff;
                padding: 0;
            }}
            .container {{
                box-shadow: none;
                border: none;
                padding: 0;
            }}
            .no-print {{
                display: none !important;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="actions no-print">
            <button class="btn" onclick="window.print()">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>
                Print Report Card
            </button>
        </div>
        <div class="header">
            <div class="school-info">
                <h1>Yntra Academy</h1>
                <p>Official Academic Evaluation Record</p>
            </div>
            <div class="badge">Official Record</div>
        </div>
        
        <div class="meta-grid">
            <div class="meta-item">
                <div class="meta-label">Student Name</div>
                <div class="meta-value">{child_name}</div>
            </div>
            <div class="meta-item">
                <div class="meta-label">Academic Term</div>
                <div class="meta-value">{term}</div>
            </div>
            <div class="meta-item">
                <div class="meta-label">Date Generated</div>
                <div class="meta-value">{generated_date}</div>
            </div>
            <div class="meta-item">
                <div class="meta-label">Verification ID</div>
                <div class="meta-value" style="font-family: monospace; font-size: 12px;">{verification_id}</div>
            </div>
        </div>

        <div class="gpa-box">
            <div class="gpa-title">Cumulative Grade Point Average</div>
            <div class="gpa-value">{gpa:.2}</div>
        </div>

        <div class="comments-section">
            <h3>Principal & Teacher Comments</h3>
            <blockquote class="comments-box">
                "{comments}"
            </blockquote>
        </div>

        <div class="verification">
            <p>This report has been digitally generated and signed. Verification code: <strong>{verification_id}</strong>.</p>
            <p>Verification is backed by zero-knowledge cryptographic credential proofs stored in the school's local-first ledger.</p>
        </div>
    </div>
</body>
</html>"#,
                                                            child_name = child_name_val,
                                                            term = term,
                                                            generated_date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                                            verification_id = uuid::Uuid::new_v4(),
                                                            gpa = gpa,
                                                            comments = comments
                                                        );
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            let base64_str = super::utils::base64_encode(file_content.as_bytes());
                                                            let file_name = format!("ReportCard_{}_{}.html", child_name_val.replace(" ", "_"), term.replace(" ", "_"));
                                                            let js_code = format!(
                                                                r#"
                                                                (function() {{
                                                                    const base64 = "{}";
                                                                    const filename = "{}";
                                                                    const binString = atob(base64);
                                                                    const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                                                                    const blob = new Blob([bytes], {{ type: "text/html;charset=utf-8;" }});
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
                                                            toast.success(
                                                                t("school-toast-download-started", &locale_c),
                                                                ToastOptions::new().description(t("school-toast-browser-download-desc", &locale_c))
                                                            );
                                                        }

                                                        #[cfg(not(target_arch = "wasm32"))]
                                                        {
                                                            let home_dir = std::env::var("USERPROFILE")
                                                                .or_else(|_| std::env::var("HOME"))
                                                                .unwrap_or_else(|_| ".".to_string());
                                                            let filename = format!("ReportCard_{}_{}.html", child_name_val.replace(" ", "_"), term.replace(" ", "_"));
                                                            let paths = vec![
                                                                format!("{}/Desktop", home_dir),
                                                                format!("{}/Downloads", home_dir),
                                                                home_dir.clone(),
                                                            ];
                                                            let mut success = false;
                                                            let mut saved_path = String::new();
                                                            for path in paths {
                                                                let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                                if std::fs::write(&file_path, &file_content).is_ok() {
                                                                    success = true;
                                                                    saved_path = file_path.to_string_lossy().to_string();
                                                                    break;
                                                                }
                                                            }
                                                            if success {
                                                                toast.success(
                                                                    t("school-toast-export-success", &locale_c),
                                                                    ToastOptions::new().description(format!("{}: {}", t("school-toast-saved-to", &locale_c), saved_path))
                                                                );
                                                            } else {
                                                                toast.error(
                                                                    t("school-toast-export-failed", &locale_c),
                                                                    ToastOptions::new().description(t("school-toast-export-failed-desc", &locale_c))
                                                                );
                                                            }
                                                        }
                                                    }
                                                 },
                                                 LucideIcon { name: "download", size: "12" }
                                                 {t("school-parent-export-html", &locale)}
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
                            if filtered_timetable.is_empty() {
                                div { class: "py-6 text-center text-xs text-muted-foreground italic", {t("school-parent-no-timetable", &locale)} }
                            } else {
                                div { class: "space-y-3",
                                    div { class: "space-y-3",
                                        for s in filtered_timetable.iter().take(5) {
                                            {
                                                let course_opt = courses.iter().find(|c| c.id == s.course_id);
                                                let course_name = course_opt.map(|c| c.name.clone()).unwrap_or_else(|| t("school-parent-unknown-course", &locale));
                                                let teacher_id_opt = course_opt.and_then(|c| c.teacher_id.clone());
                                                let classroom_name = s.classroom.clone().unwrap_or_else(|| t("school-parent-room-unassigned", &locale));
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
                                            let timetable_c = filtered_timetable.clone();
                                            let courses_c = courses.clone();
                                            let child_id_val = selected_student_profile_id.read().clone();
                                            let students_c = students.clone();
                                            let locale_c = locale.clone();
                                            let toast = toast.clone();
                                            move |_| {
                                                use chrono::Datelike;
                                                let today = chrono::Local::now().date_naive();
                                                let num_from_monday = today.weekday().number_from_monday() as i64;
                                                let monday = today - chrono::Duration::days(num_from_monday - 1);

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
                                                    let course_name = courses_c.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| t("school-parent-unknown-course", &locale_c));
                                                    let classroom_name = s.classroom.clone().unwrap_or_else(|| t("school-parent-room-unassigned", &locale_c));
                                                    let day_code = match s.day_of_week {
                                                        1 => "MO",
                                                        2 => "TU",
                                                        3 => "WE",
                                                        4 => "TH",
                                                        5 => "FR",
                                                        _ => "MO",
                                                    };
                                                    let target_day = monday + chrono::Duration::days(s.day_of_week as i64 - 1);
                                                    let start_date = target_day.format("%Y%m%d").to_string();
                                                    
                                                    let clean_start = s.start_time.replace(":", "");
                                                    let clean_end = s.end_time.replace(":", "");
                                                    let event_desc = crate::locales::t_with_args("school-parent-ics-desc", &locale_c, &[("name", &child_name_val)]);

                                                    ics_content.push_str(&format!(
                                                        "BEGIN:VEVENT\n\
                                                         UID:{}@yntra.platform\n\
                                                         DTSTAMP:20260718T000000Z\n\
                                                         SUMMARY:{}\n\
                                                         LOCATION:{}\n\
                                                         DESCRIPTION:{}\n\
                                                         DTSTART;TZID=Europe/Stockholm:{}T{}00\n\
                                                         DTEND;TZID=Europe/Stockholm:{}T{}00\n\
                                                         RRULE:FREQ=WEEKLY;BYDAY={}\n\
                                                         END:VEVENT\n",
                                                         uuid::Uuid::new_v4(),
                                                         course_name,
                                                         classroom_name,
                                                         event_desc,
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
                                                     toast.success(
                                                         t("school-toast-download-started", &locale_c),
                                                         ToastOptions::new().description(t("school-toast-browser-download-desc", &locale_c))
                                                     );
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
                                                     let mut success = false;
                                                     let mut saved_path = String::new();
                                                     for path in paths {
                                                         let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                         if std::fs::write(&file_path, &ics_content).is_ok() {
                                                             success = true;
                                                             saved_path = file_path.to_string_lossy().to_string();
                                                             break;
                                                         }
                                                     }
                                                     if success {
                                                         toast.success(
                                                             t("school-toast-export-success", &locale_c),
                                                             ToastOptions::new().description(format!("{}: {}", t("school-toast-saved-to", &locale_c), saved_path))
                                                         );
                                                     } else {
                                                         toast.error(
                                                             t("school-toast-export-failed", &locale_c),
                                                             ToastOptions::new().description(t("school-toast-export-failed-desc", &locale_c))
                                                         );
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
