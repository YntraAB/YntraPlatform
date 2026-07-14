use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{ClientProfile, Team, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct TimeReportModalProps {
    pub show_report_time_modal: Signal<bool>,
    pub active_user: WorkspaceUser,
    pub teams: Vec<Team>,
    pub clients: Vec<ClientProfile>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

fn calculate_hours(start: &str, end: &str, break_mins: &str) -> f64 {
    let parse_time = |t_str: &str| -> Option<(i32, i32)> {
        let parts: Vec<&str> = t_str.split(':').collect();
        if parts.len() == 2 {
            let h = parts[0].trim().parse::<i32>().ok()?;
            let m = parts[1].trim().parse::<i32>().ok()?;
            Some((h, m))
        } else {
            None
        }
    };

    if let (Some((sh, sm)), Some((eh, em))) = (parse_time(start), parse_time(end)) {
        let start_mins = sh * 60 + sm;
        let mut end_mins = eh * 60 + em;
        if end_mins < start_mins {
            end_mins += 24 * 60;
        }
        let break_m = break_mins.trim().parse::<i32>().unwrap_or(0);
        let duration_mins = end_mins - start_mins - break_m;
        if duration_mins > 0 {
            return (duration_mins as f64) / 60.0;
        }
    }
    8.0
}

#[component]
pub fn TimeReportModal(props: TimeReportModalProps) -> Element {
    let mut show_report_time_modal = props.show_report_time_modal;
    let active_user = props.active_user;
    let teams = props.teams;
    let clients = props.clients;
    let mut db_trigger = props.db_trigger;
    let locale = props.locale;

    let mut selected_team_id = use_signal(|| {
        teams
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "team-1".to_string())
    });
    let mut report_date = use_signal(|| {
        let now_str = yntra_core::infra::time::get_current_datetime_str();
        if now_str.len() >= 10 {
            now_str[..10].to_string()
        } else {
            "2026-07-02".to_string()
        }
    });
    let mut report_start = use_signal(|| "08:00".to_string());
    let mut report_end = use_signal(|| "16:00".to_string());
    let mut report_break = use_signal(|| "30".to_string());
    let mut report_note = use_signal(String::new);

    rsx! {
        if *show_report_time_modal.read() {
            components::Dialog {
                open: *show_report_time_modal.read(),
                title: t("timereports-report-time-btn", &locale),
                onclose: move |_| show_report_time_modal.set(false),
                div { class: "flex flex-col gap-5 text-sm",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-patient-team\", &locale)}"
                        }
                        select {
                            class: "yntra-input",
                            value: "{selected_team_id}",
                            onchange: move |e| selected_team_id.set(e.value()),
                            for t in teams.iter() {
                                option { value: "{t.id}", "{t.name}" }
                            }
                            for c in clients.iter() {
                                {
                                    let patient_lbl = t("common-patient", &locale);
                                    rsx! {
                                        option { value: "client-{c.id}", "{c.first_name} {c.last_name} ({patient_lbl})" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-date-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            r#type: "text",
                            value: "{report_date}",
                            oninput: move |e| report_date.set(e.value()),
                        }
                    }
                    div { class: "grid gap-4",
                    style: "grid-template-columns:1fr 1fr;",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                                "{t(\"timereports-start-time-label\", &locale)}"
                            }
                            input {
                                class: "yntra-input",
                                r#type: "text",
                                value: "{report_start}",
                                oninput: move |e| report_start.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                                "{t(\"timereports-end-time-label\", &locale)}"
                            }
                            input {
                                class: "yntra-input",
                                r#type: "text",
                                value: "{report_end}",
                                oninput: move |e| report_end.set(e.value()),
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-break-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            r#type: "text",
                            value: "{report_break}",
                            oninput: move |e| report_break.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground/60 uppercase",
                            "{t(\"timereports-note-label\", &locale)}"
                        }
                        input {
                            class: "yntra-input",
                            value: "{report_note}",
                            placeholder: t("timereports-note-placeholder", &locale),
                            oninput: move |e| report_note.set(e.value()),
                        }
                    }
                    div { class: "flex justify-end gap-3 mt-2",
                        button {
                            class: "yntra-btn secondary",
                            style: "padding:0.45rem 1rem;",
                            onclick: move |_| show_report_time_modal.set(false),
                            "{t(\"wizard-action-cancel\", &locale)}"
                        }
                        button {
                            class: "yntra-btn text-white",
                            style: "background: var(--accent-color); border-color: var(--accent-color); padding:0.45rem 1rem;",
                            onclick: move |_| {
                                let note = report_note.read().trim().to_string();
                                let start = report_start.read().trim().to_string();
                                let end = report_end.read().trim().to_string();
                                let brk = report_break.read().trim().to_string();
                                let hrs = calculate_hours(&start, &end, &brk);
                                let t_id = selected_team_id.read().clone();
                                let clean_team_id = if t_id.starts_with("client-") { None } else { Some(t_id) };

                                let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let user_id = active_user.id.clone();
                                let date_val = report_date.read().clone();
                                spawn(async move {
                                    let _ = yntra_core::add_time_report(
                                        user_id.clone(),
                                        workspace_id,
                                        user_id,
                                        clean_team_id,
                                        date_val,
                                        hrs,
                                        note,
                                        Some(start),
                                        Some(end),
                                    ).await;
                                });
                                let current_trig = *db_trigger.read();
                                db_trigger.set(current_trig + 1);
                                show_report_time_modal.set(false);
                                report_note.set(String::new());
                            },
                            "{t(\"timereports-report-time-btn\", &locale)}"
                        }
                    }
                }
            }
        }
    }
}
