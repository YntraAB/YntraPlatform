use crate::components;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::TimeReport;

#[derive(Props, Clone)]
pub struct MemberListProps {
    pub active_user_role: String,
    pub filtered_users: Vec<WorkspaceUser>,
    pub time_reports: Vec<TimeReport>,
    pub selected_workspace_id: Signal<Option<String>>,
    pub selected_user_id: Signal<Option<String>>,
    pub selected_team_id: Signal<Option<String>>,
    pub current_level: Signal<String>,
}

impl PartialEq for MemberListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn MemberList(props: MemberListProps) -> Element {
    let active_user_role = props.active_user_role.clone();
    let filtered_users = props.filtered_users.clone();
    let time_reports = props.time_reports.clone();
    let mut selected_workspace_id = props.selected_workspace_id;
    let mut selected_user_id = props.selected_user_id;
    let mut selected_team_id = props.selected_team_id;
    let mut current_level = props.current_level;

    rsx! {
        div { class: "duration-400 relative flex h-full flex-1 flex-col bg-background animate-in fade-in slide-in-from-bottom-2",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border/50 px-8 bg-sidebar",
                div { class: "flex items-center gap-4",
                    if active_user_role == "platform_admin" {
                        button {
                            class: "yntra-btn secondary text-xs flex items-center gap-1.5 h-8 px-3 rounded-lg font-semibold border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer",
                            onclick: move |_| {
                                selected_workspace_id.set(None);
                                current_level.set("platform_overview".to_string());
                            },
                            components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }
                            "Tillbaka"
                        }
                    }
                    h2 { class: "flex items-center gap-2.5 text-base font-semibold text-foreground m-0",
                        div { class: "rounded-md bg-primary/10 p-1.5",
                            components::LucideIcon { name: "users", class: "h-4 w-4 text-primary" }
                        }
                        "Team Members"
                    }
                }
            }
            div { class: "scrollbar-dark w-full flex-1 overflow-y-auto flex flex-col",
                if filtered_users.is_empty() {
                    div { class: "text-center text-muted-foreground/60 py-16",
                        components::LucideIcon { name: "users", class: "h-12 w-12 mx-auto mb-3 opacity-20" }
                        p { class: "text-sm m-0", "Inga medlemmar hittades." }
                    }
                } else {
                    for u in filtered_users.iter() {
                        {
                            let u_id = u.id.clone();
                            let u_name = u.full_name.clone().unwrap_or_else(|| "Unknown Member".to_string());
                            let u_role = u.role.clone();
                            let u_shifts: Vec<&TimeReport> = time_reports.iter().filter(|r| r.user_id == u_id).collect();
                            let total_hours: f64 = u_shifts.iter().map(|s| s.hours).sum();
                            let has_pending = u_shifts.iter().any(|s| s.status == "pending_attest");
                            let has_rejected = u_shifts.iter().any(|s| s.status == "rejected");
                            
                            let (status_text, status_color, status_bg) = if u_shifts.is_empty() {
                                ("Not Submitted", "var(--text-muted)", "rgba(255,255,255,0.03)")
                            } else if has_pending {
                                ("Pending Attest", "var(--warning)", "rgba(245,158,11,0.1)")
                            } else if has_rejected {
                                ("Rejected/Disputed", "var(--danger)", "rgba(239,68,68,0.1)")
                            } else {
                                ("Approved", "var(--success)", "rgba(16,185,129,0.1)")
                            };
                            
                            rsx! {
                                div {
                                    key: "{u_id}",
                                    onclick: move |_| {
                                        selected_user_id.set(Some(u_id.clone()));
                                        selected_team_id.set(None);
                                        current_level.set("shift_list".to_string());
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border/30 px-8 py-4 transition-all duration-200 hover:bg-white/[0.02] list-item-hover",
                                    
                                    div { class: "mr-5 flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] font-bold text-lg text-foreground transition-all group-hover:bg-primary group-hover:text-primary-foreground",
                                        "{u_name.chars().next().unwrap_or('?')}"
                                    }
                                    div { class: "w-64 shrink-0 pr-4 md:w-80",
                                        div { class: "text-[15px] font-semibold text-foreground transition-colors group-hover:text-primary", "{u_name}" }
                                        div { class: "mt-0.5 text-[11px] font-medium uppercase tracking-widest text-muted-foreground/60", "{u_role}" }
                                    }
                                    div { class: "min-w-0 flex-1 pr-4" }
                                    
                                    div { class: "flex w-32 shrink-0 flex-col items-end justify-center pr-4",
                                        div { class: "flex items-baseline gap-1",
                                            span { class: "text-lg font-bold text-foreground", "{total_hours}" }
                                            span { class: "text-[11px] font-medium uppercase text-muted-foreground", "h" }
                                        }
                                        span { class: "text-[10px] font-medium uppercase tracking-tighter text-muted-foreground/60", "Rapporterat" }
                                    }
                                    
                                    div { class: "flex w-40 shrink-0 items-center justify-end pr-4",
                                        span { class: "inline-flex items-center px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase",
                                            style: "background: {status_bg}; color: {status_color}; border: 1px solid {status_color}20;",
                                            "{status_text}"
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
