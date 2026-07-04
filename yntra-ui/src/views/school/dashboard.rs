use dioxus::prelude::*;
use crate::components;
use yntra_core::{StudentProfile, Course};

#[derive(Props, Clone)]
pub struct TeacherDashboardProps {
    pub students: Vec<StudentProfile>,
    pub courses: Vec<Course>,
    pub active_tab: Signal<String>,
}

impl PartialEq for TeacherDashboardProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TeacherDashboard(props: TeacherDashboardProps) -> Element {
    let students_len = props.students.len();
    let courses_len = props.courses.len();
    let mut active_tab = props.active_tab;

    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "grid gap-6 md:grid-cols-4",
                components::Card { class: "p-4 flex items-center gap-4 bg-sidebar/40 border-border/40",
                    div { class: "p-3 rounded-lg bg-primary/10 text-primary",
                        components::LucideIcon { name: "directory", size: "24" }
                    }
                    div {
                        div { class: "text-xs font-bold text-muted-foreground uppercase", "Total Students" }
                        div { class: "text-2xl font-black text-foreground mt-0.5", "{students_len}" }
                    }
                }
                components::Card { class: "p-4 flex items-center gap-4 bg-sidebar/40 border-border/40",
                    div { class: "p-3 rounded-lg bg-emerald-500/10 text-emerald-400",
                        components::LucideIcon { name: "book-open", size: "24" }
                    }
                    div {
                        div { class: "text-xs font-bold text-muted-foreground uppercase", "Active Courses" }
                        div { class: "text-2xl font-black text-foreground mt-0.5", "{courses_len}" }
                    }
                }
                components::Card { class: "p-4 flex items-center gap-4 bg-sidebar/40 border-border/40",
                    div { class: "p-3 rounded-lg bg-indigo-500/10 text-indigo-400",
                        components::LucideIcon { name: "time", size: "24" }
                    }
                    div {
                        div { class: "text-xs font-bold text-muted-foreground uppercase", "Attendance Logged" }
                        div { class: "text-2xl font-black text-foreground mt-0.5", "100%" }
                    }
                }
                components::Card { class: "p-4 flex items-center gap-4 bg-sidebar/40 border-border/40",
                    div { class: "p-3 rounded-lg bg-rose-500/10 text-rose-400",
                        components::LucideIcon { name: "reporting", size: "24" }
                    }
                    div {
                        div { class: "text-xs font-bold text-muted-foreground uppercase", "Needs Grading" }
                        div { class: "text-2xl font-black text-foreground mt-0.5", "1" }
                    }
                }
            }

            components::Card { class: "p-6 border border-border bg-sidebar shadow-md rounded-xl flex flex-col gap-3",
                h3 { class: "text-lg font-bold m-0 text-foreground", "School Preset Active" }
                p { class: "text-sm text-muted-foreground m-0 leading-relaxed",
                    "You are currently managing educational workflows for this workspace. Use the tabs above to manage classroom attendance sheets, edit courses, publish assignments, and award student grades."
                }
                div { class: "flex gap-2.5 mt-2",
                    button { class: "yntra-btn text-xs font-bold", onclick: move |_| active_tab.set("courses".to_string()), "Manage Courses" }
                    button { class: "yntra-btn secondary text-xs font-bold", onclick: move |_| active_tab.set("attendance".to_string()), "Log Attendance" }
                }
            }
        }
    }
}
