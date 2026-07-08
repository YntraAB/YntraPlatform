use dioxus::prelude::*;
use crate::components::{Input, Checkbox, Button};

#[derive(Props, Clone, PartialEq)]
pub struct DynamicFormProps {
    pub fields_schema: String,     // JSON Array of field definitions
    pub initial_values: String,    // JSON Object of current values
    pub onsubmit: EventHandler<String>,
    pub oncancel: Option<EventHandler<()>>,
}

#[component]
pub fn DynamicForm(props: DynamicFormProps) -> Element {
    let schema: serde_json::Value = serde_json::from_str(&props.fields_schema).unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
    let fields = schema.as_array().cloned().unwrap_or_default();

    let mut form_values = use_signal(|| {
        serde_json::from_str::<serde_json::Value>(&props.initial_values)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    });

    rsx! {
        div { class: "flex flex-col gap-4 text-sm w-full",
            for field in fields.iter() {
                {
                    let name = field.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let label = field.get("label").and_then(|v| v.as_str()).unwrap_or(&name).to_string();
                    let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                    let placeholder = field.get("placeholder").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(false);

                    rsx! {
                        div { class: "flex flex-col gap-1.5 w-full",
                            if field_type != "boolean" {
                                label { class: "text-xs font-bold text-muted-foreground uppercase tracking-wider",
                                    "{label}"
                                    if required { span { class: "text-red-500 ml-1", "*" } }
                                }
                            }

                            match field_type {
                                "boolean" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_bool()).unwrap_or(false);
                                    let name_clone = name.clone();
                                    rsx! {
                                        Checkbox {
                                            checked: current_val,
                                            onchange: move |val| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::Bool(val));
                                            },
                                            label: label
                                        }
                                    }
                                }
                                "number" => {
                                    let current_val = match form_values.read().get(&name) {
                                        Some(serde_json::Value::Number(n)) => n.to_string(),
                                        Some(serde_json::Value::Null) => String::new(),
                                        Some(v) => v.to_string(),
                                        None => String::new(),
                                    };
                                    let name_clone = name.clone();
                                    rsx! {
                                        input {
                                            r#type: "number",
                                            class: "yntra-input py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full",
                                            placeholder: placeholder,
                                            value: "{current_val}",
                                            oninput: move |evt| {
                                                if let Ok(num) = evt.value().parse::<f64>() {
                                                    form_values.write().insert(name_clone.clone(), serde_json::json!(num));
                                                } else {
                                                    form_values.write().insert(name_clone.clone(), serde_json::Value::Null);
                                                }
                                            }
                                        }
                                    }
                                }
                                "select" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let options = field.get("options").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                    let name_clone = name.clone();
                                    rsx! {
                                        select {
                                            class: "yntra-input py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full cursor-pointer",
                                            value: "{current_val}",
                                            oninput: move |evt| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                            },
                                            option { value: "", "Select option..." }
                                            for opt in options.iter() {
                                                {
                                                    let opt_str = opt.as_str().unwrap_or_default().to_string();
                                                    rsx! {
                                                        option { value: "{opt_str}", "{opt_str}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name_clone = name.clone();
                                    rsx! {
                                        Input {
                                            placeholder: placeholder,
                                            value: current_val,
                                            oninput: move |evt: FormEvent| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Actions (Save / Cancel)
            div { class: "flex gap-2 justify-end mt-4",
                if let Some(oncancel_cb) = props.oncancel {
                    Button {
                        variant: crate::components::ButtonVariant::Secondary,
                        onclick: move |_| oncancel_cb.call(()),
                        "Cancel"
                    }
                }
                Button {
                    onclick: move |_| {
                        let json_str = serde_json::to_string(&serde_json::Value::Object(form_values.read().clone())).unwrap_or_else(|_| "{}".to_string());
                        props.onsubmit.call(json_str);
                    },
                    "Save"
                }
            }
        }
    }
}
