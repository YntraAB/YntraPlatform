use dioxus::prelude::*;
use yntra_core::DynamicEntity;
use crate::components::LucideIcon;

#[derive(Props, Clone, PartialEq)]
pub struct DynamicListProps {
    pub entities: Vec<DynamicEntity>,
    pub ui_config: String,          // JSON Array defining columns
    pub onedit: EventHandler<DynamicEntity>,
    pub ondelete: EventHandler<String>,
}

#[component]
pub fn DynamicList(props: DynamicListProps) -> Element {
    let ui_config_val: serde_json::Value = serde_json::from_str(&props.ui_config).unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
    let columns = ui_config_val.as_array().cloned().unwrap_or_default();

    rsx! {
        div { class: "w-full overflow-x-auto rounded-lg border border-border bg-card/20 backdrop-blur-md",
            table { class: "w-full border-collapse text-left text-sm text-foreground",
                thead { class: "bg-muted/40 uppercase text-xs font-bold text-muted-foreground border-b border-border",
                    tr {
                        if columns.is_empty() {
                            th { class: "p-4", "Entity Data" }
                        } else {
                            for col in columns.iter() {
                                {
                                    let label = col.get("label").and_then(|v| v.as_str()).unwrap_or("Column");
                                    rsx! {
                                        th { class: "p-4 font-semibold", "{label}" }
                                    }
                                }
                            }
                        }
                        th { class: "p-4 text-right", "Actions" }
                    }
                }
                tbody { class: "divide-y divide-border/60",
                    if props.entities.is_empty() {
                        tr {
                            td {
                                class: "p-8 text-center text-muted-foreground",
                                colspan: if columns.is_empty() { 2 } else { columns.len() + 1 },
                                "No items found."
                            }
                        }
                    } else {
                        for entity in props.entities.iter() {
                            {
                                let entity_data: serde_json::Value = serde_json::from_str(&entity.data).unwrap_or_default();
                                let e_id = entity.id.clone();
                                let entity_clone = entity.clone();
                                let entity_delete_id = entity.id.clone();
                                rsx! {
                                    tr { key: "{e_id}", class: "hover:bg-white/[0.01] transition-colors",
                                        if columns.is_empty() {
                                            td { class: "p-4 font-medium text-foreground",
                                                "{entity.data}"
                                            }
                                        } else {
                                            for col in columns.iter() {
                                                {
                                                    let col_name = col.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                                                    let val = entity_data.get(col_name).unwrap_or(&serde_json::Value::Null);
                                                    let rendered_val = match val {
                                                        serde_json::Value::Null => String::new(),
                                                        serde_json::Value::Bool(b) => if *b { "Yes".to_string() } else { "No".to_string() },
                                                        serde_json::Value::Number(n) => n.to_string(),
                                                        serde_json::Value::String(s) => s.clone(),
                                                        v => v.to_string(),
                                                    };
                                                    rsx! {
                                                        td { class: "p-4 text-muted-foreground font-medium", "{rendered_val}" }
                                                    }
                                                }
                                            }
                                        }
                                        td { class: "p-4 text-right flex justify-end gap-2",
                                            button {
                                                class: "p-1.5 rounded-lg border border-border/40 hover:bg-white/[0.04] text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                                                onclick: move |_| props.onedit.call(entity_clone.clone()),
                                                LucideIcon { name: "edit-2", class: "h-3.5 w-3.5" }
                                            }
                                            button {
                                                class: "p-1.5 rounded-lg border border-red-500/10 hover:bg-red-500/10 text-red-400 hover:text-red-300 cursor-pointer transition-all",
                                                onclick: move |_| props.ondelete.call(entity_delete_id.clone()),
                                                LucideIcon { name: "trash-2", class: "h-3.5 w-3.5" }
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
