use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::{add_report, send_message};

#[derive(Props, Clone)]
pub struct TimeOffModalProps {
    pub show_time_off_modal: Signal<bool>,
    pub leave_type: Signal<String>,
    pub leave_start: Signal<String>,
    pub leave_end: Signal<String>,
    pub leave_reason: Signal<String>,
    pub leave_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub locale: String,
}

impl PartialEq for TimeOffModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TimeOffModal(props: TimeOffModalProps) -> Element {
    let mut show_time_off_modal = props.show_time_off_modal;
    let mut leave_type = props.leave_type;
    let mut leave_start = props.leave_start;
    let mut leave_end = props.leave_end;
    let mut leave_reason = props.leave_reason;
    let mut leave_save_status = props.leave_save_status;
    let mut db_trigger = props.db_trigger;
    let active_user = props.active_user.clone();
    let users = props.users.clone();

    // Default dates if empty
    use_effect(move || {
        if leave_start.read().is_empty() {
            leave_start.set("2026-07-01".to_string());
        }
        if leave_end.read().is_empty() {
            leave_end.set("2026-07-01".to_string());
        }
    });

    rsx! {
        components::Dialog {
            open: *show_time_off_modal.read(),
            onclose: move |_| show_time_off_modal.set(false),
            title: t("reporting-leave_request", &props.locale),
            div { 
                class: "flex flex-col gap-4 text-left",
                style: "min-width: 400px; box-sizing: border-box; padding: 0.25rem;",
                
                // Icon Header Title
                div { class: "flex items-center gap-2 border-b border-border/40 pb-3",
                    components::LucideIcon { name: "calendar", class: "h-5 w-5 text-primary" }
                    span { class: "font-bold text-base text-foreground", "{t(\"reporting-leave_request\", &props.locale)}" }
                }
                div { class: "text-xs text-muted-foreground leading-relaxed -mt-1 mb-2",
                    "Submit a new leave request. This will generate a report for administrative attestation."
                }

                // Leave Type Select
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                        "{t(\"reporting-list-type\", &props.locale)}"
                    }
                    select {
                        class: "yntra-input border-border bg-muted/50 w-full",
                        value: "{leave_type}",
                        onchange: move |e| leave_type.set(e.value()),
                        option { value: "vacation", "{t(\"reporting-leave-types-vacation\", &props.locale)}" }
                        option { value: "sick_leave", "{t(\"reporting-leave-types-sick-leave\", &props.locale)}" }
                        option { value: "care_of_child", "{t(\"reporting-leave-types-care-of-child\", &props.locale)}" }
                        option { value: "other", "{t(\"reporting-leave-types-other\", &props.locale)}" }
                    }
                }

                // Dates row
                div { class: "grid grid-cols-2 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"common-start-date\", &props.locale)}"
                        }
                        input {
                            r#type: "date",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            value: "{leave_start}",
                            oninput: move |e| leave_start.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                            "{t(\"common-end-date\", &props.locale)}"
                        }
                        input {
                            r#type: "date",
                            class: "yntra-input border-border bg-muted/50 w-full",
                            value: "{leave_end}",
                            oninput: move |e| leave_end.set(e.value()),
                        }
                    }
                }

                // Reason Notes
                div { class: "flex flex-col gap-1.5",
                    label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                        "{t(\"reporting-form-description\", &props.locale)}"
                    }
                    textarea {
                        class: "w-full min-h-[90px] resize-none rounded-lg border border-border bg-muted/50 px-3 py-2 text-foreground text-sm outline-none focus:ring-1 focus:ring-primary",
                        value: "{leave_reason}",
                        placeholder: t("scheduler-description-placeholder", &props.locale),
                        oninput: move |e| leave_reason.set(e.value()),
                    }
                }

                // Footer
                div { class: "flex justify-end gap-3 mt-4 pt-4 border-t border-border/50 items-center",
                    if *leave_save_status.read() == "success" {
                        span { class: "text-xs font-semibold text-emerald-500 mr-auto flex items-center gap-1",
                            "✓ {t(\"reporting-form-success\", &props.locale)}"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "yntra-btn secondary text-xs px-4 py-2 cursor-pointer",
                        onclick: move |_| show_time_off_modal.set(false),
                        "{t(\"common-cancel\", &props.locale)}"
                    }
                    button {
                        r#type: "button",
                        class: "yntra-btn text-xs px-5 py-2 cursor-pointer font-semibold bg-primary text-white hover:bg-primary/90",
                        disabled: *leave_save_status.read() == "saving",
                        onclick: {
                            let users_for_leave = users.clone();
                            let active_user_c = active_user.clone();
                            move |_| {
                                let l_type = leave_type.read().clone();
                                let start = leave_start.read().trim().to_string();
                                let end = leave_end.read().trim().to_string();
                                let reason_txt = leave_reason.read().trim().to_string();

                                if !start.is_empty() && !end.is_empty() {
                                    leave_save_status.set("saving".to_string());

                                    let type_label = match l_type.as_str() {
                                        "vacation" => "Vacation",
                                        "sick_leave" => "Sick Leave",
                                        "care_of_child" => "Care of Child",
                                        _ => "Other Leave",
                                    };
                                    let subject = format!("{}: {} - {}", type_label, start, end);
                                    let description = format!(
                                        "Leave request submitted by {}. Type: {}. Dates: {} to {}. Reason: {}",
                                        active_user_c.full_name.clone().unwrap_or_default(),
                                        type_label,
                                        start,
                                        end,
                                        reason_txt,
                                    );

                                    let workspace_id = active_user_c.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                    let user_id = active_user_c.id.clone();
                                    let type_label_str = type_label.to_string();
                                    let active_user_fullname = active_user_c.full_name.clone().unwrap_or_default();
                                    let start_str = start.clone();
                                    let end_str = end.clone();
                                    let reason_txt_str = reason_txt.clone();
                                    let admins = users_for_leave
                                        .iter()
                                        .filter(|u| u.role == "admin" || u.role == "platform_admin")
                                        .cloned()
                                        .collect::<Vec<_>>();

                                    spawn(async move {
                                        let _ = add_report(
                                            workspace_id.clone(),
                                            user_id.clone(),
                                            "leave_request".to_string(),
                                            false,
                                            subject,
                                            description,
                                            "2026-06-30".to_string(),
                                        ).await;

                                        for admin in admins {
                                            let _ = send_message(
                                                user_id.clone(),
                                                workspace_id.clone(),
                                                user_id.clone(),
                                                Some(admin.id.clone()),
                                                None,
                                                format!("Leave Request Notification: {}", type_label_str),
                                                format!(
                                                    "A new leave request was submitted for attestation.\n\nOperator: {}\nType: {}\nDuration: {} to {}\nNotes: {}",
                                                    active_user_fullname,
                                                    type_label_str,
                                                    start_str,
                                                    end_str,
                                                    reason_txt_str,
                                                ),
                                            ).await;
                                        }
                                    });
                                    let current_trig = *db_trigger.read();
                                    db_trigger.set(current_trig + 1);
                                    leave_reason.set(String::new());
                                    leave_save_status.set("success".to_string());
                                    show_time_off_modal.set(false);
                                }
                            }
                        },
                        if *leave_save_status.read() == "saving" {
                            "{t(\"common-saving\", &props.locale)}"
                        } else {
                            "{t(\"reporting-form-submit\", &props.locale)}"
                        }
                    }
                }
            }
        }
    }
}
