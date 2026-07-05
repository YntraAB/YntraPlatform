use dioxus::prelude::*;
use crate::components;
use yntra_core::{StudentProfile, ReportCard, TermGrade, Course};

#[derive(Props, Clone)]
pub struct TranscriptsTabProps {
    pub student: StudentProfile,
    pub report_cards: Vec<ReportCard>,
    pub term_grades: Vec<TermGrade>,
    pub courses: Vec<Course>,
    pub selected_term: Signal<String>,
}

impl PartialEq for TranscriptsTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TranscriptsTab(props: TranscriptsTabProps) -> Element {
    let student = props.student;
    let report_cards = props.report_cards;
    let term_grades = props.term_grades;
    let courses = props.courses;
    let mut selected_term = props.selected_term;

    rsx! {
        div { class: "flex flex-col gap-6",
            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                div { class: "flex justify-between items-center flex-wrap gap-4",
                    h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "award", size: "18", class: "text-primary" }
                        "Official Term Report Cards"
                    }
                    
                    div { class: "flex items-center gap-2",
                        span { class: "text-xs font-bold text-muted-foreground", "Academic Period:" }
                        select {
                            class: "yntra-input text-xs bg-sidebar py-1.5 px-3 border border-border/60 rounded-lg",
                            value: "{selected_term}",
                            onchange: move |e| selected_term.set(e.value()),
                            option { value: "Fall 2026", "Fall 2026" }
                            option { value: "Spring 2027", "Spring 2027" }
                        }
                    }
                }
                
                {
                    let term = selected_term.read().clone();
                    let rc = report_cards.iter().find(|r| r.term_name == term && r.status == "published");
                    
                    if let Some(card) = rc {
                        rsx! {
                            div { class: "border border-yellow-500/20 p-6 rounded-2xl bg-gradient-to-br from-yellow-500/5 via-sidebar/20 to-primary/5 flex flex-col gap-5 relative overflow-hidden shadow-inner",
                                div { class: "absolute right-[-20px] top-[-20px] opacity-5 pointer-events-none",
                                    components::LucideIcon { name: "award", size: "120", class: "text-yellow-500" }
                                }
                                
                                div { class: "flex justify-between items-start border-b border-border/30 pb-4",
                                    div { class: "flex flex-col gap-1",
                                        span { class: "text-[10px] font-black uppercase text-yellow-500 tracking-widest", "Official Academic Record" }
                                        h4 { class: "text-lg font-black text-foreground m-0", "Yntra Academy Term Transcript" }
                                        span { class: "text-xs text-muted-foreground", "Student: {student.first_name} {student.last_name} | {student.grade_level}" }
                                    }
                                    
                                    div { class: "text-right flex flex-col items-end gap-1",
                                        span { class: "text-2xl font-black text-primary", "{card.gpa:.2}" }
                                        span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Calculated GPA" }
                                    }
                                }
                                
                                div { class: "flex flex-col gap-3",
                                    h5 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0", "Term Courses & Evaluations" }
                                    if term_grades.is_empty() {
                                        p { class: "text-xs text-muted-foreground italic m-0", "No final course grades loaded for this term period." }
                                    } else {
                                        div { class: "flex flex-col gap-2.5",
                                            for tg in term_grades.iter() {
                                                div { class: "flex flex-col md:flex-row md:items-center justify-between border border-border/20 p-3 rounded-xl bg-sidebar/30 gap-2",
                                                    div { class: "flex flex-col",
                                                        span { class: "text-sm font-bold text-foreground",
                                                            {
                                                                let course = courses.iter().find(|c| c.id == tg.course_id);
                                                                course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                                            }
                                                        }
                                                        if let Some(ref comment) = tg.teacher_comments {
                                                            span { class: "text-xs text-muted-foreground italic mt-0.5", "\"{comment}\"" }
                                                        }
                                                    }
                                                    
                                                    div { class: "flex items-center gap-3 self-end md:self-auto",
                                                        if let Some(ref points) = tg.final_points {
                                                            span { class: "text-xs font-semibold text-muted-foreground", "Points: {points}" }
                                                        }
                                                        span { class: "text-xs font-black px-2.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/15",
                                                            "{tg.final_grade.clone().unwrap_or_else(|| \"-\".to_string())}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                if let Some(ref comm) = card.principal_comments {
                                    div { class: "border-t border-border/30 pt-4 flex flex-col gap-1.5",
                                        span { class: "text-[10px] font-black uppercase text-muted-foreground tracking-wider", "Principal Advisory Remarks" }
                                        p { class: "text-xs text-foreground/80 m-0 italic bg-sidebar/40 p-3 rounded-lg border border-border/20",
                                            "\"{comm}\""
                                        }
                                    }
                                }
                                
                                div { class: "flex justify-end border-t border-border/30 pt-4 mt-1",
                                    button {
                                        class: "yntra-btn-secondary text-xs font-bold py-2 px-4 flex items-center gap-1.5 shadow-sm",
                                        onclick: move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                if let Some(w) = web_sys::window() {
                                                    let _ = w.print();
                                                }
                                            }
                                        },
                                        components::LucideIcon { name: "printer", size: "14" }
                                        "Print Official Record"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "p-8 text-center border border-dashed border-border/30 rounded-xl bg-white/[0.01] text-muted-foreground text-xs",
                                components::LucideIcon { name: "award", size: "32", class: "mx-auto opacity-30 mb-2" }
                                "No published report card is available for {term}."
                            }
                        }
                    }
                }
            }
        }
    }
}
