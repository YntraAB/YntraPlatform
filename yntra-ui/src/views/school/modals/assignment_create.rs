use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone, PartialEq)]
pub struct AssignmentCreateModalProps {
    pub open: bool,
    pub title: Signal<String>,
    pub desc: Signal<String>,
    pub due: Signal<String>,
    pub points: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn AssignmentCreateModal(props: AssignmentCreateModalProps) -> Element {
    let mut title = props.title;
    let mut desc = props.desc;
    let mut due = props.due;
    let mut points = props.points;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Create New Assignment".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Title" }
                        input { class: "yntra-input", placeholder: "e.g. Algebra Quiz 1", value: "{title}", oninput: move |e| title.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Instructions / Description" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-20 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Provide assignment details...",
                            value: "{desc}",
                            oninput: move |e| desc.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Due Date" }
                        input { r#type: "date", class: "yntra-input", value: "{due}", oninput: move |e| due.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Maximum Points" }
                        input { r#type: "number", class: "yntra-input", value: "{points}", oninput: move |e| points.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Post Assignment"
                    }
                }
            }
        }
    }
}
