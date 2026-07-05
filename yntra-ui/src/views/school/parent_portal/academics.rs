use dioxus::prelude::*;
use crate::components;
use yntra_core::{Course, Submission, Assignment, TimetableSlot};

#[derive(Props, Clone)]
pub struct AcademicsTabProps {
    pub courses: Vec<Course>,
    pub child_subs: Vec<Submission>,
    pub all_assignments: Vec<Assignment>,
    pub timetable_slots: Vec<TimetableSlot>,
    pub locale: String,
}

impl PartialEq for AcademicsTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AcademicsTab(props: AcademicsTabProps) -> Element {
    let courses = props.courses;
    let child_subs = props.child_subs;
    let all_assignments = props.all_assignments;
    let timetable_slots = props.timetable_slots;
    let locale = props.locale;

    rsx! {
        div { class: "grid gap-6 md:grid-cols-3 items-start",
            // Left 2 columns: Assignment grades
            div { class: "md:col-span-2 flex flex-col gap-4",
                components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                    h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "book-open", size: "18", class: "text-primary" }
                        "Assignment Grades & Teacher Feedback"
                    }
                    
                    if child_subs.is_empty() {
                        div { class: "text-center p-8 text-muted-foreground text-sm",
                            "No assignments have been completed yet."
                        }
                    } else {
                        div { class: "flex flex-col gap-4",
                            for s in child_subs.iter() {
                                div { class: "border border-border/40 p-4 rounded-xl flex flex-col gap-3 bg-white/[0.01]",
                                    div { class: "flex justify-between items-start",
                                        div {
                                            div { class: "font-bold text-foreground text-sm",
                                                {
                                                    let assignment = all_assignments.iter().find(|a| a.id == s.assignment_id);
                                                    assignment.map(|a| a.title.clone()).unwrap_or_else(|| "Assignment".to_string())
                                                }
                                            }
                                            div { class: "text-xs text-muted-foreground mt-0.5", "Submitted on: {s.submitted_at}" }
                                        }
                                        
                                        div { class: "flex items-center gap-2",
                                            if let Some(ref g) = s.grade {
                                                span { class: "text-xs font-black px-2.5 py-1 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Grade: {g}" }
                                            } else {
                                                span { class: "text-xs font-black px-2.5 py-1 rounded bg-amber-500/10 text-amber-400 border border-amber-500/15 animate-pulse", "Awaiting Review" }
                                            }
                                        }
                                    }

                                    // Student Submission
                                    div { class: "text-xs bg-sidebar/50 p-2.5 rounded border border-border/20 text-muted-foreground font-medium",
                                        div { class: "font-bold text-foreground/80 mb-1", "Submitted Answer:" }
                                        "{s.content}"
                                    }

                                    // Teacher Feedback
                                    if let Some(ref fb) = s.feedback {
                                        div {
                                            class: "relative p-3 rounded-lg border border-yellow-500/20 bg-yellow-500/5 text-xs text-amber-200 flex gap-2.5 items-start",
                                            components::LucideIcon { name: "pen-tool", size: "16", class: "text-amber-400 mt-0.5" }
                                            div {
                                                div { class: "font-black text-amber-400/90 mb-0.5", "Teacher Feedback Note:" }
                                                "\"{fb}\""
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right 1 column: Student Schedule
            div { class: "md:col-span-1 flex flex-col gap-4",
                components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                        "Student Class Schedule"
                    }
                    
                    if timetable_slots.is_empty() {
                        p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", {crate::locales::t("school-timetable-no-slots", &locale)} }
                    } else {
                        div { class: "flex flex-col gap-3.5",
                            for s in timetable_slots.iter().take(6) {
                                div { class: "flex items-start gap-3 border-l-4 border-primary pl-3 py-0.5",
                                    div { class: "text-xs font-bold text-muted-foreground w-12", "{s.start_time}" }
                                    div { class: "flex flex-col gap-0.5",
                                        div { class: "text-sm font-extrabold text-foreground flex items-center gap-1.5",
                                            {
                                                let course = courses.iter().find(|c| c.id == s.course_id);
                                                course.map(|c| c.name.clone()).unwrap_or_else(|| "General Course".to_string())
                                            }
                                        }
                                        div { class: "text-xs text-muted-foreground", 
                                            "Room: {s.classroom.clone().unwrap_or_else(|| \"TBD\".to_string())} • {s.start_time} - {s.end_time}"
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
