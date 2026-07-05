use dioxus::prelude::*;
use crate::components;
use yntra_core::{Course, AttendanceRecord, WorkspaceUser};

#[derive(Props, Clone)]
pub struct AttendanceTabProps {
    pub active_user: WorkspaceUser,
    pub student_id: String,
    pub courses: Vec<Course>,
    pub attendance_records: Vec<AttendanceRecord>,
    pub db_trigger: Signal<u32>,
    pub workspace_id: String,
    pub locale: String,
}

impl PartialEq for AttendanceTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AttendanceTab(props: AttendanceTabProps) -> Element {
    let active_user = props.active_user;
    let student_id = props.student_id;
    let courses = props.courses;
    let attendance_records = props.attendance_records;
    let mut db_trigger = props.db_trigger;
    let workspace_id = props.workspace_id;
    let _locale = props.locale;

    // Absence Reporting Form state
    let mut absence_date = use_signal(|| "2026-07-04".to_string());
    let mut absence_course_id = use_signal(|| courses.first().map(|c| c.id.clone()).unwrap_or_default());
    let mut absence_notes = use_signal(String::new);
    let mut absence_status = use_signal(|| "excused".to_string());
    let mut form_success = use_signal(|| false);

    rsx! {
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
                                    let stud_id = student_id.clone();
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
    }
}
