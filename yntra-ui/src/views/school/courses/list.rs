use dioxus::prelude::*;
use yntra_core::Course;
use crate::components;
use super::get_subject_theme;

#[derive(Props, Clone)]
pub struct CourseListProps {
    pub filtered_courses: Vec<Course>,
    pub selected_course_id: Signal<Option<String>>,
    pub selected_assignment_id: Signal<Option<String>>,
    pub course_tab: Signal<String>,
    pub search_query: Signal<String>,
    pub selected_subject: Signal<String>,
}

impl PartialEq for CourseListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CourseList(props: CourseListProps) -> Element {
    let filtered_courses = props.filtered_courses;
    let mut selected_course_id = props.selected_course_id;
    let mut selected_assignment_id = props.selected_assignment_id;
    let mut course_tab = props.course_tab;
    let mut search_query = props.search_query;
    let mut selected_subject = props.selected_subject;

    rsx! {
        div { class: "flex flex-col gap-6",
            // Search and Filters Bar
            div { class: "flex flex-col md:flex-row gap-4 justify-between items-center bg-sidebar/30 border border-border/40 p-4 rounded-xl shadow-sm backdrop-blur-md",
                div { class: "relative w-full md:max-w-xs",
                    components::LucideIcon { name: "search", size: "16", class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground opacity-60" }
                    input {
                        class: "yntra-input pr-4 h-10 w-full rounded-lg bg-background border border-border/40 text-xs focus:ring-1 focus:ring-primary",
                        style: "padding-left: 2.5rem;",
                        placeholder: "Search courses...",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                    }
                }
                
                // Subject Filter Pills
                div { class: "flex gap-2 self-start md:self-auto overflow-x-auto max-w-full pb-1 md:pb-0",
                    for (slug, label) in &[("all", "All"), ("mathematics", "Math"), ("science", "Science"), ("languages", "Languages"), ("history", "History")] {
                        button {
                            class: format!("px-3.5 py-1.5 rounded-lg text-xs font-bold transition-all border bg-transparent cursor-pointer {}", 
                                if *selected_subject.read() == *slug { "bg-primary text-primary-foreground border-primary shadow" } else { "bg-sidebar/50 text-muted-foreground border-border/30 hover:text-foreground hover:bg-sidebar" }
                            ),
                            onclick: move |_| selected_subject.set(slug.to_string()),
                            "{label}"
                        }
                    }
                }
            }

            if filtered_courses.is_empty() {
                div { class: "text-center p-12 text-muted-foreground border border-dashed border-border/40 rounded-xl",
                    components::LucideIcon { name: "book-open", size: "40", class: "opacity-20 mb-2 mx-auto" }
                    p { class: "text-sm font-semibold m-0", "No courses found." }
                }
            } else {
                div { class: "grid gap-6 md:grid-cols-3",
                    for c in filtered_courses.iter() {
                        {
                            let (gradient, _badge_style, icon) = get_subject_theme(&c.subject);
                            rsx! {
                                div {
                                    class: "group relative overflow-hidden rounded-xl border border-border/40 bg-sidebar/20 hover:border-primary/40 hover:shadow-2xl hover:-translate-y-1 transition-all duration-300 flex flex-col justify-between h-48 cursor-pointer shadow-md",
                                    onclick: {
                                        let c_id = c.id.clone();
                                        move |_| {
                                            selected_course_id.set(Some(c_id.clone()));
                                            selected_assignment_id.set(None);
                                            course_tab.set("stream".to_string());
                                        }
                                    },
                                    // Header area with gradient
                                    div { class: format!("p-4 bg-gradient-to-r {} relative text-white flex flex-col justify-between h-28 overflow-hidden", gradient),
                                        // Subtle background icon overlay
                                        div { class: "absolute right-2 -bottom-4 text-white/10 select-none pointer-events-none scale-150 rotate-12",
                                            components::LucideIcon { name: icon, size: "80" }
                                        }
                                        div { class: "flex justify-between items-start z-10",
                                            span { class: "text-[10px] font-black uppercase tracking-widest px-2 py-0.5 bg-white/20 backdrop-blur-md rounded-md", "{c.subject}" }
                                            if let Some(ref room) = c.classroom {
                                                span { class: "text-xs font-semibold flex items-center gap-1 opacity-90",
                                                    components::LucideIcon { name: "home", size: "12" }
                                                    "{room}"
                                                }
                                            }
                                        }
                                        h3 { class: "text-lg font-black tracking-tight m-0 mb-1 z-10 drop-shadow-sm truncate text-white", "{c.name}" }
                                    }
                                    
                                    // Footer area
                                    div { class: "p-4 bg-sidebar/10 flex justify-between items-center text-xs text-muted-foreground border-t border-border/10",
                                        div { class: "flex items-center gap-1.5",
                                            div { class: "w-5 h-5 rounded-full bg-primary/25 flex items-center justify-center text-[10px] font-bold text-primary", "M" }
                                            span { "Ms. Andersson" }
                                        }
                                        span { class: "text-primary font-bold flex items-center gap-1 group-hover:translate-x-1 transition-all",
                                            "Enter Class"
                                            components::LucideIcon { name: "arrow-right", size: "12" }
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
