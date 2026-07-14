use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, add_client_via_directory, add_team_via_directory};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WizardClientForm {
    pub first_name: String,
    pub last_name: String,
    pub personal_number: String,
    pub care_level: String,
    pub contact_person_email: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct TeamRegistrationWizardProps {
    pub show_add_team_modal: Signal<bool>,
    pub new_team_name: Signal<String>,
    pub active_user: WorkspaceUser,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub region: String,
}

#[component]
pub fn TeamRegistrationWizard(props: TeamRegistrationWizardProps) -> Element {
    let active_user = props.active_user;
    let ws_id = props.workspace_id;
    let region = props.region;

    let mut show_add_team_modal = props.show_add_team_modal;
    let mut new_team_name = props.new_team_name;
    let db_trigger = props.db_trigger;

    // Local wizard signals
    let mut wizard_step = use_signal(|| 1);
    let mut wizard_clients = use_signal(Vec::<WizardClientForm>::new);
    let mut wizard_client_first = use_signal(String::new);
    let mut wizard_client_last = use_signal(String::new);
    let mut wizard_client_ssn = use_signal(String::new);
    let mut wizard_client_level = use_signal(|| "High Care".to_string());
    let mut wizard_client_email = use_signal(String::new);

    let step1_bg = if *wizard_step.read() == 1 {
        "var(--accent-color)"
    } else {
        "var(--border-color)"
    };
    let step2_bg = if *wizard_step.read() == 2 {
        "var(--accent-color)"
    } else {
        "var(--border-color)"
    };

    rsx! {
        if *show_add_team_modal.read() {
            components::Dialog {
                open: *show_add_team_modal.read(),
                title: if *wizard_step.read() == 1 { t("wizard-create-team-title", &region) } else { t("wizard-register-patients-title", &region) },
                onclose: move |_| {
                    show_add_team_modal.set(false);
                    wizard_step.set(1);
                    new_team_name.set(String::new());
                    wizard_clients.set(Vec::new());
                },
                div { class: "flex flex-col gap-5 w-full text-sm",
                style: "max-width:440px;",
                    // Progress indicators
                    div { class: "flex justify-between items-center border-b border-border pb-3",
                        span { class: "text-xs font-bold text-muted-foreground/60",
                            if *wizard_step.read() == 1 { "{t(\"wizard-step-1\", &region)}" } else { "{t(\"wizard-step-2\", &region)}" }
                        }
                        div { class: "flex",
                        style: "gap:0.35rem;",
                            div {
                                class: "rounded-full",
                                style: "width:8px; height:8px; background: {step1_bg};",}
                            div {
                                class: "rounded-full",
                                style: "width:8px; height:8px; background: {step2_bg};",}
                        }
                    }

                    if *wizard_step.read() == 1 {
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground", "Team Name" }
                            input {
                                class: "yntra-input",
                                value: "{new_team_name}",
                                placeholder: "e.g. Care Team Stockholm East",
                                oninput: move |e| new_team_name.set(e.value()),
                            }
                        }
                        div { class: "flex justify-end gap-2 mt-2",
                            button {
                                class: "yntra-btn secondary",
                                onclick: move |_| {
                                    show_add_team_modal.set(false);
                                    new_team_name.set(String::new());
                                },
                                "{t(\"wizard-action-cancel\", &region)}"
                            }
                            button {
                                class: "yntra-btn",
                                disabled: new_team_name.read().trim().is_empty(),
                                onclick: move |_| wizard_step.set(2),
                                "{t(\"wizard-action-next\", &region)}"
                            }
                        }
                    } else {
                        // Added Clients List
                        if !wizard_clients.read().is_empty() {
                            div { class: "flex flex-col gap-2 overflow-y-auto border border-border p-2 bg-white/[0.01]",
                            style: "max-height:120px; border-radius:6px;",
                                div { class: "text-[11px] font-bold text-muted-foreground/60 uppercase", "{t(\"wizard-added-patients\", &region)} ({wizard_clients.read().len()})" }
                                for (idx, c) in wizard_clients.read().iter().enumerate() {
                                    {
                                        let c_name = format!("{} {}", c.first_name, c.last_name);
                                        rsx! {
                                            div {
                                                key: "{idx}",
                                                class: "flex justify-between items-center text-xs bg-white/[0.02]",
                                                style: "padding:0.25rem 0.5rem; border-radius:4px;",
                                                span { class: "font-semibold",
                                                style: "color:var(--text-main);", "{c_name}" }
                                                button {
                                                    class: "bg-transparent border-0 cursor-pointer text-xs",
                                                    style: "color:var(--danger);",
                                                    onclick: move |_| {
                                                        let mut vec = wizard_clients.read().clone();
                                                        vec.remove(idx);
                                                        wizard_clients.set(vec);
                                                    },
                                                    "{t(\"common-delete\", &region)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Client Form fields
                        div { class: "flex flex-col gap-3.5 border border-dashed border-border p-4 rounded-lg bg-white/[0.01]",
                            div { class: "flex justify-between items-center",
                                div { class: "text-xs font-bold",
                                style: "color:var(--text-main);", "{t(\"wizard-register-patient\", &region)}" }
                                button {
                                    class: "yntra-btn secondary text-[11px]",
                                    style: "padding:0.2rem 0.5rem;",
                                    disabled: wizard_client_first.read().trim().is_empty() || wizard_client_last.read().trim().is_empty(),
                                    onclick: move |_| {
                                        let c = WizardClientForm {
                                            first_name: wizard_client_first.read().trim().to_string(),
                                            last_name: wizard_client_last.read().trim().to_string(),
                                            personal_number: wizard_client_ssn.read().trim().to_string(),
                                            care_level: wizard_client_level.read().clone(),
                                            contact_person_email: wizard_client_email.read().trim().to_string(),
                                        };
                                        let mut vec = wizard_clients.read().clone();
                                        vec.push(c);
                                        wizard_clients.set(vec);
                                        wizard_client_first.set(String::new());
                                        wizard_client_last.set(String::new());
                                        wizard_client_ssn.set(String::new());
                                        wizard_client_email.set(String::new());
                                    },
                                    "+ {t(\"wizard-action-add\", &region)}"
                                }
                            }
                            div { class: "grid gap-3",
                            style: "grid-template-columns: 1fr 1fr;",
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-xs text-muted-foreground", "First Name" }
                                    input {
                                        class: "yntra-input",
                                        value: "{wizard_client_first}",
                                        oninput: move |e| wizard_client_first.set(e.value()),
                                    }
                                }
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-xs text-muted-foreground", "Last Name" }
                                    input {
                                        class: "yntra-input",
                                        value: "{wizard_client_last}",
                                        oninput: move |e| wizard_client_last.set(e.value()),
                                    }
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs text-muted-foreground", "Personal Number (SSN)" }
                                input {
                                    class: "yntra-input",
                                    value: "{wizard_client_ssn}",
                                    placeholder: "19481105-4321",
                                    oninput: move |e| wizard_client_ssn.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs text-muted-foreground", "Care Level Class" }
                                select {
                                    class: "yntra-input",
                                    value: "{wizard_client_level}",
                                    onchange: move |e| wizard_client_level.set(e.value()),
                                    option { value: "High Care", "High Care Class" }
                                    option { value: "Medium Care", "Medium Care Class" }
                                    option { value: "Low Care", "Low Care Class" }
                                }
                            }
                        }

                        div { class: "flex justify-between gap-2 mt-2",
                            button {
                                class: "yntra-btn secondary",
                                onclick: move |_| wizard_step.set(1),
                                "{t(\"wizard-action-back\", &region)}"
                            }
                            div { class: "flex gap-2",
                                button {
                                    class: "yntra-btn secondary",
                                    onclick: move |_| {
                                        show_add_team_modal.set(false);
                                        wizard_step.set(1);
                                        new_team_name.set(String::new());
                                        wizard_clients.set(Vec::new());
                                    },
                                    "{t(\"wizard-action-cancel\", &region)}"
                                }
                                button {
                                    class: "yntra-btn text-white",
                                    style: "background: var(--accent-color); border-color: var(--accent-color);",
                                    onclick: move |_| {
                                        let name = new_team_name.read().trim().to_string();
                                        if !name.is_empty() {
                                            let ws_id_clone = ws_id.clone();
                                            let mut list = wizard_clients.read().clone();
                                            let current_first = wizard_client_first.read().trim().to_string();
                                            let current_last = wizard_client_last.read().trim().to_string();
                                            let current_ssn = wizard_client_ssn.read().trim().to_string();
                                            let current_level = wizard_client_level.read().clone();
                                            let mut show_add_team = show_add_team_modal;
                                            let mut w_step = wizard_step;
                                            let mut new_team = new_team_name;
                                            let mut w_clients = wizard_clients;
                                            let mut trigger = db_trigger;
                                            let requester_uid_outside = active_user.id.clone();

                                            spawn(async move {
                                                if let Ok(team) = add_team_via_directory(requester_uid_outside.clone(), ws_id_clone.clone(), name).await {
                                                    let team_id = team.id;
                                                    if !current_first.is_empty() && !current_last.is_empty() {
                                                        list.push(WizardClientForm {
                                                            first_name: current_first,
                                                            last_name: current_last,
                                                            personal_number: current_ssn,
                                                            care_level: current_level,
                                                            contact_person_email: String::new(),
                                                        });
                                                    }
                                                    let requester_uid = requester_uid_outside.clone();
                                                    for c in list.iter() {
                                                        let _ = add_client_via_directory(
                                                            requester_uid.clone(),
                                                            ws_id_clone.clone(),
                                                            Some(team_id.clone()),
                                                            c.first_name.clone(),
                                                            c.last_name.clone(),
                                                            c.personal_number.clone(),
                                                            c.care_level.clone(),
                                                        ).await;
                                                    }
                                                }
                                                show_add_team.set(false);
                                                w_step.set(1);
                                                new_team.set(String::new());
                                                w_clients.set(Vec::new());
                                                let current = *trigger.read();
                                                trigger.set(current + 1);
                                            });
                                        }
                                    },
                                    "{t(\"wizard-action-save\", &region)}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
