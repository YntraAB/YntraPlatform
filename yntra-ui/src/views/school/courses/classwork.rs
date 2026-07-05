use dioxus::prelude::*;
use yntra_core::{Course, Assignment, Submission, StudentProfile};
use crate::components;

#[derive(Props, Clone)]
pub struct CourseClassworkProps {
    pub course: Course,
    pub assignments: Vec<Assignment>,
    pub submissions: Vec<Submission>,
    pub students: Vec<StudentProfile>,
    pub selected_assignment: Option<Assignment>,
    pub selected_assignment_id: Signal<Option<String>>,
    pub selected_submission_id: Signal<Option<String>>,
    pub grade_input: Signal<String>,
    pub feedback_input: Signal<String>,
    pub show_assignment_modal: Signal<bool>,
    pub show_grading_modal: Signal<bool>,
}

impl PartialEq for CourseClassworkProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CourseClasswork(props: CourseClassworkProps) -> Element {
    let course = props.course;
    let assignments = props.assignments;
    let submissions = props.submissions;
    let students = props.students;
    let selected_assignment = props.selected_assignment;
    let mut selected_assignment_id = props.selected_assignment_id;
    let mut selected_submission_id = props.selected_submission_id;
    let mut grade_input = props.grade_input;
    let mut feedback_input = props.feedback_input;
    let mut show_assignment_modal = props.show_assignment_modal;
    let mut show_grading_modal = props.show_grading_modal;

    rsx! {
        div { class: "grid gap-6 md:grid-cols-3",
            // Left sidebar: posting details & create button
            div { class: "flex flex-col gap-4 md:col-span-1",
                components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                    h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Classroom Details" }
                    div { class: "flex flex-col gap-2.5 text-sm",
                        div {
                            div { class: "text-xs text-muted-foreground", "Classroom" }
                            div { class: "font-bold text-foreground", "{course.classroom.clone().unwrap_or_else(|| \"N/A\".to_string())}" }
                        }
                        div {
                            div { class: "text-xs text-muted-foreground", "Assigned Teacher ID" }
                            div { class: "font-mono text-xs text-muted-foreground", "{course.teacher_id.clone().unwrap_or_else(|| \"-\".to_string())}" }
                        }
                    }
                }
                
                button {
                    class: "yntra-btn text-xs font-bold flex items-center justify-center gap-1.5 py-2.5",
                    onclick: move |_| show_assignment_modal.set(true),
                    components::LucideIcon { name: "plus", size: "14" }
                    "Post Assignment"
                }
            }
            
            // Right sidebar: assignments list & submissions
            div { class: "flex flex-col gap-4 md:col-span-2",
                components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                    h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Assignments" }
                    
                    if assignments.is_empty() {
                        div { class: "text-center p-8 text-muted-foreground text-sm", "No assignments posted yet." }
                    } else {
                        div { class: "flex flex-col gap-2",
                            for a in assignments.iter() {
                                div {
                                    class: format!("border p-3 rounded-lg flex justify-between items-center hover:border-primary/50 transition-all cursor-pointer bg-white/[0.01] {}",
                                        if Some(a.id.clone()) == *selected_assignment_id.read() { "border-primary/60 bg-primary/[0.02]" } else { "border-border/40" }
                                    ),
                                    onclick: {
                                        let a_id = a.id.clone();
                                        move |_| {
                                            selected_assignment_id.set(Some(a_id.clone()));
                                        }
                                    },
                                    div { class: "flex flex-col gap-0.5",
                                        div { class: "font-bold text-foreground", "{a.title}" }
                                        div { class: "text-xs text-muted-foreground", "{a.description}" }
                                    }
                                    div { class: "text-right flex flex-col gap-1 items-end",
                                        span { class: "text-xs font-bold text-primary", "{a.max_points} Points" }
                                        span { class: "text-[10px] text-muted-foreground", "Due: {a.due_date}" }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Submissions block if assignment selected
                if let Some(ref assignment) = selected_assignment {
                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3 mt-2",
                        h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Submissions for: {assignment.title}" }
                        
                        if submissions.is_empty() {
                            div { class: "text-center p-8 text-muted-foreground text-sm", "No submissions for this assignment yet." }
                        } else {
                            div { class: "flex flex-col gap-2.5",
                                for s in submissions.iter() {
                                    div { class: "border border-border/40 p-4 rounded-xl flex flex-col gap-3 bg-white/[0.01] hover:border-border transition-all",
                                        div { class: "flex justify-between items-start",
                                            div {
                                                div { class: "font-bold text-foreground text-xs",
                                                    {
                                                        let student = students.iter().find(|st| st.id == s.student_id);
                                                        student.map(|st| format!("{} {}", st.first_name, st.last_name)).unwrap_or_else(|| "Unknown Student".to_string())
                                                    }
                                                }
                                                div { class: "text-[10px] text-muted-foreground mt-0.5", "Submitted at: {s.submitted_at}" }
                                            }
                                            div { class: "flex gap-2 items-center",
                                                if let Some(ref g) = s.grade {
                                                    span { class: "text-xs font-extrabold px-2.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20", "Grade: {g}" }
                                                } else {
                                                    span { class: "text-xs font-extrabold px-2.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20", "Ungraded" }
                                                }
                                                button {
                                                    class: "yntra-btn text-[10px] font-bold py-1 px-2.5",
                                                    onclick: {
                                                        let sub_id = s.id.clone();
                                                        let curr_grade = s.grade.clone().unwrap_or_else(|| "A".to_string());
                                                        let curr_feedback = s.feedback.clone().unwrap_or_default();
                                                        move |_| {
                                                            selected_submission_id.set(Some(sub_id.clone()));
                                                            grade_input.set(curr_grade.clone());
                                                            feedback_input.set(curr_feedback.clone());
                                                            show_grading_modal.set(true);
                                                        }
                                                    },
                                                    "Grade"
                                                }
                                            }
                                        }
                                        
                                        div { class: "text-xs bg-sidebar/50 p-3 rounded-lg border border-border/20 text-muted-foreground",
                                            div { class: "font-bold text-foreground/80 mb-1", "Submission Text:" }
                                            "{s.content}"
                                        }
                                        
                                        if let Some(ref fb) = s.feedback {
                                            div { class: "text-xs bg-primary/5 p-2.5 rounded-lg text-primary border border-primary/10",
                                                strong { "Feedback: " }
                                                "{fb}"
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
