use dioxus::prelude::*;
use yntra_core::ClientProfile;
use yntra_core::WorkspaceUser;
use yntra_core::{add_journal_entry, add_medication, get_journals, get_medications};
use crate::components;

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
    let mut show_add_med_form = use_signal(|| false);

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
                {
                    let t_previous_notes = crate::locales::t("assistance-journal-previous-notes", locale_ref);
                    let t_journal_no_notes = crate::locales::t("assistance-journal-no-notes", locale_ref);
                    let cid_for_submit = current_client.id.clone();
                    let author_id = active_user.id.clone();

                    rsx! {
                        div { class: "space-y-6 flex flex-col gap-6",
                            
                            // New note form (Caregivers only)
                            if !is_client {
                                JournalEntryForm {
                                    active_user: active_user.clone(),
                                    client_id: cid_for_submit.clone(),
                                    author_id: author_id.clone(),
                                    journal_content,
                                    locale: locale.clone(),
                                }
                            }

                            // Previous notes list
                            div { class: "space-y-4 flex flex-col gap-4",
                                h3 { class: "flex items-center gap-2 text-lg font-bold text-foreground m-0",
                                    "{t_previous_notes}"
                                    span { class: "inline-flex h-6 min-w-6 items-center justify-center rounded-full bg-muted px-2 text-xs font-medium text-muted-foreground",
                                        "{client_journals.len()}"
                                    }
                                }
                                if client_journals.is_empty() {
                                    div { class: "rounded-2xl border border-dashed border-border bg-muted/30 py-12 text-center transition-all hover:bg-muted/50",
                                        p { class: "text-sm font-medium text-muted-foreground m-0", "{t_journal_no_notes}" }
                                    }
                                } else {
                                    div { class: "grid gap-4",
                                        for (entry, author_name) in client_journals.iter().map(|entry| {
                                            let author_name = users
                                                .iter()
                                                .find(|u| Some(u.id.clone()) == entry.author_id)
                                                .and_then(|u| u.full_name.clone())
                                                .unwrap_or_else(|| crate::locales::t("assistance-journal-unknown-author", locale_ref));
                                            (entry, author_name)
                                        }) {
                                            div {
                                                key: "{entry.id}",
                                                class: "group relative overflow-hidden rounded-2xl border border-border bg-card p-5 transition-all hover:border-primary/20 hover:shadow-md flex flex-col gap-3",
                                                div { class: "mb-3 flex items-center justify-between",
                                                    div { class: "flex items-center gap-3",
                                                        div { class: "h-8 w-8 rounded-full bg-primary/10 p-1 flex items-center justify-center",
                                                            div { class: "h-full w-full rounded-full bg-primary/25" }
                                                        }
                                                        div {
                                                            span { class: "block text-sm font-bold text-foreground", "{author_name}" }
                                                            span { class: "text-[10px] font-medium uppercase tracking-tighter text-muted-foreground",
                                                                "{entry.created_at}"
                                                            }
                                                        }
                                                    }
                                                }
                                                div { class: "relative pl-3",
                                                    div { class: "absolute left-0 top-0 h-full w-1 rounded-full bg-primary/10 transition-all group-hover:bg-primary/30" }
                                                    p { class: "whitespace-pre-wrap text-sm leading-relaxed text-foreground/90 m-0",
                                                        "{entry.content}"
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
            } else {
                {
                    let t_med_title = crate::locales::t("assistance-medication-title", locale_ref);
                    let t_med_subtitle = crate::locales::t("assistance-medication-subtitle", locale_ref);
                    let t_med_add = crate::locales::t("assistance-medication-add-button", locale_ref);
                    let t_med_empty_title = crate::locales::t("assistance-medication-empty-state-title", locale_ref);
                    let t_med_empty = crate::locales::t("assistance-medication-empty-state", locale_ref);
                    let t_instructions = crate::locales::t("assistance-medication-instructions-label", locale_ref);
                    let cid_for_med_submit = current_client.id.clone();

                    rsx! {
                        div { class: "space-y-6 flex flex-col gap-6",
                            
                            // Header
                            div { class: "flex items-center justify-between",
                                div {
                                    h3 { class: "text-xl font-bold tracking-tight text-foreground m-0", "{t_med_title}" }
                                    p { class: "text-sm text-muted-foreground m-0 mt-1", "{t_med_subtitle}" }
                                }
                                if !is_client {
                                    button {
                                        onclick: move |_| show_add_med_form.toggle(),
                                        class: "rounded-full bg-primary/10 text-primary hover:bg-primary/20 font-semibold px-4 py-2 border-0 cursor-pointer flex items-center gap-1.5 transition-colors",
                                        components::LucideIcon { name: "directory", class: "h-4 w-4" }
                                        "{t_med_add}"
                                    }
                                }
                            }

                            // Add Medication Form
                            if *show_add_med_form.read() && !is_client {
                                AddMedicationForm {
                                    active_user: active_user.clone(),
                                    client_id: cid_for_med_submit.clone(),
                                    med_name,
                                    med_dosage,
                                    med_frequency,
                                    med_instructions,
                                    show_add_med_form,
                                    locale: locale.clone(),
                                }
                            }

                            // Medications List
                            if client_meds.is_empty() {
                                div { class: "flex flex-col items-center justify-center rounded-3xl border border-dashed border-border bg-card/30 py-20 text-center shadow-inner",
                                    div { class: "mb-6 flex h-20 w-20 items-center justify-center rounded-full bg-primary/5 text-primary/30",
                                        components::LucideIcon { name: "pill", class: "h-10 w-10" }
                                    }
                                    h4 { class: "text-lg font-semibold text-foreground m-0", "{t_med_empty_title}" }
                                    p { class: "mt-1 max-w-[240px] text-sm text-muted-foreground mx-auto m-0", "{t_med_empty}" }
                                }
                            } else {
                                div { class: "grid grid-cols-1 gap-6 sm:grid-cols-2",
                                    for med in client_meds.iter() {
                                        div {
                                            key: "{med.id}",
                                            class: "group relative overflow-hidden rounded-3xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-xl flex flex-col gap-3",
                                            div { class: "absolute -right-4 -top-4 h-24 w-24 rounded-full bg-primary/5 transition-all group-hover:scale-150 pointer-events-none" }
                                            
                                            div { class: "flex items-start gap-4",
                                                div { class: "flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary ring-4 ring-primary/5",
                                                    components::LucideIcon { name: "pill", class: "h-7 w-7" }
                                                }
                                                div { class: "flex-1",
                                                    h4 { class: "text-xl font-extrabold text-foreground m-0", "{med.name}" }
                                                    
                                                    div { class: "mt-4 space-y-2 flex flex-col gap-1.5",
                                                        div { class: "flex items-center gap-2 text-sm text-muted-foreground",
                                                            components::LucideIcon { name: "heart", class: "h-4 w-4 text-primary/60" }
                                                            span { class: "font-medium text-foreground/80",
                                                                if let Some(ref d) = med.dosage { "{d}" } else { "" }
                                                            }
                                                        }
                                                        div { class: "flex items-center gap-2 text-sm text-muted-foreground",
                                                            components::LucideIcon { name: "clock", class: "h-4 w-4 text-primary/60" }
                                                            span { class: "font-medium text-foreground/80",
                                                                if let Some(ref f) = med.frequency { "{f}" } else { "" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            if let Some(ref inst) = med.instructions {
                                                if !inst.trim().is_empty() {
                                                    div { class: "mt-2 rounded-2xl bg-muted/50 p-4 text-xs font-medium text-muted-foreground ring-1 ring-border/50 flex flex-col gap-1",
                                                        span { class: "block uppercase tracking-widest text-primary/70 font-bold",
                                                            "{t_instructions}"
                                                        }
                                                        "{inst}"
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
            }
        }
    }
}

#[derive(Props, Clone)]
struct AddMedicationFormProps {
    active_user: WorkspaceUser,
    client_id: String,
    med_name: Signal<String>,
    med_dosage: Signal<String>,
    med_frequency: Signal<String>,
    med_instructions: Signal<String>,
    show_add_med_form: Signal<bool>,
    locale: String,
}

impl PartialEq for AddMedicationFormProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
fn AddMedicationForm(props: AddMedicationFormProps) -> Element {
    let active_user = props.active_user;
    let client_id = props.client_id;
    let mut show_add_med_form = props.show_add_med_form;
    let mut med_name = props.med_name;
    let mut med_dosage = props.med_dosage;
    let mut med_frequency = props.med_frequency;
    let mut med_instructions = props.med_instructions;
    let locale_ref = &props.locale;
    rsx! {
        div { class: "rounded-2xl border border-border bg-card p-5 flex flex-col gap-3",
            h4 { class: "text-sm font-bold text-foreground m-0", "{crate::locales::t(\"assistance-medication-add-title\", locale_ref)}" }
            div { class: "flex flex-col gap-2",
                input {
                    class: "yntra-input",
                    value: "{med_name}",
                    placeholder: crate::locales::t("assistance-medication-name-placeholder", locale_ref),
                    oninput: move |e| med_name.set(e.value()),
                }
                input {
                    class: "yntra-input",
                    value: "{med_dosage}",
                    placeholder: crate::locales::t("assistance-medication-dosage-placeholder", locale_ref),
                    oninput: move |e| med_dosage.set(e.value()),
                }
                input {
                    class: "yntra-input",
                    value: "{med_frequency}",
                    placeholder: crate::locales::t("assistance-medication-frequency-placeholder", locale_ref),
                    oninput: move |e| med_frequency.set(e.value()),
                }
                input {
                    class: "yntra-input",
                    value: "{med_instructions}",
                    placeholder: crate::locales::t("assistance-medication-instructions-placeholder", locale_ref),
                    oninput: move |e| med_instructions.set(e.value()),
                }
                div { class: "flex justify-end gap-2 mt-2",
                    button {
                        onclick: move |_| show_add_med_form.set(false),
                        class: "yntra-btn secondary",
                        "{crate::locales::t(\"common-cancel\", locale_ref)}"
                    }
                    button {
                        disabled: med_name.read().trim().is_empty(),
                        onclick: move |_| {
                            let name = med_name.read().trim().to_string();
                            let dosage = med_dosage.read().clone();
                            let frequency = med_frequency.read().clone();
                            let instructions = med_instructions.read().clone();
                            if !name.is_empty() {
                                let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                let cid = client_id.clone();
                                let actor_id = active_user.id.clone();
                                spawn(async move {
                                    let _ = add_medication(
                                        workspace_id,
                                        cid,
                                        actor_id,
                                        name,
                                        dosage,
                                        frequency,
                                        instructions,
                                    ).await;
                                });
                                med_name.set(String::new());
                                med_dosage.set(String::new());
                                med_frequency.set(String::new());
                                med_instructions.set(String::new());
                                show_add_med_form.set(false);
                            }
                        },
                        class: "yntra-btn",
                        "{crate::locales::t(\"common-save-btn\", locale_ref)}"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
struct JournalEntryFormProps {
    active_user: WorkspaceUser,
    client_id: String,
    author_id: String,
    journal_content: Signal<String>,
    locale: String,
}

impl PartialEq for JournalEntryFormProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
fn JournalEntryForm(props: JournalEntryFormProps) -> Element {
    let active_user = props.active_user;
    let client_id = props.client_id;
    let author_id = props.author_id;
    let mut journal_content = props.journal_content;
    let locale_ref = &props.locale;
    let t_write_new_note = crate::locales::t("assistance-journal-write-new-note", locale_ref);
    let t_placeholder = crate::locales::t("assistance-journal-placeholder", locale_ref);
    let t_save_note = crate::locales::t("assistance-journal-save-note", locale_ref);

    rsx! {
        div {
            class: "group relative overflow-hidden rounded-2xl border border-border bg-card p-5 transition-all hover:shadow-lg flex flex-col gap-3",
            div { class: "absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-50 pointer-events-none" }
            h3 { class: "relative m-0 flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-muted-foreground",
                components::LucideIcon { name: "messaging", class: "h-4 w-4 text-primary" }
                "{t_write_new_note}"
            }
            textarea {
                value: "{journal_content}",
                oninput: move |e| journal_content.set(e.value()),
                class: "relative min-h-[120px] w-full resize-none rounded-xl border border-border bg-background/50 p-4 text-sm transition-all focus:border-primary/50 focus:outline-none focus:ring-4 focus:ring-primary/10",
                placeholder: "{t_placeholder}",
            }
            div { class: "relative flex justify-end",
                button {
                    disabled: journal_content.read().trim().is_empty(),
                    onclick: move |_| {
                        let text = journal_content.read().trim().to_string();
                        if !text.is_empty() {
                            let workspace_id = active_user.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                            let cid = client_id.clone();
                            let aid = author_id.clone();
                            spawn(async move {
                                let _ = add_journal_entry(
                                    workspace_id,
                                    cid,
                                    aid,
                                    text,
                                ).await;
                            });
                            journal_content.set(String::new());
                        }
                    },
                    class: "rounded-full bg-primary hover:bg-primary/95 text-primary-foreground font-semibold px-6 py-2 cursor-pointer transition-all border-0 shadow-sm disabled:opacity-50",
                    "{t_save_note}"
                }
            }
        }
    }
}
