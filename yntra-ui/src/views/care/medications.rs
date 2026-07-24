use crate::components::{Button, Card, Dialog, Input, LucideIcon};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{get_clients, get_medications, add_medication, check_care_permission};
use super::CareViewProps;

#[component]
pub fn MedicationsView(props: CareViewProps) -> Element {
    let mut db_trigger = props.db_trigger;
    let user_id = props.active_user_id.clone();
    let region = props.locale.clone();

    let mut selected_client_id = use_signal(|| "".to_string());
    let mut show_add_med_modal = use_signal(|| false);
    
    // Add medication form signals
    let mut new_med_name = use_signal(String::new);
    let mut new_med_dosage = use_signal(String::new);
    let mut new_med_frequency = use_signal(String::new);
    let mut new_med_instructions = use_signal(String::new);
    let mut save_error = use_signal(|| Option::<String>::None);

    let db_trig_val = *db_trigger.read();
    let user_id_clone = user_id.clone();
    let clients_res = use_resource(move || {
        let _ = db_trig_val;
        let uid = user_id_clone.clone();
        async move { get_clients(uid).await.unwrap_or_default() }
    });

    let active_client = selected_client_id.read().clone();
    let user_id_clone2 = user_id.clone();
    let meds_res = use_resource(move || {
        let _ = db_trig_val;
        let c_id = active_client.clone();
        let uid = user_id_clone2.clone();
        async move {
            if c_id.is_empty() {
                Vec::new()
            } else {
                get_medications(c_id, uid).await.unwrap_or_default()
            }
        }
    });

    let user_id_clone3 = user_id.clone();
    let can_manage_res = use_resource(move || {
        let uid = user_id_clone3.clone();
        async move {
            check_care_permission(uid, "can_manage_medications".to_string())
                .await
                .unwrap_or(false)
        }
    });

    let clients = clients_res.read().clone().unwrap_or_default();
    let medications = meds_res.read().clone().unwrap_or_default();
    let can_manage = can_manage_res.read().cloned().unwrap_or(false);

    let selected_client = clients.iter().find(|c| c.id == *selected_client_id.read()).cloned();

    rsx! {
        div { class: "p-6 space-y-6 max-w-5xl mx-auto animate-in fade-in slide-in-from-top-4 duration-300",
            div { class: "border-b border-border pb-4 mb-6",
                h2 { class: "text-2xl font-bold tracking-tight text-foreground m-0 flex items-center gap-2",
                    LucideIcon { name: "activity", class: "h-6 w-6 text-primary" }
                    "{t(\"assistance-medication-title\", &region)}"
                }
                p { class: "text-xs text-muted-foreground m-0 mt-1", "{t(\"assistance-medication-subtitle\", &region)}" }
            }

            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                // Left Panel: Client List
                Card { class: "border border-border p-4 bg-sidebar/50 md:col-span-1 h-[600px] flex flex-col",
                    h3 { class: "text-sm font-bold text-foreground mb-3", "{t(\"assistance-clients-list-title\", &region)}" }
                    if clients.is_empty() {
                        div { class: "flex-1 flex flex-col items-center justify-center text-center p-4 border border-dashed border-border rounded-xl",
                            LucideIcon { name: "users", class: "h-8 w-8 text-muted-foreground/30 mb-2" }
                            span { class: "text-xs text-muted-foreground", "{t(\"assistance-no-client-linked\", &region)}" }
                        }
                    } else {
                        div { class: "flex-1 overflow-y-auto space-y-2 pr-1",
                            for c in clients.iter() {
                                {
                                    let c_id = c.id.clone();
                                    let is_selected = c_id == *selected_client_id.read();
                                    let bg_class = if is_selected { "bg-primary/10 border-primary" } else { "hover:bg-muted/50 border-border" };
                                    rsx! {
                                        div {
                                            key: "{c_id}",
                                            class: "p-3 border rounded-xl cursor-pointer transition-all {bg_class}",
                                            onclick: move |_| {
                                                selected_client_id.set(c_id.clone());
                                            },
                                            div { class: "font-bold text-xs text-foreground", "{c.first_name} {c.last_name}" }
                                            if let Some(level) = &c.care_level {
                                                div { class: "text-[10px] text-muted-foreground mt-1", "{t(\"assistance-care-level\", &region)} {level}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Panel: Medication Items
                div { class: "md:col-span-2 h-[600px] flex flex-col",
                    if selected_client_id.read().is_empty() {
                        Card { class: "flex-1 border border-border p-6 flex flex-col items-center justify-center text-center",
                            LucideIcon { name: "activity", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                            h4 { class: "text-sm font-bold text-foreground m-0", "{t(\"assistance-no-client-selected\", &region)}" }
                            p { class: "text-xs text-muted-foreground mt-1 max-w-xs", "{t(\"assistance-select-client-prompt\", &region)}" }
                        }
                    } else {
                        Card { class: "flex-1 border border-border p-6 flex flex-col overflow-hidden bg-card/30",
                            div { class: "flex justify-between items-center border-b border-border pb-4 mb-4",
                                div {
                                    if let Some(c) = &selected_client {
                                        h3 { class: "text-sm font-bold text-foreground m-0", "{c.first_name} {c.last_name}" }
                                        if let Some(pn) = &c.personal_number {
                                            p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", "{t(\"assistance-personal-number\", &region)} {pn}" }
                                        }
                                    }
                                }
                                if can_manage {
                                    Button {
                                        class: "flex items-center gap-1.5 text-xs h-8 px-3 rounded-lg",
                                        onclick: move |_| {
                                            new_med_name.set(String::new());
                                            new_med_dosage.set(String::new());
                                            new_med_frequency.set(String::new());
                                            new_med_instructions.set(String::new());
                                            save_error.set(None);
                                            show_add_med_modal.set(true);
                                        },
                                        LucideIcon { name: "plus", class: "h-3.5 w-3.5" }
                                        "{t(\"assistance-medication-add-button\", &region)}"
                                    }
                                }
                            }

                            // Medications List
                            div { class: "flex-1 overflow-y-auto space-y-4 pr-1",
                                if medications.is_empty() {
                                    div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl bg-muted/5",
                                        "{t(\"assistance-medication-empty-state\", &region)}"
                                    }
                                } else {
                                    div { class: "grid gap-3 grid-cols-1 sm:grid-cols-2",
                                        for m in medications.iter() {
                                            div {
                                                key: "{m.id}",
                                                class: "p-4 border border-border rounded-xl bg-background shadow-sm hover:shadow-md transition-shadow flex flex-col justify-between",
                                                div {
                                                    div { class: "font-bold text-xs text-foreground", "{m.name}" }
                                                    if let Some(ds) = &m.dosage {
                                                        div { class: "text-[10px] text-muted-foreground mt-1",
                                                            span { class: "font-semibold", "{t(\"assistance-medication-dosage-label\", &region)} " }
                                                            "{ds}"
                                                        }
                                                    }
                                                    if let Some(fq) = &m.frequency {
                                                        div { class: "text-[10px] text-muted-foreground mt-0.5",
                                                            span { class: "font-semibold", "{t(\"care-frequency-prefix\", &region)}" }
                                                            "{fq}"
                                                        }
                                                    }
                                                }
                                                if let Some(ins) = &m.instructions {
                                                    div { class: "mt-3 pt-2 border-t border-border/60 text-[10px] text-muted-foreground",
                                                        span { class: "font-semibold block text-[9px] uppercase tracking-wider mb-0.5 text-primary", "{t(\"assistance-medication-instructions-label\", &region)}" }
                                                        "{ins}"
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

            // Add Medication Modal
            if *show_add_med_modal.read() {
                Dialog {
                    open: *show_add_med_modal.read(),
                    title: t("assistance-medication-add-title", &region),
                    max_width: "500px".to_string(),
                    onclose: move |_| show_add_med_modal.set(false),

                    div { class: "flex flex-col gap-4 text-xs",
                        if let Some(err) = save_error.read().as_ref() {
                            div { class: "p-2 border border-destructive/20 bg-destructive/10 rounded-lg text-destructive",
                                "{err}"
                            }
                        }
                        div { class: "grid gap-1.5",
                            label { class: "font-bold text-muted-foreground uppercase", "{t(\"care-medication-name\", &region)}" }
                            Input {
                                placeholder: t("assistance-medication-name-placeholder", &region),
                                value: new_med_name.read().clone(),
                                oninput: move |e: FormEvent| new_med_name.set(e.value()),
                            }
                        }
                        div { class: "grid grid-cols-2 gap-3",
                            div { class: "grid gap-1.5",
                                label { class: "font-bold text-muted-foreground uppercase", "{t(\"assistance-medication-dosage-label\", &region)}" }
                                Input {
                                    placeholder: t("assistance-medication-dosage-placeholder", &region),
                                    value: new_med_dosage.read().clone(),
                                    oninput: move |e: FormEvent| new_med_dosage.set(e.value()),
                                }
                            }
                            div { class: "grid gap-1.5",
                                label { class: "font-bold text-muted-foreground uppercase", "{t(\"care-frequency\", &region)}" }
                                Input {
                                    placeholder: t("assistance-medication-frequency-placeholder", &region),
                                    value: new_med_frequency.read().clone(),
                                    oninput: move |e: FormEvent| new_med_frequency.set(e.value()),
                                }
                            }
                        }
                        div { class: "grid gap-1.5",
                            label { class: "font-bold text-muted-foreground uppercase", "{t(\"assistance-medication-instructions-label\", &region)}" }
                            textarea {
                                class: "yntra-input py-2 px-3 min-h-[80px] bg-background border border-border rounded-xl text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 text-xs w-full",
                                placeholder: t("assistance-medication-instructions-placeholder", &region),
                                value: new_med_instructions.read().clone(),
                                oninput: move |e: FormEvent| new_med_instructions.set(e.value()),
                            }
                        }
                        div { class: "flex gap-2 justify-end mt-2",
                            Button {
                                class: "bg-muted hover:bg-muted/80 text-muted-foreground text-xs h-9 px-4 rounded-xl",
                                onclick: move |_| show_add_med_modal.set(false),
                                "{t(\"common-cancel\", &region)}"
                            }
                            Button {
                                class: "text-xs h-9 px-4 rounded-xl",
                                onclick: {
                                    let c_id = selected_client_id.read().clone();
                                    let uid = user_id.clone();
                                    move |_| {
                                        let name = new_med_name.read().clone();
                                        let dosage = new_med_dosage.read().clone();
                                        let freq = new_med_frequency.read().clone();
                                        let inst = new_med_instructions.read().clone();

                                        if name.trim().is_empty() {
                                            save_error.set(Some("Medication name cannot be empty".to_string()));
                                            return;
                                        }

                                        let current_c = c_id.clone();
                                        let current_u = uid.clone();
                                        spawn(async move {
                                            match add_medication(current_u, current_c, name, dosage, freq, inst).await {
                                                Ok(_) => {
                                                    show_add_med_modal.set(false);
                                                    let current = *db_trigger.read();
                                                    db_trigger.set(current + 1);
                                                }
                                                Err(e) => {
                                                    save_error.set(Some(e.to_string()));
                                                }
                                            }
                                        });
                                    }
                                },
                                "{t(\"common-save-btn\", &region)}"
                            }
                        }
                    }
                }
            }
        }
    }
}
