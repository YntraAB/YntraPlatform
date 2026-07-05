use dioxus::prelude::*;
use crate::components;

#[derive(Props, Clone)]
pub struct CourseSettingsProps {
    pub course_id: String,
    pub edit_name: Signal<String>,
    pub edit_subject: Signal<String>,
    pub edit_classroom: Signal<String>,
    pub edit_grading: Signal<String>,
    pub edit_late: Signal<String>,
    pub edit_stream: Signal<String>,
    pub ws_default_grading_label: String,
    pub ws_default_late_label: String,
    pub save_course_settings: Callback<(String, String, String, Option<String>, String, String, String)>,
}

impl PartialEq for CourseSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn CourseSettings(props: CourseSettingsProps) -> Element {
    let course_id = props.course_id;
    let mut edit_name = props.edit_name;
    let mut edit_subject = props.edit_subject;
    let mut edit_classroom = props.edit_classroom;
    let mut edit_grading = props.edit_grading;
    let mut edit_late = props.edit_late;
    let mut edit_stream = props.edit_stream;
    let ws_default_grading_label = props.ws_default_grading_label;
    let ws_default_late_label = props.ws_default_late_label;
    let save_course_settings = props.save_course_settings;

    rsx! {
        components::Card { class: "p-5 border border-border/40 bg-sidebar/20 flex flex-col gap-6 max-w-xl shadow-md",
            h3 { class: "text-sm font-black uppercase text-primary tracking-wider m-0 border-b border-border/40 pb-2", "Course settings" }
            
            div { class: "flex flex-col gap-4 text-xs",
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Course Name" }
                    input {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                        placeholder: "e.g. Advanced Chemistry",
                        value: "{edit_name}",
                        oninput: move |e| edit_name.set(e.value()),
                    }
                }
                
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Subject" }
                    input {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                        placeholder: "e.g. Science",
                        value: "{edit_subject}",
                        oninput: move |e| edit_subject.set(e.value()),
                    }
                }
                
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Classroom Location" }
                    input {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground",
                        placeholder: "e.g. Room 304",
                        value: "{edit_classroom}",
                        oninput: move |e| edit_classroom.set(e.value()),
                    }
                }

                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Grading Scale Override" }
                    select {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                        value: "{edit_grading}",
                        onchange: move |e| edit_grading.set(e.value()),
                        option { value: "default", "Use Workspace Default ({ws_default_grading_label})" }
                        option { value: "A-F", "A-F (Letter Grades)" }
                        option { value: "1-10", "1-10 (Numeric Scale)" }
                        option { value: "1-100", "0-100 (Percentage Scale)" }
                        option { value: "U-G-VG", "U, G, VG (Swedish University Scale)" }
                        option { value: "U-G", "U, G (Swedish Pass/Fail)" }
                        option { value: "U-3-4-5", "U, 3, 4, 5 (Swedish Engineering)" }
                    }
                }

                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Late Submission Policy Override" }
                    select {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                        value: "{edit_late}",
                        onchange: move |e| edit_late.set(e.value()),
                        option { value: "default", "Use Workspace Default ({ws_default_late_label})" }
                        option { value: "none", "None (No Penalties)" }
                        option { value: "hard_deadline", "Hard Deadline (Block Late Submissions)" }
                        option { value: "penalty_5", "5% Daily Deduction Penalty" }
                        option { value: "penalty_10", "10% Daily Deduction Penalty" }
                    }
                }

                div { class: "flex flex-col gap-1.5",
                    label { class: "text-[10px] font-bold uppercase text-muted-foreground", "Stream Posting Permissions" }
                    select {
                        class: "yntra-input h-10 px-3 bg-background border border-border/40 rounded-lg text-foreground focus:outline-none",
                        value: "{edit_stream}",
                        onchange: move |e| edit_stream.set(e.value()),
                        option { value: "anyone", "Students and Teachers can post" }
                        option { value: "teachers_only", "Only Teachers can post" }
                    }
                }

                button {
                    class: "yntra-btn text-xs font-bold py-2 px-4 self-end mt-2 flex items-center gap-1.5",
                    onclick: move |_| {
                        save_course_settings.call((
                            course_id.clone(),
                            (*edit_name.read()).clone(),
                            (*edit_subject.read()).clone(),
                            if edit_classroom.read().trim().is_empty() { None } else { Some((*edit_classroom.read()).trim().to_string()) },
                            (*edit_grading.read()).clone(),
                            (*edit_late.read()).clone(),
                            (*edit_stream.read()).clone()
                        ));
                    },
                    components::LucideIcon { name: "save", size: "14" }
                    "Save Settings"
                }
            }
        }
    }
}
