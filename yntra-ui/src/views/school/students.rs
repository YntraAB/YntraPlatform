use dioxus::prelude::*;
use crate::components;
use yntra_core::StudentProfile;

#[derive(Props, Clone)]
pub struct StudentsListProps {
    pub students: Vec<StudentProfile>,
}

impl PartialEq for StudentsListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn StudentsList(props: StudentsListProps) -> Element {
    let students = props.students.clone();

    rsx! {
        components::Card { class: "p-4 border-border/40 bg-sidebar/20",
            if students.is_empty() {
                div { class: "text-center p-12 text-muted-foreground",
                    components::LucideIcon { name: "directory", size: "40", class: "opacity-20 mb-2 mx-auto" }
                    p { class: "text-sm font-semibold m-0", "No enrolled students found. Enroll a student to get started." }
                }
            } else {
                table { class: "yntra-table w-full text-sm",
                    thead {
                        tr {
                            th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "First Name" }
                            th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "Last Name" }
                            th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "Grade Level" }
                            th { class: "text-left p-3 font-extrabold text-xs uppercase text-muted-foreground", "Parent Contact" }
                        }
                    }
                    tbody {
                        for s in students.iter() {
                            tr { class: "border-b border-border/40 hover:bg-white/[0.02]",
                                td { class: "p-3 font-bold", "{s.first_name}" }
                                td { class: "p-3 font-bold", "{s.last_name}" }
                                td { class: "p-3 text-muted-foreground", "{s.grade_level}" }
                                td { class: "p-3 text-muted-foreground", "{s.parent_contact.clone().unwrap_or_else(|| \"-\".to_string())}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
