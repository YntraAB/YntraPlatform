use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::Workspace;
use crate::components;

pub mod moving;
pub mod general;

pub use moving::MovingPortal;
pub use general::GeneralPortal;

#[derive(Props, Clone)]
pub struct ClientPortalViewProps {
    pub active_user: WorkspaceUser,
    pub db_trigger: Signal<u32>,
    pub trigger_jobs: Signal<u32>,
    pub workspace: Workspace,
}

impl PartialEq for ClientPortalViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ClientPortalView(props: ClientPortalViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user = props.active_user;
    let clients = state.clients.read().clone().unwrap_or_default();
    let trigger_jobs = props.trigger_jobs;
    let workspace = props.workspace.clone();

    // Parse the template from workspace settings
    let settings_val: serde_json::Value = serde_json::from_str(&workspace.settings).unwrap_or_default();
    let template = settings_val.get("template").and_then(|v| v.as_str()).unwrap_or("general").to_string();

    let current_client = clients
        .iter()
        .find(|c| {
            if let Some(ref pn) = c.personal_number
                && let Some(ref active_pn) = active_user.personal_number {
                    pn == active_pn
                } else {
                    false
                }
        })
        .cloned();

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            
            // Header / Greeting Card
            components::Card {
                class: "relative overflow-hidden border border-border/80 bg-sidebar p-6 shadow-md rounded-2xl",
                div { class: "absolute right-0 top-0 -mr-16 -mt-16 h-48 w-48 rounded-full bg-primary/5 blur-3xl" }
                div { class: "relative flex flex-col gap-4 md:flex-row md:items-center md:justify-between",
                    div { class: "space-y-1.5",
                        h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0",
                            "Välkommen, {active_user.full_name.clone().unwrap_or_else(|| \"Kund\".to_string())}!"
                        }
                        p { class: "text-xs text-muted-foreground font-semibold mt-0.5 tracking-wide m-0",
                            if template == "moving" {
                                "Här är en översikt av dina beställda uppdrag."
                            } else {
                                if let Some(ref client) = current_client
                                    && let Some(ref care) = client.care_level {
                                        "Vårdnivå: {care}"
                                } else {
                                    "Här är en översikt av din portal."
                                }
                            }
                        }
                    }
                }
            }

            if template == "moving" {
                MovingPortal {
                    active_user_id: active_user.id.clone(),
                    db_trigger: trigger_jobs
                }
            } else {
                GeneralPortal {
                    active_user_id: active_user.id.clone(),
                    db_trigger: trigger_jobs
                }
            }
        }
    }
}
