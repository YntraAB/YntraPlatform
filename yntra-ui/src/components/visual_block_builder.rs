use crate::components::{Button, DynamicForm, FormFieldSchema, FieldType, LucideIcon};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::update_block_schema;

#[derive(Props, Clone, PartialEq)]
pub struct VisualBlockBuilderProps {
    pub active_user_id: String,
    pub block_id: String,
    pub initial_schema_json: String,
    pub initial_ui_config_json: String,
    pub locale: String,
    pub onsave: EventHandler<()>,
    pub oncancel: EventHandler<()>,
}

#[component]
pub fn VisualBlockBuilder(props: VisualBlockBuilderProps) -> Element {
    let loc = props.locale.as_str();

    // Parse initial fields
    let initial_fields = FormFieldSchema::parse_json(&props.initial_schema_json).unwrap_or_else(|_| {
        vec![
            FormFieldSchema {
                name: "title".to_string(),
                label: "Title".to_string(),
                field_type: FieldType::Text,
                placeholder: Some("Enter title".to_string()),
                help_text: None,
                required: true,
                min_length: None,
                max_length: None,
                min: None,
                max: None,
                pattern: None,
                compiled_pattern: None,
                options: None,
                col_span: None,
            },
            FormFieldSchema {
                name: "status".to_string(),
                label: "Status".to_string(),
                field_type: FieldType::Select,
                placeholder: None,
                help_text: None,
                required: true,
                min_length: None,
                max_length: None,
                min: None,
                max: None,
                pattern: None,
                compiled_pattern: None,
                options: Some(vec!["New".to_string(), "In Progress".to_string(), "Completed".to_string()]),
                col_span: None,
            },
        ]
    });

    let mut fields = use_signal(|| initial_fields);
    let mut selected_field_idx = use_signal(|| Option::<usize>::None);
    let mut is_saving = use_signal(|| false);

    let active_idx = *selected_field_idx.read();
    let current_fields = fields.read().clone();

    // Serialize current schema for live preview
    let preview_schema_json = serde_json::to_string(&current_fields).unwrap_or_else(|_| "[]".to_string());

    rsx! {
        div { class: "flex flex-col h-full bg-background rounded-2xl border border-border overflow-hidden shadow-xl",
            // Toolbar Header
            div { class: "flex items-center justify-between px-6 py-4 border-b border-border bg-card/60 backdrop-blur-xs",
                div { class: "flex items-center gap-3",
                    div { class: "h-9 w-9 rounded-xl bg-primary/10 flex items-center justify-center text-primary",
                        LucideIcon { name: "layers", class: "h-5 w-5" }
                    }
                    div {
                        h3 { class: "text-sm font-bold text-foreground m-0", "Visual Block Schema Builder" }
                        p { class: "text-[11px] text-muted-foreground m-0 mt-0.5", "Drag, add, and configure dynamic fields for non-technical users" }
                    }
                }

                div { class: "flex items-center gap-2",
                    Button {
                        class: "text-xs h-8 px-3 rounded-xl border border-border bg-secondary text-secondary-foreground hover:bg-secondary/80",
                        onclick: move |_| props.oncancel.call(()),
                        "{t(\"button-cancel\", loc)}"
                    }
                    Button {
                        class: "text-xs h-8 px-4 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-md shadow-primary/20",
                        disabled: *is_saving.read(),
                        onclick: {
                            let block_id = props.block_id.clone();
                            let req_uid = props.active_user_id.clone();
                            let onsave_cb = props.onsave.clone();
                            move |_| {
                                is_saving.set(true);
                                let b_id = block_id.clone();
                                let r_uid = req_uid.clone();
                                let current_f = fields.read().clone();
                                let schema_json = serde_json::to_string(&current_f).unwrap_or_else(|_| "[]".to_string());

                                // Generate ui_config from field labels
                                let ui_conf: Vec<serde_json::Value> = current_f
                                    .iter()
                                    .take(4)
                                    .map(|f| serde_json::json!({ "name": f.name, "label": f.label }))
                                    .collect();
                                let ui_config_json = serde_json::to_string(&ui_conf).unwrap_or_else(|_| "[]".to_string());

                                let onsave_cb = onsave_cb.clone();
                                spawn(async move {
                                    let _ = update_block_schema(r_uid, b_id, schema_json, ui_config_json).await;
                                    is_saving.set(false);
                                    onsave_cb.call(());
                                });
                            }
                        },
                        if *is_saving.read() {
                            LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5 animate-spin mr-1" }
                            "Saving Schema..."
                        } else {
                            LucideIcon { name: "check", class: "h-3.5 w-3.5 mr-1" }
                            "Save Block Schema"
                        }
                    }
                }
            }

            // Main Editor Split Body
            div { class: "grid grid-cols-1 lg:grid-cols-12 flex-1 overflow-hidden",
                // Left Panel: Field Structure Tree & Quick-Add Toolbar (5 cols)
                div { class: "lg:col-span-5 border-r border-border p-5 flex flex-col gap-4 bg-card/20 overflow-y-auto",
                    // Add Field Toolbar
                    div { class: "space-y-2",
                        span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Add Field Controls" }
                        div { class: "grid grid-cols-2 sm:grid-cols-3 gap-1.5",
                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("field_{}", idx + 1),
                                        label: format!("Text Field {}", idx + 1),
                                        field_type: FieldType::Text,
                                        placeholder: Some("Enter text...".to_string()),
                                        help_text: None,
                                        required: false,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: None,
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "type", class: "h-3.5 w-3.5 text-primary" }
                                "Text Input"
                            }

                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("num_{}", idx + 1),
                                        label: format!("Number Field {}", idx + 1),
                                        field_type: FieldType::Number,
                                        placeholder: Some("0".to_string()),
                                        help_text: None,
                                        required: false,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: None,
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "hash", class: "h-3.5 w-3.5 text-emerald-500" }
                                "Number"
                            }

                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("status_{}", idx + 1),
                                        label: format!("Dropdown Status {}", idx + 1),
                                        field_type: FieldType::Select,
                                        placeholder: None,
                                        help_text: None,
                                        required: true,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: Some(vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()]),
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "chevron-down-square", class: "h-3.5 w-3.5 text-indigo-500" }
                                "Dropdown"
                            }

                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("check_{}", idx + 1),
                                        label: format!("Checkbox Flag {}", idx + 1),
                                        field_type: FieldType::Boolean,
                                        placeholder: None,
                                        help_text: None,
                                        required: false,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: None,
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "check-square", class: "h-3.5 w-3.5 text-amber-500" }
                                "Checkbox"
                            }

                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("date_{}", idx + 1),
                                        label: format!("Date Field {}", idx + 1),
                                        field_type: FieldType::Date,
                                        placeholder: None,
                                        help_text: None,
                                        required: false,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: None,
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "calendar", class: "h-3.5 w-3.5 text-rose-500" }
                                "Date"
                            }

                            button {
                                class: "flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-lg border border-border bg-background hover:bg-accent text-foreground transition-all cursor-pointer",
                                onclick: move |_| {
                                    let idx = fields.read().len();
                                    fields.write().push(FormFieldSchema {
                                        name: format!("desc_{}", idx + 1),
                                        label: format!("Details Textarea {}", idx + 1),
                                        field_type: FieldType::Textarea,
                                        placeholder: Some("Enter details...".to_string()),
                                        help_text: None,
                                        required: false,
                                        min_length: None,
                                        max_length: None,
                                        min: None,
                                        max: None,
                                        pattern: None,
                                        compiled_pattern: None,
                                        options: None,
                                        col_span: None,
                                    });
                                    selected_field_idx.set(Some(idx));
                                },
                                LucideIcon { name: "align-left", class: "h-3.5 w-3.5 text-cyan-500" }
                                "Textarea"
                            }
                        }
                    }

                    // Field Ordering List
                    div { class: "space-y-2 flex-1 overflow-y-auto pt-2 border-t border-border",
                        span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Configured Schema Fields ({current_fields.len()})" }
                        
                        if current_fields.is_empty() {
                            div { class: "p-6 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic",
                                "No fields configured. Click buttons above to add controls."
                            }
                        } else {
                            div { class: "space-y-1.5",
                                for (idx, f) in current_fields.iter().enumerate() {
                                    {
                                        let is_selected = active_idx == Some(idx);
                                        let field_type_name = format!("{:?}", f.field_type);
                                        rsx! {
                                            div {
                                                key: "{f.name}_{idx}",
                                                class: format!(
                                                    "flex items-center justify-between p-2.5 rounded-xl border text-xs cursor-pointer transition-all {}",
                                                    if is_selected { "bg-primary/10 border-primary/40 text-primary shadow-xs" } else { "bg-card border-border hover:bg-accent text-foreground" }
                                                ),
                                                onclick: move |_| selected_field_idx.set(Some(idx)),
                                                
                                                div { class: "flex items-center gap-2 truncate",
                                                    LucideIcon { name: "grip-vertical", class: "h-3.5 w-3.5 text-muted-foreground/40 shrink-0" }
                                                    div { class: "truncate",
                                                        p { class: "font-semibold m-0 truncate", "{f.label}" }
                                                        span { class: "text-[10px] text-muted-foreground font-mono truncate", "{f.name} • {field_type_name}" }
                                                    }
                                                }

                                                div { class: "flex items-center gap-1 shrink-0",
                                                    if idx > 0 {
                                                        button {
                                                            class: "p-1 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground cursor-pointer",
                                                            title: "Move Up",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                let mut list = fields.read().clone();
                                                                list.swap(idx, idx - 1);
                                                                fields.set(list);
                                                                selected_field_idx.set(Some(idx - 1));
                                                            },
                                                            LucideIcon { name: "arrow-up", class: "h-3 w-3" }
                                                        }
                                                    }
                                                    if idx + 1 < current_fields.len() {
                                                        button {
                                                            class: "p-1 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground cursor-pointer",
                                                            title: "Move Down",
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                let mut list = fields.read().clone();
                                                                list.swap(idx, idx + 1);
                                                                fields.set(list);
                                                                selected_field_idx.set(Some(idx + 1));
                                                            },
                                                            LucideIcon { name: "arrow-down", class: "h-3 w-3" }
                                                        }
                                                    }
                                                    button {
                                                        class: "p-1 rounded-md hover:bg-destructive/10 text-muted-foreground hover:text-destructive cursor-pointer ml-1",
                                                        title: "Remove Field",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            let mut list = fields.read().clone();
                                                            list.remove(idx);
                                                            fields.set(list);
                                                            selected_field_idx.set(None);
                                                        },
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

                // Center/Right Panel: Field Properties Inspector & Live Interactive Preview (7 cols)
                div { class: "lg:col-span-7 p-6 flex flex-col gap-6 overflow-y-auto bg-background",
                    // Selected Field Inspector
                    if let Some(s_idx) = active_idx {
                        if let Some(target_field) = current_fields.get(s_idx) {
                            div { class: "rounded-2xl border border-border bg-card p-4 space-y-3 shadow-xs",
                                h4 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-1.5",
                                    LucideIcon { name: "edit-3", class: "h-4 w-4 text-primary" }
                                    "Field Property Settings: {target_field.name}"
                                }

                                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs",
                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Field Display Label" }
                                        input {
                                            class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground outline-none",
                                            value: "{target_field.label}",
                                            oninput: move |e| {
                                                let val = e.value();
                                                let mut list = fields.read().clone();
                                                if let Some(f) = list.get_mut(s_idx) {
                                                    f.label = val;
                                                }
                                                fields.set(list);
                                            }
                                        }
                                    }

                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Field Key (Data JSON Name)" }
                                        input {
                                            class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-mono text-foreground outline-none",
                                            value: "{target_field.name}",
                                            oninput: move |e| {
                                                let val = e.value().to_lowercase().replace(' ', "_");
                                                let mut list = fields.read().clone();
                                                if let Some(f) = list.get_mut(s_idx) {
                                                    f.name = val;
                                                }
                                                fields.set(list);
                                            }
                                        }
                                    }

                                    div { class: "space-y-1",
                                        label { class: "font-semibold text-foreground", "Placeholder Hint" }
                                        input {
                                            class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground outline-none",
                                            value: target_field.placeholder.clone().unwrap_or_default(),
                                            oninput: move |e| {
                                                let val = e.value();
                                                let mut list = fields.read().clone();
                                                if let Some(f) = list.get_mut(s_idx) {
                                                    f.placeholder = if val.is_empty() { None } else { Some(val) };
                                                }
                                                fields.set(list);
                                            }
                                        }
                                    }

                                    div { class: "flex items-center gap-2 pt-5",
                                        input {
                                            type: "checkbox",
                                            id: "req_check",
                                            class: "rounded border-border text-primary focus:ring-primary h-4 w-4 cursor-pointer",
                                            checked: target_field.required,
                                            onchange: move |e| {
                                                let val = e.value() == "true";
                                                let mut list = fields.read().clone();
                                                if let Some(f) = list.get_mut(s_idx) {
                                                    f.required = val;
                                                }
                                                fields.set(list);
                                            }
                                        }
                                        label { for: "req_check", class: "font-semibold text-foreground cursor-pointer select-none", "Required Input Field" }
                                    }
                                }

                                if target_field.field_type == FieldType::Select || target_field.field_type == FieldType::Multiselect {
                                    div { class: "space-y-1 pt-2 border-t border-border/40 text-xs",
                                        label { class: "font-semibold text-foreground", "Select Options (Comma-separated)" }
                                        input {
                                            class: "w-full rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground outline-none",
                                            value: target_field.options.clone().unwrap_or_default().join(", "),
                                            oninput: move |e| {
                                                let raw = e.value();
                                                let opts: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                                let mut list = fields.read().clone();
                                                if let Some(f) = list.get_mut(s_idx) {
                                                    f.options = Some(opts);
                                                }
                                                fields.set(list);
                                            },
                                            placeholder: "Option 1, Option 2, Option 3",
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Live Form Preview Box
                    div { class: "rounded-2xl border border-border bg-card/40 p-5 space-y-4 shadow-sm flex-1 flex flex-col",
                        div { class: "flex items-center justify-between border-b border-border pb-3",
                            h4 { class: "text-xs font-bold text-foreground uppercase tracking-wider m-0 flex items-center gap-2",
                                LucideIcon { name: "eye", class: "h-4 w-4 text-emerald-500" }
                                "Live Form Preview"
                            }
                            span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "Interactive" }
                        }

                        div { class: "p-4 border border-border rounded-xl bg-background flex-1 overflow-y-auto",
                            DynamicForm {
                                fields_schema: current_fields.clone(),
                                initial_values: "{}".to_string(),
                                locale: props.locale.clone(),
                                onsubmit: move |_| {},
                                oncancel: move |_| {}
                            }
                        }
                    }
                }
            }
        }
    }
}
