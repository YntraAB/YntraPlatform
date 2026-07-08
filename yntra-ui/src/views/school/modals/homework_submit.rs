use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct HomeworkSubmitModalProps {
    pub open: bool,
    pub title: String,
    pub desc: String,
    pub submission_text: Signal<String>,
    pub due_date: String,
    pub late_policy: String,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn HomeworkSubmitModal(props: HomeworkSubmitModalProps) -> Element {
    let mut submission_text = props.submission_text;
    
    let today = yntra_core::infra::time::get_current_datetime_str()[0..10].to_string(); // "YYYY-MM-DD"
    let is_late = today > props.due_date;
    let policy = props.late_policy.clone();
    
    let can_submit = !is_late || policy != "hard_deadline";
    
    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: format!("Submit: {}", props.title),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-96",
                    div { class: "text-xs text-muted-foreground italic bg-sidebar/30 p-2.5 rounded border border-border/40",
                        "{props.desc}"
                    }
                    
                    if is_late {
                        div { class: "p-3 rounded-lg border text-xs font-semibold flex flex-col gap-1 bg-red-500/10 border-red-500/20 text-red-400",
                            span { "⚠️ Assignment is past the due date ({props.due_date})!" }
                            if policy == "hard_deadline" {
                                span { class: "font-black text-[10px] uppercase tracking-wider text-red-500", "Submissions are blocked by course policy." }
                            } else if policy == "penalty_5" {
                                span { "A 5% daily deduction penalty will be applied to your grade." }
                            } else if policy == "penalty_10" {
                                span { "A 10% daily deduction penalty will be applied to your grade." }
                            }
                        }
                    }
                    
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Your Answer / Homework Submission" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-24 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Type your answer or submit assignment details...",
                            value: "{submission_text}",
                            oninput: move |e| submission_text.set(e.value()),
                            disabled: !can_submit
                        }
                    }
                    
                    button {
                        class: "yntra-btn mt-2 flex items-center justify-center gap-1.5",
                        disabled: !can_submit,
                        onclick: move |_| props.onsubmit.call(()),
                        components::LucideIcon { name: "check", size: "14" }
                        "Submit Answer"
                    }
                }
            }
        }
    }
}
