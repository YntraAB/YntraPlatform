use crate::components::{Button, Checkbox, DatePicker, Input, TextArea};
use crate::locales::t;
use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Props, Clone, PartialEq)]
pub struct DynamicFormProps {
    pub fields_schema: String,  // JSON Array of field definitions
    pub initial_values: String, // JSON Object of current values
    pub onsubmit: EventHandler<String>,
    pub oncancel: Option<EventHandler<()>>,
    #[props(default = "en".to_string())]
    pub locale: String,
}

#[component]
pub fn DynamicForm(props: DynamicFormProps) -> Element {
    let loc = props.locale.clone();

    let fields_schema_str = props.fields_schema.clone();
    let fields_memo = use_memo(move || {
        let schema: serde_json::Value = serde_json::from_str(&fields_schema_str)
            .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
        schema.as_array().cloned().unwrap_or_default()
    });
    let fields = fields_memo.read().clone();

    let mut form_values = use_signal(|| {
        serde_json::from_str::<serde_json::Value>(&props.initial_values)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    });

    let mut validation_errors = use_signal(HashMap::<String, String>::new);

    let fields_for_val = fields.clone();
    let mut validate_form = move || -> bool {
        let mut errs = HashMap::new();
        let current_vals = form_values.read().clone();

        for field in fields_for_val.iter() {
            let name = field.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let label = field.get("label").and_then(|v| v.as_str()).unwrap_or(&name).to_string();
            let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("text");
            let required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(false);

            let val_opt = current_vals.get(&name);

            // 1. Required check
            if required {
                let is_empty = match val_opt {
                    None | Some(serde_json::Value::Null) => true,
                    Some(serde_json::Value::String(s)) => s.trim().is_empty(),
                    Some(serde_json::Value::Bool(_)) => false,
                    Some(serde_json::Value::Number(_)) => false,
                    Some(serde_json::Value::Array(arr)) => arr.is_empty(),
                    Some(serde_json::Value::Object(obj)) => obj.is_empty(),
                };
                if is_empty {
                    errs.insert(name.clone(), format!("{label} is required."));
                    continue;
                }
            }

            // 2. Email format validation
            if field_type == "email" {
                if let Some(serde_json::Value::String(s_val)) = val_opt {
                    if !s_val.is_empty() {
                        let email_re = regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
                        if !email_re.is_match(s_val) {
                            errs.insert(name.clone(), format!("{label} must be a valid email address."));
                            continue;
                        }
                    }
                }
            }

            // 3. Text / String constraints (Min length, Max length, Regex format)
            if let Some(serde_json::Value::String(s_val)) = val_opt {
                if !s_val.is_empty() {
                    let min_len = field
                        .get("min_length")
                        .or_else(|| field.get("minLength"))
                        .and_then(|v| v.as_u64());
                    if let Some(min) = min_len {
                        if (s_val.chars().count() as u64) < min {
                            errs.insert(name.clone(), format!("{label} must be at least {min} characters."));
                            continue;
                        }
                    }

                    let max_len = field
                        .get("max_length")
                        .or_else(|| field.get("maxLength"))
                        .and_then(|v| v.as_u64());
                    if let Some(max) = max_len {
                        if (s_val.chars().count() as u64) > max {
                            errs.insert(name.clone(), format!("{label} must be at most {max} characters."));
                            continue;
                        }
                    }

                    let pattern = field
                        .get("pattern")
                        .or_else(|| field.get("regex"))
                        .and_then(|v| v.as_str());
                    if let Some(pat_str) = pattern {
                        if let Ok(re) = regex::Regex::new(pat_str) {
                            if !re.is_match(s_val) {
                                errs.insert(name.clone(), format!("{label} format is invalid."));
                                continue;
                            }
                        }
                    }
                }
            }

            // 4. Numeric constraints (Min, Max)
            if field_type == "number" {
                let num_val = match val_opt {
                    Some(serde_json::Value::Number(n)) => n.as_f64(),
                    Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
                    _ => None,
                };

                if let Some(n) = num_val {
                    let min_boundary = field
                        .get("min")
                        .or_else(|| field.get("minimum"))
                        .and_then(|v| v.as_f64());
                    if let Some(min) = min_boundary {
                        if n < min {
                            errs.insert(name.clone(), format!("{label} must be at least {min}."));
                            continue;
                        }
                    }

                    let max_boundary = field
                        .get("max")
                        .or_else(|| field.get("maximum"))
                        .and_then(|v| v.as_f64());
                    if let Some(max) = max_boundary {
                        if n > max {
                            errs.insert(name.clone(), format!("{label} must be at most {max}."));
                            continue;
                        }
                    }
                }
            }
        }

        let is_valid = errs.is_empty();
        validation_errors.set(errs);
        is_valid
    };

    rsx! {
        div { class: "flex flex-col gap-4 text-sm w-full",
            for field in fields.iter() {
                {
                    let name = field.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    let label = field.get("label").and_then(|v| v.as_str()).unwrap_or(&name).to_string();
                    let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                    let placeholder = field.get("placeholder").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                    let err_msg = validation_errors.read().get(&name).cloned();

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
                                                validation_errors.write().remove(&name_clone);
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
                                        crate::components::NumberInput {
                                            class: if err_msg.is_some() {
                                                "py-1.5 px-3 text-xs bg-background border border-red-500/80 text-foreground w-full focus:ring-1 focus:ring-red-500"
                                            } else {
                                                "py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full"
                                            },
                                            placeholder: placeholder,
                                            value: "{current_val}",
                                            oninput: move |evt: FormEvent| {
                                                if let Ok(num) = evt.value().parse::<f64>() {
                                                    form_values.write().insert(name_clone.clone(), serde_json::json!(num));
                                                } else {
                                                    form_values.write().insert(name_clone.clone(), serde_json::Value::Null);
                                                }
                                                validation_errors.write().remove(&name_clone);
                                            }
                                        }
                                    }
                                }
                                "select" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let options = field.get("options").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                    let name_clone = name.clone();
                                    let select_opt_label = t("block-select-option", &loc);
                                    rsx! {
                                        crate::components::NativeSelect {
                                            class: if err_msg.is_some() {
                                                "py-1.5 px-3 text-xs bg-background border border-red-500/80 text-foreground w-full cursor-pointer focus:ring-1 focus:ring-red-500"
                                            } else {
                                                "py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full cursor-pointer"
                                            },
                                            value: "{current_val}",
                                            oninput: move |evt: FormEvent| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                                validation_errors.write().remove(&name_clone);
                                            },
                                            option { value: "", "{select_opt_label}" }
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
                                "textarea" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name_clone = name.clone();
                                    rsx! {
                                        TextArea {
                                            placeholder: placeholder,
                                            value: current_val,
                                            rows: 3,
                                            class: if err_msg.is_some() { "border-red-500/80 focus:ring-1 focus:ring-red-500".to_string() } else { "".to_string() },
                                            oninput: move |evt: FormEvent| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                                validation_errors.write().remove(&name_clone);
                                            }
                                        }
                                    }
                                }
                                "date" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name_clone = name.clone();
                                    rsx! {
                                        DatePicker {
                                            value: current_val,
                                            class: if err_msg.is_some() { "border-red-500/80 focus:ring-1 focus:ring-red-500".to_string() } else { "".to_string() },
                                            onchange: move |evt: FormEvent| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                                validation_errors.write().remove(&name_clone);
                                            }
                                        }
                                    }
                                }
                                "email" => {
                                    let current_val = form_values.read().get(&name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name_clone = name.clone();
                                    let email_placeholder = if placeholder.is_empty() { "name@example.com".to_string() } else { placeholder };
                                    rsx! {
                                        Input {
                                            r#type: "email",
                                            placeholder: email_placeholder,
                                            value: current_val,
                                            class: if err_msg.is_some() { "py-1.5 px-3 text-xs bg-background border border-red-500/80 text-foreground w-full focus:ring-1 focus:ring-red-500" } else { "py-1.5 px-3 text-xs bg-background border border-border text-foreground w-full" },
                                            oninput: move |evt: FormEvent| {
                                                form_values.write().insert(name_clone.clone(), serde_json::Value::String(evt.value()));
                                                validation_errors.write().remove(&name_clone);
                                            }
                                        }
                                    }
                                }
                                "multiselect" => {
                                    let options = field.get("options").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                    let selected_arr = match form_values.read().get(&name) {
                                        Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<String>>(),
                                        _ => Vec::new(),
                                    };
                                    let name_clone = name.clone();
                                    rsx! {
                                        div { class: "flex flex-wrap gap-2 py-2 px-3 bg-background border border-border rounded-lg min-h-[38px] items-center",
                                            for opt in options.iter() {
                                                {
                                                    let opt_str = opt.as_str().unwrap_or_default().to_string();
                                                    let is_selected = selected_arr.contains(&opt_str);
                                                    let name_c = name_clone.clone();
                                                    let opt_c = opt_str.clone();
                                                    let sel_c = selected_arr.clone();

                                                    rsx! {
                                                        button {
                                                            r#type: "button",
                                                            class: if is_selected {
                                                                "px-2.5 py-1 text-xs font-semibold rounded-md bg-primary text-primary-foreground border border-primary transition-all flex items-center gap-1 cursor-pointer"
                                                            } else {
                                                                "px-2.5 py-1 text-xs font-medium rounded-md bg-muted/60 text-muted-foreground hover:bg-muted border border-border transition-all cursor-pointer"
                                                            },
                                                            onclick: move |_| {
                                                                let mut new_sel = sel_c.clone();
                                                                if is_selected {
                                                                    new_sel.retain(|s| s != &opt_c);
                                                                } else {
                                                                    new_sel.push(opt_c.clone());
                                                                }
                                                                let json_arr = serde_json::Value::Array(new_sel.into_iter().map(serde_json::Value::String).collect());
                                                                form_values.write().insert(name_c.clone(), json_arr);
                                                                validation_errors.write().remove(&name_c);
                                                            },
                                                            if is_selected { "✓ " }
                                                            "{opt_str}"
                                                        }
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
                                                validation_errors.write().remove(&name_clone);
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(msg) = err_msg {
                                p { class: "text-xs font-medium text-red-500 mt-1 m-0 flex items-center gap-1",
                                    "{msg}"
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
                        "{t(\"common-cancel\", &loc)}"
                    }
                }
                Button {
                    onclick: move |_| {
                        if validate_form() {
                            let json_str = serde_json::to_string(&serde_json::Value::Object(form_values.read().clone())).unwrap_or_else(|_| "{}".to_string());
                            props.onsubmit.call(json_str);
                        }
                    },
                    "{t(\"common-save-btn\", &loc)}"
                }
            }
        }
    }
}



