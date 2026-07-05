use dioxus::prelude::*;
use yntra_core::{StudentProfile, Assignment, Submission};
use crate::components;

#[derive(Props, Clone)]
pub struct CourseGradebookProps {
    pub students: Vec<StudentProfile>,
    pub assignments: Vec<Assignment>,
    pub submissions: Vec<Submission>,
}

impl PartialEq for CourseGradebookProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CourseGradebook(props: CourseGradebookProps) -> Element {
    let students = props.students;
    let assignments = props.assignments;
    let submissions = props.submissions;

    rsx! {
        components::Card { class: "p-5 border border-border/40 bg-sidebar/20 flex flex-col gap-4 overflow-hidden",
            h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0", "Class Gradebook" }
            
            if students.is_empty() || assignments.is_empty() {
                div { class: "text-center p-8 text-muted-foreground text-xs", "Enrolled students and assignments required to display Gradebook." }
            } else {
                div { class: "overflow-x-auto w-full border border-border/40 rounded-xl",
                    table { class: "w-full border-collapse text-xs text-left",
                        thead { class: "bg-sidebar border-b border-border/40",
                            tr {
                                th { class: "p-3 font-bold text-muted-foreground min-w-[150px] border-r border-border/40", "Student" }
                                for a in assignments.iter() {
                                    th { class: "p-3 font-bold text-muted-foreground text-center min-w-[120px] border-r border-border/40",
                                        div { class: "truncate max-w-[120px]", "{a.title}" }
                                        div { class: "text-[9px] font-normal text-primary/70 mt-0.5", "{a.max_points} pts" }
                                    }
                                }
                            }
                        }
                        tbody {
                            for s in students.iter() {
                                tr { class: "border-b border-border/40 hover:bg-white/[0.01]",
                                    td { class: "p-3 font-bold text-foreground border-r border-border/40", "{s.first_name} {s.last_name}" }
                                    for a in assignments.iter() {
                                        {
                                            let submission = submissions.iter().find(|sub| sub.assignment_id == a.id && sub.student_id == s.id);
                                            rsx! {
                                                td { class: "p-3 text-center border-r border-border/40",
                                                    if let Some(sub) = submission {
                                                        if let Some(g) = &sub.grade {
                                                            span { class: "font-extrabold text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20", "{g}" }
                                                        } else {
                                                            span { class: "text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20 text-[10px] font-bold", "Ungraded" }
                                                        }
                                                    } else {
                                                        span { class: "text-muted-foreground opacity-40", "-" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
