use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TimeReport;

#[derive(Props, Clone)]
pub struct AssistantTeamsListProps {
    pub active_user_id: String,
    pub assistant_teams_list: Vec<yntra_core::Team>,
    pub time_reports: Vec<TimeReport>,
    pub selected_team_id: Signal<Option<String>>,
    pub selected_user_id: Signal<Option<String>>,
    pub current_level: Signal<String>,
}

impl PartialEq for AssistantTeamsListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AssistantTeamsList(props: AssistantTeamsListProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let region = state.auth_region.read();

    let active_user_id = props.active_user_id.clone();
    let assistant_teams_list = props.assistant_teams_list.clone();
    let time_reports = props.time_reports.clone();
    let mut selected_team_id = props.selected_team_id;
    let mut selected_user_id = props.selected_user_id;
    let mut current_level = props.current_level;

    rsx! {
        div { class: "duration-400 relative flex h-full flex-1 flex-col bg-background animate-in fade-in slide-in-from-bottom-2",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border/50 px-8 bg-sidebar",
                h2 { class: "flex items-center gap-2.5 text-base font-semibold text-foreground m-0",
                    div { class: "rounded-md bg-primary/10 p-1.5",
                        components::LucideIcon { name: "calendar", class: "h-4 w-4 text-primary" }
                    }
                    "{t(\"time-my-teams-shifts\", &region)}"
                }
            }
            div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",
                if assistant_teams_list.is_empty() {
                    div { class: "text-center text-muted-foreground/60 py-16",
                        components::LucideIcon { name: "calendar", class: "h-12 w-12 mx-auto mb-3 opacity-20" }
                        p { class: "text-sm m-0", "{t(\"time-no-registered-teams\", &region)}" }
                    }
                } else {
                    for team in assistant_teams_list.iter() {
                        {
                            let t_id = team.id.clone();
                            let t_name = team.name.clone();
                            let t_shifts: Vec<&TimeReport> = time_reports.iter().filter(|r| r.team_id.as_ref() == Some(&t_id) && r.user_id == active_user_id).collect();
                            let total_hours: f64 = t_shifts.iter().map(|s| s.hours).sum();
                            let has_pending = t_shifts.iter().any(|s| s.status == "pending_attest");
                            let active_user_id_clone = active_user_id.clone();

                            rsx! {
                                div {
                                    key: "{t_id}",
                                    onclick: move |_| {
                                        selected_team_id.set(Some(t_id.clone()));
                                        selected_user_id.set(Some(active_user_id_clone.clone()));
                                        current_level.set("shift_list".to_string());
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-8 py-4 transition-all duration-200 hover:bg-white/[0.02] list-item-hover",

                                    div { class: "mr-5 flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] font-bold text-primary transition-all group-hover:bg-primary group-hover:text-primary-foreground",
                                        "{t_name.chars().next().unwrap_or('T')}"
                                    }
                                    div { class: "w-64 shrink-0 pr-4 md:w-80",
                                        div { class: "text-[15px] font-semibold text-foreground transition-colors group-hover:text-primary", "{t_name}" }
                                        div { class: "mt-0.5 text-[11px] font-medium uppercase tracking-widest text-muted-foreground/60", "ID: {t_id}" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }

                                    div { class: "flex w-32 shrink-0 flex-col items-end justify-center pr-4",
                                        div { class: "flex items-baseline gap-1",
                                            span { class: "text-lg font-bold text-foreground", "{total_hours}" }
                                            span { class: "text-[11px] font-medium uppercase text-muted-foreground", "h" }
                                        }
                                        span { class: "text-[10px] font-medium uppercase tracking-tighter text-muted-foreground/60", "{t(\"timereports-attested\", &region)}" }
                                    }

                                    div { class: "flex w-40 shrink-0 items-center justify-end pr-4",
                                        if has_pending {
                                            span { class: "inline-flex items-center px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase bg-amber-500/10 border border-amber-500/20 text-amber-400",
                                                "{t(\"time-status-pending-attest\", &region)}"
                                            }
                                        } else {
                                            span { class: "inline-flex items-center px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase bg-emerald-500/10 border border-emerald-500/20 text-emerald-400",
                                                "{t(\"time-all-done\", &region)}"
                                            }
                                        }
                                    }

                                    div { class: "flex w-8 shrink-0 items-center justify-end text-muted-foreground/30 transition-all group-hover:translate-x-1 group-hover:text-foreground",
                                        components::LucideIcon { name: "chevron-right", class: "h-5 w-5" }
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
