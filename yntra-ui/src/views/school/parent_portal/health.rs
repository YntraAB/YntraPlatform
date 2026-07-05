use dioxus::prelude::*;
use crate::components;
use yntra_core::{HealthRecord, HealthIncident};

#[derive(Props, Clone)]
pub struct HealthTabProps {
    pub health_records: Vec<HealthRecord>,
    pub health_incidents: Vec<HealthIncident>,
}

impl PartialEq for HealthTabProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn HealthTab(props: HealthTabProps) -> Element {
    let health_records = props.health_records;
    let health_incidents = props.health_incidents;

    rsx! {
        div { class: "grid gap-6 md:grid-cols-2 items-start",
            // Left: Vaccine Schedule Card
            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "award", size: "18", class: "text-primary" }
                    "Official Immunization Card"
                }
                
                if health_records.is_empty() {
                    p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No immunization data recorded." }
                } else {
                    div { class: "flex flex-col gap-3.5",
                        for r in health_records.iter() {
                            div { class: "flex items-center justify-between border border-border/30 p-3 rounded-xl bg-white/[0.01]",
                                div { class: "flex flex-col gap-0.5",
                                    span { class: "text-xs font-black text-foreground", "{r.vaccine_name}" }
                                    if let Some(ref date) = r.administered_at {
                                        span { class: "text-[10px] text-muted-foreground font-semibold", "Administered: {date}" }
                                    } else {
                                        span { class: "text-[10px] text-muted-foreground italic", "Pending schedule" }
                                    }
                                }
                                match r.status.as_str() {
                                    "completed" => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/15 shadow-sm", "Completed" } },
                                    "exempted" => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-indigo-500/10 text-indigo-400 border border-indigo-500/15 shadow-sm", "Exempt" } },
                                    _ => rsx! { span { class: "px-2.5 py-1 rounded text-xs font-black bg-amber-500/10 text-amber-400 border border-amber-500/15 animate-pulse", "Pending" } }
                                }
                            }
                        }
                    }
                }
            }

            // Right: Nurse Log History Card
            components::Card { class: "p-6 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                h3 { class: "text-base font-black text-foreground m-0 flex items-center gap-2",
                    components::LucideIcon { name: "heart", size: "18", class: "text-primary" }
                    "School Nurse Check-in Logs"
                }
                
                if health_incidents.is_empty() {
                    p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No nurse visit logs recorded for this student." }
                } else {
                    div { class: "flex flex-col gap-3.5",
                        for inc in health_incidents.iter() {
                            div { class: "border border-border/30 p-3 rounded-xl bg-white/[0.01] flex flex-col gap-2",
                                div { class: "flex justify-between items-center",
                                    span { class: "text-xs font-black text-foreground", "{inc.visit_reason}" }
                                    span { class: "text-[10px] text-muted-foreground font-semibold", "{inc.checked_in_at}" }
                                }
                                div { class: "text-[10px] text-muted-foreground", "Treatment: {inc.treatment}" }
                                if let Some(ref note) = inc.notes {
                                    div { class: "text-[10px] bg-sidebar/50 p-2 rounded border border-border/20 text-muted-foreground italic",
                                        "\"{note}\""
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
