use crate::components;
use dioxus::prelude::*;
use yntra_core::create_workspace_via_hub;

#[derive(Props, Clone)]
pub struct DevHubDialogProps {
    pub open: bool,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

impl PartialEq for DevHubDialogProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn DevHubDialog(props: DevHubDialogProps) -> Element {
    let open = props.open;
    let onclose = props.onclose;
    let onsubmit = props.onsubmit;

    let mut ws_name = use_signal(String::new);
    let mut admin_email = use_signal(String::new);
    let mut selected_preset = use_signal(|| "assistance".to_string());
    let mut is_loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let submit_disabled = *is_loading.read()
        || ws_name.read().trim().is_empty()
        || admin_email.read().trim().is_empty();

    let handle_create = move |_| {
        let name = ws_name.read().trim().to_string();
        let email = admin_email.read().trim().to_string();
        let school = *selected_preset.read() == "school";
        let assistance = *selected_preset.read() == "assistance";

        spawn(async move {
            is_loading.set(true);
            error_msg.set(None);

            let modules_json = format!(
                "{{\"school\":{},\"assistance\":{},\"journals\":{},\"medications\":{}}}",
                school, assistance, assistance, assistance
            );

            match create_workspace_via_hub(name, email, modules_json).await {
                Ok(_) => {
                    onsubmit.call(());
                    onclose.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to create workspace: {:?}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    rsx! {
        components::Dialog {
            open,
            title: "Developer Hub: Create Workspace".to_string(),
            onclose: move |_| onclose.call(()),
            div { class: "flex flex-col gap-5 w-full text-sm",
                    style: "max-width:420px;",
                p { class: "m-0 text-muted-foreground text-xs",
                    style: "line-height:1.4;",
                    "Initialize a new care organization workspace and register its administrator. Toggled modules will be configured automatically."
                }

                if let Some(err) = error_msg.read().clone() {
                    div { class: "p-3 rounded-lg text-xs",
                    style: "background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: var(--danger);",
                        "{err}"
                    }
                }

                // Inputs
                div { class: "flex flex-col gap-3.5",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase",
                            "Workspace Name"
                        }
                        input {
                            class: "yntra-input",
                            placeholder: "e.g. Gothenburg Health Hub",
                            value: "{ws_name}",
                            oninput: move |e| ws_name.set(e.value()),
                        }
                    }

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase",
                            "Administrator Emails (comma-separated)"
                        }
                        input {
                            class: "yntra-input",
                            placeholder: "e.g. admin1@healthhub.se, admin2@healthhub.se",
                            value: "{admin_email}",
                            oninput: move |e| admin_email.set(e.value()),
                        }
                    }
                }

                // Modules Selection
                div { class: "flex flex-col gap-2",
                    label { class: "text-xs font-bold text-muted-foreground uppercase",
                        "Workspace Template Preset"
                    }
                    div { class: "grid gap-3",
                    style: "grid-template-columns:1fr 1fr;",
                        // School Preset Toggle Card
                        div {
                            style: format!(
                                "display:flex; flex-direction:column; gap:0.5rem; padding:0.85rem; border:2px solid {}; background:{}; border-radius:10px; cursor:pointer; transition:all 0.15s ease;",
                                if *selected_preset.read() == "school" { "var(--accent-color)" } else { "var(--border-color)" },
                                if *selected_preset.read() == "school" { "var(--accent-color-soft)" } else { "transparent" }
                            ),
                            onclick: move |_| {
                                selected_preset.set("school".to_string());
                            },
                            div { class: "flex justify-between items-center",
                                span { class: "text-xs font-bold text-foreground", "School & Education" }
                                input {
                                    r#type: "radio",
                                    name: "preset_template",
                                    checked: *selected_preset.read() == "school",
                                    class: "cursor-pointer",
                                    style: "accent-color:var(--accent-color);",
                                }
                            }
                            span { class: "text-muted-foreground/60",
                    style: "font-size:0.65rem; line-height:1.25;",
                                "Active learning and schedule management for educational entities."
                            }
                        }

                        // Assistance Preset Toggle Card
                        div {
                            style: format!(
                                "display:flex; flex-direction:column; gap:0.5rem; padding:0.85rem; border:2px solid {}; background:{}; border-radius:10px; cursor:pointer; transition:all 0.15s ease;",
                                if *selected_preset.read() == "assistance" { "var(--accent-color)" } else { "var(--border-color)" },
                                if *selected_preset.read() == "assistance" { "var(--accent-color-soft)" } else { "transparent" }
                            ),
                            onclick: move |_| {
                                selected_preset.set("assistance".to_string());
                            },
                            div { class: "flex justify-between items-center",
                                span { class: "text-xs font-bold text-foreground", "Care & Assistance" }
                                input {
                                    r#type: "radio",
                                    name: "preset_template",
                                    checked: *selected_preset.read() == "assistance",
                                    class: "cursor-pointer",
                                    style: "accent-color:var(--accent-color);",
                                }
                            }
                            span { class: "text-muted-foreground/60",
                    style: "font-size:0.65rem; line-height:1.25;",
                                "Medication administration plans and logs."
                            }
                        }
                    }
                }

                // Footer Actions
                div { class: "flex justify-end gap-2 border-t border-border pt-4 mt-2",
                    button {
                        class: "yntra-btn btn-secondary text-xs",
                        style: "padding:0.6rem 1.2rem;",
                        onclick: move |_| onclose.call(()),
                        "Cancel"
                    }
                    button {
                        class: "yntra-btn btn-primary text-xs",
                        style: "padding:0.6rem 1.2rem;",
                        disabled: submit_disabled,
                        onclick: handle_create,
                        if *is_loading.read() { "Creating..." } else { "Create & Invite" }
                    }
                }
            }
        }
    }
}
