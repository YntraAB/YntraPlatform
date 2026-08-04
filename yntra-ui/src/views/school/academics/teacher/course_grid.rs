use crate::components::LucideIcon;
use dioxus::prelude::*;
use yntra_core::Course;

#[component]
pub fn CourseGrid(
    courses: Vec<Course>,
    can_manage_schedule: bool,
    mut active_menu_id: Signal<String>,
    mut selected_course_id: Signal<String>,
    mut db_trigger: Signal<u32>,
    user_id: String,
) -> Element {
    rsx! {
        div { class: "space-y-6",
            if courses.is_empty() {
                div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                    LucideIcon { name: "library", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                    h4 { class: "text-sm font-bold text-foreground m-0", "No Classrooms Registered" }
                    p { class: "text-xs text-muted-foreground mt-1 max-w-sm", "Click the options menu next to the view tabs and select 'Create Course' to start setting up your digital classrooms." }
                }
            } else {
                div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6",
                    for c in courses.iter() {
                        {
                            let c_id_dropdown = c.id.clone();
                            let subject_lower = c.subject.trim().to_lowercase();
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
                                div {
                                    key: "{c.id}",
                                    class: "relative group flex flex-col rounded-2xl overflow-visible border border-border/85 hover:border-primary/45 transition-all hover:shadow-lg duration-200 bg-background cursor-pointer",
                                    onclick: {
                                        let c_id_select = c.id.clone();
                                        move |_| {
                                            selected_course_id.set(c_id_select.clone());
                                        }
                                    },

                                    // Colorful header banner
                                    div { class: "h-28 bg-gradient-to-br {gradient_class} p-4 text-white relative flex flex-col justify-between shadow-inner rounded-t-2xl",
                                        div { class: "flex items-start justify-between w-full",
                                            div { class: "space-y-0.5 max-w-[80%]",
                                                h3 { class: "font-extrabold text-sm tracking-tight m-0 text-white truncate", "{c.name}" }
                                                span { class: "text-[9px] font-bold text-white/90 uppercase tracking-wider", "{c.subject}" }
                                            }
                                            if can_manage_schedule {
                                                button {
                                                    class: "p-1.5 rounded-full hover:bg-white/20 text-white/80 hover:text-white border-0 bg-transparent cursor-pointer transition-colors z-20",
                                                    r#type: "button",
                                                    onclick: {
                                                        let c_id_d = c_id_dropdown.clone();
                                                        move |e| {
                                                            e.stop_propagation();
                                                            if *active_menu_id.read() == c_id_d {
                                                                active_menu_id.set("".to_string());
                                                            } else {
                                                                active_menu_id.set(c_id_d.clone());
                                                            }
                                                        }
                                                    },
                                                    LucideIcon { name: "settings", class: "h-4 w-4" }
                                                }
                                            }
                                        }

                                        if let Some(ref room) = c.classroom {
                                            span { class: "text-[10px] text-white/80 font-semibold", "Room: {room}" }
                                        }

                                        // settings menu dropdown
                                        if can_manage_schedule && *active_menu_id.read() == c_id_dropdown {
                                            div { class: "absolute top-11 right-3 z-30 bg-popover border border-border rounded-xl shadow-2xl p-1 min-w-[130px] animate-in fade-in slide-in-from-top-2 duration-150",
                                                button {
                                                    class: "flex w-full items-center gap-1.5 text-left px-2.5 py-1.5 text-xs font-semibold text-red-500 hover:bg-red-500/10 border-0 bg-transparent rounded-lg cursor-pointer transition-colors",
                                                    r#type: "button",
                                                    onclick: {
                                                        let uid = user_id.clone();
                                                        let cid = c.id.clone();
                                                        let mut db_trigger = db_trigger.clone();
                                                        move |e| {
                                                            e.stop_propagation();
                                                            let uid_c = uid.clone();
                                                            let cid_c = cid.clone();
                                                            spawn(async move {
                                                                let _ = yntra_core::delete_course(uid_c, cid_c).await;
                                                            });
                                                            active_menu_id.set("".to_string());
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    },
                                                    LucideIcon { name: "trash", class: "h-3.5 w-3.5 text-red-500" }
                                                    "Delete Course"
                                                }
                                            }
                                        }
                                    }

                                    // Card content (Roster info)
                                    div { class: "p-4 flex flex-col justify-between flex-1 bg-card h-20 border-x border-b border-border/60 rounded-b-2xl",
                                        div { class: "text-[11px] text-muted-foreground flex items-center gap-1.5",
                                            LucideIcon { name: "user", class: "h-3.5 w-3.5" }
                                            "{c.teacher_id.clone().unwrap_or_default()}"
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
