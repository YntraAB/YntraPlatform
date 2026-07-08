use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct StudentEnrollModalProps {
    pub open: bool,
    pub first_name: Signal<String>,
    pub last_name: Signal<String>,
    pub grade: Signal<String>,
    pub contact: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn StudentEnrollModal(props: StudentEnrollModalProps) -> Element {
    let mut first_name = props.first_name;
    let mut last_name = props.last_name;
    let mut grade = props.grade;
    let mut contact = props.contact;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Enroll New Student".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "First Name" }
                        input { class: "yntra-input", placeholder: "e.g. Liam", value: "{first_name}", oninput: move |e| first_name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Last Name" }
                        input { class: "yntra-input", placeholder: "e.g. Johansson", value: "{last_name}", oninput: move |e| last_name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Grade Level / Class" }
                        select {
                            class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground",
                            value: "{grade}",
                            onchange: move |e| grade.set(e.value()),
                            option { value: "Grade 7", "Grade 7" }
                            option { value: "Grade 8", "Grade 8" }
                            option { value: "Grade 9", "Grade 9" }
                            option { value: "Grade 10", "Grade 10" }
                            option { value: "Gymnasiet Yrkesförberedande", "Gymnasiet (Yrkesförberedande)" }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Parent Contact Email / Phone" }
                        input { class: "yntra-input", placeholder: "e.g. parent@example.se", value: "{contact}", oninput: move |e| contact.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Enroll Student"
                    }
                }
            }
        }
    }
}
