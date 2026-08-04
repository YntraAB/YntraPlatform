#![allow(unused_imports)]
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
pub fn StudentDirectoryView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school_directory;
    let trigger_school_directory = state.trigger_school_directory;
    let trigger_school_health = state.trigger_school_health;
    let trigger_users = state.trigger_users;
    let _locale = props.locale.clone();
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    let current_role = state.active_user_role.read().clone();
    let can_edit = current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student";

    let locale_lower = props.locale.to_lowercase();
    let grade_suggestions = if locale_lower.starts_with("sv") || locale_lower.starts_with("se") {
        vec![
            "Klass 1A".to_string(), "Klass 1B".to_string(),
            "Klass 2A".to_string(), "Klass 2B".to_string(),
            "Klass 3A".to_string(), "Klass 3B".to_string(),
            "Klass 4A".to_string(), "Klass 4B".to_string(),
            "Klass 5A".to_string(), "Klass 5B".to_string(),
            "Klass 6A".to_string(), "Klass 6B".to_string(),
            "Klass 7A".to_string(), "Klass 7B".to_string(),
            "Klass 8A".to_string(), "Klass 8B".to_string(),
            "Klass 9A".to_string(), "Klass 9B".to_string(),
        ]
    } else {
        vec![
            "1st Grade".to_string(), "2nd Grade".to_string(),
            "3rd Grade".to_string(), "4th Grade".to_string(),
            "5th Grade".to_string(), "6th Grade".to_string(),
            "7th Grade".to_string(), "8th Grade".to_string(),
            "9th Grade".to_string(), "10th Grade".to_string(),
            "11th Grade".to_string(), "12th Grade".to_string(),
            "10A".to_string(), "10B".to_string(),
            "9A".to_string(), "9B".to_string(),
        ]
    };

    // Local states
    let mut selected_student_id = use_signal(|| "".to_string());
    let mut show_student_modal = use_signal(|| false);
    let mut enroll_first_name = use_signal(String::new);
    let mut enroll_last_name = use_signal(String::new);
    let mut enroll_grade = use_signal(|| "10A".to_string());
    let mut enroll_contact = use_signal(String::new);
    let mut enroll_student_user_id = use_signal(String::new);
    let mut editing_student_id = use_signal(|| Option::<String>::None);

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_student = use_signal(|| Option::<StudentProfile>::None);

    let mut show_link_modal = use_signal(|| false);
    let mut select_parent_user_id = use_signal(String::new);

    let mut show_vaccine_modal = use_signal(|| false);
    let mut vaccine_name = use_signal(String::new);
    let vaccine_status = use_signal(|| "administered".to_string());
    let mut vaccine_date = use_signal(|| chrono::Local::now().format("%Y-%m-%d").to_string());

    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let is_parent = current_role == "parent" || current_role == "role-school-parent";
    let students_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move {
            if is_parent {
                get_parent_students(uid.clone(), ws, uid).await.unwrap_or_default()
            } else {
                get_student_profiles(uid, ws).await.unwrap_or_default()
            }
        }
    });

    use_effect(move || {
        let role = state.active_user_role.read().clone();
        if role == "parent" || role == "role-school-parent" {
            let student_list = students_res.read().clone().unwrap_or_default();
            if !student_list.is_empty() && selected_student_id.read().is_empty() {
                selected_student_id.set(student_list[0].id.clone());
            }
        }
    });

    let active_student = selected_student_id.read().clone();
    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let parents_res = use_resource(move || {
        let _trig = trigger_school_directory.read();
        let s_id = active_student.clone();
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_parents(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    let active_student_h = selected_student_id.read().clone();
    let user_id_clone3 = user_id.clone();
    let ws_id_clone3 = ws_id.clone();
    let health_res = use_resource(move || {
        let _trig = trigger_school_health.read();
        let s_id = active_student_h.clone();
        let uid = user_id_clone3.clone();
        let ws = ws_id_clone3.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_health_records(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    let user_id_clone4 = user_id.clone();
    let users_res = use_resource(move || {
        let _trig = trigger_users.read();
        let uid = user_id_clone4.clone();
        async move { get_users(uid).await.unwrap_or_default() }
    });

    let seed = state.get_passkey_seed();
    let students = students_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut s| {
            s.first_name = decrypt_field(&seed, &s.first_name);
            s.last_name = decrypt_field(&seed, &s.last_name);
            s.grade_level = decrypt_field(&seed, &s.grade_level);
            s.parent_contact = decrypt_opt_field(&seed, s.parent_contact);
            s
        })
        .collect::<Vec<_>>();
    let parents = parents_res.read().clone().unwrap_or_default();
    let health_records = health_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut r| {
            r.vaccine_name = decrypt_field(&seed, &r.vaccine_name);
            r.status = decrypt_field(&seed, &r.status);
            r.administered_at = decrypt_opt_field(&seed, r.administered_at);
            r
        })
        .collect::<Vec<_>>();
    let all_users = users_res.read().clone().unwrap_or_default();

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "users", class: "h-6 w-6 text-primary" }
                        "Student & Parent Directory"
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", "Enroll students, maintain family parent links, and track academic directory profiles." }
                }
                if can_edit {
                    Button {
                        class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                        onclick: move |_| {
                            enroll_first_name.set(String::new());
                            enroll_last_name.set(String::new());
                            enroll_grade.set("10A".to_string());
                            enroll_contact.set(String::new());
                            enroll_student_user_id.set(String::new());
                            editing_student_id.set(None);
                            show_student_modal.set(true);
                        },
                        LucideIcon { name: "plus", class: "h-4 w-4" }
                        "Enroll Student"
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                // Left roster
                div { class: "md:col-span-1 space-y-4",
                    h4 { class: "text-sm font-bold text-foreground uppercase tracking-wider mb-2", "Student Roster" }
                    if students.is_empty() {
                        div { class: "p-8 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No students registered." }
                    } else {
                        for s in students.iter() {
                            {
                                let s_c = s.clone();
                                let onclick_s = s.id.clone();
                                let oncontext_s = s.clone();
                                let is_selected = *selected_student_id.read() == s.id;
                                rsx! {
                                    div {
                                        oncontextmenu: move |evt| {
                                            if can_edit {
                                                evt.prevent_default();
                                                let coords = evt.client_coordinates();
                                                context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                context_menu_student.set(Some(oncontext_s.clone()));
                                                context_menu_open.set(true);
                                            }
                                        },
                                        Card {
                                            class: format!(
                                                "cursor-pointer border transition-all hover:bg-muted/30 {}",
                                                if is_selected { "border-primary bg-primary/5" } else { "border-border" }
                                            ),
                                            onclick: move |_| selected_student_id.set(onclick_s.clone()),
                                            CardContent { class: "p-4",
                                                div { class: "font-bold text-foreground text-sm", "{s_c.first_name} {s_c.last_name}" }
                                                div { class: "text-xs text-muted-foreground mt-1", "Grade: {s_c.grade_level}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Profile View
                div { class: "md:col-span-2 space-y-6",
                    if selected_student_id.read().is_empty() {
                        div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                            LucideIcon { name: "user", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                            h4 { class: "text-sm font-bold text-foreground m-0", "No Student Selected" }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-xs", "Please select a student from the left roster panel to manage parent associations and immunization health records." }
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
                                div { class: "space-y-6",
                                    // Card 1: Parent linking
                                    Card { class: "border-border shadow-sm",
                                        CardHeader {
                                            class: "flex flex-row items-center justify-between space-y-0 pb-3",
                                            div {
                                                CardTitle { class: "text-lg font-extrabold", "{student.first_name} {student.last_name}" }
                                                CardDescription { "Family & Contact Information" }
                                            }
                                            if can_edit {
                                                Button {
                                                    class: "flex items-center gap-1.5 text-xs h-8 px-3 rounded-lg",
                                                    onclick: move |_| show_link_modal.set(true),
                                                    LucideIcon { name: "link", class: "h-3.5 w-3.5" }
                                                    "Link Parent"
                                                }
                                            }
                                        }
                                        CardContent { class: "space-y-3",
                                            div { class: "grid grid-cols-2 gap-2 text-xs",
                                                div { span { class: "font-semibold text-muted-foreground", "Grade Level: " } "{student.grade_level}" }
                                                div { span { class: "font-semibold text-muted-foreground", "Direct Contact: " } "{student.parent_contact.clone().unwrap_or_default()}" }
                                            }
                                            h5 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground mt-4 mb-2", "Associated Parent Users" }
                                            if parents.is_empty() {
                                                div { class: "text-xs text-muted-foreground py-2 italic", "No parent accounts linked." }
                                            } else {
                                                div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                                    for p in parents.iter() {
                                                        div { key: "{p.id}", class: "p-3 flex justify-between items-center text-xs",
                                                            div {
                                                                div { class: "font-semibold text-foreground", "{p.full_name.clone().unwrap_or_default()}" }
                                                                div { class: "text-muted-foreground mt-0.5", "{p.email}" }
                                                            }
                                                            span { class: "text-[10px] bg-primary/10 text-primary px-2 py-0.5 rounded-full uppercase font-bold", "Parent Account" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Card 2: Immunization records
                                    Card { class: "border-border shadow-sm",
                                        CardHeader {
                                            class: "flex flex-row items-center justify-between space-y-0 pb-3",
                                            div {
                                                CardTitle { class: "text-base font-bold", "Immunization Records" }
                                                CardDescription { "Student health registry check list" }
                                            }
                                            if can_edit {
                                                Button {
                                                    class: "flex items-center gap-1.5 text-xs h-8 px-3 rounded-lg border border-primary text-primary hover:bg-primary/5",
                                                    onclick: move |_| show_vaccine_modal.set(true),
                                                    LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                                                    "Record Vaccine"
                                                }
                                            }
                                        }
                                        CardContent {
                                            if health_records.is_empty() {
                                                div { class: "text-xs text-muted-foreground py-4 text-center border border-dashed border-border rounded-xl", "No vaccine records registered." }
                                            } else {
                                                div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                                    for hr in health_records.iter() {
                                                        div { key: "{hr.id}", class: "p-3.5 flex justify-between items-center text-xs",
                                                            div {
                                                                div { class: "font-bold text-foreground", "{hr.vaccine_name}" }
                                                                div { class: "text-muted-foreground mt-0.5", "Administered: {hr.administered_at.clone().unwrap_or_default()}" }
                                                            }
                                                            span { class: "text-[10px] bg-green-500/10 text-green-600 border border-green-500/20 px-2.5 py-0.5 rounded-full font-semibold", "{hr.status}" }
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

            // Enroll Student Modal Dialog
            if *show_student_modal.read() {
                Dialog {
                    open: *show_student_modal.read(),
                    title: if editing_student_id.read().is_some() { "Edit Student Profile" } else { "Enroll New Student" },
                    onclose: move |_| show_student_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "First Name" }
                            Input {
                                placeholder: "Jane",
                                value: enroll_first_name.read().clone(),
                                oninput: move |evt: FormEvent| enroll_first_name.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Last Name" }
                            Input {
                                placeholder: "Smith",
                                value: enroll_last_name.read().clone(),
                                oninput: move |evt: FormEvent| enroll_last_name.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Grade level" }
                            SuggestionInput {
                                placeholder: "e.g., 10A",
                                value: enroll_grade.read().clone(),
                                suggestions: grade_suggestions.clone(),
                                onchange: move |val| enroll_grade.set(val),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Contact Email / Phone" }
                            Input {
                                placeholder: "parent@smith.com",
                                value: enroll_contact.read().clone(),
                                oninput: move |evt: FormEvent| enroll_contact.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Link Student User Account" }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                value: enroll_student_user_id.read().clone(),
                                onchange: move |evt: FormEvent| enroll_student_user_id.set(evt.value()),
                                option { value: "", "Select user account (optional)..." }
                                for u in all_users.iter().filter(|u| u.role == "student" || u.role == "role-school-student") {
                                    option { value: "{u.id}", "{u.full_name.clone().unwrap_or_else(|| u.email.clone())} ({u.email})" }
                                }
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_student_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                onclick: {
                                     let uid = user_id.clone();
                                     let ws = ws_id.clone();
                                     let state = state;
                                     move |_| {
                                          let role = state.active_user_role.read().clone();
                                          let u_id = state.active_user_id.read().clone();
                                          let proof = yntra_core::ZkCryptoTrust::new()
                                              .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                              .ok();
                                          let student_user_id = enroll_student_user_id.read().clone();
                                          let user_id_val = if student_user_id.is_empty() { None } else { Some(student_user_id) };
                                          let seed_val = state.get_passkey_seed();
                                          let sp = StudentProfile {
                                             id: editing_student_id.read().clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                             workspace_id: ws.clone(),
                                             user_id: user_id_val,
                                             first_name: encrypt_field_with_proof(&seed_val, &enroll_first_name.read(), &u_id, &role),
                                             last_name: encrypt_field_with_proof(&seed_val, &enroll_last_name.read(), &u_id, &role),
                                             grade_level: encrypt_field_with_proof(&seed_val, &enroll_grade.read(), &u_id, &role),
                                             parent_contact: Some(encrypt_field_with_proof(&seed_val, &enroll_contact.read(), &u_id, &role)),
                                             updated_at: 0,
                                         };
                                         let uid_c = uid.clone();
                                         spawn(async move {
                                             let _ = save_student_profile(uid_c, sp, proof).await;
                                         });
                                        enroll_first_name.set(String::new());
                                        enroll_last_name.set(String::new());
                                        enroll_student_user_id.set(String::new());
                                        show_student_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                     }
                                },
                                if editing_student_id.read().is_some() { "Save Details" } else { "Enroll Student" }
                            }
                        }
                    }
                }
            }

            // Link Parent Modal Dialog
            if *show_link_modal.read() {
                Dialog {
                    open: *show_link_modal.read(),
                    title: "Link Parent Account",
                    onclose: move |_| show_link_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Choose Parent User" }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none",
                                value: select_parent_user_id.read().clone(),
                                onchange: move |evt: FormEvent| select_parent_user_id.set(evt.value()),
                                option { value: "", "Select parent..." }
                                for u in all_users.iter().filter(|u| u.role == "parent" || u.role == "role-school-parent" || u.role == "user") {
                                    option { value: "{u.id}", "{u.full_name.clone().unwrap_or_default()} ({u.email})" }
                                }
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_link_modal.set(false),
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
                                          let parent_id = select_parent_user_id.read().clone();
                                         if !parent_id.is_empty() {
                                             let uid_c = uid.clone();
                                             let ws_c = ws.clone();
                                             let s_id = active_s.clone();
                                             spawn(async move {
                                                 let _ = link_parent_to_student(uid_c, ws_c, s_id, parent_id, proof).await;
                                             });
                                            show_link_modal.set(false);
                                            let current = *db_trigger.read();
                                            db_trigger.set(current + 1);
                                        }
                                    }
                                },
                                "Link Parent"
                            }
                        }
                    }
                }
            }

            // Record Vaccine Modal Dialog
            if *show_vaccine_modal.read() {
                Dialog {
                    open: *show_vaccine_modal.read(),
                    title: "Record Immunization Vaccine",
                    onclose: move |_| show_vaccine_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Vaccine Name" }
                            Input {
                                placeholder: "e.g., Measles, Mumps, Rubella (MMR)",
                                value: vaccine_name.read().clone(),
                                oninput: move |evt: FormEvent| vaccine_name.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Date Administered" }
                            Input {
                                value: vaccine_date.read().clone(),
                                oninput: move |evt: FormEvent| vaccine_date.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_vaccine_modal.set(false),
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
                                         let hr = HealthRecord {
                                             id: uuid::Uuid::new_v4().to_string(),
                                             workspace_id: ws.clone(),
                                             student_id: active_s.clone(),
                                             vaccine_name: encrypt_field_with_proof(&seed_val, &vaccine_name.read(), &u_id, &role),
                                             status: encrypt_field_with_proof(&seed_val, &vaccine_status.read(), &u_id, &role),
                                             administered_at: Some(encrypt_field_with_proof(&seed_val, &vaccine_date.read(), &u_id, &role)),
                                             updated_at: 0,
                                         };
                                         let uid_c = uid.clone();
                                         spawn(async move {
                                             let _ = save_student_health_record(uid_c, hr, proof).await;
                                         });
                                        vaccine_name.set(String::new());
                                        show_vaccine_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Record Immunization"
                            }
                        }
                    }
                }
            }

            // Context Menu Overlay
            if let Some(student) = context_menu_student.read().clone() {
                {
                    let _std = student.clone();
                    let std_edit = student.clone();
                    let std_link = student.clone();
                    let std_vac = student.clone();

                    rsx! {
                        crate::components::ContextMenu {
                            open: *context_menu_open.read() && can_edit,
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    enroll_first_name.set(std_edit.first_name.clone());
                                    enroll_last_name.set(std_edit.last_name.clone());
                                    enroll_grade.set(std_edit.grade_level.clone());
                                    enroll_contact.set(std_edit.parent_contact.clone().unwrap_or_default());
                                    enroll_student_user_id.set(std_edit.user_id.clone().unwrap_or_default());
                                    editing_student_id.set(Some(std_edit.id.clone()));
                                    show_student_modal.set(true);
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "user-cog", size: "14" }
                                "Edit Profile"
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    selected_student_id.set(std_link.id.clone());
                                    show_link_modal.set(true);
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "link", size: "14" }
                                "Link Parent"
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    selected_student_id.set(std_vac.id.clone());
                                    show_vaccine_modal.set(true);
                                    context_menu_open.set(false);
                                },
                                crate::components::LucideIcon { name: "shield-alert", size: "14" }
                                "Add Vaccine"
                            }
                        }
                    }
                }
            }
        }
    }
}

