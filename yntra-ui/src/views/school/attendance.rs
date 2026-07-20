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
pub fn AttendanceView(props: SchoolViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school;
    let user_id = props.active_user_id.clone();
    let ws_id = props.workspace_id.clone();

    let mut selected_course_id = use_signal(|| "".to_string());
    let mut selected_date = use_signal(|| "2026-07-16".to_string());

    // Context Menu signals
    let mut student_context_menu_open = use_signal(|| false);
    let mut student_context_menu_pos = use_signal(|| (0, 0));
    let mut student_context_menu_val = use_signal(|| Option::<StudentProfile>::None);

    // Note Modal state
    let mut show_note_modal = use_signal(|| false);
    let mut editing_note_student = use_signal(|| Option::<StudentProfile>::None);
    let mut note_text = use_signal(String::new);

    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let ws_id_clone = ws_id.clone();
    let courses_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone.clone();
        let ws = ws_id_clone.clone();
        async move { get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

    let user_id_clone2 = user_id.clone();
    let ws_id_clone2 = ws_id.clone();
    let students_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone2.clone();
        let ws = ws_id_clone2.clone();
        async move { get_student_profiles(uid, ws).await.unwrap_or_default() }
    });

    let active_course = selected_course_id.read().clone();
    let active_date = selected_date.read().clone();
    let user_id_clone3 = user_id.clone();
    let ws_id_clone3 = ws_id.clone();
    let attendance_res = use_resource(move || {
        let _ = db_trig_val;
        let c_id = active_course.clone();
        let dt = active_date.clone();
        let uid = user_id_clone3.clone();
        let ws = ws_id_clone3.clone();
        async move {
            if c_id.is_empty() {
                Vec::new()
            } else {
                yntra_core::get_attendance_records(uid, ws, c_id, dt).await.unwrap_or_default()
            }
        }
    });

    let courses = courses_res.read().clone().unwrap_or_default();
    let students = students_res.read().clone().unwrap_or_default();
    let attendance = attendance_res.read().clone().unwrap_or_default();

    rsx! {
        div { class: "p-6 space-y-6 max-w-5xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "border-b border-border pb-4 mb-6",
                h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                    LucideIcon { name: "user-check", class: "h-6 w-6 text-primary" }
                    "Attendance Tracking"
                }
                p { class: "text-xs text-muted-foreground m-0 mt-1", "Mark present, absent, or tardy records for students enrolled in courses." }
            }

            // Controls Roster card
            Card { class: "border border-border p-4 bg-sidebar",
                div { class: "flex flex-col sm:flex-row gap-4 items-center",
                    div { class: "grid gap-1 flex-1 w-full",
                        span { class: "text-xs font-bold text-muted-foreground", "Select Course" }
                        select {
                            class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                            value: selected_course_id.read().clone(),
                            onchange: move |evt: FormEvent| selected_course_id.set(evt.value()),
                            option { value: "", "Choose a course..." }
                            for c in courses.iter() {
                                option { value: "{c.id}", "{c.name}" }
                            }
                        }
                    }
                    div { class: "grid gap-1 w-full sm:w-48",
                        span { class: "text-xs font-bold text-muted-foreground", "Select Date" }
                        Input {
                            value: selected_date.read().clone(),
                            oninput: move |evt: FormEvent| selected_date.set(evt.value()),
                        }
                    }
                }
            }

            // Student list grid
            if selected_course_id.read().is_empty() {
                div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                    LucideIcon { name: "user-check", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                    h4 { class: "text-sm font-bold text-foreground m-0", "No Course Selected" }
                    p { class: "text-xs text-muted-foreground mt-1 max-w-xs", "Please select an active course above to populate the student attendance sheet." }
                }
            } else {
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { "Student Attendance Sheet" }
                        CardDescription { "Date: {selected_date}" }
                    }
                    CardContent {
                        if students.is_empty() {
                            div { class: "py-8 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl",
                                "No students registered in the workspace registry directory. Please add students first."
                            }
                        } else {
                            div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                                for s in students.iter() {
                                    {
                                        let student_id = s.id.clone();
                                        let student_name = format!("{} {}", s.first_name, s.last_name);
                                        let current_status = attendance.iter().find(|a| a.student_id == student_id).map(|a| a.status.clone()).unwrap_or_else(|| "Present".to_string());
                                        let c_id = selected_course_id.read().clone();
                                        let dt = selected_date.read().clone();
                                        let uid = user_id.clone();
                                        let ws = ws_id.clone();
                                        let s_clone = s.clone();
                                        let record_c = attendance.iter().find(|a| a.student_id == student_id).cloned();
                                        rsx! {
                                            div {
                                                key: "{student_id}",
                                                oncontextmenu: move |evt| {
                                                    evt.prevent_default();
                                                    let coords = evt.client_coordinates();
                                                    student_context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                    student_context_menu_val.set(Some(s_clone.clone()));
                                                    student_context_menu_open.set(true);
                                                },
                                                class: "p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3",
                                                div {
                                                    div { class: "font-semibold text-sm text-foreground", "{student_name}" }
                                                    div { class: "text-[10px] text-muted-foreground mt-0.5", "Grade: {s.grade_level}" }
                                                    if let Some(record) = record_c && let Some(ref note) = record.notes {
                                                        div { class: "text-[10px] text-amber-500/80 font-medium italic mt-1 flex items-center gap-1",
                                                            LucideIcon { name: "file-text", size: "10" }
                                                            "{note}"
                                                        }
                                                    }
                                                }
                                                // Status Radio buttons
                                                div { class: "flex gap-2 items-center",
                                                    for st in &["Present", "Absent", "Late", "Excused"] {
                                                        {
                                                            let is_selected = current_status.as_str() == *st;
                                                            let st_val = st.to_string();
                                                            let s_id = student_id.clone();
                                                            let c_id = c_id.clone();
                                                            let dt = dt.clone();
                                                            let uid = uid.clone();
                                                            let ws = ws.clone();
                                                            rsx! {
                                                                Button {
                                                                    class: format!(
                                                                        "text-xs px-3 py-1.5 h-8 rounded-lg font-medium transition-colors border {}",
                                                                        if is_selected {
                                                                            match *st {
                                                                                "Present" => "bg-green-500/10 text-green-600 border-green-500/20",
                                                                                "Absent" => "bg-red-500/10 text-red-600 border-red-500/20",
                                                                                "Late" => "bg-amber-500/10 text-amber-600 border-amber-500/20",
                                                                                _ => "bg-primary/10 text-primary border-primary/20"
                                                                            }
                                                                        } else {
                                                                            "bg-muted/30 text-muted-foreground border-border hover:bg-muted/60"
                                                                        }
                                                                    ),
                                                                    onclick: {
                                                                        let ws = ws.clone();
                                                                        let s_id = s_id.clone();
                                                                        let c_id = c_id.clone();
                                                                        let dt = dt.clone();
                                                                        let st_val = st_val.clone();
                                                                        let uid = uid.clone();
                                                                        let state = state;
                                                                        move |_| {
                                                                            let role = state.active_user_role.read().clone();
                                                                            let u_id = state.active_user_id.read().clone();
                                                                            let proof = yntra_core::ZkCryptoTrust::new()
                                                                                .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                                                .ok();
                                                                            let r = yntra_core::AttendanceRecord {
                                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                                workspace_id: ws.clone(),
                                                                                student_id: s_id.clone(),
                                                                                course_id: c_id.clone(),
                                                                                date: dt.clone(),
                                                                                status: st_val.clone(),
                                                                                notes: None,
                                                                                updated_at: 0,
                                                                            };
                                                                            let uid_c = uid.clone();
                                                                            spawn(async move {
                                                                                let _ = save_attendance_record(uid_c, r, proof).await;
                                                                            });
                                                                            let current = *db_trigger.read();
                                                                            db_trigger.set(current + 1);
                                                                        }
                                                                    },
                                                                    "{st}"
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
        }

        // Student Context Menu Overlay
        if let Some(student) = student_context_menu_val.read().clone() {
            {
                let s_val = student.clone();
                let s_id = student.id.clone();

                rsx! {
                    crate::components::ContextMenu {
                        open: *student_context_menu_open.read(),
                        x: student_context_menu_pos.read().0,
                        y: student_context_menu_pos.read().1,
                        onclose: move |_| student_context_menu_open.set(false),

                        button {
                            class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                            onclick: {
                                let attendance = attendance.clone();
                                let s_val = s_val.clone();
                                let s_id = s_id.clone();
                                move |_| {
                                    let record = attendance.iter().find(|a| a.student_id == s_id);
                                    let current_notes = record.and_then(|r| r.notes.clone()).unwrap_or_default();
                                    note_text.set(current_notes);
                                    editing_note_student.set(Some(s_val.clone()));
                                    show_note_modal.set(true);
                                    student_context_menu_open.set(false);
                                }
                            },
                            crate::components::LucideIcon { name: "file-text", size: "14" }
                            "Add/Edit Note"
                        }
                    }
                }
            }
        }

        // Add/Edit Note Dialog
        if *show_note_modal.read() && let Some(student) = editing_note_student.read().clone() {
            {
                let s_id = student.id.clone();
                let student_name = format!("{} {}", student.first_name, student.last_name);
                let c_id = selected_course_id.read().clone();
                let dt = selected_date.read().clone();
                let uid = user_id.clone();
                let ws = ws_id.clone();
                let record = attendance.iter().find(|a| a.student_id == s_id);
                let record_id = record.map(|a| a.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let current_status = record.map(|a| a.status.clone()).unwrap_or_else(|| "Present".to_string());
                let mut db_trig = db_trigger;

                rsx! {
                    Dialog {
                        open: *show_note_modal.read(),
                        title: "Attendance Note: {student_name}",
                        onclose: move |_| show_note_modal.set(false),
                        div { class: "flex flex-col gap-4 text-sm w-full py-2",
                            div { class: "grid gap-1.5",
                                span { class: "font-bold text-foreground text-xs", "Attendance Note" }
                                Input {
                                    placeholder: "e.g., Doctor's appointment",
                                    value: note_text.read().clone(),
                                    oninput: move |evt: FormEvent| note_text.set(evt.value()),
                                }
                            }
                            div { class: "flex justify-end gap-3 border-t border-border pt-4 mt-2",
                                Button {
                                    class: "px-4 py-2 text-xs rounded-xl bg-muted text-foreground",
                                    onclick: move |_| show_note_modal.set(false),
                                    "Cancel"
                                }
                                Button {
                                    class: "px-4 py-2 text-xs rounded-xl bg-primary text-primary-foreground",
                                    onclick: {
                                        let record_id = record_id.clone();
                                        let ws = ws.clone();
                                        let s_id = s_id.clone();
                                        let c_id = c_id.clone();
                                        let dt = dt.clone();
                                        let current_status = current_status.clone();
                                        let uid = uid.clone();
                                        let mut db_trig = db_trig.clone();
                                        let state = state;
                                        move |_| {
                                            let role = state.active_user_role.read().clone();
                                            let u_id = state.active_user_id.read().clone();
                                            let proof = yntra_core::ZkCryptoTrust::new()
                                                .generate_role_proof(state.get_passkey_seed(), u_id, role)
                                                .ok();
                                            let note_val = note_text.read().trim().to_string();
                                            let note_opt = if note_val.is_empty() { None } else { Some(note_val) };

                                            let r = yntra_core::AttendanceRecord {
                                                id: record_id.clone(),
                                                workspace_id: ws.clone(),
                                                student_id: s_id.clone(),
                                                course_id: c_id.clone(),
                                                date: dt.clone(),
                                                status: current_status.clone(),
                                                notes: note_opt,
                                                updated_at: 0,
                                            };
                                            let uid_c = uid.clone();
                                            spawn(async move {
                                                let _ = save_attendance_record(uid_c, r, proof).await;
                                            });
                                            show_note_modal.set(false);
                                            let current = *db_trig.read();
                                            db_trig.set(current + 1);
                                        }
                                    },
                                    "Save Note"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

