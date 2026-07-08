use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, Course, TimetableSlot};
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct TimetableSlotsViewProps {
    pub active_user: WorkspaceUser,
    pub courses: Vec<Course>,
    pub slots: Vec<TimetableSlot>,
    pub selected_course_id: Signal<String>,
    pub selected_day: Signal<i32>,
    pub timetable_start_time: Signal<String>,
    pub timetable_end_time: Signal<String>,
    pub classroom_input: Signal<String>,
    pub sync_success: Signal<bool>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn TimetableSlotsView(props: TimetableSlotsViewProps) -> Element {
    let active_user = props.active_user;
    let courses = props.courses;
    let slots = props.slots;
    let mut selected_course_id = props.selected_course_id;
    let mut selected_day = props.selected_day;
    let mut timetable_start_time = props.timetable_start_time;
    let mut timetable_end_time = props.timetable_end_time;
    let mut classroom_input = props.classroom_input;
    let mut sync_success = props.sync_success;
    let db_trigger = props.db_trigger;

    rsx! {
        div { class: "flex-1 flex flex-col gap-6 p-6 overflow-y-auto scrollbar-dark",
            div { class: "flex flex-col gap-1 border-b border-border pb-4",
                h3 { class: "text-lg font-black text-foreground flex items-center gap-2 m-0",
                    components::LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                    "Weekly Timetable Slots"
                }
                p { class: "text-xs text-muted-foreground m-0 leading-relaxed font-medium",
                    "Configure recurring weekly lessons and class slots, then sync them to generate calendar events."
                }
            }
            
            // Weekly Grid + Add Form Row
            div { class: "grid gap-6 lg:grid-cols-4 items-start w-full",
                // Left 3 columns: Weekly schedule board
                div { class: "lg:col-span-3 flex flex-col gap-4 w-full",
                    div { class: "grid gap-3 grid-cols-1 md:grid-cols-5 w-full",
                        for day_idx in 1..=5 {
                            {
                                let day_name = match day_idx {
                                    1 => "Monday",
                                    2 => "Tuesday",
                                    3 => "Wednesday",
                                    4 => "Thursday",
                                    5 => "Friday",
                                    _ => "Unknown"
                                };
                                let day_slots: Vec<yntra_core::TimetableSlot> = slots.iter().filter(|s| s.day_of_week == day_idx).cloned().collect();
                                
                                rsx! {
                                    div { class: "border border-border/40 bg-sidebar/20 rounded-xl p-3 flex flex-col gap-2 min-h-[300px] w-full",
                                        h4 { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider m-0 text-center border-b border-border/30 pb-1.5", "{day_name}" }
                                        
                                        if day_slots.is_empty() {
                                            div { class: "flex-1 flex flex-col items-center justify-center text-center opacity-30 p-2",
                                                components::LucideIcon { name: "calendar", size: "18", class: "mb-1" }
                                                span { class: "text-[9px] font-bold", "No slots" }
                                            }
                                        } else {
                                            div { class: "flex flex-col gap-2",
                                                for s in day_slots.iter() {
                                                    div { 
                                                        key: "{s.id}",
                                                        class: "group relative border border-border/30 p-2 rounded-lg bg-sidebar/40 hover:border-primary/40 hover:bg-sidebar/60 transition-all flex flex-col gap-1",
                                                        
                                                        // Delete overlay button on hover
                                                        button {
                                                            class: "absolute top-1 right-1 opacity-0 group-hover:opacity-100 p-0.5 rounded bg-destructive/10 text-destructive hover:bg-destructive/20 transition-all border-0 cursor-pointer flex items-center justify-center",
                                                            onclick: {
                                                                let s_id = s.id.clone();
                                                                let db_trig = db_trigger;
                                                                move |_| {
                                                                    let id_clone = s_id.clone();
                                                                    let mut db_trig_inner = db_trig;
                                                                    spawn(async move {
                                                                        if yntra_core::delete_timetable_slot("user-1".to_string(), id_clone).await.is_ok() {
                                                                            let current = *db_trig_inner.read();
                                                                            db_trig_inner.set(current + 1);
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            components::LucideIcon { name: "x", size: "10" }
                                                        }
                                                        
                                                        span { class: "text-[9px] font-black uppercase text-primary px-1.5 py-0.5 bg-primary/10 rounded self-start tracking-wider",
                                                            {
                                                                let c = courses.iter().find(|c| c.id == s.course_id);
                                                                c.map(|c| c.subject.clone()).unwrap_or_else(|| "Class".to_string())
                                                            }
                                                        }
                                                        span { class: "text-xs font-bold text-foreground leading-tight truncate",
                                                            {
                                                                let c = courses.iter().find(|c| c.id == s.course_id);
                                                                c.map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string())
                                                            }
                                                        }
                                                        div { class: "flex items-center gap-1 text-[9px] text-muted-foreground font-semibold mt-0.5",
                                                            components::LucideIcon { name: "clock", size: "9" }
                                                            span { "{s.start_time} - {s.end_time}" }
                                                        }
                                                        if let Some(ref room) = s.classroom {
                                                            if !room.is_empty() {
                                                                div { class: "flex items-center gap-1 text-[9px] text-muted-foreground font-semibold",
                                                                    components::LucideIcon { name: "map-pin", size: "9" }
                                                                    span { "{room}" }
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
                
                // Right column: Add Weekly Slot form
                div { class: "border border-border/50 bg-card/30 rounded-xl p-4 flex flex-col gap-4 shadow-sm",
                    h4 { class: "text-xs font-bold uppercase tracking-wider text-muted-foreground m-0 border-b border-border/30 pb-2", "Add Weekly Slot" }
                    
                    if courses.is_empty() {
                        div { class: "py-4 text-center text-xs text-muted-foreground",
                            "No courses found. Add courses in Courses & Grading first."
                        }
                    } else {
                        div { class: "flex flex-col gap-3.5",
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Subject / Course" }
                                select {
                                    class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                    value: "{selected_course_id}",
                                    onchange: move |e| selected_course_id.set(e.value()),
                                    for c in courses.iter() {
                                        option { value: "{c.id}", "{c.name}" }
                                    }
                                }
                            }
                            
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Day of Week" }
                                select {
                                    class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                    value: "{selected_day}",
                                    onchange: move |e| {
                                        if let Ok(d) = e.value().parse::<i32>() {
                                            selected_day.set(d);
                                        }
                                    },
                                    option { value: "1", "Monday" }
                                    option { value: "2", "Tuesday" }
                                    option { value: "3", "Wednesday" }
                                    option { value: "4", "Thursday" }
                                    option { value: "5", "Friday" }
                                }
                            }
                            
                            div { class: "grid grid-cols-2 gap-2",
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Start Time" }
                                    input {
                                        r#type: "text",
                                        class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                        value: "{timetable_start_time}",
                                        oninput: move |e| timetable_start_time.set(e.value()),
                                    }
                                }
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-[10px] font-bold text-muted-foreground uppercase", "End Time" }
                                    input {
                                        r#type: "text",
                                        class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                        value: "{timetable_end_time}",
                                        oninput: move |e| timetable_end_time.set(e.value()),
                                    }
                                }
                            }
                            
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-[10px] font-bold text-muted-foreground uppercase", "Classroom" }
                                input {
                                    r#type: "text",
                                    class: "yntra-input py-1.5 px-2 text-xs bg-background border border-border text-foreground w-full",
                                    placeholder: "Room 204",
                                    value: "{classroom_input}",
                                    oninput: move |e| classroom_input.set(e.value()),
                                }
                            }
                            
                            button {
                                class: "yntra-btn mt-1 text-xs py-2 w-full",
                                onclick: {
                                    let ws_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                    let db_trig = db_trigger;
                                    move |_| {
                                        let cid = selected_course_id.read().clone();
                                        let day = *selected_day.read();
                                        let start = timetable_start_time.read().clone();
                                        let end = timetable_end_time.read().clone();
                                        let room = if classroom_input.read().is_empty() { None } else { Some(classroom_input.read().clone()) };
                                        let ws = ws_id.clone();
                                        let mut db_trig_inner = db_trig;
                                        
                                        spawn(async move {
                                            if yntra_core::save_timetable_slot(
                                                "user-1".to_string(),
                                                ws,
                                                cid,
                                                day,
                                                start,
                                                end,
                                                room
                                            ).await.is_ok() {
                                                classroom_input.set(String::new());
                                                let current = *db_trig_inner.read();
                                                db_trig_inner.set(current + 1);
                                            }
                                        });
                                    }
                                },
                                "Add Slot"
                            }
                        }
                    }
                }
            }
            
            // Sync button
            div { class: "flex justify-end pt-2",
                button {
                    class: "yntra-btn text-xs font-bold py-2.5 px-5 flex items-center gap-2 shadow-md",
                    onclick: {
                        let ws_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                        let db_trig = db_trigger;
                        move |_| {
                            let ws_clone = ws_id.clone();
                            let mut db_trig_inner = db_trig;
                            spawn(async move {
                                if yntra_core::sync_timetable_to_calendar(ws_clone, "user-1".to_string()).await.is_ok() {
                                    sync_success.set(true);
                                    let current = *db_trig_inner.read();
                                    db_trig_inner.set(current + 1);
                                }
                            });
                        }
                    },
                    components::LucideIcon { name: "refresh-cw", size: "14" }
                    if *sync_success.read() { "Synced to Calendar Successfully!" } else { "Sync Slots to Calendar" }
                }
            }
        }
    }
}
