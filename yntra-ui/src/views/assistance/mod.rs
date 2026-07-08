use dioxus::prelude::*;
use yntra_core::ClientProfile;
use yntra_core::WorkspaceUser;
use yntra_core::{get_journals, get_medications};
use crate::components;

mod journal;
mod medication;

#[derive(Props, Clone)]
pub struct AssistanceViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub clients: Vec<ClientProfile>,
    pub selected_client_id: Signal<String>,
    pub med_name: Signal<String>,
    pub med_dosage: Signal<String>,
    pub med_frequency: Signal<String>,
    pub med_instructions: Signal<String>,
    pub journal_content: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub is_client: bool,
    pub locale: String,
    pub journals_enabled: bool,
    pub medications_enabled: bool,
    pub initial_tab: Option<String>,
}

impl PartialEq for AssistanceViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn AssistanceView(props: AssistanceViewProps) -> Element {
    let active_user = props.active_user;
    let users = props.users.clone();
    let clients = props.clients.clone();
    let is_client = props.is_client;
    let locale = props.locale.clone();

    let selected_client_id = props.selected_client_id;
    let med_name = props.med_name;
    let med_dosage = props.med_dosage;
    let med_frequency = props.med_frequency;
    let med_instructions = props.med_instructions;
    let journal_content = props.journal_content;
    let db_trigger = props.db_trigger;

    let locale_ref = &locale;

    if clients.is_empty() {
        let t_no_client_linked = crate::locales::t("assistance-no-client-linked", locale_ref);
        let t_no_client_description = crate::locales::t("assistance-no-client-description", locale_ref);
        return rsx! {
            div { class: "mx-auto flex h-full max-w-4xl flex-col items-center justify-center p-8 text-center",
                components::LucideIcon { name: "directory", class: "mb-4 h-16 w-16 text-muted-foreground/30" }
                h2 { class: "mb-2 text-xl font-bold", "{t_no_client_linked}" }
                p { class: "mx-auto max-w-md text-muted-foreground", "{t_no_client_description}" }
            }
        };
    }

