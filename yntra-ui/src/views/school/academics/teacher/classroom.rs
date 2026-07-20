use dioxus::prelude::*;
use crate::components::{Card, CardContent, LucideIcon};
use yntra_core::Course;

#[component]
pub fn ClassroomContainer(
    course: Course,
    mut selected_course_id: Signal<String>,
    mut sub_tab: Signal<String>,
    children: Element,
) -> Element {
    let subject_lower = course.subject.trim().to_lowercase();
    let gradient_class = if subject_lower == "matematik" || subject_lower == "mathematics" || subject_lower == "matematikk" || subject_lower == "matematiikka" {
        "from-blue-600 to-indigo-600"
    } else if subject_lower == "naturvetenskap" || subject_lower == "science" || subject_lower == "naturfag" || subject_lower == "luonnontiede" {
        "from-teal-600 to-emerald-600"
    } else if subject_lower == "bild" || subject_lower == "art" || subject_lower == "billedkunst" || subject_lower == "kuvataide" {
        "from-purple-600 to-pink-600"
    } else if subject_lower == "musik" || subject_lower == "music" || subject_lower == "musiikki" {
        "from-rose-500 to-red-600"
    } else if subject_lower == "engelska" || subject_lower == "english" || subject_lower == "engelsk" || subject_lower == "englanti" {
        "from-amber-500 to-orange-600"
    } else if subject_lower == "historia" || subject_lower == "history" || subject_lower == "historie" {
        "from-cyan-600 to-sky-600"
    } else {
        "from-gray-600 to-slate-700"
    };

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            // Back button
            button {
                class: "flex items-center gap-1.5 text-xs font-bold text-muted-foreground hover:text-foreground cursor-pointer border-0 bg-transparent pb-1 transition-colors",
                r#type: "button",
                onclick: move |_| selected_course_id.set("".to_string()),
                LucideIcon { name: "arrow-left", class: "h-4 w-4" }
                "Back to Classrooms"
            }

            // Gorgeous Classroom banner card
            div { class: "h-36 rounded-2xl bg-gradient-to-r {gradient_class} p-6 text-white flex flex-col justify-end shadow-md relative overflow-hidden shadow-inner",
                h2 { class: "text-2xl font-extrabold m-0 text-white tracking-tight", "{course.name}" }
                p { class: "text-xs font-bold text-white/90 m-0 mt-1.5 uppercase tracking-wider", 
                    "Subject: {course.subject} • Room: {course.classroom.clone().unwrap_or_default()} • Teacher: {course.teacher_id.clone().unwrap_or_default()}"
                }
            }

            // Horizontal Google Classroom tab selector
            Card { class: "border-border shadow-sm rounded-2xl overflow-hidden",
                CardContent { class: "p-6",
                    div { class: "flex border-b border-border pb-2.5 mb-6 gap-6 text-xs font-extrabold tracking-wider",
                        button {
                            class: format!(
                                "pb-2.5 transition-all border-b-2 {}",
                                if *sub_tab.read() == "stream" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| sub_tab.set("stream".to_string()),
                            "Stream"
                        }
                        button {
                            class: format!(
                                "pb-2.5 transition-all border-b-2 {}",
                                if *sub_tab.read() == "syllabus" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| sub_tab.set("syllabus".to_string()),
                            "Classwork"
                        }
                        button {
                            class: format!(
                                "pb-2.5 transition-all border-b-2 {}",
                                if *sub_tab.read() == "grading" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| sub_tab.set("grading".to_string()),
                            "Grades"
                        }
                        button {
                            class: format!(
                                "pb-2.5 transition-all border-b-2 {}",
                                if *sub_tab.read() == "submissions" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| sub_tab.set("submissions".to_string()),
                            "Submissions"
                        }
                    }

                    {children}
                }
            }
        }
    }
}
