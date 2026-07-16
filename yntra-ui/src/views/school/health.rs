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
    Assignment, Course, SchoolInvoice, HealthRecord, HealthIncident, StudentProfile, TermGrade, ReportCard,
    Submission, TimetableSlot,
};

use super::SchoolViewProps;

#[component]
pub fn HealthClinicView(props: SchoolViewProps) -> Element {
    let mut db_trigger = props.db_trigger;
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

    let incidents = incidents_res.read().clone().unwrap_or_default();
    let students = students_res.read().clone().unwrap_or_default();

    rsx! {
        div { class: "p-6 space-y-6 max-w-6xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "flex items-center justify-between border-b border-border pb-4",
                div {
                    h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                        LucideIcon { name: "activity", class: "h-6 w-6 text-primary" }
                        "Nurse Clinic Logs"
                    }
                    p { class: "text-xs text-muted-foreground m-0 mt-1", "Log school wellness nurse visits, clinic incidents, treatments, and student checks." }
                }
                Button {
                    class: "flex items-center gap-1.5 text-xs h-9 px-4 rounded-xl",
                    onclick: move |_| show_incident_modal.set(true),
                    LucideIcon { name: "plus", class: "h-4 w-4" }
                    "Log Clinic Visit"
                }
            }

            // Ledger card
            Card { class: "border-border shadow-sm",
                CardHeader {
                    CardTitle { "Visit logs" }
                    CardDescription { "Clinic check-in incidents records ledger" }
                }
                CardContent {
                    if incidents.is_empty() {
                        div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                            "No clinic incidents logged today."
                        }
                    } else {
                        div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                            for inc in incidents.iter() {
                                {
                                    let inc_clone = inc.clone();
                                    let student_name = students.iter()
                                        .find(|s| s.id == inc.student_id)
                                        .map(|s| format!("{} {}", s.first_name, s.last_name))
                                        .unwrap_or_else(|| "Unknown Student".to_string());
                                    rsx! {
                                        div {
                                            key: "{inc.id}",
                                            oncontextmenu: move |evt| {
                                                evt.prevent_default();
                                                let coords = evt.client_coordinates();
                                                health_context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                health_context_menu_val.set(Some(inc_clone.clone()));
                                                health_context_menu_open.set(true);
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
                                    move |_| {
                                        let inc = HealthIncident {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            workspace_id: ws.clone(),
                                            student_id: new_inc_student_id.read().clone(),
                                            visit_reason: new_inc_reason.read().clone(),
                                            treatment: new_inc_treatment.read().clone(),
                                            checked_in_at: new_inc_checkin.read().clone(),
                                            checked_out_at: Some("13:00".to_string()),
                                            notes: Some(new_inc_notes.read().clone()),
                                            updated_at: 0,
                                        };
                                        let uid_c = uid.clone();
                                        spawn(async move {
                                            let _ = save_health_incident(uid_c, inc).await;
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

                rsx! {
                    crate::components::ContextMenu {
                        open: *health_context_menu_open.read(),
                        x: health_context_menu_pos.read().0,
                        y: health_context_menu_pos.read().1,
                        onclose: move |_| health_context_menu_open.set(false),

                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: move |_| {
                                let now_str = chrono::Local::now().format("%H:%M").to_string();
                                inc_val.checked_out_at = Some(now_str);
                                let u = uid.clone();
                                let incident = inc_val.clone();
                                let mut d_trig = db_trig;
                                spawn(async move {
                                    let _ = save_health_incident(u, incident).await;
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

