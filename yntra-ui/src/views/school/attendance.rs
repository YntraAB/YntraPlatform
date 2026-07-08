use dioxus::prelude::*;
use crate::components;
use yntra_core::{Course, StudentProfile, AttendanceRecord};

#[derive(Props, Clone)]
pub struct AttendanceTrackerProps {
    pub active_user_id: String,
    pub courses: Vec<Course>,
    pub students: Vec<StudentProfile>,
    pub attendance_records: Vec<AttendanceRecord>,
    pub attendance_course_id: Signal<Option<String>>,
    pub attendance_date: Signal<String>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for AttendanceTrackerProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AttendanceTracker(props: AttendanceTrackerProps) -> Element {
    let active_user_id = props.active_user_id.clone();
    let courses = props.courses.clone();
    let students = props.students.clone();
    let attendance_records = props.attendance_records.clone();
    let mut attendance_course_id = props.attendance_course_id;
    let mut attendance_date = props.attendance_date;
    let workspace_id = props.workspace_id.clone();
    let mut db_trigger = props.db_trigger;

    let attendance_map: std::collections::HashMap<String, &AttendanceRecord> = attendance_records
        .iter()
        .map(|r| (r.student_id.clone(), r))
        .collect();

    rsx! {
        div { class: "flex flex-col gap-4",
            components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-wrap gap-4 items-center justify-between",
                div { class: "flex gap-4 items-center flex-wrap",
                    div { class: "flex flex-col gap-1",
                        label { class: "text-[10px] font-black uppercase text-muted-foreground", "Select Course" }
                        select {
                            class: "yntra-input py-1 px-3 text-xs bg-background border border-border text-foreground rounded",
                            value: attendance_course_id.read().clone().unwrap_or_default(),
                            onchange: move |e| {
                                let val = e.value();
                                if val.is_empty() {
                                    attendance_course_id.set(None);
                                } else {
                                    attendance_course_id.set(Some(val));
                                }
                            },
                            option { value: "", "Select Course..." }
                            for c in courses.iter() {
                                option { value: "{c.id}", "{c.name} ({c.subject})" }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        label { class: "text-[10px] font-black uppercase text-muted-foreground", "Date" }
                        input {
                            r#type: "date",
                            class: "yntra-input py-1 px-3 text-xs bg-background border border-border text-foreground rounded",
                            value: "{attendance_date}",
                            oninput: move |e| attendance_date.set(e.value())
                        }
                    }
                }
                
                if attendance_course_id.read().is_some() {
                    span { class: "text-xs font-semibold text-emerald-400 bg-emerald-500/10 px-2.5 py-1 rounded border border-emerald-500/15",
                        "Auto-Saving Presence Sheets"
                    }
                }
            }
            
            if let Some(ref course_id) = *attendance_course_id.read() {
                components::Card { class: "p-4 border-border/40 bg-sidebar/20",
                    if students.is_empty() {
                        div { class: "text-center p-8 text-muted-foreground text-sm", "No students enrolled in the school system." }
                    } else {
                        table { class: "yntra-table w-full text-sm",
                            thead {
                                tr {
                                    th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "Student Name" }
                                    th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "Grade Level" }
                                    th { class: "text-center p-3 font-extrabold text-xs uppercase text-muted-foreground", "Attendance Status" }
                                }
                            }
                            tbody {
                                for s in students.iter() {
                                    {
                                        let student_id = s.id.clone();
                                        let record = attendance_map.get(&student_id).copied();
                                        let current_status = record.map(|r| r.status.clone()).unwrap_or_else(|| "present".to_string());
                                        let ws = workspace_id.clone();
                                        let cid = course_id.clone();
                                        let adate = attendance_date.read().clone();
                                        let uid = active_user_id.clone();
                                        
                                        rsx! {
                                            tr { class: "border-b border-border/40 hover:bg-white/[0.01]",
                                                td { class: "p-3 font-bold text-foreground", "{s.first_name} {s.last_name}" }
                                                td { class: "p-3 text-muted-foreground", "{s.grade_level}" }
                                                td { class: "p-3 text-center flex justify-center gap-1.5",
                                                    button {
                                                        class: format!("px-2.5 py-1 text-xs font-bold rounded-lg transition-all {}", if current_status == "present" { "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20" } else { "bg-transparent text-muted-foreground hover:bg-white/[0.02] border border-transparent" }),
                                                        onclick: {
                                                            let ws_clone = ws.clone();
                                                            let cid_clone = cid.clone();
                                                            let adate_clone = adate.clone();
                                                            let sid_clone = student_id.clone();
                                                            let uid_clone = uid.clone();
                                                            move |_| {
                                                                let w = ws_clone.clone();
                                                                let c = cid_clone.clone();
                                                                let d = adate_clone.clone();
                                                                let s = sid_clone.clone();
                                                                let u = uid_clone.clone();
                                                                spawn(async move {
                                                                    let _ = yntra_core::save_attendance_record(u, w, s, c, d, "present".to_string(), None).await;
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                });
                                                            }
                                                        },
                                                        "Present"
                                                    }
                                                    button {
                                                        class: format!("px-2.5 py-1 text-xs font-bold rounded-lg transition-all {}", if current_status == "absent" { "bg-rose-500/10 text-rose-400 border border-rose-500/20" } else { "bg-transparent text-muted-foreground hover:bg-white/[0.02] border border-transparent" }),
                                                        onclick: {
                                                            let ws_clone = ws.clone();
                                                            let cid_clone = cid.clone();
                                                            let adate_clone = adate.clone();
                                                            let sid_clone = student_id.clone();
                                                            let uid_clone = uid.clone();
                                                            move |_| {
                                                                let w = ws_clone.clone();
                                                                let c = cid_clone.clone();
                                                                let d = adate_clone.clone();
                                                                let s = sid_clone.clone();
                                                                let u = uid_clone.clone();
                                                                spawn(async move {
                                                                    let _ = yntra_core::save_attendance_record(u, w, s, c, d, "absent".to_string(), None).await;
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                });
                                                            }
                                                        },
                                                        "Absent"
                                                    }
                                                    button {
                                                        class: format!("px-2.5 py-1 text-xs font-bold rounded-lg transition-all {}", if current_status == "late" { "bg-amber-500/10 text-amber-400 border border-amber-500/20" } else { "bg-transparent text-muted-foreground hover:bg-white/[0.02] border border-transparent" }),
                                                        onclick: {
                                                            let ws_clone = ws.clone();
                                                            let cid_clone = cid.clone();
                                                            let adate_clone = adate.clone();
                                                            let sid_clone = student_id.clone();
                                                            let uid_clone = uid.clone();
                                                            move |_| {
                                                                let w = ws_clone.clone();
                                                                let c = cid_clone.clone();
                                                                let d = adate_clone.clone();
                                                                let s = sid_clone.clone();
                                                                let u = uid_clone.clone();
                                                                spawn(async move {
                                                                    let _ = yntra_core::save_attendance_record(u, w, s, c, d, "late".to_string(), None).await;
                                                                    let current = *db_trigger.read();
                                                                    db_trigger.set(current + 1);
                                                                });
                                                            }
                                                        },
                                                        "Late"
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
            if attendance_course_id.read().is_none() {
                div { class: "text-center p-12 text-muted-foreground border border-dashed border-border/40 rounded-xl",
                    components::LucideIcon { name: "scheduling", size: "40", class: "opacity-20 mb-2 mx-auto" }
                    p { class: "text-sm font-semibold m-0", "Select a course above to view presence checklist roster." }
                }
            }
        }
    }
}
