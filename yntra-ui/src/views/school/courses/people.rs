use dioxus::prelude::*;
use yntra_core::StudentProfile;
use crate::components;

#[derive(Props, Clone)]
pub struct CoursePeopleProps {
    pub students: Vec<StudentProfile>,
}

impl PartialEq for CoursePeopleProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CoursePeople(props: CoursePeopleProps) -> Element {
    let students = props.students;

    rsx! {
        div { class: "grid gap-6 md:grid-cols-2",
            components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Teachers" }
                div { class: "flex items-center gap-3",
                    div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                    div {
                        div { class: "text-xs font-bold text-foreground", "Ms. Andersson" }
                        div { class: "text-[10px] text-muted-foreground", "Primary Instructor" }
                    }
                }
            }
            
            components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Students ({students.len()})" }
                if students.is_empty() {
                    p { class: "text-xs text-muted-foreground m-0", "No students enrolled in this workspace." }
                } else {
                    div { class: "flex flex-col gap-3 max-h-80 overflow-y-auto pr-1",
                        for s in students.iter() {
                            div { class: "flex items-center gap-3",
                                div { class: "w-7 h-7 rounded-full bg-sidebar flex items-center justify-center font-bold text-muted-foreground text-xs border border-border/30",
                                    "{s.first_name.chars().next().unwrap_or('S')}"
                                }
                                div {
                                    div { class: "text-xs font-bold text-foreground", "{s.first_name} {s.last_name}" }
                                    div { class: "text-[10px] text-muted-foreground", "{s.grade_level}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
