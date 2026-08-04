use crate::components::{Button, Checkbox, DatePicker, Input, LucideIcon, TextArea};
use crate::locales::{t, t_with_args};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

static EMAIL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("Invalid static EMAIL_REGEX")
});

static URL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(https?|ftp)://[^\s/$.?#].[^\s]*$").expect("Invalid static URL_REGEX")
});

static TEL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^\+?[0-9\s\-()]{7,20}$").expect("Invalid static TEL_REGEX")
});

/// Helper function to retrieve a nested `serde_json::Value` from a `Map` using dot-path syntax (e.g. "user.profile.name").
pub fn get_nested_value<'a>(
    map: &'a serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Option<&'a serde_json::Value> {
    if !path.contains('.') {
        return map.get(path);
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut current: Option<&serde_json::Value> = map.get(parts[0]);
    for part in parts.iter().skip(1) {
        match current {
            Some(serde_json::Value::Object(obj)) => {
                current = obj.get(*part);
            }
            _ => return None,
        }
    }
    current
}

/// Helper function to set a nested `serde_json::Value` into a `Map` using dot-path syntax (e.g. "user.profile.name").
pub fn set_nested_value(
    map: &mut serde_json::Map<String, serde_json::Value>,
    path: &str,
    value: serde_json::Value,
) {
    if !path.contains('.') {
        map.insert(path.to_string(), value);
        return;
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut current_map = map;
    for (i, &part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            current_map.insert(part.to_string(), value);
            break;
        }
        let is_obj = current_map.get(part).map_or(false, |v| v.is_object());
        if !is_obj {
            current_map.insert(
                part.to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        if let Some(serde_json::Value::Object(next_map)) = current_map.get_mut(part) {
            current_map = next_map;
        } else {
            break;
        }
    }
}

/// Strongly-typed classification enum for form field controls.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    #[default]
    Text,
    Number,
    Email,
    Password,
    Select,
    Multiselect,
    Boolean,
    Textarea,
    Date,
    Time,
    Color,
    #[serde(
        rename = "section",
        alias = "heading",
        alias = "group",
        alias = "divider"
    )]
    Section,
    #[serde(rename = "tel", alias = "phone")]
    Tel,
    Url,
    Search,
    #[serde(other)]
    Unknown,
}

/// Strongly-typed schema definition for dynamic form fields.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FormFieldSchema {
    /// Unique identifier key for the form field value in submission payload.
    pub name: String,
    /// Human-readable display label for the field.
    #[serde(default)]
    pub label: String,
    /// Input type or layout element type (e.g., Text, Number, Email, Select, Section).
    #[serde(default, rename = "type", alias = "field_type")]
    pub field_type: FieldType,
    /// Optional placeholder hint text rendered inside empty inputs.
    #[serde(default)]
    pub placeholder: Option<String>,
    /// Optional descriptive help text rendered beneath the field label.
    #[serde(default, alias = "description", alias = "helpText")]
    pub help_text: Option<String>,
    /// Whether non-empty field input is required before form submission.
    #[serde(default)]
    pub required: bool,
    /// Optional minimum string character count constraint.
    #[serde(default, alias = "minLength")]
    pub min_length: Option<usize>,
    /// Optional maximum string character count constraint.
    #[serde(default, alias = "maxLength")]
    pub max_length: Option<usize>,
    /// Optional minimum numeric value boundary.
    #[serde(default, alias = "minimum")]
    pub min: Option<f64>,
    /// Optional maximum numeric value boundary.
    #[serde(default, alias = "maximum")]
    pub max: Option<f64>,
    /// Optional regex validation pattern.
    #[serde(default, alias = "regex")]
    pub pattern: Option<String>,
    /// Pre-compiled regex pattern for fast validation without re-compilation.
    #[serde(skip)]
    pub compiled_pattern: Option<Arc<regex::Regex>>,
    /// Optional selectable options list for select and multiselect field types.
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Responsive grid column span (e.g., 1-12 or "half", "third", "full").
    #[serde(default, alias = "colSpan", alias = "col_width", alias = "width")]
    pub col_span: Option<serde_json::Value>,
}

impl PartialEq for FormFieldSchema {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.label == other.label
            && self.field_type == other.field_type
            && self.placeholder == other.placeholder
            && self.help_text == other.help_text
            && self.required == other.required
            && self.min_length == other.min_length
            && self.max_length == other.max_length
            && self.min == other.min
            && self.max == other.max
            && self.pattern == other.pattern
            && self.options == other.options
            && self.col_span == other.col_span
    }
}

