use crate::components;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct FinanceSettingsProps {
    pub settings_save_status: Signal<String>,
    pub db_trigger: Signal<u32>,
    pub workspace: yntra_core::Workspace,
    pub locale: String,
}

impl PartialEq for FinanceSettingsProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn FinanceSettings(props: FinanceSettingsProps) -> Element {
    let mut settings_save_status = props.settings_save_status;
    let mut db_trigger = props.db_trigger;
    let workspace = props.workspace.clone();

    let block_settings_val: serde_json::Value = serde_json::from_str(&workspace.block_settings).unwrap_or_default();

    // Local state for school_finance display name
    let mut display_name = use_signal(|| {
        block_settings_val.get("finance")
            .and_then(|sf| sf.get("display_name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Finance")
            .to_string()
    });

    let handle_update_settings = {
        let ws_block_settings_raw = workspace.block_settings.clone();
        let ws_id = workspace.id.clone();
        move |new_name: String| {
            settings_save_status.set("saving".to_string());
            
            let mut settings_map: serde_json::Value = serde_json::from_str(&ws_block_settings_raw).unwrap_or_default();
            
            if !settings_map.is_object() {
                settings_map = serde_json::json!({});
            }
            
            settings_map["finance"] = serde_json::json!({
                "display_name": new_name
            });
            
            let settings_str = serde_json::to_string(&settings_map).unwrap_or_default();
            let ws_id = ws_id.clone();
            let settings_str_clone = settings_str.clone();
            spawn(async move {
                let _ = yntra_core::update_workspace_block_settings(ws_id, settings_str_clone).await;
            });

            let current_trig = *db_trigger.read();
            db_trigger.set(current_trig + 1);
            settings_save_status.set("saved".to_string());
        }
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "grid grid-cols-1 gap-6 md:grid-cols-2",
                
                // 1. Module Name Rebranding Card
                components::Card { class: "border-2 border-border/50 bg-card/40 shadow-sm backdrop-blur-sm",
                    components::CardHeader {
                        div { class: "flex items-center gap-3",
                            div { class: "rounded-lg bg-primary/10 p-2 text-primary",
                                components::LucideIcon { name: "credit-card", class: "h-5 w-5" }
                            }
                            div {
                                components::CardTitle { class: "text-lg", "Finance Rebranding" }
                                components::CardDescription { "Rebrand the Finance module names for your school workspace" }
                            }
                        }
                    }
                    components::CardContent { class: "space-y-6",
                        div { class: "space-y-2",
                            label { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                "Module Display Name"
                            }
                            input {
                                class: "yntra-input h-10 border-border/50 bg-background/50 text-sm py-2 px-3",
                                value: "{display_name}",
                                placeholder: "e.g. School Fees, Tuition Billing",
                                oninput: move |e| {
                                    display_name.set(e.value());
                                }
                            }
                            p { class: "text-xs text-muted-foreground mt-1",
                                "Change this to match your organization (e.g. 'School Fees' or 'Billing & Invoices')"
                            }
                        }

                        button {
                            class: "yntra-btn mt-2 w-full flex items-center justify-center gap-2",
                            onclick: {
                                let mut handle_update_settings = handle_update_settings.clone();
                                move |_| {
                                    handle_update_settings((*display_name.read()).clone());
                                }
                            },
                            components::LucideIcon { name: "save", size: "16" }
                            span { "Save Settings" }
                        }
                    }
                }
            }
        }
    }
}
