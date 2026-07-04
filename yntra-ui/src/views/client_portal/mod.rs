use dioxus::prelude::*;
use yntra_core::ClientProfile;
use yntra_core::TeamEvent;
use yntra_core::WorkspaceUser;
use yntra_core::Workspace;

pub mod care;
pub mod moving;
pub mod general;

pub use care::CarePortal;
pub use moving::MovingPortal;
pub use general::GeneralPortal;

#[derive(Props, Clone)]
pub struct ClientPortalViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub clients: Vec<ClientProfile>,
    pub events: Vec<TeamEvent>,
    pub db_trigger: Signal<u32>,
    pub workspace: Workspace,
}

impl PartialEq for ClientPortalViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ClientPortalView(props: ClientPortalViewProps) -> Element {
    let active_user = props.active_user;
    let users = props.users.clone();
    let clients = props.clients.clone();
    let events = props.events.clone();
    let db_trigger = props.db_trigger;
    let workspace = props.workspace.clone();

    // Parse the template from workspace settings
    let settings_val: serde_json::Value = serde_json::from_str(&workspace.settings).unwrap_or_default();
    let template = settings_val.get("template").and_then(|v| v.as_str()).unwrap_or("care").to_string();

    let current_client = clients
        .iter()
        .find(|c| {
            if let Some(ref name) = active_user.full_name {
                name.contains(&c.first_name) && name.contains(&c.last_name)
            } else {
                false
            }
        })
        .cloned()
        .unwrap_or_else(|| {
            clients
                .iter()
                .find(|c| c.id == "client-1")
                .cloned()
                .unwrap_or_else(|| ClientProfile {
                    id: "client-1".to_string(),
                    workspace_id: "workspace-1".to_string(),
                    team_id: Some("team-1".to_string()),
                    first_name: "Sven".to_string(),
                    last_name: "Svensson".to_string(),
                    personal_number: Some("19450312-1234".to_string()),
                    care_level: Some("High Care".to_string()),
                    message_settings: "{}".to_string(),
                    created_at: "2026-06-30 00:00:00".to_string(),
                    updated_at: 0,
                    sync_status: "synced".to_string(),
                })
        });

    let client_id = current_client.id.clone();
    let client_name = format!("{} {}", current_client.first_name, current_client.last_name);

    let client_user_id = active_user.id.clone();
    
    // Filter visits for today (Care template)
    let todays_events: Vec<TeamEvent> = events
        .iter()
        .filter(|ev| ev.user_id == Some(client_user_id.clone()))
        .cloned()
        .collect();

    rsx! {
        // Main Container matching mx-auto w-full max-w-5xl space-y-8
        div { class: "mx-auto w-full max-w-5xl space-y-8 p-6 flex flex-col gap-8",
            
            // 1. Welcome Header (Template dependent)
            div { class: "flex flex-col gap-2 md:flex-row md:items-center md:justify-between animate-in fade-in slide-in-from-top-2 duration-300",
                div {
                    h1 { class: "bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text text-3xl font-bold tracking-tight text-transparent m-0",
                        "Välkommen tillbaka, {client_name}"
                    }
                    p { class: "text-sm text-muted-foreground m-0 mt-1",
                        if template == "moving" {
                            "Här är din flyttplanering, offertstatus och bokningsdetaljer."
                        } else if template == "general" {
                            "Här är en översikt av dina beställda uppdrag."
                        } else {
                            if let Some(ref care) = current_client.care_level {
                                "Vårdnivå: {care}"
                            } else {
                                "Här är en översikt av din vård och assistans."
                            }
                        }
                    }
                }
            }

            if template == "moving" {
                MovingPortal {
                    active_user_id: active_user.id.clone(),
                    db_trigger
                }
            } else if template == "general" {
                GeneralPortal {
                    active_user_id: active_user.id.clone(),
                    db_trigger
                }
            } else {
                CarePortal {
                    client_id,
                    active_user,
                    users,
                    todays_events,
                    db_trigger
                }
            }
        }
    }
}