impl FormFieldSchema {
    pub fn parse_json(json_str: &str) -> Result<Vec<FormFieldSchema>, serde_json::Error> {
        let mut schemas: Vec<FormFieldSchema> = serde_json::from_str(json_str)?;
        for schema in schemas.iter_mut() {
            schema.compile_pattern();
        }
        Ok(schemas)
    }

    pub fn compile_pattern(&mut self) {
        if let Some(ref pat) = self.pattern {
            if self.compiled_pattern.is_none() {
                if let Ok(re) = regex::Regex::new(pat) {
                    self.compiled_pattern = Some(Arc::new(re));
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct DynamicFormProps {
    pub fields_schema: Vec<FormFieldSchema>,
    pub initial_values: String, // JSON Object of current values
    pub onsubmit: EventHandler<String>,
    pub oncancel: Option<EventHandler<()>>,
    #[props(default = "en".to_string())]
    pub locale: String,
    #[props(default = false)]
    pub is_submitting: bool,
}

fn get_col_span_class(field: &FormFieldSchema) -> &'static str {
    if let Some(ref s) = field.col_span {
        if let Some(n) = s.as_u64() {
            return match n {
                1 => "col-span-12 md:col-span-1",
                2 => "col-span-12 md:col-span-2",
                3 => "col-span-12 md:col-span-3",
                4 => "col-span-12 md:col-span-4",
                5 => "col-span-12 md:col-span-5",
                6 => "col-span-12 md:col-span-6",
                7 => "col-span-12 md:col-span-7",
                8 => "col-span-12 md:col-span-8",
                9 => "col-span-12 md:col-span-9",
                10 => "col-span-12 md:col-span-10",
                11 => "col-span-12 md:col-span-11",
                _ => "col-span-12",
            };
        } else if let Some(str_val) = s.as_str() {
            return match str_val {
                "half" | "1/2" => "col-span-12 md:col-span-6",
                "third" | "1/3" => "col-span-12 md:col-span-4",
                "two-thirds" | "2/3" => "col-span-12 md:col-span-8",
                "quarter" | "1/4" => "col-span-12 md:col-span-3",
                "full" | "1" => "col-span-12",
                _ => "col-span-12",
            };
        }
    }
    "col-span-12"
}

#[component]
pub fn DynamicForm(props: DynamicFormProps) -> Element {
    let loc = props.locale.clone();
    let fields = {
        let mut list = props.fields_schema.clone();
        for field in list.iter_mut() {
            field.compile_pattern();
        }
        list
    };

    if fields.is_empty() {
        let warning_title = t("form-schema-empty-title", &loc);
        let warning_title = if warning_title == "form-schema-empty-title" {
            "No Schema Fields Available".to_string()
        } else {
            warning_title
        };
        let warning_desc = t("form-schema-empty-desc", &loc);
        let warning_desc = if warning_desc == "form-schema-empty-desc" {
            "The form schema is empty or contains invalid field definitions.".to_string()
        } else {
            warning_desc
        };

        return rsx! {
            div { class: "p-4 rounded-lg border border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400 text-xs flex items-start gap-3 w-full my-2",
                LucideIcon { name: "alert-triangle", class: "h-5 w-5 shrink-0 mt-0.5" }
                div { class: "flex flex-col gap-0.5",
                    span { class: "font-semibold text-sm text-foreground", "{warning_title}" }
                    p { class: "m-0 text-muted-foreground", "{warning_desc}" }
                }
            }
        };
    }

    let mut form_values = use_signal(serde_json::Map::<String, serde_json::Value>::new);

    use_effect(use_reactive(
        (&props.initial_values,),
        move |(initial_json,)| {
            let parsed = serde_json::from_str::<serde_json::Value>(&initial_json)
                .ok()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            form_values.set(parsed);
        },
    ));

    let mut validation_errors = use_signal(HashMap::<String, String>::new);

    let fields_for_val = fields.clone();
    let loc_val = loc.clone();
    let mut validate_form = move || -> Vec<String> {
        let mut errs = HashMap::new();
        let mut invalid_ids = Vec::new();
        let current_vals = form_values.read().clone();

        for field in fields_for_val.iter() {
            let name = field.name.clone();
            let field_type = &field.field_type;
            if name.is_empty() || matches!(field_type, FieldType::Section) {
                continue;
            }
            let label = if field.label.is_empty() {
                name.clone()
            } else {
                field.label.clone()
            };
            let required = field.required;

            let val_opt = get_nested_value(&current_vals, &name);
            let field_id = format!("field-{}", name);

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
                    errs.insert(
                        name.clone(),
                        t_with_args("validation-required", &loc_val, &[("label", &label)]),
                    );
                    invalid_ids.push(field_id);
                    continue;
                }
            }

            // 2. Built-in format validations (Email, URL, Phone) using static pre-compiled regexes
            if let Some(serde_json::Value::String(s_val)) = val_opt {
                if !s_val.is_empty() {
                    let mut format_err = false;
                    match field_type {
                        FieldType::Email => {
                            if !EMAIL_REGEX.is_match(s_val) {
                                errs.insert(
                                    name.clone(),
                                    t_with_args(
                                        "validation-email-invalid",
                                        &loc_val,
                                        &[("label", &label)],
                                    ),
                                );
                                format_err = true;
                            }
                        }
                        FieldType::Url => {
                            if !URL_REGEX.is_match(s_val) {
                                errs.insert(
                                    name.clone(),
                                    t_with_args(
                                        "validation-pattern-invalid",
                                        &loc_val,
                                        &[("label", &label)],
                                    ),
                                );
                                format_err = true;
                            }
                        }
                        FieldType::Tel => {
                            if !TEL_REGEX.is_match(s_val) {
                                errs.insert(
                                    name.clone(),
                                    t_with_args(
                                        "validation-pattern-invalid",
                                        &loc_val,
                                        &[("label", &label)],
                                    ),
                                );
                                format_err = true;
                            }
                        }
                        _ => {}
                    }
                    if format_err {
                        invalid_ids.push(field_id);
                        continue;
                    }
                }
            }

            // 3. Text / String constraints (Min length, Max length, Custom Regex pattern)
            if let Some(serde_json::Value::String(s_val)) = val_opt {
                if !s_val.is_empty() {
                    let mut text_err = false;
                    if let Some(min) = field.min_length {
                        if s_val.chars().count() < min {
                            let min_str = min.to_string();
                            errs.insert(
                                name.clone(),
                                t_with_args(
                                    "validation-min-length",
                                    &loc_val,
                                    &[("label", &label), ("min", &min_str)],
                                ),
                            );
                            text_err = true;
                        }
                    }

                    if !text_err {
                        if let Some(max) = field.max_length {
                            if s_val.chars().count() > max {
                                let max_str = max.to_string();
                                errs.insert(
                                    name.clone(),
                                    t_with_args(
                                        "validation-max-length",
                                        &loc_val,
                                        &[("label", &label), ("max", &max_str)],
                                    ),
                                );
                                text_err = true;
                            }
                        }
                    }

                    if !text_err {
                        if let Some(ref re) = field.compiled_pattern {
                            if !re.is_match(s_val) {
                                errs.insert(
                                    name.clone(),
                                    t_with_args(
                                        "validation-pattern-invalid",
                                        &loc_val,
                                        &[("label", &label)],
                                    ),
                                );
                                text_err = true;
                            }
                        } else if let Some(ref pat_str) = field.pattern {
                            match regex::Regex::new(pat_str) {
                                Ok(re) => {
                                    if !re.is_match(s_val) {
                                        errs.insert(
                                            name.clone(),
                                            t_with_args(
                                                "validation-pattern-invalid",
                                                &loc_val,
                                                &[("label", &label)],
                                            ),
                                        );
                                        text_err = true;
                                    }
                                }
                                Err(err) => {
                                    log::warn!(
                                        "DynamicForm: Custom regex pattern '{}' on field '{}' failed to compile: {}",
                                        pat_str,
                                        name,
                                        err
                                    );
                                }
                            }
                        }
                    }

                    if text_err {
                        invalid_ids.push(field_id);
                        continue;
                    }
                }
            }

            // 4. Numeric constraints (Min, Max)
            if field_type == &FieldType::Number {
                let mut num_err = false;
                let num_val = match val_opt {
                    Some(serde_json::Value::Number(n)) => n.as_f64(),
                    Some(serde_json::Value::String(s)) => {
                        let trimmed = s.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            match trimmed.parse::<f64>() {
                                Ok(n) => Some(n),
                                Err(_) => {
                                    errs.insert(
                                        name.clone(),
                                        t_with_args(
                                            "validation-pattern-invalid",
                                            &loc_val,
                                            &[("label", &label)],
                                        ),
                                    );
                                    num_err = true;
                                    None
                                }
                            }
                        }
                    }
                    _ => None,
                };

                if num_err {
                    invalid_ids.push(field_id);
                    continue;
                }

                if let Some(n) = num_val {
                    if let Some(min) = field.min {
                        if n < min {
                            let min_str = min.to_string();
                            errs.insert(
                                name.clone(),
                                t_with_args(
                                    "validation-min-number",
                                    &loc_val,
                                    &[("label", &label), ("min", &min_str)],
                                ),
                            );
                            invalid_ids.push(field_id);
                            continue;
                        }
                    }

                    if let Some(max) = field.max {
                        if n > max {
                            let max_str = max.to_string();
                            errs.insert(
                                name.clone(),
                                t_with_args(
                                    "validation-max-number",
                                    &loc_val,
                                    &[("label", &label), ("max", &max_str)],
                                ),
                            );
                            invalid_ids.push(field_id);
                            continue;
                        }
                    }
                }
            }
        }

        validation_errors.set(errs);
        invalid_ids
    };

    rsx! {
        form {
            class: "grid grid-cols-12 gap-4 text-sm w-full",
            onsubmit: move |evt: FormEvent| {
                evt.prevent_default();
                let invalid_field_ids = validate_form();
                if invalid_field_ids.is_empty() {
                    let mut final_values = form_values.read().clone();
                    for field in fields.iter() {
                        if field.field_type == FieldType::Number {
                            if let Some(serde_json::Value::String(s)) = get_nested_value(&final_values, &field.name) {
                                let trimmed = s.trim();
                                if trimmed.is_empty() {
                                    set_nested_value(&mut final_values, &field.name, serde_json::Value::Null);
                                } else if let Ok(num) = trimmed.parse::<f64>() {
                                    set_nested_value(&mut final_values, &field.name, serde_json::json!(num));
                                }
                            }
                        }
                    }
                    let json_str = serde_json::to_string(&serde_json::Value::Object(final_values))
                        .unwrap_or_else(|_| "{}".to_string());
                    props.onsubmit.call(json_str);
                } else if let Some(first_err_id) = invalid_field_ids.first() {
                    let js = format!(
                        r#"
                        const el = document.getElementById('{first_err_id}');
                        if (el) {{
                            el.focus();
                            el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
                        }}
                        "#
                    );
                    let _ = dioxus::document::eval(&js);
                }
            },
            for field in fields.iter() {
                {
                    let name = field.name.clone();
                    let label = if field.label.is_empty() { name.clone() } else { field.label.clone() };
                    let field_type = field.field_type.clone();
                    let placeholder = field.placeholder.clone().unwrap_or_default();
                    let required = field.required;
                    let help_text = field.help_text.clone().unwrap_or_default();
                    let col_span_class = get_col_span_class(field);
                    let err_msg = validation_errors.read().get(&name).cloned();
                    let options = field.options.clone().unwrap_or_default();

                    let field_id = format!("field-{}", name);
                    let err_id = format!("err-{}", name);

                    if field_type == FieldType::Section {
                        let section_title = if label.is_empty() { name } else { label };
                        rsx! {
                            div { class: "col-span-12 border-b border-border/60 pb-2 mt-4 mb-1 flex flex-col gap-0.5",
                                h3 { class: "text-sm font-semibold text-foreground m-0 flex items-center gap-2",
                                    "{section_title}"
                                }
                                if !help_text.is_empty() {
                                    p { class: "text-xs text-muted-foreground m-0", "{help_text}" }
                                }
                            }
                        }
                    } else if field_type == FieldType::Multiselect {
                        let selected_arr = match get_nested_value(&form_values.read(), &name) {
                            Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<String>>(),
                            _ => Vec::new(),
                        };
                        let name_clone = name.clone();
                        rsx! {
                            fieldset { class: "flex flex-col gap-1.5 w-full {col_span_class} border-0 p-0 m-0",
                                legend { class: "text-xs font-bold text-muted-foreground uppercase tracking-wider mb-1.5 p-0",
                                    "{label}"
                                    if required { span { class: "text-red-500 ml-1", "*" } }
                                }
                                if !help_text.is_empty() {
                                    p { class: "text-[11px] text-muted-foreground/80 m-0 leading-tight -mt-1 mb-0.5",
                                        "{help_text}"
                                    }
                                }
                                div {
                                    id: field_id,
                                    class: if err_msg.is_some() {
                                        "flex flex-wrap gap-2 py-2 px-3 bg-background border border-red-500/80 rounded-lg min-h-[38px] items-center"
                                    } else {
                                        "flex flex-wrap gap-2 py-2 px-3 bg-background border border-border rounded-lg min-h-[38px] items-center"
                                    },
                                    for opt in options.iter() {
                                        {
                                            let opt_str = opt.clone();
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
                                                        set_nested_value(&mut form_values.write(), &name_c, json_arr);
                                                        validation_errors.write().remove(&name_c);
                                                    },
                                                    if is_selected { "✓ " }
                                                    "{opt_str}"
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(msg) = err_msg.as_ref() {
                                    p {
                                        id: "{err_id}",
                                        role: "alert",
                                        class: "text-xs font-medium text-red-500 mt-1 m-0 flex items-center gap-1",
                                        "{msg}"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "flex flex-col gap-1.5 w-full {col_span_class}",
                                if field_type != FieldType::Boolean {
                                    label {
                                        r#for: "{field_id}",
                                        class: "text-xs font-bold text-muted-foreground uppercase tracking-wider",
                                        "{label}"
                                        if required { span { class: "text-red-500 ml-1", "*" } }
                                    }
                                }
                                if !help_text.is_empty() {
                                    p { class: "text-[11px] text-muted-foreground/80 m-0 leading-tight -mt-0.5 mb-0.5",
                                        "{help_text}"
                                    }
                                }

                                match field_type {
                                    FieldType::Boolean => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_bool()).unwrap_or(false);
                                        let name_clone = name.clone();
                                        rsx! {
                                            Checkbox {
                                                checked: current_val,
                                                onchange: move |val| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::Bool(val));
                                                    validation_errors.write().remove(&name_clone);
                                                },
                                                label: label
                                            }
                                        }
                                    }
                                    FieldType::Number => {
                                        let current_val = match get_nested_value(&form_values.read(), &name) {
                                            Some(serde_json::Value::String(s)) => s.clone(),
                                            Some(serde_json::Value::Number(n)) => n.to_string(),
                                            _ => String::new(),
                                        };
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            crate::components::NumberInput {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                placeholder: placeholder,
                                                value: "{current_val}",
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Select => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        let select_opt_label = t("block-select-option", &loc);
                                        rsx! {
                                            crate::components::NativeSelect {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                value: "{current_val}",
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                },
                                                option { value: "", "{select_opt_label}" }
                                                for opt in options.iter() {
                                                    {
                                                        let opt_str = opt.clone();
                                                        rsx! {
                                                            option { value: "{opt_str}", "{opt_str}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Textarea => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            TextArea {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                placeholder: placeholder,
                                                value: current_val,
                                                rows: 3,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Date => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            DatePicker {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                value: current_val,
                                                onchange: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Email => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        let email_placeholder = if placeholder.is_empty() { "name@example.com".to_string() } else { placeholder };
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "email",
                                                placeholder: email_placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Password => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "password",
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Tel => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "tel",
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Url => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "url",
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Search => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "search",
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Time => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "time",
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Color => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                r#type: "color",
                                                placeholder: placeholder,
                                                value: current_val,
                                                class: "h-9 w-full cursor-pointer bg-background border border-border rounded-md p-1",
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Text => {
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                    FieldType::Unknown | FieldType::Section | FieldType::Multiselect => {
                                        log::warn!("DynamicForm: Unknown or unsupported field type '{:?}' for field '{}'. Defaulting to text input.", field_type, name);
                                        let current_val = get_nested_value(&form_values.read(), &name).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                        let name_clone = name.clone();
                                        let err_id_clone = err_id.clone();
                                        rsx! {
                                            Input {
                                                id: field_id,
                                                is_invalid: err_msg.is_some(),
                                                aria_describedby: err_msg.as_ref().map(|_| err_id_clone.clone()),
                                                placeholder: placeholder,
                                                value: current_val,
                                                oninput: move |evt: FormEvent| {
                                                    set_nested_value(&mut form_values.write(), &name_clone, serde_json::Value::String(evt.value()));
                                                    validation_errors.write().remove(&name_clone);
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(msg) = err_msg {
                                    p {
                                        id: "{err_id}",
                                        role: "alert",
                                        class: "text-xs font-medium text-red-500 mt-1 m-0 flex items-center gap-1",
                                        "{msg}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Actions (Save / Cancel)
            div { class: "col-span-12 flex gap-2 justify-end mt-4",
                if let Some(oncancel_cb) = props.oncancel {
                    Button {
                        variant: crate::components::ButtonVariant::Secondary,
                        r#type: "button",
                        disabled: props.is_submitting,
                        onclick: move |_| oncancel_cb.call(()),
                        "{t(\"common-cancel\", &loc)}"
                    }
                }
                Button {
                    r#type: "submit",
                    disabled: props.is_submitting,
                    if props.is_submitting {
                        LucideIcon { name: "loader-2", class: "animate-spin mr-2 h-4 w-4" }
                    }
                    "{t(\"common-save-btn\", &loc)}"
                }
            }
        }
    }
}
