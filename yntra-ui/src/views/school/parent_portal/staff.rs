use dioxus::prelude::*;
use crate::components;
use yntra_core::Course;

#[derive(Props, Clone)]
pub struct StaffTabProps {
    pub courses: Vec<Course>,
    pub on_message: EventHandler<(String, String)>,
}

impl PartialEq for StaffTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn StaffTab(props: StaffTabProps) -> Element {
    let courses = props.courses;
    let on_message = props.on_message;

    rsx! {
        components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
            h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                components::LucideIcon { name: "users", size: "18", class: "text-primary" }
                "Course Teachers & Advisors"
            }
            
            div { class: "grid gap-4 md:grid-cols-2",
                for c in courses.iter() {
                    div { class: "border border-border/30 p-4 rounded-xl flex justify-between items-center bg-white/[0.01] hover:border-primary/30 transition-all",
                        div { class: "flex flex-col gap-1.5",
                            span { class: "text-[10px] font-black uppercase bg-primary/10 text-primary px-2 py-0.5 rounded self-start", "{c.subject}" }
                            div { class: "font-bold text-foreground text-sm", "{c.name}" }
                            div { class: "text-xs text-muted-foreground flex items-center gap-1",
                                components::LucideIcon { name: "home", size: "12" }
                                "Classroom: {c.classroom.clone().unwrap_or_else(|| \"TBD\".to_string())}"
                            }
                        }
                        
                        button {
                            class: "yntra-btn text-xs font-bold py-1.5 px-3.5 flex items-center gap-1.5 shadow-sm",
                            onclick: {
                                let teacher_id = c.teacher_id.clone().unwrap_or_else(|| "user-1".to_string());
                                let course_name = c.name.clone();
                                move |_| {
                                    on_message.call((teacher_id.clone(), course_name.clone()));
                                }
                            },
                            components::LucideIcon { name: "message-square", size: "12" }
                            "Message"
                        }
                    }
                }
            }
        }
    }
}
