use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{WorkspaceTemplateType, Course};

#[derive(Props, Clone, PartialEq)]
pub struct TemplateInputsProps {
    pub template: WorkspaceTemplateType,
    pub courses: Vec<Course>,
    pub selected_course_id: Signal<String>,
    pub classroom_text: Signal<String>,
    pub vehicle_id_text: Signal<String>,
    pub cargo_volume_text: Signal<String>,
    pub destination_text: Signal<String>,
    pub locale: String,
}

#[component]
pub fn TemplateInputs(props: TemplateInputsProps) -> Element {
    let mut vehicle_id_text = props.vehicle_id_text;
    let mut cargo_volume_text = props.cargo_volume_text;
    let mut destination_text = props.destination_text;

    match props.template {
        WorkspaceTemplateType::School => {
            let mut classroom_text = props.classroom_text;
            let mut selected_course_id = props.selected_course_id;
            let mut course_dropdown_open = use_signal(|| false);

            let current_course_name = if *selected_course_id.read() == "none" {
                t("scheduler-select-course", &props.locale)
            } else if let Some(course) = props.courses.iter().find(|c| c.id == *selected_course_id.read()) {
                format!("{} ({})", course.name, course.subject)
            } else {
                t("scheduler-select-course", &props.locale)
            };

            rsx! {
                div { class: "grid grid-cols-2 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-course\", &props.locale)}"
                        }
                        crate::components::Dropdown {
                            label: current_course_name,
                            open: *course_dropdown_open.read(),
                            ontoggle: move |_| {
                                let cur = *course_dropdown_open.read();
                                course_dropdown_open.set(!cur);
                            },
                            crate::components::DropdownItem {
                                label: t("scheduler-select-course", &props.locale),
                                onclick: move |_| {
                                    selected_course_id.set("none".to_string());
                                    course_dropdown_open.set(false);
                                }
                            }
                            for course in props.courses.iter() {
                                {
                                    let course_id = course.id.clone();
                                    let course_name = format!("{} ({})", course.name, course.subject);
                                    let course_classroom = course.classroom.clone();
                                    rsx! {
                                        crate::components::DropdownItem {
                                            label: course_name,
                                            onclick: move |_| {
                                                selected_course_id.set(course_id.clone());
                                                course_dropdown_open.set(false);
                                                // Intelligent UX: Auto-fill default classroom
                                                if let Some(ref room) = course_classroom {
                                                    classroom_text.set(room.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-classroom\", &props.locale)}"
                        }
                        crate::components::Input {
                            r#type: "text",
                            class: "border-border bg-muted/50 w-full",
                            placeholder: "e.g. Room 204B",
                            value: "{classroom_text}",
                            oninput: move |e: FormEvent| classroom_text.set(e.value())
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
                        crate::components::Input {
                            r#type: "text",
                            class: "border-border bg-muted/50 w-full",
                            placeholder: "e.g. Truck A",
                            value: "{vehicle_id_text}",
                            oninput: move |e: FormEvent| vehicle_id_text.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-volume\", &props.locale)}"
                        }
                        crate::components::Input {
                            r#type: "text",
                            class: "border-border bg-muted/50 w-full",
                            placeholder: "e.g. 25",
                            value: "{cargo_volume_text}",
                            oninput: move |e: FormEvent| cargo_volume_text.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"scheduler-destination\", &props.locale)}"
                        }
                        crate::components::Input {
                            r#type: "text",
                            class: "border-border bg-muted/50 w-full",
                            placeholder: "e.g. Storgatan 1",
                            value: "{destination_text}",
                            oninput: move |e: FormEvent| destination_text.set(e.value())
                        }
                    }
                }
            }
        }
        _ => rsx! {},
    }
}
