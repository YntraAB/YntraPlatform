use dioxus::prelude::*;
use crate::components::{Button, LucideIcon};
use crate::locales::t;
use yntra_core::{Assignment, Course, StudentProfile, delete_assignment};

#[component]
pub fn StudentStreamTab(
    announcements: Vec<Assignment>,
    comments: Vec<Assignment>,
    courses: Vec<Course>,
    student_name: String,
    current_student: StudentProfile,
    workspace_id: String,
    active_user_id: String,
    mut db_trigger: Signal<u32>,
    mut new_announcement_text: Signal<String>,
    mut comment_inputs: Signal<std::collections::HashMap<String, String>>,
    mut editing_item_id: Signal<Option<String>>,
    mut edit_text: Signal<String>,
    locale: String,
) -> Element {
    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            // Announcement editor box
            div { class: "p-4 border border-border/80 rounded-xl bg-muted/20 space-y-3",
                textarea {
                    class: "w-full min-h-[80px] p-3 text-xs bg-background border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/60 resize-none font-medium",
                    placeholder: "Share something with your class...",
                    value: "{new_announcement_text}",
                    oninput: move |evt| new_announcement_text.set(evt.value().clone()),
                }
                div { class: "flex justify-end",
                    Button {
                        class: "px-4 h-8 text-xs font-semibold rounded-lg bg-primary text-primary-foreground flex items-center gap-1",
                        disabled: new_announcement_text.read().trim().is_empty(),
                        onclick: {
                            let current_student_c = current_student.clone();
                            let ws = workspace_id.clone();
                            let uid = active_user_id.clone();
                            let trig = db_trigger;
                            let courses_c = courses.clone();
                            move |_| {
                                let text = new_announcement_text.read().clone();
                                let author_name = format!("{} {}", current_student_c.first_name, current_student_c.last_name);
                                let course_id = courses_c.first().map(|c| c.id.clone()).unwrap_or_default();
                                let assignment = yntra_core::Assignment {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    workspace_id: ws.clone(),
                                    course_id,
                                    title: author_name,
                                    description: text,
                                    due_date: "student".to_string(),
                                    max_points: -1,
                                    updated_at: yntra_core::infra::time::get_current_time_ms(),
                                };
                                let uid_c = uid.clone();
                                let mut trig_c = trig;
                                spawn(async move {
                                    if yntra_core::save_assignment(uid_c, assignment, None).await.is_ok() {
                                        new_announcement_text.set(String::new());
                                        let current = *trig_c.read();
                                        trig_c.set(current + 1);
                                    }
                                });
                            }
                        },
                        LucideIcon { name: "send", class: "h-3 w-3" }
                        "Post"
                    }
                }
            }

            // Stream Announcements Feed
            if announcements.is_empty() {
                div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic", "No announcements yet." }
            } else {
                div { class: "space-y-4",
                    for ann in announcements.clone().into_iter().rev() {
                        {
                            let ann_id = ann.id.clone();
                            let ann_comments = comments.iter().filter(|comm| comm.course_id == ann_id).cloned().collect::<Vec<_>>();
                            let comment_text = comment_inputs.read().get(&ann_id).cloned().unwrap_or_default();
                            let is_author = ann.due_date == "student";
                            let ann_title = ann.title.clone();
                            let ann_desc = ann.description.clone();
                            rsx! {
                                div { key: "{ann_id}", class: "p-5 border border-border rounded-xl bg-background space-y-4 shadow-sm",
                                    div { class: "flex items-start justify-between gap-3",
                                        div { class: "flex items-center gap-2.5",
                                            div { class: "h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-xs",
                                                "{ann_title.chars().next().unwrap_or('?')}"
                                            }
                                            div {
                                                div { class: "text-xs font-bold text-foreground flex items-center gap-1.5", 
                                                    "{ann_title}"
                                                    span { 
                                                        class: format!(
                                                            "text-[9px] px-1.5 py-0.5 rounded font-extrabold uppercase tracking-wide {}",
                                                            if is_author { "bg-accent/10 text-accent" } else { "bg-primary/10 text-primary" }
                                                        ),
                                                        if is_author { "Student" } else { "Teacher" }
                                                    }
                                                }
                                                div { class: "text-[10px] text-muted-foreground",
                                                    {
                                                        let ms = ann.updated_at;
                                                        let formatted = format_timestamp(ms);
                                                        formatted
                                                    }
                                                }
                                            }
                                        }
                                        if is_author && ann_title == student_name {
                                            div { class: "flex items-center gap-1.5",
                                                button {
                                                    class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-none bg-transparent cursor-pointer transition-colors",
                                                    r#type: "button",
                                                    onclick: {
                                                        let ann_desc_c = ann_desc.clone();
                                                        let ann_id_c = ann_id.clone();
                                                        move |_| {
                                                            editing_item_id.set(Some(ann_id_c.clone()));
                                                            edit_text.set(ann_desc_c.clone());
                                                        }
                                                    },
                                                    LucideIcon { name: "pencil", class: "h-3.5 w-3.5" }
                                                }
                                                button {
                                                    class: "p-1 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 border-none bg-transparent cursor-pointer transition-colors",
                                                    r#type: "button",
                                                    onclick: {
                                                        let ann_id_c = ann_id.clone();
                                                        let uid_c = active_user_id.clone();
                                                        let trig_c = db_trigger;
                                                        move |_| {
                                                            let uid_del = uid_c.clone();
                                                            let id_del = ann_id_c.clone();
                                                            let mut trig_del = trig_c;
                                                            spawn(async move {
                                                                if delete_assignment(uid_del, id_del).await.is_ok() {
                                                                    let val = *trig_del.read();
                                                                    trig_del.set(val + 1);
                                                                }
                                                            });
                                                        }
                                                    },
                                                    LucideIcon { name: "trash", class: "h-3.5 w-3.5" }
                                                }
                                            }
                                        }
                                    }
                                    if *editing_item_id.read() == Some(ann_id.clone()) {
                                        div { class: "flex flex-col gap-2 p-2 bg-muted/10 border border-border rounded-lg",
                                            textarea {
                                                class: "w-full min-h-[75px] p-2.5 text-xs bg-background border border-border rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground resize-none",
                                                value: "{edit_text}",
                                                oninput: move |evt| edit_text.set(evt.value().clone()),
                                            }
                                            div { class: "flex justify-end gap-2",
                                                Button {
                                                    class: "px-3 h-7 text-[10px] font-bold rounded-lg border border-border bg-background hover:bg-muted text-foreground",
                                                    onclick: move |_| editing_item_id.set(None),
                                                    "Cancel"
                                                }
                                                Button {
                                                    class: "px-3 h-7 text-[10px] font-bold rounded-lg bg-primary text-primary-foreground",
                                                    disabled: edit_text.read().trim().is_empty(),
                                                    onclick: {
                                                        let updated_ann = ann.clone();
                                                        let uid_c = active_user_id.clone();
                                                        let trig_c = db_trigger;
                                                        move |_| {
                                                            let text_val = edit_text.read().clone();
                                                            let mut item_val = updated_ann.clone();
                                                            item_val.description = text_val;
                                                            item_val.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                            let uid_save = uid_c.clone();
                                                            let mut trig_save = trig_c;
                                                            spawn(async move {
                                                                if yntra_core::save_assignment(uid_save, item_val, None).await.is_ok() {
                                                                    editing_item_id.set(None);
                                                                    let val = *trig_save.read();
                                                                    trig_save.set(val + 1);
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "Save"
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "text-xs text-foreground font-medium whitespace-pre-line leading-relaxed", "{ann_desc}" }
                                    }
                                    
                                    div { class: "border-t border-border/40 pt-3 space-y-3",
                                        div { class: "text-[10px] font-bold text-muted-foreground flex items-center gap-1",
                                            LucideIcon { name: "message-square", size: "11" }
                                            "Class comments ({ann_comments.len()})"
                                        }
                                        
                                        if !ann_comments.is_empty() {
                                            div { class: "space-y-3 pl-3 border-l-2 border-muted",
                                                for comm in ann_comments.into_iter() {
                                                    {
                                                        let comm_id = comm.id.clone();
                                                        let comm_title = comm.title.clone();
                                                        let comm_due_date = comm.due_date.clone();
                                                        let comm_desc = comm.description.clone();
                                                        let comm_updated_at = comm.updated_at;
                                                        rsx! {
                                                            div { key: "{comm_id}", class: "text-xs space-y-0.5",
                                                                div { class: "flex items-center justify-between gap-1.5 w-full",
                                                                    div { class: "flex items-center gap-1.5",
                                                                        span { class: "font-bold text-foreground", "{comm_title}" }
                                                                        span { class: "text-[8px] font-bold px-1 rounded bg-muted text-muted-foreground uppercase", "{comm_due_date}" }
                                                                        span { class: "text-[9px] text-muted-foreground/60",
                                                                            {
                                                                                let ms = comm_updated_at;
                                                                                let formatted = format_timestamp(ms);
                                                                                formatted
                                                                            }
                                                                        }
                                                                    }
                                                                    if comm_due_date == "student" && comm_title == student_name {
                                                                        div { class: "flex items-center gap-1",
                                                                            button {
                                                                                class: "p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-none bg-transparent cursor-pointer transition-colors",
                                                                                r#type: "button",
                                                                                onclick: {
                                                                                    let comm_desc_c = comm_desc.clone();
                                                                                    let comm_id_c = comm_id.clone();
                                                                                    move |_| {
                                                                                        editing_item_id.set(Some(comm_id_c.clone()));
                                                                                        edit_text.set(comm_desc_c.clone());
                                                                                    }
                                                                                },
                                                                                LucideIcon { name: "pencil", class: "h-3 w-3" }
                                                                            }
                                                                            button {
                                                                                class: "p-0.5 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 border-none bg-transparent cursor-pointer transition-colors",
                                                                                r#type: "button",
                                                                                onclick: {
                                                                                    let comm_id_c = comm_id.clone();
                                                                                    let uid_c = active_user_id.clone();
                                                                                    let trig_c = db_trigger;
                                                                                    move |_| {
                                                                                        let uid_del = uid_c.clone();
                                                                                        let id_del = comm_id_c.clone();
                                                                                        let mut trig_del = trig_c;
                                                                                        spawn(async move {
                                                                                            if delete_assignment(uid_del, id_del).await.is_ok() {
                                                                                                let val = *trig_del.read();
                                                                                                trig_del.set(val + 1);
                                                                                            }
                                                                                        });
                                                                                    }
                                                                                },
                                                                                LucideIcon { name: "trash", class: "h-3 w-3" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                if *editing_item_id.read() == Some(comm_id.clone()) {
                                                                    div { class: "flex flex-col gap-2 p-1.5 bg-muted/10 border border-border rounded-lg mt-1",
                                                                        input {
                                                                            class: "w-full h-8 px-2 text-xs bg-background border border-border rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground font-medium",
                                                                            value: "{edit_text}",
                                                                            oninput: move |evt| edit_text.set(evt.value().clone()),
                                                                        }
                                                                        div { class: "flex justify-end gap-1.5",
                                                                            Button {
                                                                                class: "px-2 h-6 text-[9px] font-bold rounded-lg border border-border bg-background hover:bg-muted text-foreground",
                                                                                onclick: move |_| editing_item_id.set(None),
                                                                                "Cancel"
                                                                            }
                                                                            Button {
                                                                                class: "px-2 h-6 text-[9px] font-bold rounded-lg bg-primary text-primary-foreground",
                                                                                disabled: edit_text.read().trim().is_empty(),
                                                                                onclick: {
                                                                                    let updated_comm = comm.clone();
                                                                                    let uid_c = active_user_id.clone();
                                                                                    let trig_c = db_trigger;
                                                                                    move |_| {
                                                                                        let text_val = edit_text.read().clone();
                                                                                        let mut item_val = updated_comm.clone();
                                                                                        item_val.description = text_val;
                                                                                        item_val.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                                                        let uid_save = uid_c.clone();
                                                                                        let mut trig_save = trig_c;
                                                                                        spawn(async move {
                                                                                            if yntra_core::save_assignment(uid_save, item_val, None).await.is_ok() {
                                                                                                editing_item_id.set(None);
                                                                                                let val = *trig_save.read();
                                                                                                trig_save.set(val + 1);
                                                                                            }
                                                                                        });
                                                                                    }
                                                                                },
                                                                                "Save"
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    div { class: "text-muted-foreground font-medium pl-1", "{comm_desc}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        div { class: "flex items-center gap-2 pt-1.5",
                                            input {
                                                class: "flex-1 h-8 px-3 text-xs bg-muted/30 border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/50 font-medium",
                                                placeholder: "Add class comment...",
                                                value: "{comment_text}",
                                                oninput: {
                                                    let ann_id_c = ann_id.clone();
                                                    move |evt| {
                                                        comment_inputs.write().insert(ann_id_c.clone(), evt.value().clone());
                                                    }
                                                },
                                                onkeydown: {
                                                    let ann_id_c = ann_id.clone();
                                                    let ws = workspace_id.clone();
                                                    let author_name = student_name.clone();
                                                    let uid = active_user_id.clone();
                                                    let text = comment_text.clone();
                                                    let trig = db_trigger;
                                                    move |evt| {
                                                        if evt.key() == Key::Enter && !text.trim().is_empty() {
                                                            let val = text.clone();
                                                            let aid = ann_id_c.clone();
                                                            let c_item = yntra_core::Assignment {
                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                workspace_id: ws.clone(),
                                                                course_id: aid,
                                                                title: author_name.clone(),
                                                                description: val,
                                                                due_date: "student".to_string(),
                                                                max_points: -2,
                                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                            };
                                                            let uid_c = uid.clone();
                                                            comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                            let mut trig_save = trig;
                                                            spawn(async move {
                                                                if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                    let current = *trig_save.read();
                                                                    trig_save.set(current + 1);
                                                                }
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                            button {
                                                class: "h-8 w-8 rounded-lg bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground border-0 cursor-pointer flex items-center justify-center transition-colors",
                                                r#type: "button",
                                                onclick: {
                                                    let ann_id_c = ann_id.clone();
                                                    let ws = workspace_id.clone();
                                                    let author_name = student_name.clone();
                                                    let uid = active_user_id.clone();
                                                    let text = comment_text.clone();
                                                    let trig = db_trigger;
                                                    move |_| {
                                                        if !text.trim().is_empty() {
                                                            let val = text.clone();
                                                            let aid = ann_id_c.clone();
                                                            let c_item = yntra_core::Assignment {
                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                workspace_id: ws.clone(),
                                                                course_id: aid,
                                                                title: author_name.clone(),
                                                                description: val,
                                                                due_date: "student".to_string(),
                                                                max_points: -2,
                                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                            };
                                                            let uid_c = uid.clone();
                                                            comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                            let mut trig_save = trig;
                                                            spawn(async move {
                                                                if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                    let current = *trig_save.read();
                                                                    trig_save.set(current + 1);
                                                                }
                                                            });
                                                        }
                                                    }
                                                },
                                                LucideIcon { name: "arrow-right", class: "h-3.5 w-3.5" }
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

fn format_timestamp(ms: i64) -> String {
    let seconds = ms / 1000;
    if let Some(dt) = chrono::DateTime::from_timestamp(seconds, 0) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Just now".to_string()
    }
}
