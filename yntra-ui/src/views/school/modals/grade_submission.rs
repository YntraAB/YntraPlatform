use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct GradeSubmissionModalProps {
    pub open: bool,
    pub grade: Signal<String>,
    pub feedback: Signal<String>,
    pub grading_system: String,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn GradeSubmissionModal(props: GradeSubmissionModalProps) -> Element {
    let mut grade = props.grade;
    let mut feedback = props.feedback;
    let grading_system = props.grading_system.clone();

    rsx! {
        if props.open {
            crate::components::Dialog {
                open: props.open,
                title: "Grade Submission".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Grade" }
                        
                        if grading_system == "1-100" {
                            input {
                                r#type: "number",
                                min: "0",
                                max: "100",
                                class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground h-9 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary",
                                placeholder: "Enter score (0-100)",
                                value: "{grade}",
                                oninput: move |e| grade.set(e.value()),
                            }
                        } else {
                            select {
                                class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground h-9 rounded-lg focus:outline-none",
                                value: "{grade}",
                                onchange: move |e| grade.set(e.value()),
                                if grading_system == "U-G" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "G", "G (Godkänd)" }
                                } else if grading_system == "U-G-VG" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "G", "G (Godkänd)" }
                                    option { value: "VG", "VG (Väl Godkänd)" }
                                } else if grading_system == "U-3-4-5" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "3", "3" }
                                    option { value: "4", "4" }
                                    option { value: "5", "5" }
                                } else if grading_system == "1-10" {
                                    option { value: "1", "1" }
                                    option { value: "2", "2" }
                                    option { value: "3", "3" }
                                    option { value: "4", "4" }
                                    option { value: "5", "5" }
                                    option { value: "6", "6" }
                                    option { value: "7", "7" }
                                    option { value: "8", "8" }
                                    option { value: "9", "9" }
                                    option { value: "10", "10" }
                                } else {
                                    option { value: "A", "A" }
                                    option { value: "B", "B" }
                                    option { value: "C", "C" }
                                    option { value: "D", "D" }
                                    option { value: "E", "E" }
                                    option { value: "F", "F" }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Feedback" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-20 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Add feedback notes for the student...",
                            value: "{feedback}",
                            oninput: move |e| feedback.set(e.value())
                        }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Save Grade"
                    }
                }
            }
        }
    }
}
