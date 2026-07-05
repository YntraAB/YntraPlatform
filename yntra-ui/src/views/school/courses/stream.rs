use dioxus::prelude::*;
use yntra_core::Assignment;
use crate::components;

#[derive(Props, Clone)]
pub struct CourseStreamProps {
    pub assignments: Vec<Assignment>,
    pub course_tab: Signal<String>,
    pub announcements: Signal<Vec<(String, String, String)>>,
    pub new_announcement: Signal<String>,
    pub is_authorized_to_post: bool,
}

impl PartialEq for CourseStreamProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CourseStream(props: CourseStreamProps) -> Element {
    let assignments = props.assignments;
    let mut course_tab = props.course_tab;
    let mut announcements = props.announcements;
    let mut new_announcement = props.new_announcement;
    let is_authorized_to_post = props.is_authorized_to_post;

    rsx! {
        div { class: "grid gap-6 md:grid-cols-4",
            // Left sidebar: Upcoming assignments list
            div { class: "md:col-span-1 flex flex-col gap-4",
                components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                    h4 { class: "text-xs font-black uppercase text-muted-foreground/80 m-0", "Upcoming Tasks" }
                    if assignments.is_empty() {
                        p { class: "text-xs text-muted-foreground m-0", "Woohoo, no work due soon!" }
                    } else {
                        div { class: "flex flex-col gap-2.5",
                            for a in assignments.iter().take(2) {
                                div { class: "text-xs flex flex-col gap-0.5",
                                    span { class: "font-semibold text-foreground truncate", "{a.title}" }
                                    span { class: "text-[10px] text-muted-foreground", "Due: {a.due_date}" }
                                }
                            }
                            button {
                                class: "text-[10px] font-bold text-primary hover:underline p-0 bg-transparent text-left mt-1",
                                onclick: move |_| course_tab.set("classwork".to_string()),
                                "View all assignments"
                            }
                        }
                    }
                }
            }
            
            // Right side: Announcements post and list
            div { class: "md:col-span-3 flex flex-col gap-4",
                // Create announcement card (only visible if allowed)
                if is_authorized_to_post {
                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                        div { class: "flex items-start gap-3",
                            div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                            textarea {
                                class: "yntra-input py-2.5 px-3 h-16 w-full text-xs text-foreground bg-background border border-border/40 rounded-lg placeholder-muted-foreground/60 resize-none",
                                placeholder: "Announce something to your class...",
                                value: "{new_announcement}",
                                oninput: move |e| new_announcement.set(e.value()),
                            }
                        }
                        div { class: "flex justify-end",
                            button {
                                class: "yntra-btn text-xs font-bold py-1.5 px-3",
                                disabled: new_announcement.read().trim().is_empty(),
                                onclick: move |_| {
                                    let text = new_announcement.read().trim().to_string();
                                    if !text.is_empty() {
                                        let mut list = announcements.read().clone();
                                        list.insert(0, ("Ms. Andersson".to_string(), text, "Just now".to_string()));
                                        announcements.set(list);
                                        new_announcement.set(String::new());
                                    }
                                },
                                "Post Announcement"
                            }
                        }
                    }
                }
                
                // Announcements Feed list
                for (author, content, time) in announcements.read().iter() {
                    components::Card { class: "p-4 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                        div { class: "flex justify-between items-center",
                            div { class: "flex items-center gap-3",
                                div { class: "w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary text-xs", "M" }
                                div {
                                    div { class: "text-xs font-bold text-foreground", "{author}" }
                                    div { class: "text-[10px] text-muted-foreground", "{time}" }
                                }
                            }
                        }
                        p { class: "text-xs text-muted-foreground/90 m-0 leading-relaxed", "{content}" }
                    }
                }
            }
        }
    }
}
