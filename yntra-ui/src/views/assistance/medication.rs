use crate::components;
use dioxus::prelude::*;
use yntra_core::{WorkspaceUser, ClientProfile, MedicationItem, add_medication};

#[derive(Props, Clone, PartialEq)]
pub struct MedicationTabContentProps {
    pub active_user: WorkspaceUser,
    pub current_client: ClientProfile,
    pub client_meds: Vec<MedicationItem>,
    pub med_name: Signal<String>,
    pub med_dosage: Signal<String>,
    pub med_frequency: Signal<String>,
    pub med_instructions: Signal<String>,
    pub show_add_med_form: Signal<bool>,
    pub is_client: bool,
    pub locale: String,
}

#[component]
pub fn MedicationTabContent(props: MedicationTabContentProps) -> Element {
    let active_user = props.active_user;
    let current_client = props.current_client;
    let client_meds = props.client_meds;
    let med_name = props.med_name;
    let med_dosage = props.med_dosage;
    let med_frequency = props.med_frequency;
    let med_instructions = props.med_instructions;
    let mut show_add_med_form = props.show_add_med_form;
    let is_client = props.is_client;
    let locale = props.locale;

    let locale_ref = &locale;
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

#[derive(Props, Clone, PartialEq)]
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
