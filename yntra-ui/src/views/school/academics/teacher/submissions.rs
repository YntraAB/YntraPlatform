use super::super::utils::parse_submission_content_and_advanced_attachment;
use crate::components::{Button, LucideIcon};
use dioxus::prelude::*;
use yntra_core::{Assignment, StudentProfile, Submission};

#[component]
pub fn CourseSubmissionsTab(
    assignments: Vec<Assignment>,
    all_submissions: Vec<Submission>,
    students: Vec<StudentProfile>,
    mut selected_homework_sub: Signal<Option<Submission>>,
    mut homework_grade: Signal<String>,
    mut homework_feedback: Signal<String>,
    mut show_homework_grade_modal: Signal<bool>,
) -> Element {
    let course_assignment_ids: std::collections::HashSet<String> =
        assignments.iter().map(|a| a.id.clone()).collect();
    let course_submissions = all_submissions
        .iter()
        .filter(|sub| course_assignment_ids.contains(&sub.assignment_id))
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div { class: "space-y-4",
            h5 { class: "font-bold text-sm m-0 text-foreground", "Student Homework Submissions" }

            if course_submissions.is_empty() {
                div { class: "py-10 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl", "No homework submissions received yet for this course." }
            } else {
                div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                    for sub in course_submissions.into_iter() {
                        {
                            let student_name = students.iter()
                                .find(|s| s.id == sub.student_id)
                                .map(|s| format!("{} {}", s.first_name, s.last_name))
                                .unwrap_or_else(|| "Unknown Student".to_string());
                            let assignment_title = assignments.iter()
                                .find(|a| a.id == sub.assignment_id)
                                .map(|a| a.title.clone())
                                .unwrap_or_else(|| "Unknown Assignment".to_string());
                            let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&sub.content);
                            let is_graded = sub.grade.is_some();
                            let grade_str = sub.grade.clone().unwrap_or_default();
                            let sub_c = sub.clone();
                            rsx! {
                                div { key: "{sub.id}", class: "p-4 flex flex-col md:flex-row md:items-center justify-between gap-4 text-xs hover:bg-muted/5 transition-colors",
                                    div { class: "space-y-1.5 flex-1 min-w-0",
                                        div { class: "flex items-center gap-2 flex-wrap",
                                            span { class: "font-bold text-foreground text-sm", "{student_name}" }
                                            span { class: "text-[10px] text-muted-foreground", "for" }
                                            span { class: "font-semibold text-primary", "{assignment_title}" }
                                        }
                                        div { class: "text-muted-foreground break-words text-[11px] max-w-2xl bg-card/45 p-2.5 rounded-lg border border-border mt-1", "{desc_text}" }
                                        if let Some(staged) = attachment {
                                            a {
                                                href: "{staged.dataurl}",
                                                download: "{staged.filename}",
                                                class: "flex items-center gap-2 p-1.5 bg-card/45 border border-primary/20 hover:bg-card/75 rounded-lg no-underline text-foreground cursor-pointer transition-all mt-2 w-fit max-w-sm",
                                                LucideIcon { name: "file-text", class: "h-3.5 w-3.5 text-primary shrink-0" }
                                                span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                LucideIcon { name: "download", class: "h-3 w-3 text-muted-foreground ml-1.5 shrink-0" }
                                            }
                                        }
                                        div { class: "text-[10px] text-muted-foreground mt-1", "Submitted: {sub.submitted_at}" }
                                    }
                                    div { class: "text-slate-600 dark:text-slate-400 flex items-center gap-3.5 self-start md:self-center shrink-0",
                                        if is_graded {
                                            div { class: "flex flex-col items-end gap-1",
                                                span { class: "font-bold px-2.5 py-1 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-[10px] uppercase",
                                                    "Graded: {grade_str}"
                                                }
                                                if let Some(ref f) = sub.feedback {
                                                    span { class: "text-[10px] text-muted-foreground italic max-w-[180px] truncate", "\"{f}\"" }
                                                }
                                            }
                                        } else {
                                            span { class: "text-amber-600 font-bold bg-amber-500/10 border border-amber-500/20 px-2 py-0.5 rounded-full text-[10px] uppercase", "Pending Grade" }
                                        }
                                        Button {
                                            class: "px-3 py-1.5 text-xs h-8 bg-primary text-primary-foreground rounded-lg font-semibold",
                                            onclick: move |_| {
                                                selected_homework_sub.set(Some(sub_c.clone()));
                                                homework_grade.set(sub_c.grade.clone().unwrap_or_else(|| "A".to_string()));
                                                homework_feedback.set(sub_c.feedback.clone().unwrap_or_default());
                                                show_homework_grade_modal.set(true);
                                            },
                                            if is_graded { "Edit Grade" } else { "Grade Homework" }
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
