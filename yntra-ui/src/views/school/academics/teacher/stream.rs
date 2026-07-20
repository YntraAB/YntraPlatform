use dioxus::prelude::*;
use crate::components::{Button, LucideIcon};
use yntra_core::Assignment;

#[component]
pub fn CourseStreamTab(
    announcements: Vec<Assignment>,
    comments: Vec<Assignment>,
    course_id: String,
    workspace_id: String,
    teacher_name: String,
    active_user_id: String,
    mut db_trigger: Signal<u32>,
    mut new_announcement_text: Signal<String>,
    mut comment_inputs: Signal<std::collections::HashMap<String, String>>,
) -> Element {
    rsx! {
        div { class: "space-y-6",
            // Announcement editor box
            div { class: "p-4 border border-border/80 rounded-xl bg-muted/20 space-y-3",
                textarea {
                    class: "w-full min-h-[80px] p-3 text-xs bg-background border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/60 resize-none font-medium",
                    placeholder: "Announce something to your class...",
                    value: "{new_announcement_text}",
                    oninput: move |evt| new_announcement_text.set(evt.value().clone()),
                }
                div { class: "flex justify-end",
                    Button {
                        class: "px-4 h-8 text-xs font-semibold rounded-lg bg-primary text-primary-foreground flex items-center gap-1",
                        disabled: new_announcement_text.read().trim().is_empty(),
                        onclick: {
                            let cid = course_id.clone();
                            let ws = workspace_id.clone();
                            let teacher_name = teacher_name.clone();
                            let uid = active_user_id.clone();
                            move |_| {
                                let text = new_announcement_text.read().clone();
                                let assignment = yntra_core::Assignment {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    workspace_id: ws.clone(),
                                    course_id: cid.clone(),
                                    title: teacher_name.clone(),
                                    description: text,
                                    due_date: "teacher".to_string(),
                                    max_points: -1,
                                    updated_at: yntra_core::infra::time::get_current_time_ms(),
                                };
                                let uid_c = uid.clone();
                                let mut trig = db_trigger;
                                spawn(async move {
                                    if yntra_core::save_assignment(uid_c, assignment, None).await.is_ok() {
                                        new_announcement_text.set(String::new());
                                        let current = *trig.read();
                                        trig.set(current + 1);
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
                div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic", "No announcements yet. Welcome your class with a post!" }
            } else {
                div { class: "space-y-4",
                    for ann in announcements.iter().rev() {
                        {
                            let ann_id = ann.id.clone();
                            let ann_comments = comments.iter().filter(|comm| comm.course_id == ann_id).collect::<Vec<_>>();
                            let comment_text = comment_inputs.read().get(&ann_id).cloned().unwrap_or_default();
                            rsx! {
                                div { key: "{ann.id}", class: "p-5 border border-border rounded-xl bg-background space-y-4 shadow-sm",
                                    div { class: "flex items-start justify-between gap-3",
                                        div { class: "flex items-center gap-2.5",
                                            div { class: "h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-xs",
                                                "{ann.title.chars().next().unwrap_or('?')}"
                                            }
                                            div {
                                                div { class: "text-xs font-bold text-foreground flex items-center gap-1.5", 
                                                    "{ann.title}"
                                                    span { class: "text-[9px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-extrabold uppercase tracking-wide", "Teacher" }
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
                                        button {
                                            class: "p-1.5 rounded hover:bg-muted text-muted-foreground hover:text-red-500 border-0 cursor-pointer transition-colors",
                                            r#type: "button",
                                            onclick: {
                                                let ann_id_del = ann.id.clone();
                                                let uid_c = active_user_id.clone();
                                                let mut trig = db_trigger;
                                                move |_| {
                                                    let aid = ann_id_del.clone();
                                                    let uid_del = uid_c.clone();
                                                    spawn(async move {
                                                        if yntra_core::delete_assignment(uid_del, aid).await.is_ok() {
                                                            let current = *trig.read();
                                                            trig.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            LucideIcon { name: "trash", size: "13" }
                                        }
                                    }
                                    div { class: "text-xs text-foreground font-medium whitespace-pre-line leading-relaxed", "{ann.description}" }
                                    
                                    div { class: "border-t border-border/40 pt-3 space-y-3",
                                        div { class: "text-[10px] font-bold text-muted-foreground flex items-center gap-1",
                                            LucideIcon { name: "message-square", size: "11" }
                                            "Class comments ({ann_comments.len()})"
                                        }
                                        
                                        if !ann_comments.is_empty() {
                                            div { class: "space-y-3 pl-3 border-l-2 border-muted",
                                                for comm in ann_comments.iter() {
                                                    div { key: "{comm.id}", class: "text-xs space-y-0.5",
                                                        div { class: "flex items-center gap-1.5",
                                                            span { class: "font-bold text-foreground", "{comm.title}" }
                                                            span { class: "text-[8px] font-bold px-1 rounded bg-muted text-muted-foreground uppercase", "{comm.due_date}" }
                                                            span { class: "text-[9px] text-muted-foreground/60",
                                                                {
                                                                    let ms = comm.updated_at;
                                                                    let formatted = format_timestamp(ms);
                                                                    formatted
                                                                }
                                                            }
                                                        }
                                                        div { class: "text-muted-foreground font-medium", "{comm.description}" }
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
                                                    let teacher_name = teacher_name.clone();
                                                    let uid = active_user_id.clone();
                                                    let text = comment_text.clone();
                                                    let trig = db_trigger;
                                                    move |evt| {
                                                        if evt.key() == Key::Enter && !text.trim().is_empty() {
                                                            let comment = yntra_core::Assignment {
                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                workspace_id: ws.clone(),
                                                                course_id: ann_id_c.clone(),
                                                                title: teacher_name.clone(),
                                                                description: text.clone(),
                                                                due_date: "teacher".to_string(),
                                                                max_points: -2,
                                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                            };
                                                            let uid_c = uid.clone();
                                                            let ann_id_c_clone = ann_id_c.clone();
                                                            let mut comment_inputs_clone = comment_inputs;
                                                            let mut trig_clone = trig;
                                                            spawn(async move {
                                                                if yntra_core::save_assignment(uid_c, comment, None).await.is_ok() {
                                                                    comment_inputs_clone.write().insert(ann_id_c_clone, String::new());
                                                                    let current = *trig_clone.read();
                                                                    trig_clone.set(current + 1);
                                                                }
                                                            });
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

fn format_timestamp(ms: i64) -> String {
    let seconds = ms / 1000;
    if let Some(dt) = chrono::DateTime::from_timestamp(seconds, 0) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Just now".to_string()
    }
}