    let client_id = selected_client_id.read().clone();
    let current_client = clients
        .iter()
        .find(|c| c.id == client_id)
        .cloned()
        .unwrap_or_else(|| {
            clients.first().cloned().unwrap_or_else(|| ClientProfile {
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

    let client_id_for_meds = current_client.id.clone();
    let actor_id_for_meds = active_user.id.clone();
    let meds_res = use_resource(move || {
        let _trig = db_trigger.read();
        let cid = client_id_for_meds.clone();
        let aid = actor_id_for_meds.clone();
        async move { get_medications(cid, aid).await.unwrap_or_default() }
    });

    let client_id_for_journals = current_client.id.clone();
    let actor_id_for_journals = active_user.id.clone();
    let journals_res = use_resource(move || {
        let _trig = db_trigger.read();
        let cid = client_id_for_journals.clone();
        let aid = actor_id_for_journals.clone();
        async move { get_journals(cid, aid).await.unwrap_or_default() }
    });

    let client_meds = meds_res.read().clone().unwrap_or_default();
    let client_journals = journals_res.read().clone().unwrap_or_default();

    let initial_val = match props.initial_tab.as_deref() {
        Some("medication") if props.medications_enabled => "medication".to_string(),
        Some("journal") if props.journals_enabled => "journal".to_string(),
        _ => if props.journals_enabled { "journal".to_string() } else { "medication".to_string() }
    };
    let mut active_tab = use_signal(|| initial_val);
    let show_add_med_form = use_signal(|| false);

    let client_initials = {
        let f = current_client.first_name.chars().next().unwrap_or(' ').to_string().to_uppercase();
        let l = current_client.last_name.chars().next().unwrap_or(' ').to_string().to_uppercase();
        format!("{}{}", f, l)
    };
    let client_fullname = format!("{} {}", current_client.first_name, current_client.last_name);

    let t_personal_number = crate::locales::t("assistance-personal-number", locale_ref);
    let t_care_level = crate::locales::t("assistance-care-level", locale_ref);
    let t_daily_notes = crate::locales::t("assistance-daily-notes", locale_ref);
    let t_active_medication_list = crate::locales::t("assistance-active-medication-list", locale_ref);

    rsx! {
        div { class: "mx-auto w-full max-w-5xl flex-1 p-8 flex flex-col gap-6",
            
            // Client Profile Card
            div { class: "mb-6 flex items-start gap-6 rounded-xl border border-border bg-sidebar p-6 shadow-sm",
                div { class: "flex h-20 w-20 items-center justify-center rounded-[14px] bg-gradient-to-br from-primary to-primary/60 text-3xl font-bold text-primary-foreground shadow-inner select-none",
                    "{client_initials}"
                }
                div {
                    h1 { class: "text-2xl font-bold m-0", "{client_fullname}" }
                    p { class: "mt-1 text-sm font-medium text-muted-foreground m-0",
                        "{t_personal_number} {current_client.personal_number.clone().unwrap_or_default()}"
                    }
                    div { class: "mt-4 flex gap-2",
                        span { class: "rounded-full border border-border bg-secondary px-3 py-1 text-[11px] font-bold uppercase tracking-wider text-muted-foreground",
                            "{t_care_level} {current_client.care_level.clone().unwrap_or_default()}"
                        }
                    }
                }
            }

            // Tabs navigation
            if props.journals_enabled && props.medications_enabled {
                div { class: "mb-6 flex gap-2 border-b border-border",
                    button {
                        onclick: move |_| active_tab.set("journal".to_string()),
                        class: if *active_tab.read() == "journal" {
                            "border-b-2 border-primary text-foreground px-4 py-2 text-sm font-medium transition-colors bg-transparent border-t-0 border-l-0 border-r-0 cursor-pointer pb-2"
                        } else {
                            "border-b-2 border-transparent text-muted-foreground hover:text-foreground px-4 py-2 text-sm font-medium transition-colors bg-transparent border-t-0 border-l-0 border-r-0 cursor-pointer pb-2"
                        },
                        div { class: "flex items-center gap-2",
                            components::LucideIcon { name: "notes", class: "h-4 w-4" }
                            "{t_daily_notes}"
                        }
                    }
                    button {
                        onclick: move |_| active_tab.set("medication".to_string()),
                        class: if *active_tab.read() == "medication" {
                            "border-b-2 border-primary text-foreground px-4 py-2 text-sm font-medium transition-colors bg-transparent border-t-0 border-l-0 border-r-0 cursor-pointer pb-2"
                        } else {
                            "border-b-2 border-transparent text-muted-foreground hover:text-foreground px-4 py-2 text-sm font-medium transition-colors bg-transparent border-t-0 border-l-0 border-r-0 cursor-pointer pb-2"
                        },
                        div { class: "flex items-center gap-2",
                            components::LucideIcon { name: "pill", class: "h-4 w-4" }
                            "{t_active_medication_list}"
                        }
                    }
                }
            }

            // Tab contents
            if *active_tab.read() == "journal" {
                journal::JournalTabContent {
                    active_user: active_user.clone(),
                    current_client: current_client.clone(),
                    client_journals: client_journals,
                    users: users,
                    journal_content: journal_content,
                    is_client: is_client,
                    locale: locale.clone(),
                }
            } else {
                medication::MedicationTabContent {
                    active_user: active_user.clone(),
                    current_client: current_client.clone(),
                    client_meds: client_meds,
                    med_name: med_name,
                    med_dosage: med_dosage,
                    med_frequency: med_frequency,
                    med_instructions: med_instructions,
                    show_add_med_form: show_add_med_form,
                    is_client: is_client,
                    locale: locale.clone(),
                }
            }
        }
    }
}
