use dioxus::prelude::*;
use yntra_core::{Course, WorkspaceTemplateType};
use crate::locales::t;

#[derive(Props, Clone, PartialEq)]
pub struct TemplateInputsProps {
    pub template: WorkspaceTemplateType,
    pub selected_course_id: Signal<String>,
    pub classroom_text: Signal<String>,
    pub vehicle_id_text: Signal<String>,
    pub cargo_volume_text: Signal<String>,
    pub destination_text: Signal<String>,
    pub courses: Vec<Course>,
    pub locale: String,
}

#[component]
pub fn TemplateInputs(props: TemplateInputsProps) -> Element {
    let mut selected_course_id = props.selected_course_id;
    let mut classroom_text = props.classroom_text;
    let mut vehicle_id_text = props.vehicle_id_text;
    let mut cargo_volume_text = props.cargo_volume_text;
    let mut destination_text = props.destination_text;

    match props.template {
        WorkspaceTemplateType::School => {
            rsx! {
                div { class: "grid grid-cols-2 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-course\", &props.locale)}"
                        }
                        select {
                            class: "yntra-input border-border bg-muted/50 w-full cursor-pointer",
                            value: "{selected_course_id}",
                            onchange: move |e| selected_course_id.set(e.value()),
                            option { value: "none", "{t(\"scheduler-select-course\", &props.locale)}" }
                            for course in props.courses.iter() {
                                option { value: "{course.id}", "{course.name}" }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-classroom\", &props.locale)}"
                        }
                        input {
                            r#type: "text",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            placeholder: "e.g. Sal 101",
                            value: "{classroom_text}",
                            oninput: move |e| classroom_text.set(e.value())
                        }
                    }
                }
            }
        }
        WorkspaceTemplateType::MovingCompany => {
            rsx! {
                div { class: "grid grid-cols-3 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-vehicle\", &props.locale)}"
                        }
                        input {
                            r#type: "text",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            placeholder: "e.g. Truck A",
                            value: "{vehicle_id_text}",
                            oninput: move |e| vehicle_id_text.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-volume\", &props.locale)}"
                        }
                        input {
                            r#type: "text",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            placeholder: "e.g. 25",
                            value: "{cargo_volume_text}",
                            oninput: move |e| cargo_volume_text.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-destination\", &props.locale)}"
                        }
                        input {
                            r#type: "text",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            placeholder: "e.g. Storgatan 1",
                            value: "{destination_text}",
                            oninput: move |e| destination_text.set(e.value())
                        }
                    }
                }
            }
        }
        _ => rsx! {},
    }
}
