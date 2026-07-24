use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::TimeReport;
use yntra_core::Workspace;

#[derive(Props, Clone)]
pub struct OrganizationListProps {
    pub workspaces: Vec<Workspace>,
    pub time_reports: Vec<TimeReport>,
    pub selected_workspace_id: Signal<Option<String>>,
    pub current_level: Signal<String>,
}

impl PartialEq for OrganizationListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn OrganizationList(props: OrganizationListProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let region = state.auth_region.read();

    let workspaces = props.workspaces.clone();
    let time_reports = props.time_reports.clone();
    let mut selected_workspace_id = props.selected_workspace_id;
    let mut current_level = props.current_level;

    rsx! {
        div { class: "duration-400 relative flex h-full flex-1 flex-col bg-background animate-in fade-in slide-in-from-bottom-2",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border/50 px-8 bg-sidebar",
                h2 { class: "flex items-center gap-2.5 text-base font-semibold text-foreground m-0",
                    div { class: "rounded-md bg-primary/10 p-1.5",
                        components::LucideIcon { name: "layout-grid", class: "h-4 w-4 text-primary" }
                    }
                    "{t(\"time-organizations\", &region)}"
                }
            }
            div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",
                if workspaces.is_empty() {
                    div { class: "text-center text-muted-foreground/60 py-16",
                        components::LucideIcon { name: "layout-grid", class: "h-12 w-12 mx-auto mb-3 opacity-20" }
                        p { class: "text-sm m-0", "{t(\"time-no-registered-organizations\", &region)}" }
                    }
                } else {
                    for ws in workspaces.iter() {
                        {
                            let ws_id = ws.id.clone();
                            let ws_name = ws.name.clone();
                            let ws_shifts: Vec<&TimeReport> = time_reports.iter().filter(|r| r.workspace_id == ws_id).collect();
                            let total_hours: f64 = ws_shifts.iter().map(|s| s.hours).sum();
                            let pending_attest = ws_shifts.iter().filter(|s| s.status == "pending_attest").count();

                            rsx! {
                                div {
                                    key: "{ws_id}",
                                    onclick: move |_| {
                                        selected_workspace_id.set(Some(ws_id.clone()));
                                        current_level.set("team_overview".to_string());
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-8 py-4 transition-all duration-200 hover:bg-white/[0.02] list-item-hover",

                                    div { class: "mr-5 flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] font-bold text-primary transition-colors group-hover:bg-primary/10",
                                        components::LucideIcon { name: "layout-grid", class: "h-5 w-5" }
                                    }
                                    div { class: "w-64 shrink-0 pr-4 md:w-80",
                                        div { class: "text-[15px] font-semibold text-foreground transition-colors group-hover:text-primary", "{ws_name}" }
                                        div { class: "mt-0.5 text-[11px] font-medium uppercase tracking-widest text-muted-foreground/60", "Företag" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }

                                    div { class: "flex w-32 shrink-0 flex-col items-end justify-center pr-4",
                                        div { class: "flex items-baseline gap-1",
                                            span { class: "text-lg font-bold text-foreground", "{total_hours}" }
                                            span { class: "text-[11px] font-medium uppercase text-muted-foreground", "h" }
                                        }
                                        span { class: "text-[10px] font-medium uppercase tracking-tighter text-muted-foreground/60", "Totalt klara" }
                                    }

                                    div { class: "flex w-40 shrink-0 items-center justify-end pr-4",
                                        if pending_attest > 0 {
                                            span { class: "inline-flex items-center px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase bg-amber-500/10 border border-amber-500/20 text-amber-400",
                                                "{pending_attest} OATTESTERAT"
                                            }
                                        } else {
                                            span { class: "inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase bg-emerald-500/10 border border-emerald-500/20 text-emerald-400",
                                                components::LucideIcon { name: "check-circle", class: "h-3 w-3 text-emerald-400" }
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
