use dioxus::prelude::*;
use crate::components::{Card, LucideIcon};
use crate::locales::t;
use yntra_core::StudentProfile;

#[component]
pub fn ProfileSelector(
    mut selected_student_profile_id: Signal<String>,
    students: Vec<StudentProfile>,
    locale: String,
    role: String,
) -> Element {
    let r = role.as_str();
    if r != "student" && r != "role-school-student" && r != "parent" && r != "role-school-parent" {
        rsx! {
            Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                div { class: "flex items-center gap-3 w-full sm:w-auto",
                    LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                    div {
                        h4 { class: "text-sm font-bold text-foreground m-0", {t("school-change-profile", &locale)} }
                        p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-student-preview-desc", &locale)} }
                    }
                }
                select {
                    class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                    value: selected_student_profile_id.read().clone(),
                    onchange: move |evt: FormEvent| selected_student_profile_id.set(evt.value()),
                    for s in students.iter() {
                        option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                    }
                }
            }
        }
    } else if r == "parent" || r == "role-school-parent" {
        rsx! {
            Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                div { class: "flex items-center gap-3 w-full sm:w-auto",
                    LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                    div {
                        h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-select-child", &locale)} }
                        p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-parent-select-child-desc", &locale)} }
                    }
                }
                select {
                    class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                    value: selected_student_profile_id.read().clone(),
                    onchange: move |evt: FormEvent| selected_student_profile_id.set(evt.value()),
                    for s in students.iter() {
                        option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}
