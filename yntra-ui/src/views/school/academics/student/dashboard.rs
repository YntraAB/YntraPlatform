use dioxus::prelude::*;
use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, LucideIcon};
use crate::locales::{t, t_with_args};
use yntra_core::{Course, StudentProfile, TimetableSlot};
use super::super::utils::base64_encode;

#[component]
pub fn StudentDashboardTab(
    student_name: String,
    locale: String,
    stars_count: usize,
    is_active_scholar: bool,
    is_avid_reader: bool,
    attendance_rate: f32,
    stroke_dasharray: f64,
    stroke_dashoffset: f64,
    filtered_timetable: Vec<TimetableSlot>,
    courses: Vec<Course>,
    students: Vec<StudentProfile>,
    selected_student_profile_id: String,
) -> Element {
    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            // Welcome Banner Card
            div { class: "p-6 rounded-2xl bg-gradient-to-r from-primary/10 to-accent/5 border border-primary/20 flex items-center justify-between shadow-sm",
                div { class: "space-y-1.5",
                    h3 { class: "text-lg font-extrabold text-foreground m-0", 
                        {t_with_args("school-welcome", &locale, &[("name", &student_name)])}
                    }
                    p { class: "text-xs text-muted-foreground m-0", {t("school-ready-msg", &locale)} }
                }
                LucideIcon { name: "award", class: "h-10 w-10 text-primary" }
            }

            // Stars & Badges & Attendance grid
            div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-my-stars", &locale)} }
                        CardDescription { {t("school-student-stars-desc", &locale)} }
                    }
                    CardContent { class: "flex flex-col items-center py-6 space-y-4",
                        div { class: "flex gap-2.5",
                            for i in 0..5 {
                                {
                                    let active = i < stars_count;
                                    rsx! {
                                        LucideIcon { 
                                            name: "star", 
                                            class: format!("h-10 w-10 {}", if active { "text-amber-500 fill-amber-500 animate-pulse" } else { "text-muted-foreground/10" }) 
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "text-xs font-bold text-foreground", {t_with_args("school-student-earned-stars", &locale, &[("count", &stars_count.to_string())])} }
                    }
                }

                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-my-badges", &locale)} }
                        CardDescription { {t("school-student-badges-desc", &locale)} }
                    }
                    CardContent { class: "flex flex-wrap gap-3 py-6 justify-center items-center",
                        if is_active_scholar {
                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-[10px] font-bold uppercase tracking-wider",
                                LucideIcon { name: "zap", class: "h-3.5 w-3.5" }
                                {t("school-badge-scholar", &locale)}
                            }
                        }
                        if is_avid_reader {
                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-purple-500/10 text-purple-600 border border-purple-500/20 text-[10px] font-bold uppercase tracking-wider",
                                LucideIcon { name: "book-open", class: "h-3.5 w-3.5" }
                                {t("school-badge-reader", &locale)}
                            }
                        }
                        if !is_active_scholar && !is_avid_reader {
                            div { class: "text-xs text-muted-foreground italic py-4", {t("school-badges-locked", &locale)} }
                        }
                    }
                }

                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-attendance-rate", &locale)} }
                        CardDescription { {t("school-student-attendance-desc", &locale)} }
                    }
                    CardContent { class: "flex items-center justify-center py-4 gap-6",
                        div { class: "relative h-20 w-20 flex items-center justify-center",
                            svg { class: "h-20 w-20 transform -rotate-90",
                                circle {
                                    class: "text-muted-foreground/10",
                                    stroke_width: "6",
                                    stroke: "currentColor",
                                    fill: "transparent",
                                    r: "30",
                                    cx: "40",
                                    cy: "40",
                                }
                                circle {
                                    class: "text-primary transition-all duration-500",
                                    stroke_width: "6",
                                    stroke_dasharray: "{stroke_dasharray}",
                                    stroke_dashoffset: "{stroke_dashoffset}",
                                    stroke_linecap: "round",
                                    stroke: "currentColor",
                                    fill: "transparent",
                                    r: "30",
                                    cx: "40",
                                    cy: "40",
                                }
                            }
                            span { class: "absolute text-xs font-extrabold text-foreground", "{attendance_rate as i32}%" }
                        }
                        div { class: "space-y-1 text-xs",
                            div { class: "font-bold text-foreground",
                                if attendance_rate >= 90.0 { {t("school-attendance-good", &locale)} } else { {t("school-attendance-warning", &locale)} }
                            }
                            div { class: "text-[10px] text-muted-foreground", {t("school-attendance-tip", &locale)} }
                        }
                    }
                }
            }

            // Timetable Card
            Card { class: "border border-border shadow-sm",
                CardHeader {
                    CardTitle { {t("school-my-timetable", &locale)} }
                    CardDescription { {t("school-student-timetable-desc", &locale)} }
                }
                CardContent {
                    if filtered_timetable.is_empty() {
                        div { class: "py-8 text-center text-xs text-muted-foreground", {t("school-timetable-no-slots", &locale)} }
                    } else {
                        div { class: "space-y-4",
                            div { class: "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4",
                                for s in filtered_timetable.iter() {
                                    {
                                        let course_name = courses.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                        let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                        let day_name = match s.day_of_week {
                                            1 => t("common-monday", &locale),
                                            2 => t("common-tuesday", &locale),
                                            3 => t("common-wednesday", &locale),
                                            4 => t("common-thursday", &locale),
                                            5 => t("common-friday", &locale),
                                            _ => "Weekday".to_string(),
                                        };
                                        rsx! {
                                            div { key: "{s.id}", class: "p-4 rounded-xl border border-border bg-background flex flex-col gap-2 shadow-sm hover:border-primary/20 transition-all",
                                                div { class: "flex items-center gap-2",
                                                    LucideIcon { name: "clock", class: "h-4 w-4 text-primary" }
                                                    span { class: "text-xs font-bold text-foreground", "{day_name} • {s.start_time} - {s.end_time}" }
                                                }
                                                div { class: "font-semibold text-sm text-foreground", "{course_name}" }
                                                div { class: "text-[10px] text-muted-foreground flex items-center gap-1",
                                                    LucideIcon { name: "map-pin", class: "h-3 w-3" }
                                                    span { {t_with_args("school-student-classroom-label", &locale, &[("room", &classroom_name)])} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Button {
                                class: "mt-2 bg-primary hover:bg-primary/90 text-primary-foreground font-bold uppercase tracking-wider text-[9px] py-2 px-4 rounded-lg flex items-center justify-center gap-1.5 transition-all shadow-sm border-0 cursor-pointer w-full sm:w-auto",
                                onclick: {
                                    let timetable_c = filtered_timetable.clone();
                                    let courses_c = courses.clone();
                                    let students_c = students.clone();
                                    let selected_id = selected_student_profile_id.clone();
                                    move |_| {
                                        let user_name = students_c.iter()
                                            .find(|s| s.id == selected_id)
                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                            .unwrap_or_else(|| "Student".to_string());
                                        let mut ics_content = format!(
                                            "BEGIN:VCALENDAR\n\
                                             VERSION:2.0\n\
                                             PRODID:-//YntraPlatform//ClassSchedule//EN\n\
                                             CALSCALE:GREGORIAN\n\
                                             METHOD:PUBLISH\n"
                                        );

                                        for s in timetable_c.iter() {
                                            let course_name = courses_c.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                            let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                            let day_code = match s.day_of_week {
                                                1 => "MO",
                                                2 => "TU",
                                                3 => "WE",
                                                4 => "TH",
                                                5 => "FR",
                                                _ => "MO",
                                            };
                                            let start_date = match s.day_of_week {
                                                1 => "20260720",
                                                2 => "20260721",
                                                3 => "20260722",
                                                4 => "20260723",
                                                5 => "20260724",
                                                _ => "20260720",
                                            };
                                            
                                            let clean_start = s.start_time.replace(":", "");
                                            let clean_end = s.end_time.replace(":", "");

                                            ics_content.push_str(&format!(
                                                "BEGIN:VEVENT\n\
                                                 UID:{}@yntra.platform\n\
                                                 DTSTAMP:20260718T000000Z\n\
                                                 SUMMARY:{}\n\
                                                 LOCATION:{}\n\
                                                 DESCRIPTION:Weekly recurring school class for {}\n\
                                                 DTSTART;TZID=Europe/Stockholm:{}T{}00\n\
                                                 DTEND;TZID=Europe/Stockholm:{}T{}00\n\
                                                 RRULE:FREQ=WEEKLY;BYDAY={}\n\
                                                 END:VEVENT\n",
                                                 uuid::Uuid::new_v4(),
                                                 course_name,
                                                 classroom_name,
                                                 user_name,
                                                 start_date,
                                                 clean_start,
                                                 start_date,
                                                 clean_end,
                                                 day_code
                                            ));
                                        }
                                        ics_content.push_str("END:VCALENDAR\n");

                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let base64_str = base64_encode(ics_content.as_bytes());
                                            let file_name = format!("Schema_{}.ics", user_name.replace(" ", "_"));
                                            let js_code = format!(
                                                r#"
                                                (function() {{
                                                    const base64 = "{}";
                                                    const filename = "{}";
                                                    const binString = atob(base64);
                                                    const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                                                    const blob = new Blob([bytes], {{ type: "text/calendar;charset=utf-8;" }});
                                                    const url = URL.createObjectURL(blob);
                                                    const a = document.createElement("a");
                                                    a.href = url;
                                                    a.download = filename;
                                                    document.body.appendChild(a);
                                                    a.click();
                                                    document.body.removeChild(a);
                                                    URL.revokeObjectURL(url);
                                                }})();
                                                "#,
                                                base64_str, file_name
                                            );
                                            let _ = js_sys::eval(&js_code);
                                        }

                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            let home_dir = std::env::var("USERPROFILE")
                                                .or_else(|_| std::env::var("HOME"))
                                                .unwrap_or_else(|_| ".".to_string());
                                            let filename = format!("Schema_{}.ics", user_name.replace(" ", "_"));
                                            let paths = vec![
                                                format!("{}/Desktop", home_dir),
                                                format!("{}/Downloads", home_dir),
                                                home_dir.clone(),
                                            ];
                                            for path in paths {
                                                let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                if std::fs::write(&file_path, &ics_content).is_ok() {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                },
                                LucideIcon { name: "calendar", size: "12" }
                                {t("school-calendar-sync-btn", &locale)}
                            }
                        }
                    }
                }
            }
        }
    }
}
