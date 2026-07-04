use dioxus::prelude::*;
use yntra_core::add_time_report;
use crate::locales::t;

#[derive(Props, Clone)]
pub struct TimeReportModalProps {
    pub assistant_teams_list: Vec<yntra_core::Team>,
    pub time_date: Signal<String>,
    pub time_start: Signal<String>,
    pub time_end: Signal<String>,
    pub time_hours: Signal<String>,
    pub time_note: Signal<String>,
    pub selected_report_team_id: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub active_user_id: String,
    pub workspace_id: String,
    pub show_report_modal: Signal<bool>,
    pub locale: String,
}

impl PartialEq for TimeReportModalProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn TimeReportModal(props: TimeReportModalProps) -> Element {
    let assistant_teams_list = props.assistant_teams_list.clone();
    let active_user_id = props.active_user_id.clone();
    let workspace_id = props.workspace_id.clone();
    
    let mut time_date = props.time_date;
    let mut time_start = props.time_start;
    let mut time_end = props.time_end;
    let mut time_hours = props.time_hours;
    let mut time_note = props.time_note;
    let mut selected_report_team_id = props.selected_report_team_id;
    let mut db_trigger = props.db_trigger;
    let mut show_report_modal = props.show_report_modal;

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm",
            div { class: "w-full max-w-sm rounded-xl border border-border bg-sidebar p-6 shadow-2xl animate-in zoom-in-95 duration-150",
                div { class: "flex items-center justify-between mb-4 border-b border-border/40 pb-3",
                    h3 { class: "text-base font-bold text-foreground m-0", "{t(\"timereports-report-time-btn\", &props.locale)}" }
                    button {
                        class: "text-muted-foreground hover:text-foreground bg-transparent border-0 cursor-pointer font-bold text-sm",
                        onclick: move |_| show_report_modal.set(false),
                        "✕"
                    }
                }
                div { class: "flex flex-col gap-4",
                    // Team / Brukare selector
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-patient-team\", &props.locale)}" }
                        select {
                            class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-1 focus:ring-primary/30",
                            value: "{selected_report_team_id}",
                            onchange: move |e| {
                                selected_report_team_id.set(e.value());
                            },
                            for t in assistant_teams_list.iter() {
                                option { value: "{t.id}", "{t.name}" }
                            }
                        }
                    }
                    // Date
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-date-label\", &props.locale)}" }
                        input {
                            class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                            r#type: "date",
                            value: "{time_date}",
                            oninput: move |e| time_date.set(e.value()),
                        }
                    }
                    // Start & End time
                    div { class: "flex gap-4",
                        div { class: "flex-1 flex flex-col gap-1.5",
                            label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-start-time-label\", &props.locale)}" }
                            input {
                                class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                                r#type: "time",
                                value: "{time_start}",
                                oninput: move |e| time_start.set(e.value()),
                            }
                        }
                        div { class: "flex-1 flex flex-col gap-1.5",
                            label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-end-time-label\", &props.locale)}" }
                            input {
                                class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                                r#type: "time",
                                value: "{time_end}",
                                oninput: move |e| time_end.set(e.value()),
                            }
                        }
                    }
                    // Total hours
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-calculated-hours\", &props.locale)}" }
                        input {
                            class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                            r#type: "number",
                            step: "0.5",
                            value: "{time_hours}",
                            oninput: move |e| time_hours.set(e.value()),
                        }
                    }
                    // Comment/Note
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground", "{t(\"timereports-note-label\", &props.locale)}" }
                        input {
                            class: "yntra-input text-sm h-9 px-3 bg-background border border-border rounded-lg text-foreground",
                            placeholder: t("timereports-note-placeholder", &props.locale),
                            value: "{time_note}",
                            oninput: move |e| time_note.set(e.value()),
                        }
                    }
                }
                div { class: "flex justify-end gap-3 mt-6 border-t border-border/40 pt-4",
                    button {
                        class: "yntra-btn secondary h-8 text-xs px-4 border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer rounded-lg",
                        onclick: move |_| show_report_modal.set(false),
                        "{t(\"wizard-action-cancel\", &props.locale)}"
                    }
                    button {
                        class: "yntra-btn h-8 text-xs px-4 cursor-pointer rounded-lg",
                        onclick: move |_| {
                            let note = time_note.read().trim().to_string();
                            let hrs = time_hours.read().parse::<f64>().unwrap_or(8.0);
                            let t_id = selected_report_team_id.read().clone();
                             let w_id = workspace_id.clone();
                             let u_id = active_user_id.clone();
                             let date_val = time_date.read().clone();
                             let start_val = time_start.read().clone();
                             let end_val = time_end.read().clone();
                             spawn(async move {
                                 let _ = add_time_report(
                                     w_id,
                                     u_id,
                                     Some(t_id),
                                     date_val,
                                     hrs,
                                     note,
                                     Some(start_val),
                                     Some(end_val),
                                 ).await;
                             });
                            let current_trig = *db_trigger.read();
                            db_trigger.set(current_trig + 1);
                            time_note.set(String::new());
                            show_report_modal.set(false);
                        },
                        "{t(\"timereports-save-report\", &props.locale)}"
                    }
                }
            }
        }
    }
}
