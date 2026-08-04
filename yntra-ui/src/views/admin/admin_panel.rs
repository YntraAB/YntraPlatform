use super::{AuditExportsView, OnboardingWizardView, RbacManagementView, TeamInvitationsView};
use crate::components;
use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct AdminPanelProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub users: Vec<WorkspaceUser>,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn AdminPanelView(props: AdminPanelProps) -> Element {
    let mut active_tab = use_signal(|| "onboarding".to_string());

    let workspace = props.workspace.clone();
    let active_user = props.active_user.clone();
    let users = props.users.clone();
    let db_trigger = props.db_trigger;
    let locale = props.locale.clone();

    rsx! {
        div { class: "flex flex-col min-h-screen bg-background text-foreground animate-in fade-in duration-200",
            // Admin Panel Sticky Top Navigation
            div { class: "sticky top-0 z-10 bg-card/80 backdrop-blur-md border-b border-border/40 px-6 py-3.5 flex flex-col md:flex-row md:items-center justify-between gap-4 shadow-xs",
                div { class: "flex items-center gap-3",
                    div { class: "h-9 w-9 rounded-xl bg-primary/10 text-primary border border-primary/20 flex items-center justify-center shadow-xs",
                        components::LucideIcon { name: "shield-check", class: "h-5 w-5" }
                    }
                    div {
                        h1 { class: "text-base font-extrabold text-foreground m-0 tracking-tight", "Workspace Admin Panel" }
                        p { class: "text-xs text-muted-foreground m-0", "{workspace.name}" }
                    }
                }

                // Tab Selector Buttons
                div { class: "flex items-center gap-1.5 p-1 rounded-xl bg-muted/40 border border-border/40 text-xs font-semibold",
                    button {
                        class: format!(
                            "px-3.5 py-1.5 rounded-lg transition-all flex items-center gap-1.5 {}",
                            if *active_tab.read() == "onboarding" {
                                "bg-primary text-primary-foreground font-bold shadow-sm"
                            } else {
                                "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                            }
                        ),
                        onclick: move |_| active_tab.set("onboarding".to_string()),
                        components::LucideIcon { name: "sparkles", class: "h-3.5 w-3.5" }
                        "Onboarding Wizard"
                    }

                    button {
                        class: format!(
                            "px-3.5 py-1.5 rounded-lg transition-all flex items-center gap-1.5 {}",
                            if *active_tab.read() == "invitations" {
                                "bg-primary text-primary-foreground font-bold shadow-sm"
                            } else {
                                "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                            }
                        ),
                        onclick: move |_| active_tab.set("invitations".to_string()),
                        components::LucideIcon { name: "user-plus", class: "h-3.5 w-3.5" }
                        "Team Invites"
                    }

                    button {
                        class: format!(
                            "px-3.5 py-1.5 rounded-lg transition-all flex items-center gap-1.5 {}",
                            if *active_tab.read() == "rbac" {
                                "bg-primary text-primary-foreground font-bold shadow-sm"
                            } else {
                                "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                            }
                        ),
                        onclick: move |_| active_tab.set("rbac".to_string()),
                        components::LucideIcon { name: "shield-alert", class: "h-3.5 w-3.5" }
                        "RBAC Access"
                    }

                    button {
                        class: format!(
                            "px-3.5 py-1.5 rounded-lg transition-all flex items-center gap-1.5 {}",
                            if *active_tab.read() == "audit" {
                                "bg-primary text-primary-foreground font-bold shadow-sm"
                            } else {
                                "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                            }
                        ),
                        onclick: move |_| active_tab.set("audit".to_string()),
                        components::LucideIcon { name: "file-text", class: "h-3.5 w-3.5" }
                        "Audit Exports"
                    }
                }
            }

            // Tab View Router
            div { class: "flex-1",
                match active_tab.read().as_str() {
                    "onboarding" => rsx! {
                        OnboardingWizardView {
                            active_user: active_user.clone(),
                            workspace: workspace.clone(),
                            db_trigger: db_trigger,
                            locale: locale.clone(),
                            on_complete: move |_| active_tab.set("invitations".to_string())
                        }
                    },
                    "invitations" => rsx! {
                        TeamInvitationsView {
                            active_user: active_user.clone(),
                            workspace: workspace.clone(),
                            db_trigger: db_trigger,
                            locale: locale.clone()
                        }
                    },
                    "rbac" => rsx! {
                        RbacManagementView {
                            active_user: active_user.clone(),
                            workspace: workspace.clone(),
                            users: users.clone(),
                            db_trigger: db_trigger,
                            locale: locale.clone()
                        }
                    },
                    "audit" => rsx! {
                        AuditExportsView {
                            active_user: active_user.clone(),
                            workspace: workspace.clone(),
                            db_trigger: db_trigger,
                            locale: locale.clone()
                        }
                    },
                    _ => rsx! {
                        div { class: "p-8 text-center text-muted-foreground text-sm", "Unknown admin tab selection." }
                    }
                }
            }
        }
    }
}
