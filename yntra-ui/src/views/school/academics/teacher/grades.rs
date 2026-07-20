use dioxus::prelude::*;
use crate::components::LucideIcon;
use yntra_core::{Assignment, StudentProfile, Submission, TermGrade};

#[component]
pub fn CourseGradesTab(
    students: Vec<StudentProfile>,
    assignments: Vec<Assignment>,
    all_submissions: Vec<Submission>,
    course_grades: Vec<TermGrade>,
    mut grade_student_id: Signal<String>,
    mut grade_term: Signal<String>,
    mut grade_letter: Signal<String>,
    mut grade_points: Signal<i32>,
    mut grade_comments: Signal<String>,
    mut show_grade_modal: Signal<bool>,
    mut selected_homework_sub: Signal<Option<Submission>>,
    mut homework_grade: Signal<String>,
    mut homework_feedback: Signal<String>,
    mut show_homework_grade_modal: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "space-y-4",
            h5 { class: "font-bold text-sm m-0 text-foreground", "Student Grades Matrix" }
            
            if students.is_empty() {
                div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No students registered in this course." }
            } else {
                div { class: "overflow-x-auto border border-border rounded-xl bg-background shadow-sm",
                    table { class: "w-full border-collapse text-left text-xs",
                        thead { class: "bg-muted/30 border-b border-border text-[10px] font-extrabold text-muted-foreground uppercase tracking-wider",
                            tr {
                                th { class: "p-3.5", "Student Name" }
                                for a in assignments.iter() {
                                    th { key: "{a.id}", class: "p-3.5 text-center min-w-[80px] truncate max-w-[120px]", "{a.title}" }
                                }
                                th { class: "p-3.5 text-center min-w-[100px]", "Final Term Grade" }
                            }
                        }
                        tbody { class: "divide-y divide-border/60",
                            for s in students.iter() {
                                {
                                    let student_id = s.id.clone();
                                    let student_name = format!("{} {}", s.first_name, s.last_name);
                                    let current_grade = course_grades.iter().find(|g| g.student_id == student_id).cloned();
                                    rsx! {
                                        tr { key: "{s.id}", class: "hover:bg-muted/10 transition-colors font-medium text-foreground/90",
                                            td { class: "p-3.5 align-middle",
                                                div { class: "flex items-center gap-2",
                                                    div { class: "h-7 w-7 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-[10px]",
                                                        "{s.first_name.chars().next().unwrap_or('?')}"
                                                    }
                                                    div {
                                                        div { class: "font-bold text-foreground", "{student_name}" }
                                                        div { class: "text-[9px] text-muted-foreground", "Grade: {s.grade_level}" }
                                                    }
                                                }
                                            }
                                            for a in assignments.iter() {
                                                {
                                                    let a_id = a.id.clone();
                                                    let sub_opt = all_submissions.iter().find(|sub| sub.assignment_id == a_id && sub.student_id == student_id).cloned();
                                                    rsx! {
                                                        td { key: "{a.id}", class: "p-3.5 text-center align-middle",
                                                            if let Some(sub_rec) = sub_opt {
                                                                {
                                                                    let is_graded = sub_rec.grade.is_some();
                                                                    let grade_val = sub_rec.grade.clone().unwrap_or_default();
                                                                    let sub_c = sub_rec.clone();
                                                                    rsx! {
                                                                        button {
                                                                            class: format!(
                                                                                "px-2.5 py-1 rounded-lg border font-bold text-[9px] cursor-pointer transition-all hover:scale-105 {}",
                                                                                if is_graded { "bg-emerald-500/10 text-emerald-600 border-emerald-500/20" } else { "bg-amber-500/10 text-amber-600 border-amber-500/20" }
                                                                            ),
                                                                            r#type: "button",
                                                                            onclick: move |_| {
                                                                                selected_homework_sub.set(Some(sub_c.clone()));
                                                                                homework_grade.set(sub_c.grade.clone().unwrap_or_else(|| "A".to_string()));
                                                                                homework_feedback.set(sub_c.feedback.clone().unwrap_or_default());
                                                                                show_homework_grade_modal.set(true);
                                                                            },
                                                                            if is_graded { "{grade_val}" } else { "PENDING" }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                span { class: "text-muted-foreground/40 font-medium text-[10px]", "-" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            td { class: "p-3.5 text-center align-middle",
                                                div { class: "flex items-center justify-center gap-2",
                                                    if let Some(ref g) = current_grade {
                                                        span { class: "font-bold px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-[10px]",
                                                            "{g.final_grade.clone().unwrap_or_else(|| \"-\".to_string())}"
                                                        }
                                                    } else {
                                                        span { class: "text-muted-foreground/30 italic text-[10px]", "-" }
                                                    }
                                                    button {
                                                        class: "p-1 rounded bg-muted/40 hover:bg-muted text-foreground border-0 cursor-pointer transition-colors",
                                                        r#type: "button",
                                                        onclick: {
                                                            let sid = s.id.clone();
                                                            let current_grade_c = current_grade.clone();
                                                            move |_| {
                                                                grade_student_id.set(sid.clone());
                                                                if let Some(ref g) = current_grade_c {
                                                                    grade_letter.set(g.final_grade.clone().unwrap_or_else(|| "A".to_string()));
                                                                    grade_points.set(g.final_points.unwrap_or(90));
                                                                    grade_comments.set(g.teacher_comments.clone().unwrap_or_default());
                                                                } else {
                                                                    grade_letter.set("A".to_string());
                                                                    grade_points.set(90);
                                                                    grade_comments.set(String::new());
                                                                }
                                                                show_grade_modal.set(true);
                                                            }
                                                        },
                                                        LucideIcon { name: "edit-2", size: "11" }
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
