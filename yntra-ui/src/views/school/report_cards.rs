use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Dialog, Input, LucideIcon, SuggestionInput};
use crate::locales::t;
use crate::views::school::academics::utils::{decrypt_field, decrypt_opt_field, encrypt_field_with_proof, encrypt_opt_field_with_proof};
use dioxus::prelude::*;
use yntra_core::{
    checkout_book, create_school_invoice, get_assignments, get_library_books,
    get_library_lending_logs, get_school_invoices, get_student_profiles, get_workspace_courses,
    get_users, record_school_payment, return_book, save_assignment, save_attendance_record, save_course,
    link_parent_to_student, get_student_parents, get_student_health_records, save_student_health_record,
    get_health_incidents, save_health_incident, save_student_profile,
    get_course_term_grades, save_term_grade, publish_report_card, get_report_cards,
    get_student_submissions, save_submission, get_timetable_slots, save_timetable_slot,
    get_parent_students,
    Assignment, Course, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot,
};

use super::SchoolViewProps;

#[component]
pub fn ReportCardsView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school_report_cards;
    let trigger_school_report_cards = state.trigger_school_report_cards;
    let trigger_school_directory = state.trigger_school_directory;
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();
    let locale = props.locale.clone();

    // Local states
    let mut selected_student_id = use_signal(|| "".to_string());
    let mut show_publish_modal = use_signal(|| false);
    let mut report_term = use_signal(|| "Fall 2026".to_string());
    let mut report_gpa = use_signal(|| 4.0);
    let mut report_comments = use_signal(String::new);

    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let reports_res = use_resource(move || {
        let _trig = trigger_school_report_cards.read();
        let s_id = selected_student_id.read().clone();
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_report_cards(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    let user_id_clone_p = user_id.clone();
    let ws_id_clone_p = ws_id.clone();
    let parent_students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone_p.clone();
        let ws = ws_id_clone_p.clone();
        async move { get_parent_students(uid.clone(), ws, uid).await.unwrap_or_default() }
    });

    let current_role = state.active_user_role.read().clone();
    let seed = state.get_passkey_seed();
    let students = {
        let raw = if current_role == "parent" || current_role == "role-school-parent" {
            parent_students_res.read().clone().unwrap_or_default()
        } else {
            students_res.read().clone().unwrap_or_default()
        };
        raw.into_iter()
            .map(|mut s| {
                s.first_name = decrypt_field(&seed, &s.first_name);
                s.last_name = decrypt_field(&seed, &s.last_name);
                s.grade_level = decrypt_field(&seed, &s.grade_level);
                s.parent_contact = decrypt_opt_field(&seed, s.parent_contact);
                s
            })
            .collect::<Vec<_>>()
    };
    let report_cards = reports_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut rc| {
            rc.principal_comments = decrypt_opt_field(&seed, rc.principal_comments);
            rc
        })
        .collect::<Vec<_>>();

    use_effect(move || {
        let role = state.active_user_role.read().clone();
        if role == "student" || role == "role-school-student" {
            let uid = state.active_user_id.read().clone();
            let student_list = students_res.read().clone().unwrap_or_default();
            if let Some(profile) = student_list.iter().find(|s| s.user_id.as_ref() == Some(&uid)) {
                if selected_student_id.read().as_str() != profile.id.as_str() {
                    selected_student_id.set(profile.id.clone());
                }
            }
        } else if role == "parent" || role == "role-school-parent" {
            let student_list = parent_students_res.read().clone().unwrap_or_default();
            if !student_list.is_empty() && selected_student_id.read().is_empty() {
                selected_student_id.set(student_list[0].id.clone());
            }
        }
    });

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "award", class: "h-6 w-6 text-primary" }
                        "Student Report Cards"
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", "Review and publish final GPA evaluations and official school report cards." }
                }
            }

            if current_role == "parent" || current_role == "role-school-parent" {
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
                        value: selected_student_id.read().clone(),
                        onchange: move |evt: FormEvent| selected_student_id.set(evt.value()),
                        for s in students.iter() {
                            option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                // Left student roster
                if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                    div { class: "md:col-span-1 space-y-4",
                        h4 { class: "text-sm font-bold text-foreground uppercase tracking-wider mb-2", "Students List" }
                        if students.is_empty() {
                            div { class: "p-8 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No students registered." }
                        } else {
                            for s in students.iter() {
                                {
                                    let is_selected = *selected_student_id.read() == s.id;
                                    let s_id = s.id.clone();
                                    rsx! {
                                        Card {
                                            class: format!(
                                                "cursor-pointer border transition-all hover:bg-muted/30 {}",
                                                if is_selected { "border-primary bg-primary/5" } else { "border-border" }
                                            ),
                                            onclick: move |_| selected_student_id.set(s_id.clone()),
                                            CardContent { class: "p-4",
                                                div { class: "font-bold text-foreground text-sm", "{s.first_name} {s.last_name}" }
                                                div { class: "text-xs text-muted-foreground mt-1", "Grade: {s.grade_level}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right column: Report Cards list
                div {
                    class: format!(
                        "space-y-6 {}",
                        if current_role == "parent" || current_role == "role-school-parent" || current_role == "student" || current_role == "role-school-student" { "md:col-span-3" } else { "md:col-span-2" }
                    ),
                    if selected_student_id.read().is_empty() {
                        div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                            LucideIcon { name: "award", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                            h4 { class: "text-sm font-bold text-foreground m-0", "No Student Selected" }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-xs", "Please select a student from the left panel to review or issue official term report cards." }
                        }
                    } else {
                        {
                            let selected_id = selected_student_id.read().clone();
                            let student = students.iter().find(|s| s.id == selected_id).cloned().unwrap_or_else(|| StudentProfile {
                                id: String::new(),
                                workspace_id: String::new(),
                                user_id: None,
                                first_name: String::new(),
                                last_name: String::new(),
                                grade_level: String::new(),
                                parent_contact: None,
                                updated_at: 0,
                            });
                            rsx! {
                                Card { class: "border-border shadow-sm",
                                    CardHeader {
                                        class: "flex flex-row items-center justify-between space-y-0 pb-3 border-b border-border mb-4",
                                        div {
                                            CardTitle { class: "text-lg font-extrabold", "{student.first_name} {student.last_name}" }
                                            CardDescription { "GPA & Official Evaluations History" }
                                        }
                                        if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                            Button {
                                                class: "flex items-center gap-1.5 text-xs h-8 px-3 rounded-lg",
                                                onclick: move |_| show_publish_modal.set(true),
                                                LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                                                "Publish Report Card"
                                            }
                                        }
                                    }
                                    CardContent { class: "space-y-4",
                                        if report_cards.is_empty() {
                                            div { class: "py-8 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No report cards published yet." }
                                        } else {
                                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                                for rc in report_cards.iter() {
                                                    div { key: "{rc.id}", class: "p-4 flex justify-between items-start text-xs hover:bg-muted/5 transition-colors",
                                                        div { class: "space-y-1",
                                                            div { class: "font-bold text-sm text-foreground", "{rc.term_name}" }
                                                            if let Some(ref comment) = rc.principal_comments {
                                                                div { class: "text-muted-foreground italic pl-2 border-l border-primary/30 mt-1.5", "{comment}" }
                                                            }
                                                            div { class: "text-[10px] bg-green-500/10 text-green-600 border border-green-500/20 px-2 py-0.5 rounded-full w-max mt-2 uppercase font-semibold", "{rc.status}" }
                                                        }
                                                        div { class: "text-right shrink-0",
                                                            div { class: "text-lg font-extrabold text-primary", "GPA: {rc.gpa:.2}" }
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

            // Publish Report Card Modal Dialog
            if *show_publish_modal.read() {
                Dialog {
                    open: *show_publish_modal.read(),
                    title: "Publish Report Card",
                    onclose: move |_| show_publish_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Academic Term" }
                            Input {
                                placeholder: "e.g., Fall 2026",
                                value: report_term.read().clone(),
                                oninput: move |evt: FormEvent| report_term.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Cumulative GPA" }
                            Input {
                                placeholder: "4.00",
                                value: format!("{:.2}", *report_gpa.read()),
                                oninput: move |evt: FormEvent| {
                                    if let Ok(v) = evt.value().parse::<f64>() {
                                        report_gpa.set(v);
                                    }
                                },
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Principal / Teacher Evaluation Comments" }
                            Input {
                                placeholder: "e.g., Exceeded all learning goals with distinction.",
                                value: report_comments.read().clone(),
                                oninput: move |evt: FormEvent| report_comments.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_publish_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                    let active_s = selected_student_id.read().clone();
                                    let uid = user_id.clone();
                                    let ws = ws_id.clone();
                                    let state = state;
                                    move |_| {
                                         let role = state.active_user_role.read().clone();
                                         let u_id = state.active_user_id.read().clone();
                                         let proof = yntra_core::ZkCryptoTrust::new()
                                             .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                             .ok();
                                         let seed_val = state.get_passkey_seed();
                                         let rc = ReportCard {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: ws.clone(),
                                            student_id: active_s.clone(),
                                            term_name: report_term.read().clone(),
                                            gpa: *report_gpa.read(),
                                            principal_comments: Some(encrypt_field_with_proof(&seed_val, &report_comments.read(), &u_id, &role)),
                                            status: "published".to_string(),
                                            updated_at: 0,
                                        };
                                        let uid_c = uid.clone();
                                        spawn(async move {
                                            let _ = publish_report_card(uid_c, rc, proof).await;
                                        });
                                        report_comments.set(String::new());
                                        show_publish_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Publish Report Card"
                            }
                        }
                    }
                }
            }
        }
    }
}

