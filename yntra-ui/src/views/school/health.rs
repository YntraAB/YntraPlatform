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
    get_parent_students,
    Assignment, Course, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot,
};

use super::SchoolViewProps;

#[component]
pub fn HealthClinicView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school;
    let locale = props.locale.clone();
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    // Local states
    let mut show_incident_modal = use_signal(|| false);
    let mut new_inc_student_id = use_signal(String::new);
    let mut new_inc_reason = use_signal(String::new);
    let mut new_inc_treatment = use_signal(String::new);
    let mut new_inc_notes = use_signal(String::new);
    let new_inc_checkin = use_signal(|| "12:30".to_string());

    // Context Menu signals
    let mut health_context_menu_open = use_signal(|| false);
    let mut health_context_menu_pos = use_signal(|| (0, 0));
    let mut health_context_menu_val = use_signal(|| Option::<HealthIncident>::None);

    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let incidents_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_health_incidents(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let students_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone_p = user_id.clone();
    let ws_id_clone_p = ws_id.clone();
    let parent_students_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone_p.clone();
        let ws = ws_id_clone_p.clone();
        async move { get_parent_students(uid.clone(), ws, uid).await.unwrap_or_default() }
    });

    let user_id_clone_hr = user_id.clone();
    let ws_id_clone_hr = ws_id.clone();
    let state_c = state.clone();
    let parent_students_res_c = parent_students_res.clone();
    let students_res_c = students_res.clone();
    let health_records_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone_hr.clone();
        let ws = ws_id_clone_hr.clone();
        let state = state_c.clone();
        let parent_res = parent_students_res_c.clone();
        let std_res = students_res_c.clone();
        async move {
            let role = state.active_user_role.read().clone();
            let st_list = if role == "parent" || role == "role-school-parent" {
                parent_res.read().clone().unwrap_or_default()
            } else {
                std_res.read().clone().unwrap_or_default()
            };
            let mut list = Vec::new();
            for s in st_list {
                if let Ok(mut hrs) = yntra_core::get_student_health_records(uid.clone(), ws.clone(), s.id.clone()).await {
                    list.append(&mut hrs);
                }
            }
            list
        }
    });

    let mut selected_student_id = use_signal(|| "".to_string());

    use_effect(move || {
        let role = state.active_user_role.read().clone();
        if role == "parent" || role == "role-school-parent" {
            let student_list = parent_students_res.read().clone().unwrap_or_default();
            if !student_list.is_empty() && selected_student_id.read().is_empty() {
                selected_student_id.set(student_list[0].id.clone());
            }
        }
    });

    let incidents_raw = incidents_res.read().clone().unwrap_or_default();
    let current_role = state.active_user_role.read().clone();
    let students = if current_role == "parent" || current_role == "role-school-parent" {
        parent_students_res.read().clone().unwrap_or_default()
    } else {
        students_res.read().clone().unwrap_or_default()
    };

    let incidents = if current_role == "parent" || current_role == "role-school-parent" {
        let s_id = selected_student_id.read().clone();
        incidents_raw.into_iter().filter(|inc| inc.student_id == s_id).collect::<Vec<_>>()
    } else if current_role == "student" || current_role == "role-school-student" {
        let student_ids: std::collections::HashSet<String> = students.iter()
            .filter(|s| s.user_id.as_ref() == Some(&user_id))
            .map(|s| s.id.clone())
            .collect();
        incidents_raw.into_iter().filter(|inc| student_ids.contains(&inc.student_id)).collect::<Vec<_>>()
    } else {
        incidents_raw
    };

    let health_records_all = health_records_res.read().clone().unwrap_or_default();
    let health_records = if current_role == "parent" || current_role == "role-school-parent" {
        let s_id = selected_student_id.read().clone();
        health_records_all.into_iter().filter(|hr| hr.student_id == s_id).collect::<Vec<_>>()
    } else {
        health_records_all
    };

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "activity", class: "h-6 w-6 text-primary" }
                        {t("school-parent-health-title", &locale)}
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", {t("school-parent-health-desc", &locale)} }
                }
                if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                    Button {
                        class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                        onclick: move |_| show_incident_modal.set(true),
                        LucideIcon { name: "plus", class: "h-4 w-4" }
                        "Log Clinic Visit"
                    }
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

            // Split layout grid
            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                // Left Side: Visit Logs
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-parent-health-visit-logs", &locale)} }
                        CardDescription { {t("school-parent-health-visit-desc", &locale)} }
                    }
                    CardContent {
                        if incidents.is_empty() {
                            div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                                {t("school-parent-health-no-incidents-today", &locale)}
                            }
                        } else {
                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                for inc in incidents.iter() {
                                    {
                                        let current_role = current_role.clone();
                                        let inc_clone = inc.clone();
                                        let student_name = students.iter()
                                            .find(|s| s.id == inc.student_id)
                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                            .unwrap_or_else(|| "Unknown Student".to_string());
                                        rsx! {
                                            div {
                                                key: "{inc.id}",
                                                oncontextmenu: move |evt| {
                                                    if current_role != "parent" && current_role != "role-school-parent" && current_role != "student" && current_role != "role-school-student" {
                                                        evt.prevent_default();
                                                        let coords = evt.client_coordinates();
                                                        health_context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                        health_context_menu_val.set(Some(inc_clone.clone()));
                                                        health_context_menu_open.set(true);
                                                    }
                                                },
                                                class: "p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-4 text-xs hover:bg-muted/10 transition-colors",
                                                div { class: "space-y-1",
                                                    div { class: "font-semibold text-sm text-foreground", "{student_name}" }
                                                    div { class: "text-xs text-muted-foreground", span { class: "font-bold text-foreground", "Reason: " } "{inc.visit_reason}" }
                                                    div { class: "text-xs text-muted-foreground", span { class: "font-bold text-foreground", "Treatment: " } "{inc.treatment}" }
                                                    if let Some(ref nt) = inc.notes {
                                                        div { class: "text-muted-foreground italic mt-1.5 pl-2 border-l border-primary/30", "{nt}" }
                                                    }
                                                }
                                                div { class: "text-right shrink-0",
                                                    div { class: "font-medium text-foreground", "In: {inc.checked_in_at}" }
                                                    if let Some(ref out) = inc.checked_out_at {
                                                        div { class: "text-muted-foreground mt-0.5", "Out: {out}" }
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

                // Right Side: Immunization Ledger & Parental Consent
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-health-vaccine-consent-title", &locale)} }
                        CardDescription { {t("school-health-vaccine-consent-desc", &locale)} }
                    }
                    CardContent {
                        if health_records.is_empty() {
                            div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                                {t("school-parent-health-no-vaccine-sched", &locale)}
                            }
                        } else {
                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                for hr in health_records.iter() {
                                    {
                                        let hr_c = hr.clone();
                                        let student_name = students.iter()
                                            .find(|s| s.id == hr.student_id)
                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                            .unwrap_or_else(|| "Unknown Student".to_string());
                                        rsx! {
                                            div { key: "{hr.id}", class: "p-4 flex justify-between items-center text-xs hover:bg-muted/10 transition-colors",
                                                div { class: "space-y-1.5",
                                                    div { class: "font-semibold text-sm text-foreground", "{student_name}" }
                                                    div { class: "text-xs text-muted-foreground", span { class: "font-bold text-foreground", "Vaccine: " } "{hr.vaccine_name}" }
                                                    div { class: "text-[10px] text-muted-foreground", "Scheduled Date: {hr.administered_at.clone().unwrap_or_else(|| \"TBD\".to_string())}" }
                                                }
                                                if hr.status == "consented" {
                                                    span { class: "text-[9px] font-black uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 px-2 py-0.5 rounded", {t("school-health-consent-signed", &locale)} }
                                                } else if hr.status == "administered" {
                                                    span { class: "text-[9px] font-black uppercase bg-blue-500/10 text-blue-600 border border-blue-500/20 px-2 py-0.5 rounded", {t("school-health-administered", &locale)} }
                                                } else {
                                                    div { class: "flex flex-col gap-1.5 items-end",
                                                        span { class: "text-[9px] font-black uppercase bg-amber-500/10 text-amber-600 border border-amber-500/20 px-2 py-0.5 rounded", {t("school-health-awaiting-consent", &locale)} }
                                                        if current_role == "parent" || current_role == "role-school-parent" {
                                                            Button {
                                                                class: "text-[9px] h-6 px-2.5 rounded bg-primary text-primary-foreground hover:bg-primary/90 font-bold uppercase tracking-wider border-0 cursor-pointer",
                                                                onclick: {
                                                                    let mut hr_update = hr_c.clone();
                                                                    let uid_c = user_id.clone();
                                                                    let mut db_trigger = db_trigger.clone();
                                                                    let role_c = current_role.clone();
                                                                    let state = state;
                                                                    move |_| {
                                                                        hr_update.status = "consented".to_string();
                                                                        let proof = state.get_passkey_seed();
                                                                        let hr_save = hr_update.clone();
                                                                        let u = uid_c.clone();
                                                                        let r = role_c.clone();
                                                                        let mut db_t = db_trigger.clone();
                                                                        spawn(async move {
                                                                            let proof_val = yntra_core::ZkCryptoTrust::new()
                                                                                .generate_role_proof(proof, u.clone(), r)
                                                                                .ok();
                                                                            let _ = yntra_core::save_student_health_record(u, hr_save, proof_val).await;
                                                                            let cur = *db_t.read();
                                                                            db_t.set(cur + 1);
                                                                        });
                                                                    }
                                                                },
                                                                {t("school-health-sign-consent-btn", &locale)}
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

            // Log incident Modal Dialog
            if *show_incident_modal.read() {
                Dialog {
                    open: *show_incident_modal.read(),
                    title: "Log Nurse Clinic Visit",
                    onclose: move |_| show_incident_modal.set(false),
                    div { class: "flex flex-col gap-4 text-sm w-full py-2",
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Select Student" }
                            select {
                                class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none",
                                value: new_inc_student_id.read().clone(),
                                onchange: move |evt: FormEvent| new_inc_student_id.set(evt.value()),
                                option { value: "", "Select student..." }
                                for s in students.iter() {
                                    option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                                }
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Visit Reason / Symptoms" }
                            Input {
                                placeholder: "e.g., Slight headache and low fever",
                                value: new_inc_reason.read().clone(),
                                oninput: move |evt: FormEvent| new_inc_reason.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Treatment Administered" }
                            Input {
                                placeholder: "e.g., Ice pack and rest",
                                value: new_inc_treatment.read().clone(),
                                oninput: move |evt: FormEvent| new_inc_treatment.set(evt.value()),
                            }
                        }
                        div { class: "grid gap-1.5",
                            span { class: "font-bold text-foreground text-xs", "Nurse Notes" }
                            Input {
                                placeholder: "e.g., Parent notified, student returned to class",
                                value: new_inc_notes.read().clone(),
                                oninput: move |evt: FormEvent| new_inc_notes.set(evt.value()),
                            }
                        }
                        div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                            Button {
                                class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                onclick: move |_| show_incident_modal.set(false),
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
                                             .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                             .ok();
                                         let inc = HealthIncident {
                                             id: uuid::Uuid::new_v4().to_string(),
                                             workspace_id: ws.clone(),
                                             student_id: new_inc_student_id.read().clone(),
                                             visit_reason: new_inc_reason.read().clone(),
                                             treatment: new_inc_treatment.read().clone(),
                                             checked_in_at: new_inc_checkin.read().clone(),
                                             checked_out_at: None,
                                             notes: Some(new_inc_notes.read().clone()),
                                             updated_at: 0,
                                         };
                                         let uid_c = uid.clone();
                                         spawn(async move {
                                             let _ = save_health_incident(uid_c, inc, proof).await;
                                         });
                                        new_inc_reason.set(String::new());
                                        new_inc_treatment.set(String::new());
                                        show_incident_modal.set(false);
                                        let current = *db_trigger.read();
                                        db_trigger.set(current + 1);
                                    }
                                },
                                "Log Visit"
                            }
                        }
                    }
                }
            }
        }

        // Incident Context Menu Overlay
        if let Some(inc) = health_context_menu_val.read().clone() {
            {
                let mut inc_val = inc.clone();
                let uid = user_id.clone();
                let db_trig = db_trigger;
                let state = state;

                rsx! {
                    crate::components::ContextMenu {
                        open: *health_context_menu_open.read(),
                        x: health_context_menu_pos.read().0,
                        y: health_context_menu_pos.read().1,
                        onclose: move |_| health_context_menu_open.set(false),

                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: move |_| {
                                let role = state.active_user_role.read().clone();
                                let u_id = state.active_user_id.read().clone();
                                 let proof = yntra_core::ZkCryptoTrust::new()
                                     .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                     .ok();
                                let now_str = chrono::Local::now().format("%H:%M").to_string();
                                inc_val.checked_out_at = Some(now_str);
                                let u = uid.clone();
                                let incident = inc_val.clone();
                                let mut d_trig = db_trig;
                                spawn(async move {
                                    let _ = save_health_incident(u, incident, proof).await;
                                });
                                health_context_menu_open.set(false);
                                let current = *d_trig.read();
                                d_trig.set(current + 1);
                            },
                            crate::components::LucideIcon { name: "log-out", size: "14" }
                            "Check Out Student"
                        }
                    }
                }
            }
        }
    }
}

