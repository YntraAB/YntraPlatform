use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct CourseCreateModalProps {
    pub open: bool,
    pub name: Signal<String>,
    pub subject: Signal<String>,
    pub classroom: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn CourseCreateModal(props: CourseCreateModalProps) -> Element {
    let mut name = props.name;
    let mut subject = props.subject;
    let mut classroom = props.classroom;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Add New Course".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Course Name" }
                        input { class: "yntra-input", placeholder: "e.g. Algebra I", value: "{name}", oninput: move |e| name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Subject" }
                        input { class: "yntra-input", placeholder: "e.g. Math, Physics, Art", value: "{subject}", oninput: move |e| subject.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Classroom / Room" }
                        input { class: "yntra-input", placeholder: "e.g. Room 204", value: "{classroom}", oninput: move |e| classroom.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Create Course"
                    }
                }
            }
        }
    }
}
