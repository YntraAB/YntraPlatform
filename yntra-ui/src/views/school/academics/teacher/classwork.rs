use dioxus::prelude::*;
use crate::components::{Button, LucideIcon};
use yntra_core::Assignment;
use super::super::utils::parse_submission_content_and_advanced_attachment;

#[component]
pub fn CourseClassworkTab(
    assignments: Vec<Assignment>,
    mut show_assignment_modal: Signal<bool>,
    mut assignment_to_delete: Signal<Option<String>>,
    active_user_id: String,
    mut db_trigger: Signal<u32>,
) -> Element {
    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-center justify-between border-b border-border pb-3",
                h5 { class: "font-bold text-sm m-0 text-foreground", "Course Classwork Syllabus" }
                Button {
                    class: "px-3 h-8 text-xs font-semibold rounded-lg bg-primary text-primary-foreground flex items-center gap-1",
                    onclick: move |_| show_assignment_modal.set(true),
                    LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                    "Post Assignment"
                }
            }

            if assignments.is_empty() {
                div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No assignments posted yet for this course." }
            } else {
                div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                    for a in assignments.iter() {
                        {
                            let a_id = a.id.clone();
                            let is_delete_confirm = *assignment_to_delete.read() == Some(a_id.clone());
                            let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&a.description);
                            rsx! {
                                div { key: "{a.id}", class: "p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-4 text-xs hover:bg-muted/5 transition-colors",
                                    div { class: "space-y-1.5 flex-1 min-w-0",
                                        div { class: "flex items-center gap-2 flex-wrap",
                                            span { class: "font-bold text-foreground text-sm", "{a.title}" }
                                            span { class: "text-[9px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "Max Points: {a.max_points}" }
                                            span { class: "text-[10px] text-muted-foreground font-semibold flex items-center gap-1",
                                                LucideIcon { name: "calendar", class: "h-3.5 w-3.5" }
                                                "Due: {a.due_date}"
                                            }
                                        }
                                        if !desc_text.is_empty() {
                                            div { class: "text-muted-foreground break-words font-medium leading-relaxed max-w-2xl", "{desc_text}" }
                                        }
                                        
                                        if let Some(staged) = attachment {
                                            a {
                                                href: "{staged.dataurl}",
                                                download: "{staged.filename}",
                                                class: "flex items-center gap-2 p-1.5 bg-card/45 border border-primary/20 hover:bg-card/75 rounded-lg no-underline text-foreground cursor-pointer transition-all mt-2 w-fit max-w-sm",
                                                LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary shrink-0" }
                                                span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                LucideIcon { name: "download", class: "h-3 w-3 text-muted-foreground ml-1.5 shrink-0" }
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2 self-start sm:self-center shrink-0",
                                        if is_delete_confirm {
                                            button {
                                                class: "px-2.5 h-8 text-[10px] font-bold rounded-lg bg-red-500 text-white border-0 cursor-pointer hover:bg-red-600 transition-colors",
                                                r#type: "button",
                                                onclick: {
                                                    let a_id_del = a.id.clone();
                                                    let uid_c = active_user_id.clone();
                                                    let mut trig = db_trigger;
                                                    move |_| {
                                                        let aid = a_id_del.clone();
                                                        let uid_del = uid_c.clone();
                                                        spawn(async move {
                                                            if yntra_core::delete_assignment(uid_del, aid).await.is_ok() {
                                                                assignment_to_delete.set(None);
                                                                let current = *trig.read();
                                                                trig.set(current + 1);
                                                            }
                                                        });
                                                    }
                                                },
                                                "Confirm Delete"
                                            }
                                            button {
                                                class: "px-2.5 h-8 text-[10px] font-bold rounded-lg bg-muted text-foreground border border-border/80 cursor-pointer transition-colors",
                                                r#type: "button",
                                                onclick: move |_| assignment_to_delete.set(None),
                                                "Cancel"
                                            }
                                        } else {
                                            button {
                                                class: "p-2 rounded hover:bg-muted text-muted-foreground hover:text-red-500 border-0 cursor-pointer transition-colors",
                                                r#type: "button",
                                                onclick: move |_| assignment_to_delete.set(Some(a_id.clone())),
                                                LucideIcon { name: "trash", size: "14" }
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
